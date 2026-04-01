//! Keychain file I/O: load, save, parse, render.
//!
//! The keychain file is a binary envelope (see `crypto` module). The decrypted
//! contents are a KDL document:
//!
//! ```kdl
//! keychain active="0"
//! identity nsec="nsec1..." type="local" created="2026-03-26T12:00:00Z" alias="Desk Key"
//! identity nsec="nsec1..." type="imported" created="2026-03-28T09:35:00Z"
//! ```
//!
//! `load_keychain` prompts for a password externally; this module takes the
//! password as a string so it stays testable without stdin interaction.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use kdl::{KdlDocument, KdlNode};

use super::crypto::{decrypt_keychain, encrypt_keychain};
use super::{Identity, IdentityType, Keychain};
use crate::paths::data_root;

// ---------------------------------------------------------------------------
// Path resolution
// ---------------------------------------------------------------------------

/// Return the platform-appropriate keychain file path:
///   `~/.local/share/nc/keychain.kdl` (Linux/macOS XDG)
///   `%APPDATA%\nc\keychain.kdl` (Windows)
pub fn keychain_path() -> PathBuf {
    data_root().join("keychain.kdl")
}

// ---------------------------------------------------------------------------
// Load / save
// ---------------------------------------------------------------------------

/// Load and decrypt the keychain at the default path.
///
/// Returns `Ok(None)` when the keychain file does not exist yet.
pub fn load_keychain(password: &str) -> Result<Option<Keychain>, Box<dyn std::error::Error>> {
    load_keychain_from(password, &keychain_path())
}

/// Load and decrypt the keychain from a specific path.
pub fn load_keychain_from(
    password: &str,
    path: &std::path::Path,
) -> Result<Option<Keychain>, Box<dyn std::error::Error>> {
    match fs::read(path) {
        Ok(blob) => {
            let kdl_str = decrypt_keychain(&blob, password)?;
            let keychain = parse_keychain_str(&kdl_str)?;
            Ok(Some(keychain))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Encrypt and save the keychain to the default path.
pub fn save_keychain(keychain: &Keychain, password: &str) -> Result<(), Box<dyn std::error::Error>> {
    save_keychain_to(keychain, password, &keychain_path())
}

/// Encrypt and save the keychain to a specific path.
pub fn save_keychain_to(
    keychain: &Keychain,
    password: &str,
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure parent directory exists.
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let kdl_str = render_keychain(keychain);
    let blob = encrypt_keychain(&kdl_str, password)?;

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

/// Parse the decrypted KDL keychain document into a `Keychain`.
pub fn parse_keychain_str(kdl: &str) -> Result<Keychain, Box<dyn std::error::Error>> {
    let doc: KdlDocument = kdl.parse()?;

    // The first node must be `keychain active="N"`.
    let keychain_node = doc
        .nodes()
        .first()
        .filter(|n| n.name().value() == "keychain")
        .ok_or("missing `keychain` node")?;

    let active: usize = keychain_node
        .get("active")
        .and_then(|v| v.as_string())
        .ok_or("missing or non-string `active` attribute on keychain node")?
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

    Ok(Keychain { active, identities })
}

/// Render a `Keychain` to its KDL string (decrypted form).
pub fn render_keychain(keychain: &Keychain) -> String {
    let mut out = String::new();
    out.push_str(&format!("keychain active=\"{}\"\n", keychain.active));
    for id in &keychain.identities {
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
