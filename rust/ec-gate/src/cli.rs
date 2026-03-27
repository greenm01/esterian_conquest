use std::env;

use nostr_sdk::{Keys, ToBech32};

use crate::config::io::{config_path, load_config};
use crate::identity::io::{identity_path, load_identity, save_identity};
use crate::serve::run_serve;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let cmd = args.next();

    match cmd.as_deref() {
        None | Some("--help" | "-h" | "help") => {
            print_usage();
            Ok(())
        }
        Some("--version") => {
            println!("ec-gate {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("init") => cmd_init(args.collect()),
        Some("serve") => cmd_serve(args.collect()),
        Some(other) => Err(format!("unknown command: {other}").into()),
    }
}

fn cmd_init(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    // Optional: --identity <path> override
    let path = parse_identity_flag(&args).unwrap_or_else(identity_path);

    if path.exists() {
        // Safe to re-run: just report the existing identity.
        let identity = load_identity(&path)?;
        let npub = identity
            .keys
            .public_key()
            .to_bech32()
            .map_err(|err| format!("npub bech32: {err}"))?;
        println!("Daemon identity already exists at: {}", path.display());
        println!("Public key (npub): {npub}");
        println!("Created: {}", identity.created);
        return Ok(());
    }

    let keys = Keys::generate();
    let created = now_iso8601();
    save_identity(&path, &keys, &created)?;

    let npub = keys
        .public_key()
        .to_bech32()
        .map_err(|err| format!("npub bech32: {err}"))?;
    println!("Daemon identity created at: {}", path.display());
    println!("Public key (npub): {npub}");
    Ok(())
}

fn cmd_serve(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let identity_path = parse_identity_flag(&args).unwrap_or_else(identity_path);
    let config_path = parse_config_flag(&args).unwrap_or_else(config_path);

    let identity = load_identity(&identity_path)
        .map_err(|e| format!("cannot load identity (run `ec-gate init` first): {e}"))?;
    let config = load_config(&config_path)
        .map_err(|e| format!("cannot load config at {}: {e}", config_path.display()))?;

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(run_serve(&config, &identity.keys))
}

/// Parse an optional `--identity <path>` argument from the arg list.
fn parse_identity_flag(args: &[String]) -> Option<std::path::PathBuf> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--identity" {
            return iter.next().map(std::path::PathBuf::from);
        }
        if let Some(val) = arg.strip_prefix("--identity=") {
            return Some(std::path::PathBuf::from(val));
        }
    }
    None
}

/// Parse an optional `--config <path>` argument from the arg list.
fn parse_config_flag(args: &[String]) -> Option<std::path::PathBuf> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--config" {
            return iter.next().map(std::path::PathBuf::from);
        }
        if let Some(val) = arg.strip_prefix("--config=") {
            return Some(std::path::PathBuf::from(val));
        }
    }
    None
}

fn now_iso8601() -> String {
    // No chrono dep yet — format the Unix timestamp manually.
    // This produces a UTC timestamp in ISO-8601 format accurate to the second.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_iso8601(secs)
}

/// Format a Unix timestamp (seconds since epoch) as `YYYY-MM-DDTHH:MM:SSZ`.
pub fn format_iso8601(secs: u64) -> String {
    // Gregorian calendar arithmetic — no external dep needed for a simple UTC timestamp.
    let s = secs % 60;
    let total_minutes = secs / 60;
    let m = total_minutes % 60;
    let total_hours = total_minutes / 60;
    let h = total_hours % 24;
    let total_days = total_hours / 24;

    // Days since 1970-01-01 → Gregorian date.
    let (year, month, day) = days_to_ymd(total_days);

    format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}Z")
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    // Algorithm: iterate years, accounting for leap years.
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

fn print_usage() {
    println!(
        "\
ec-gate — Esterian Conquest Nostr auth daemon

Usage:
  ec-gate init                         Generate daemon identity
  ec-gate init --identity <path>       Write identity to a specific path
  ec-gate serve                        Start the auth daemon
  ec-gate serve --config <path>        Use a specific config file
  ec-gate serve --identity <path>      Use a specific identity file

Options:
  --version                            Print version
  --help                               Print this help"
    );
}
