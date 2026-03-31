use std::time::Duration;

use ec_nostr::hash::sha256_hex;
use ec_nostr::hosted::PublishedGameDefinition;
use nostr_sdk::{Client, Event, Filter, Keys, Kind};

use crate::connect::resolve::{ResolvedTarget, parse_invite_code};

pub const GAME_DISCOVERY_TIMEOUT_SECS: u64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InviteResolution {
    FirstJoin,
    SameIdentityRejoin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredGame {
    pub gate_npub: String,
    pub game_id: String,
    pub game_name: String,
    pub ssh_host: String,
    pub ssh_port: u16,
    pub seat: u32,
    pub resolution: InviteResolution,
}

pub async fn discover_game_for_invite(
    player_keys: &Keys,
    target: &ResolvedTarget,
    invite_code: &str,
) -> Result<DiscoveredGame, String> {
    let client = Client::new(player_keys.clone());
    client
        .add_relay(&target.relay_url)
        .await
        .map_err(|e| relay_transport_error(target, "could not add the relay", &e.to_string()))?;
    client.connect().await;

    let timeout = Duration::from_secs(GAME_DISCOVERY_TIMEOUT_SECS);
    let events = client
        .fetch_events(Filter::new().kinds([Kind::Custom(30500)]), timeout)
        .await
        .map_err(|e| {
            relay_transport_error(
                target,
                "could not fetch hosted game definitions from the relay",
                &e.to_string(),
            )
        })?;

    client.disconnect().await;

    let player_pubkey_hex = player_keys.public_key().to_hex();
    select_discovered_game_from_events(
        events.iter(),
        target,
        invite_code,
        Some(player_pubkey_hex.as_str()),
    )
}

pub fn select_discovered_game_from_events<'a>(
    events: impl IntoIterator<Item = &'a Event>,
    target: &ResolvedTarget,
    invite_code: &str,
    player_pubkey_hex: Option<&str>,
) -> Result<DiscoveredGame, String> {
    let invite_words = parse_invite_code(invite_code)
        .map(|parsed| parsed.words)
        .unwrap_or_else(|_| invite_code.trim().to_ascii_lowercase());
    let invite_hash = sha256_hex(invite_words.as_bytes());
    let target_host = target.server_host.to_ascii_lowercase();

    let events_collected: Vec<&Event> = events.into_iter().collect();

    // Find all games that have a matching pending slot, ignoring host/port initially.
    let mut hash_matches = Vec::new();

    for event in &events_collected {
        let Some(game) = parse_game_definition(event) else {
            continue;
        };
        let pending_seat = game
            .slots
            .iter()
            .find(|slot| slot.status == "pending" && slot.invite_code_hash == invite_hash)
            .map(|slot| slot.seat);
        if let Some(seat) = pending_seat {
            hash_matches.push((game, seat, InviteResolution::FirstJoin));
            continue;
        }

        let claimed_same_identity_seat = player_pubkey_hex
            .and_then(|player| {
                game.slots.iter().find(|slot| {
                    slot.status == "claimed"
                        && slot.invite_code_hash == invite_hash
                        && slot.player_npub.as_deref() == Some(player)
                })
            })
            .map(|slot| slot.seat);
        if let Some(seat) = claimed_same_identity_seat {
            hash_matches.push((game, seat, InviteResolution::SameIdentityRejoin));
        }
    }

    if hash_matches.is_empty() {
        // Check whether this code was already claimed by another player.
        let claimed_match_count = events_collected
            .iter()
            .filter_map(|event| parse_game_definition(event))
            .flat_map(|game| game.slots.into_iter())
            .filter(|slot| slot.status == "claimed" && slot.invite_code_hash == invite_hash)
            .count();
        if claimed_match_count > 0 {
            return Err(
                "this invite code has already been claimed by another player; ask your sysop to reissue the seat"
                    .to_string(),
            );
        }
        tracing::debug!(
            relay = %target.relay_url,
            invite_hash = %short_hash(&invite_hash),
            fetched_definitions = events_collected.iter().filter(|event| parse_game_definition(event).is_some()).count(),
            claimed_matches = claimed_match_count,
            "relay fetch succeeded but no pending invite hash matched"
        );
        return Err(no_game_found_error(target, events_collected.len()));
    }

    // If exactly one game matches the invite hash, accept it even if the host/port
    // don't match (this fixes localhost/LAN testing and NAT port-forwarding mismatches).
    if hash_matches.len() == 1 {
        let (game, seat, resolution) = hash_matches.remove(0);
        return Ok(DiscoveredGame {
            gate_npub: game.gate_npub,
            game_id: game.game_id,
            game_name: game.game_name,
            ssh_host: game.ssh_host,
            ssh_port: game.ssh_port,
            seat,
            resolution,
        });
    }

    // Multiple games matched the hash (extremely rare collision or misconfiguration).
    // Attempt to disambiguate by checking which one matches the target host and port.
    let exact_matches: Vec<_> = hash_matches
        .into_iter()
        .filter(|(game, _, _)| {
            game.ssh_host.to_ascii_lowercase() == target_host && game.ssh_port == target.server_port
        })
        .collect();

    match exact_matches.len() {
        1 => {
            let (game, seat, resolution) = exact_matches.into_iter().next().unwrap();
            Ok(DiscoveredGame {
                gate_npub: game.gate_npub,
                game_id: game.game_id,
                game_name: game.game_name,
                ssh_host: game.ssh_host,
                ssh_port: game.ssh_port,
                seat,
                resolution,
            })
        }
        0 => Err("multiple hosted games matched this invite code on the relay, but none matched this server address; open the game from the picker instead".to_string()),
        _ => Err(
            "multiple hosted games matched this invite code on the relay; open the game from the picker and choose the right one"
                .to_string(),
        ),
    }
}

fn no_game_found_error(target: &ResolvedTarget, event_count: usize) -> String {
    if is_local_dev_relay(&target.relay_url) {
        format!(
            "could not find this hosted game on the local relay at {}; make sure your local relay and ec-sysop nostr serve are running for this game, then try again",
            target.relay_url
        )
    } else {
        format!(
            "the relay was reachable, but no pending hosted seat matched this invite ({} public game definition{} checked); check the invite code and relay, and if your sysop recently created or reissued this seat, ask them to republish hosted metadata",
            event_count,
            if event_count == 1 { "" } else { "s" },
        )
    }
}

fn relay_transport_error(target: &ResolvedTarget, context: &str, detail: &str) -> String {
    if is_local_dev_relay(&target.relay_url) {
        format!(
            "{context} at {}: make sure your local relay and ec-sysop nostr serve are running for this game ({detail})",
            target.relay_url
        )
    } else {
        format!("{context}: {detail}")
    }
}

fn is_local_dev_relay(relay_url: &str) -> bool {
    relay_url.starts_with("ws://localhost")
        || relay_url.starts_with("ws://127.")
        || relay_url.starts_with("ws://[::1]")
}

pub fn parse_game_definition(event: &Event) -> Option<PublishedGameDefinition> {
    ec_nostr::hosted::parse_game_definition(event)
}

fn short_hash(value: &str) -> &str {
    &value[..value.len().min(12)]
}
