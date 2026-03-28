//! Wallet file I/O: load, save, parse, render.
//!
//! The wallet file is a binary envelope (see `crypto` module). The decrypted
//! contents are a KDL document:
//!
//! ```kdl
//! wallet active="0"
//! identity nsec="nsec1..." type="local" created="2026-03-26T12:00:00Z" alias="Desk Key"
//! identity nsec="nsec1..." type="imported" created="2026-03-28T09:35:00Z"
//! ```
//!
//! `load_wallet` prompts for a password externally; this module takes the
//! password as a string so it stays testable without stdin interaction.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use kdl::{KdlDocument, KdlNode};

use super::crypto::{decrypt_wallet, encrypt_wallet};
use super::{Identity, IdentityType, Wallet};

// ---------------------------------------------------------------------------
// Path resolution
// ---------------------------------------------------------------------------

/// Return the platform-appropriate wallet file path:
///   `~/.local/share/ec/wallet.kdl` (Linux/macOS XDG)
///   `%APPDATA%\ec\wallet.kdl` (Windows)
pub fn wallet_path() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .map(|h| h.join(".local").join("share"))
            .unwrap_or_else(|| PathBuf::from("."))
    });
    base.join("ec").join("wallet.kdl")
}

// ---------------------------------------------------------------------------
// Load / save
// ---------------------------------------------------------------------------

/// Load and decrypt the wallet at the default path.
///
/// Returns `Ok(None)` when the wallet file does not exist yet.
pub fn load_wallet(password: &str) -> Result<Option<Wallet>, Box<dyn std::error::Error>> {
    load_wallet_from(password, &wallet_path())
}

/// Load and decrypt the wallet from a specific path.
pub fn load_wallet_from(
    password: &str,
    path: &std::path::Path,
) -> Result<Option<Wallet>, Box<dyn std::error::Error>> {
    match fs::read(path) {
        Ok(blob) => {
            let kdl_str = decrypt_wallet(&blob, password)?;
            let wallet = parse_wallet_str(&kdl_str)?;
            Ok(Some(wallet))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Encrypt and save the wallet to the default path.
pub fn save_wallet(wallet: &Wallet, password: &str) -> Result<(), Box<dyn std::error::Error>> {
    save_wallet_to(wallet, password, &wallet_path())
}

/// Encrypt and save the wallet to a specific path.
pub fn save_wallet_to(
    wallet: &Wallet,
    password: &str,
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure parent directory exists.
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let kdl_str = render_wallet(wallet);
    let blob = encrypt_wallet(&kdl_str, password)?;

    // Atomic write via sibling .tmp file.
    let tmp = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(&blob)?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// KDL parse / render
// ---------------------------------------------------------------------------

/// Parse the decrypted KDL wallet document into a `Wallet`.
pub fn parse_wallet_str(kdl: &str) -> Result<Wallet, Box<dyn std::error::Error>> {
    let doc: KdlDocument = kdl.parse()?;

    // The first node must be `wallet active="N"`.
    let wallet_node = doc
        .nodes()
        .first()
        .filter(|n| n.name().value() == "wallet")
        .ok_or("missing `wallet` node")?;

    let active: usize = wallet_node
        .get("active")
        .and_then(|v| v.as_string())
        .ok_or("missing or non-string `active` attribute on wallet node")?
        .parse::<usize>()
        .map_err(|_| "invalid `active` index")?;

    let mut identities = Vec::new();
    for node in doc.nodes().iter().skip(1) {
        if node.name().value() != "identity" {
            continue;
        }
        let identity = parse_identity_node(node)?;
        identities.push(identity);
    }

    Ok(Wallet { active, identities })
}

/// Render a `Wallet` to its KDL string (decrypted form).
pub fn render_wallet(wallet: &Wallet) -> String {
    let mut out = String::new();
    out.push_str(&format!("wallet active=\"{}\"\n", wallet.active));
    for id in &wallet.identities {
        let nsec_escaped = id.nsec.replace('\\', "\\\\").replace('"', "\\\"");
        let created_escaped = id.created.replace('\\', "\\\\").replace('"', "\\\"");
        out.push_str(&format!(
            "identity nsec=\"{nsec_escaped}\" type=\"{}\" created=\"{created_escaped}\"\n",
            id.identity_type.as_str(),
        ));
        if let Some(alias) = id.alias.as_deref().filter(|alias| !alias.is_empty()) {
            let alias_escaped = alias.replace('\\', "\\\\").replace('"', "\\\"");
            out.pop();
            out.push_str(&format!(" alias=\"{alias_escaped}\"\n"));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_identity_node(node: &KdlNode) -> Result<Identity, Box<dyn std::error::Error>> {
    let nsec = node
        .get("nsec")
        .and_then(|v| v.as_string())
        .ok_or("missing `nsec` on identity node")?
        .to_string();

    let type_str = node
        .get("type")
        .and_then(|v| v.as_string())
        .ok_or("missing `type` on identity node")?;
    let identity_type =
        IdentityType::parse(type_str).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let created = node
        .get("created")
        .and_then(|v| v.as_string())
        .ok_or("missing `created` on identity node")?
        .to_string();
    let alias = node
        .get("alias")
        .and_then(|v| v.as_string())
        .map(str::to_string)
        .filter(|alias| !alias.is_empty());

    Ok(Identity {
        nsec,
        identity_type,
        created,
        alias,
    })
}

/// Format a Unix timestamp as `YYYY-MM-DDTHH:MM:SSZ` (UTC, no sub-second).
pub fn format_iso8601(secs: u64) -> String {
    // Days since Unix epoch (1970-01-01).
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hh = time_of_day / 3600;
    let mm = (time_of_day % 3600) / 60;
    let ss = time_of_day % 60;

    // Gregorian calendar calculation.
    // Using the algorithm from the C standard library gmtime equivalent.
    let mut year = 1970u64;
    let mut remaining_days = days;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let leap = is_leap_year(year);
    let month_days: [u64; 12] = [
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
    for &md in &month_days {
        if remaining_days < md {
            break;
        }
        remaining_days -= md;
        month += 1;
    }
    let day = remaining_days + 1;

    format!("{year:04}-{month:02}-{day:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

fn is_leap_year(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

/// Return the current time as an ISO-8601 string.
pub fn now_iso8601() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_iso8601(secs)
}
