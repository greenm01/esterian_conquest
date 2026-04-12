use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use kdl::{KdlDocument, KdlNode};

use super::crypto::{decrypt_blob, encrypt_blob};
use super::{Identity, IdentityType, Keychain};
use crate::paths::data_root;

pub fn keychain_path() -> PathBuf {
    data_root().join("keychain.kdl")
}

pub fn load_keychain(password: &str) -> Result<Option<Keychain>, Box<dyn std::error::Error>> {
    load_keychain_from(password, &keychain_path())
}

pub fn load_keychain_from(
    password: &str,
    path: &std::path::Path,
) -> Result<Option<Keychain>, Box<dyn std::error::Error>> {
    match fs::read(path) {
        Ok(blob) => {
            let kdl_str = decrypt_blob(&blob, password)?;
            Ok(Some(parse_keychain_str(&kdl_str)?))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn save_keychain(
    keychain: &Keychain,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    save_keychain_to(keychain, password, &keychain_path())
}

pub fn save_keychain_to(
    keychain: &Keychain,
    password: &str,
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let kdl_str = render_keychain(keychain);
    let blob = encrypt_blob(&kdl_str, password)?;
    let tmp = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(&blob)?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn parse_keychain_str(kdl: &str) -> Result<Keychain, Box<dyn std::error::Error>> {
    let doc: KdlDocument = kdl.parse()?;
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
        identities.push(parse_identity_node(node)?);
    }

    Ok(Keychain { active, identities })
}

pub fn render_keychain(keychain: &Keychain) -> String {
    let mut out = String::new();
    out.push_str(&format!("keychain active=\"{}\"\n", keychain.active));
    for id in &keychain.identities {
        let nsec_escaped = kdl_escape(&id.nsec);
        let created_escaped = kdl_escape(&id.created);
        out.push_str(&format!(
            "identity nsec=\"{nsec_escaped}\" type=\"{}\" created=\"{created_escaped}\"",
            id.identity_type.as_str(),
        ));
        if let Some(handle) = id.handle.as_deref() {
            out.push_str(&format!(" handle=\"{}\"", kdl_escape(handle)));
        }
        out.push('\n');
    }
    out
}

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
    let handle = node
        .get("handle")
        .and_then(|v| v.as_string())
        .map(str::to_string)
        .filter(|handle| !handle.is_empty());

    Ok(Identity {
        nsec,
        identity_type,
        created,
        handle,
    })
}

fn kdl_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

pub fn format_iso8601(secs: u64) -> String {
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hh = time_of_day / 3600;
    let mm = (time_of_day % 3600) / 60;
    let ss = time_of_day % 60;

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

pub fn now_iso8601() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format_iso8601(secs)
}
