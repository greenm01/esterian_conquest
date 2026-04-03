use std::path::{Path, PathBuf};
use std::time::Duration;

use nc_data::{CampaignStore, HostedSeat, HostedSeatStatus};
use nc_gate::publish::{PublishedGameDefinitionReceipt, publish_game_definition_for_dir};
use nc_nostr::hash::sha256_hex;
use nc_nostr::hosted::{PublishedGameDefinition, parse_game_definition};
use nostr_sdk::{Client, Filter, Kind};

use crate::nostr;

const VERIFY_TIMEOUT_SECS: u64 = 10;

enum RepublishOutcome {
    Published(PublishedGameDefinitionReceipt),
    Skipped(String),
}

pub fn publish_hosted_game(
    dir: &Path,
    config_path: Option<PathBuf>,
    identity_path: Option<PathBuf>,
) -> Result<String, Box<dyn std::error::Error>> {
    let (config_path, identity_path) = required_publish_paths(config_path, identity_path)?;
    let receipt = publish_game_definition_for_dir(dir, Some(config_path), Some(identity_path))?;
    Ok(format!(
        "Republished 30500 for {} to {} ({})",
        receipt.game_id,
        receipt.relay_url,
        short_hash(&receipt.event_id),
    ))
}

pub fn verify_hosted_game(
    dir: &Path,
    config_path: Option<PathBuf>,
    identity_path: Option<PathBuf>,
) -> Result<String, Box<dyn std::error::Error>> {
    let store = CampaignStore::open_default_in_dir(dir)?;
    let settings = store.load_campaign_settings()?;
    let seats = store.hosted_seats()?;
    let (config_path, identity_path) = required_publish_paths(config_path, identity_path)?;
    let (relay_url, published) =
        fetch_published_game_definition(&settings.slug, &config_path, &identity_path)?;

    let mut issues = Vec::new();
    if published.game_name != settings.game_name {
        issues.push(format!(
            "game name mismatch: local='{}' relay='{}'",
            settings.game_name, published.game_name
        ));
    }

    let config = nc_gate::config::load_config(&config_path)?;
    if published.ssh_host != config.ssh_host || published.ssh_port != config.ssh_port {
        issues.push(format!(
            "ssh target mismatch: local='{}:{}' relay='{}:{}'",
            config.ssh_host, config.ssh_port, published.ssh_host, published.ssh_port
        ));
    }

    if published.slots.len() != seats.len() {
        issues.push(format!(
            "seat count mismatch: local={} relay={}",
            seats.len(),
            published.slots.len()
        ));
    }

    issues.extend(compare_slots(&seats, &published));

    let mut report = String::new();
    report.push_str(&format!("Game:  {}\n", settings.game_name));
    report.push_str(&format!("Slug:  {}\n", settings.slug));
    report.push_str(&format!("Dir:   {}\n", dir.display()));
    report.push_str(&format!("Relay: {}\n", relay_url));

    if issues.is_empty() {
        report.push_str("Status: OK\n");
        return Ok(report);
    }

    report.push_str("Status: MISMATCH\n");
    for issue in issues {
        report.push_str(&format!("{issue}\n"));
    }
    Err(report.trim_end().to_string().into())
}

pub fn reissue_hosted_seat_with_publish(
    dir: &Path,
    player_record_index_1_based: usize,
    nuke_seat: bool,
    config_path: Option<PathBuf>,
    identity_path: Option<PathBuf>,
) -> Result<String, Box<dyn std::error::Error>> {
    let seat = nostr::reissue_hosted_seat_record(dir, player_record_index_1_based, nuke_seat)?;
    let base = if seat.runtime_seat_nuked {
        format!(
            "Reissued invite and nuked seat {}: {}",
            seat.player_record_index_1_based, seat.invite_code
        )
    } else {
        format!(
            "Reissued invite for seat {}: {}",
            seat.player_record_index_1_based, seat.invite_code
        )
    };
    format_mutation_with_republish(
        dir,
        base,
        config_path,
        identity_path,
        Some(format!(
            "Local seat change succeeded; if the relay publish is still stale, run `nc-sysop nostr publish --dir {}` after fixing the relay/config.",
            dir.display()
        )),
    )
}

pub fn claim_hosted_seat_with_publish(
    dir: &Path,
    player_record_index_1_based: usize,
    player_npub: &str,
    config_path: Option<PathBuf>,
    identity_path: Option<PathBuf>,
) -> Result<String, Box<dyn std::error::Error>> {
    let claimed = nostr::claim_hosted_seat_record(dir, player_record_index_1_based, player_npub)?;
    let base = format!(
        "Claimed seat {} for {}",
        claimed.player_record_index_1_based, claimed.player_pubkey_hex
    );
    format_mutation_with_republish(
        dir,
        base,
        config_path,
        identity_path,
        Some(format!(
            "Local seat change succeeded; if the relay publish is still stale, run `nc-sysop nostr publish --dir {}` after fixing the relay/config.",
            dir.display()
        )),
    )
}

fn format_mutation_with_republish(
    dir: &Path,
    base: String,
    config_path: Option<PathBuf>,
    identity_path: Option<PathBuf>,
    failure_note: Option<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    match maybe_republish_hosted_game(dir, config_path, identity_path) {
        Ok(RepublishOutcome::Published(receipt)) => Ok(format!(
            "{base}\nRepublished 30500 for {} to {} ({})",
            receipt.game_id,
            receipt.relay_url,
            short_hash(&receipt.event_id),
        )),
        Ok(RepublishOutcome::Skipped(reason)) => Ok(format!(
            "{base}\nWarning: {reason}. Run `nc-sysop nostr publish --dir {}` to refresh relay metadata.",
            dir.display()
        )),
        Err(err) => {
            let note = failure_note.unwrap_or_default();
            Err(format!("{base}\nRelay publish failed: {err}\n{note}").into())
        }
    }
}

fn maybe_republish_hosted_game(
    dir: &Path,
    config_path: Option<PathBuf>,
    identity_path: Option<PathBuf>,
) -> Result<RepublishOutcome, Box<dyn std::error::Error>> {
    let Some((config_path, identity_path)) = optional_publish_paths(config_path, identity_path)?
    else {
        return Ok(RepublishOutcome::Skipped(
            "gate config or daemon identity was not found locally".to_string(),
        ));
    };

    let receipt = publish_game_definition_for_dir(dir, Some(config_path), Some(identity_path))?;
    Ok(RepublishOutcome::Published(receipt))
}

fn required_publish_paths(
    config_path: Option<PathBuf>,
    identity_path: Option<PathBuf>,
) -> Result<(PathBuf, PathBuf), Box<dyn std::error::Error>> {
    match optional_publish_paths(config_path, identity_path)? {
        Some(paths) => Ok(paths),
        None => Err("gate config or daemon identity was not found locally".into()),
    }
}

fn optional_publish_paths(
    config_path: Option<PathBuf>,
    identity_path: Option<PathBuf>,
) -> Result<Option<(PathBuf, PathBuf)>, Box<dyn std::error::Error>> {
    let config_path = resolve_existing_path(config_path, nc_gate::config::config_path)?;
    let identity_path = resolve_existing_path(identity_path, nc_gate::identity::identity_path)?;
    match (config_path, identity_path) {
        (Some(config_path), Some(identity_path)) => Ok(Some((config_path, identity_path))),
        _ => Ok(None),
    }
}

fn resolve_existing_path(
    explicit: Option<PathBuf>,
    default_fn: impl FnOnce() -> PathBuf,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    if let Some(path) = explicit {
        if !path.exists() {
            return Err(format!("path does not exist: {}", path.display()).into());
        }
        return Ok(Some(path));
    }
    let path = default_fn();
    if path.exists() {
        Ok(Some(path))
    } else {
        Ok(None)
    }
}

fn fetch_published_game_definition(
    game_id: &str,
    config_path: &Path,
    identity_path: &Path,
) -> Result<(String, PublishedGameDefinition), Box<dyn std::error::Error>> {
    let config = nc_gate::config::load_config(config_path)?;
    let identity = nc_gate::identity::load_identity(identity_path)?;
    let relay_url = config.relay.clone();
    let game_id = game_id.to_string();
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move {
        let client = Client::new(identity.keys.clone());
        let filter = Filter::new()
            .author(identity.keys.public_key())
            .kind(Kind::Custom(30500))
            .identifier(game_id.clone());
        let events = client
            .fetch_events_from(
                [relay_url.as_str()],
                filter,
                Duration::from_secs(VERIFY_TIMEOUT_SECS),
            )
            .await
            .map_err(|err| format!("could not fetch 30500 from {}: {err}", relay_url))?;
        let published = events
            .iter()
            .find_map(parse_game_definition)
            .ok_or_else(|| {
                format!(
                    "no published 30500 was found on {} for game '{}'",
                    relay_url, game_id
                )
            })?;
        Ok((relay_url, published))
    })
}

fn compare_slots(local: &[HostedSeat], published: &PublishedGameDefinition) -> Vec<String> {
    let mut issues = Vec::new();
    for seat in local {
        let Some(remote) = published
            .slots
            .iter()
            .find(|slot| slot.seat == seat.player_record_index_1_based as u32)
        else {
            issues.push(format!(
                "seat {} missing from relay publication",
                seat.player_record_index_1_based
            ));
            continue;
        };

        let local_status = match seat.status {
            HostedSeatStatus::Pending => "pending",
            HostedSeatStatus::Claimed => "claimed",
        };
        if remote.status != local_status {
            issues.push(format!(
                "seat {} status mismatch: local={} relay={}",
                seat.player_record_index_1_based, local_status, remote.status
            ));
        }

        let local_hash = sha256_hex(seat.invite_code.to_ascii_lowercase().as_bytes());
        if remote.invite_code_hash != local_hash {
            issues.push(format!(
                "seat {} invite hash mismatch: local={} relay={}",
                seat.player_record_index_1_based,
                short_hash(&local_hash),
                short_hash(&remote.invite_code_hash),
            ));
        }

        let local_npub = seat.player_npub.as_deref().unwrap_or("");
        let remote_npub = remote.player_npub.as_deref().unwrap_or("");
        if local_npub != remote_npub {
            issues.push(format!(
                "seat {} player mismatch: local={} relay={}",
                seat.player_record_index_1_based,
                short_or_empty(local_npub),
                short_or_empty(remote_npub),
            ));
        }
    }
    issues
}

fn short_hash(value: &str) -> &str {
    &value[..value.len().min(12)]
}

fn short_or_empty(value: &str) -> String {
    if value.is_empty() {
        return "<empty>".to_string();
    }
    short_hash(value).to_string()
}
