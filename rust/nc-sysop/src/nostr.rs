use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use nc_data::{CampaignSettings, CampaignStore, GameConfig, HostedSeat, HostedSeatStatus};
use nc_gate::config::io::{config_path, load_config};
use nc_gate::invite::generate_invite_code;
use nc_gate::roster::io::load_roster;
use nc_nostr::hosted::invite_address_from_relay;
use nostr_sdk::PublicKey;

pub struct ReissuedHostedSeat {
    pub player_record_index_1_based: usize,
    pub invite_code: String,
}

pub struct ClaimedHostedSeat {
    pub player_record_index_1_based: usize,
    pub player_pubkey_hex: String,
}

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
                nc_gate::roster::SeatStatus::Pending => HostedSeatStatus::Pending,
                nc_gate::roster::SeatStatus::Claimed => HostedSeatStatus::Claimed,
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
                            Ok(invite) => out.push_str(&format!("  nc-connect --join {invite}\n")),
                            Err(_) => {
                                out.push_str(&format!("  nc-connect --join {}\n", seat.invite_code))
                            }
                        }
                    }
                    None => out.push_str(&format!("  nc-connect --join {}\n", seat.invite_code)),
                }
            }
            HostedSeatStatus::Claimed => {
                out.push_str(&format!(
                    "Seat {}  [claimed]\n",
                    seat.player_record_index_1_based
                ));
                out.push_str(&format!("  {npub}\n"));
            }
        }
        out.push('\n');
    }
    Ok(out)
}

pub fn reissue_hosted_seat_record(
    dir: &Path,
    player_record_index_1_based: usize,
) -> Result<ReissuedHostedSeat, Box<dyn std::error::Error>> {
    let store = CampaignStore::open_default_in_dir(dir)?;
    let invite_code = generate_unique_invite_code(&store.hosted_seats()?);
    let Some(seat) = store.reissue_hosted_seat(player_record_index_1_based, &invite_code)? else {
        return Err(format!(
            "player {} not found in hosted seats",
            player_record_index_1_based
        )
        .into());
    };
    Ok(ReissuedHostedSeat {
        player_record_index_1_based: seat.player_record_index_1_based,
        invite_code: seat.invite_code,
    })
}

pub fn claim_hosted_seat_record(
    dir: &Path,
    player_record_index_1_based: usize,
    player_npub: &str,
) -> Result<ClaimedHostedSeat, Box<dyn std::error::Error>> {
    let normalized = PublicKey::parse(player_npub.trim())
        .map_err(|err| format!("invalid npub/pubkey: {err}"))?
        .to_hex();
    let store = CampaignStore::open_default_in_dir(dir)?;
    let Some(seat) =
        store.claim_hosted_seat_for_player(player_record_index_1_based, &normalized)?
    else {
        return Err(format!(
            "player {} not found in hosted seats",
            player_record_index_1_based
        )
        .into());
    };
    Ok(ClaimedHostedSeat {
        player_record_index_1_based: seat.player_record_index_1_based,
        player_pubkey_hex: normalized,
    })
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
