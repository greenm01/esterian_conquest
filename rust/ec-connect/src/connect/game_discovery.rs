use std::time::Duration;

use ec_nostr::hash::sha256_hex;
use nostr_sdk::{Client, Event, Filter, Keys, Kind, ToBech32};

use crate::connect::resolve::{ResolvedTarget, parse_invite_code};

pub const GAME_DISCOVERY_TIMEOUT_SECS: u64 = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredGame {
    pub gate_npub: String,
    pub game_id: String,
    pub game_name: String,
    pub seat: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedGameDefinition {
    pub gate_npub: String,
    pub game_id: String,
    pub game_name: String,
    pub ssh_host: String,
    pub ssh_port: u16,
    pub slots: Vec<PublishedSeatSlot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedSeatSlot {
    pub seat: u32,
    pub invite_code_hash: String,
    pub player_npub: Option<String>,
    pub status: String,
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
        .map_err(|e| format!("add relay: {e}"))?;
    client.connect().await;

    let timeout = Duration::from_secs(GAME_DISCOVERY_TIMEOUT_SECS);
    let events = client
        .fetch_events(Filter::new().kinds([Kind::Custom(30500)]), timeout)
        .await
        .map_err(|e| format!("fetch game definitions: {e}"))?;

    client.disconnect().await;

    select_discovered_game_from_events(events.iter(), target, invite_code)
}

pub fn select_discovered_game_from_events<'a>(
    events: impl IntoIterator<Item = &'a Event>,
    target: &ResolvedTarget,
    invite_code: &str,
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
        if !game
            .slots
            .iter()
            .any(|slot| slot.status == "pending" && slot.invite_code_hash == invite_hash)
        {
            continue;
        }
        
        let seat = game
            .slots
            .iter()
            .find(|slot| slot.status == "pending" && slot.invite_code_hash == invite_hash)
            .map(|slot| slot.seat)
            .unwrap_or(0);
            
        hash_matches.push((game, seat));
    }

    if hash_matches.is_empty() {
        // Check whether this code was already claimed — gives a more useful message
        // than the generic "not found" when a player retries after a failed first attempt.
        let already_claimed = events_collected.iter().any(|event| {
            parse_game_definition(event).is_some_and(|game| {
                game.slots
                    .iter()
                    .any(|slot| slot.status == "claimed" && slot.invite_code_hash == invite_hash)
            })
        });
        if already_claimed {
            return Err(
                "this invite code has already been claimed — your seat is reserved; \
                 connect with: ec-connect <server>"
                    .to_string(),
            );
        }
        return Err(
            "could not discover this hosted game on the relay; supply --gate <npub>".to_string(),
        );
    }

    // If exactly one game matches the invite hash, accept it even if the host/port
    // don't match (this fixes localhost/LAN testing and NAT port-forwarding mismatches).
    if hash_matches.len() == 1 {
        let (game, seat) = hash_matches.remove(0);
        return Ok(DiscoveredGame {
            gate_npub: game.gate_npub,
            game_id: game.game_id,
            game_name: game.game_name,
            seat,
        });
    }

    // Multiple games matched the hash (extremely rare collision or misconfiguration).
    // Attempt to disambiguate by checking which one matches the target host and port.
    let exact_matches: Vec<_> = hash_matches
        .into_iter()
        .filter(|(game, _)| {
            game.ssh_host.to_ascii_lowercase() == target_host && game.ssh_port == target.server_port
        })
        .collect();

    match exact_matches.len() {
        1 => {
            let (game, seat) = exact_matches.into_iter().next().unwrap();
            Ok(DiscoveredGame {
                gate_npub: game.gate_npub,
                game_id: game.game_id,
                game_name: game.game_name,
                seat,
            })
        }
        0 => Err(
            "multiple hosted games matched this invite hash, but none matched the server host/port; supply --gate <npub>"
                .to_string(),
        ),
        _ => Err(
            "multiple hosted games matched this invite exactly on the relay; supply --gate <npub>"
                .to_string(),
        ),
    }
}

pub fn parse_game_definition(event: &Event) -> Option<PublishedGameDefinition> {
    let gate_npub = event.pubkey.to_bech32().ok()?;
    let mut game_id = None;
    let mut game_name = None;
    let mut ssh_host = None;
    let mut ssh_port = None;
    let mut slots = Vec::new();

    for tag in event.tags.iter() {
        let values = tag.clone().to_vec();
        let Some(kind) = values.first().map(String::as_str) else {
            continue;
        };
        match kind {
            "d" if values.len() >= 2 => game_id = Some(values[1].clone()),
            "name" if values.len() >= 2 => game_name = Some(values[1].clone()),
            "ssh-host" if values.len() >= 2 => ssh_host = Some(values[1].clone()),
            "ssh-port" if values.len() >= 2 => {
                ssh_port = values[1].parse::<u16>().ok();
            }
            "slot" if values.len() >= 5 => slots.push(PublishedSeatSlot {
                seat: values[1].parse::<u32>().ok()?,
                invite_code_hash: values[2].clone(),
                player_npub: Some(values[3].clone()).filter(|value| !value.trim().is_empty()),
                status: values[4].clone(),
            }),
            _ => {}
        }
    }

    Some(PublishedGameDefinition {
        gate_npub,
        game_id: game_id?,
        game_name: game_name?,
        ssh_host: ssh_host?,
        ssh_port: ssh_port?,
        slots,
    })
}

