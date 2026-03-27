pub mod config;
pub mod identity;
pub mod invite;
pub mod roster;
pub mod serve;

use std::path::PathBuf;

use nostr_sdk::{Keys, ToBech32};

pub struct InitializedGateIdentity {
    pub path: PathBuf,
    pub npub: String,
    pub created: String,
    pub already_exists: bool,
}

pub fn init_identity_at(
    path: Option<PathBuf>,
) -> Result<InitializedGateIdentity, Box<dyn std::error::Error>> {
    let path = path.unwrap_or_else(identity::identity_path);
    if path.exists() {
        let identity = identity::load_identity(&path)?;
        let npub = identity
            .keys
            .public_key()
            .to_bech32()
            .map_err(|err| format!("npub bech32: {err}"))?;
        return Ok(InitializedGateIdentity {
            path,
            npub,
            created: identity.created,
            already_exists: true,
        });
    }

    let keys = Keys::generate();
    let created = now_iso8601();
    identity::save_identity(&path, &keys, &created)?;
    let npub = keys
        .public_key()
        .to_bech32()
        .map_err(|err| format!("npub bech32: {err}"))?;
    Ok(InitializedGateIdentity {
        path,
        npub,
        created,
        already_exists: false,
    })
}

pub fn serve_from_paths(
    config_path: Option<PathBuf>,
    identity_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let identity_path = identity_path.unwrap_or_else(identity::identity_path);
    let config_path = config_path.unwrap_or_else(config::config_path);
    let identity = identity::load_identity(&identity_path)
        .map_err(|e| format!("cannot load identity (run `ec-sysop nostr init` first): {e}"))?;
    let config = config::load_config(&config_path)
        .map_err(|e| format!("cannot load config at {}: {e}", config_path.display()))?;

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(serve::run_serve(&config, &identity.keys))?;
    Ok(())
}

fn now_iso8601() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_iso8601(secs)
}

pub fn format_iso8601(secs: u64) -> String {
    let s = secs % 60;
    let total_minutes = secs / 60;
    let m = total_minutes % 60;
    let total_hours = total_minutes / 60;
    let h = total_hours % 24;
    let total_days = total_hours / 24;
    let (year, month, day) = days_to_ymd(total_days);
    format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}Z")
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    loop {
        let in_year = if is_leap(year) { 366 } else { 365 };
        if days < in_year {
            break;
        }
        days -= in_year;
        year += 1;
    }
    let leap = is_leap(year);
    let months: [u64; 12] = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u64;
    for days_in_month in months {
        if days < days_in_month {
            break;
        }
        days -= days_in_month;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}
