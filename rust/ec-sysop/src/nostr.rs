use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use ec_data::{CampaignSettings, CampaignStore, GameConfig, HostedSeat, HostedSeatStatus};
use ec_gate::config::io::{config_path, load_config};
use ec_gate::invite::generate_invite_code;
use ec_gate::roster::io::load_roster;
use ec_nostr::hosted::invite_address_from_relay;
use nostr_sdk::ToBech32;

pub fn migrate_roster(dir: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let roster_path = dir.join("roster.kdl");
    let roster = load_roster(&roster_path)?;
    let expected_game_id = dir
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| format!("cannot derive game-id from {}", dir.display()))?;
    if roster.id != expected_game_id {
        return Err(format!(
            "roster game id '{}' does not match directory basename '{}'",
            roster.id, expected_game_id
        )
        .into());
    }
    let store = CampaignStore::open_default_in_dir(dir)?;
    let seats = roster
        .seats
        .into_iter()
        .map(|seat| HostedSeat {
            player_record_index_1_based: seat.player,
            invite_code: seat.code,
            status: match seat.status {
                ec_gate::roster::SeatStatus::Pending => HostedSeatStatus::Pending,
                ec_gate::roster::SeatStatus::Claimed => HostedSeatStatus::Claimed,
            },
            player_npub: seat.npub,
        })
        .collect::<Vec<_>>();
    store.replace_hosted_seats(&seats)?;

    let roster_name = roster.name.clone();
    let settings = if dir.join("config.kdl").exists() {
        let game_config = GameConfig::load_kdl(&dir.join("config.kdl"))?;
        CampaignSettings::from_legacy_game_config(expected_game_id, &game_config, None)
    } else {
        CampaignSettings::new(expected_game_id, &roster_name)
    };
    let settings = CampaignSettings {
        game_name: roster_name,
        reservations: settings.reservations,
        ..settings
    };
    store.save_campaign_settings(&settings)?;

    let legacy_path = dir.join("roster.kdl.legacy");
    fs::rename(&roster_path, &legacy_path)?;
    Ok(format!(
        "Migrated hosted roster into {} and archived {}",
        store.path().display(),
        legacy_path.display()
    ))
}

pub fn render_hosted_seats(dir: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let store = CampaignStore::open_default_in_dir(dir)?;
    let settings = store.load_campaign_settings()?;
    let seats = store.hosted_seats()?;

    let relay = load_config(&config_path()).ok().map(|cfg| cfg.relay);

    let mut out = String::new();
    out.push_str(&format!("Game: {}\n", settings.game_name));
    out.push_str(&format!("Dir:  {}\n", dir.display()));
    out.push('\n');

    for seat in &seats {
        let npub = seat.player_npub.as_deref().unwrap_or("");
        match seat.status {
            HostedSeatStatus::Pending => {
                out.push_str(&format!(
                    "Seat {}  [pending]\n",
                    seat.player_record_index_1_based
                ));
                match relay.as_deref() {
                    Some(relay_url) => {
                        match invite_address_from_relay(&seat.invite_code, relay_url) {
                            Ok(invite) => out.push_str(&format!("  ec-connect --join {invite}\n")),
                            Err(_) => {
                                out.push_str(&format!("  ec-connect --join {}\n", seat.invite_code))
                            }
                        }
                    }
                    None => out.push_str(&format!("  ec-connect --join {}\n", seat.invite_code)),
                }
            }
            HostedSeatStatus::Claimed => {
                out.push_str(&format!(
                    "Seat {}  [claimed]\n",
                    seat.player_record_index_1_based
                ));
                let display_npub = nostr_sdk::PublicKey::from_hex(npub)
                    .ok()
                    .and_then(|pk| pk.to_bech32().ok())
                    .unwrap_or_else(|| npub.to_string());
                out.push_str(&format!("  {display_npub}\n"));
            }
        }
        out.push('\n');
    }
    Ok(out)
}

pub fn reissue_hosted_seat(
    dir: &Path,
    player_record_index_1_based: usize,
) -> Result<String, Box<dyn std::error::Error>> {
    let store = CampaignStore::open_default_in_dir(dir)?;
    let invite_code = generate_unique_invite_code(&store.hosted_seats()?);
    let Some(seat) = store.reissue_hosted_seat(player_record_index_1_based, &invite_code)? else {
        return Err(format!(
            "player {} not found in hosted seats",
            player_record_index_1_based
        )
        .into());
    };
    Ok(format!(
        "Reissued invite for seat {}: {}",
        seat.player_record_index_1_based, seat.invite_code
    ))
}

pub(crate) fn build_pending_seats(player_count: usize) -> Vec<HostedSeat> {
    let mut seen_codes = BTreeSet::new();
    (1..=player_count)
        .map(|player| HostedSeat {
            player_record_index_1_based: player,
            invite_code: generate_unique_invite_code_from_set(&mut seen_codes),
            status: HostedSeatStatus::Pending,
            player_npub: None,
        })
        .collect()
}

fn generate_unique_invite_code(existing: &[HostedSeat]) -> String {
    let mut seen_codes = existing
        .iter()
        .map(|seat| seat.invite_code.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    generate_unique_invite_code_from_set(&mut seen_codes)
}

fn generate_unique_invite_code_from_set(seen_codes: &mut BTreeSet<String>) -> String {
    let mut existing = seen_codes.iter().cloned().collect::<HashSet<_>>();
    let invite_code = generate_invite_code(&existing);
    existing.insert(invite_code.clone());
    seen_codes.insert(invite_code.clone());
    invite_code
}

pub fn parse_required_dir_flag(args: &[String]) -> Result<PathBuf, Box<dyn std::error::Error>> {
    parse_path_flag(args, "--dir")?.ok_or_else(|| "missing value for --dir".into())
}

pub fn parse_path_flag(
    args: &[String],
    flag: &str,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    let mut value = None;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == flag {
            i += 1;
            let Some(next) = args.get(i) else {
                return Err(format!("missing value for {flag}").into());
            };
            value = Some(PathBuf::from(next));
        } else if let Some(next) = arg.strip_prefix(&format!("{flag}=")) {
            value = Some(PathBuf::from(next));
        } else {
            return Err(format!("unexpected argument: {arg}").into());
        }
        i += 1;
    }
    Ok(value)
}

pub fn parse_dir_and_player_flags(
    args: &[String],
) -> Result<(PathBuf, usize), Box<dyn std::error::Error>> {
    let mut dir = None;
    let mut player = None;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--dir" {
            i += 1;
            let Some(next) = args.get(i) else {
                return Err("missing value for --dir".into());
            };
            dir = Some(PathBuf::from(next));
        } else if let Some(next) = arg.strip_prefix("--dir=") {
            dir = Some(PathBuf::from(next));
        } else if arg == "--player" {
            i += 1;
            let Some(next) = args.get(i) else {
                return Err("missing value for --player".into());
            };
            player = Some(next.parse::<usize>()?);
        } else if let Some(next) = arg.strip_prefix("--player=") {
            player = Some(next.parse::<usize>()?);
        } else {
            return Err(format!("unexpected argument: {arg}").into());
        }
        i += 1;
    }
    Ok((
        dir.ok_or_else(|| "missing value for --dir")?,
        player.ok_or_else(|| "missing value for --player")?,
    ))
}
