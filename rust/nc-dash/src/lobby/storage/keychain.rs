use std::path::Path;

use kdl::KdlDocument;

use super::paths;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeychainIdentityRecord {
    pub npub: String,
    pub nsec: String,
    pub identity_type: String,
    pub created: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LobbyKeychainRecord {
    pub active: usize,
    pub handle: Option<String>,
    pub identities: Vec<KeychainIdentityRecord>,
}

pub fn keychain_path() -> std::path::PathBuf {
    paths::keychain_path()
}

pub fn keychain_exists() -> bool {
    keychain_path().exists()
}

pub fn load_keychain_stub_from(path: &Path) -> Result<Option<LobbyKeychainRecord>, Box<dyn std::error::Error>> {
    match std::fs::read_to_string(path) {
        Ok(raw) => Ok(Some(parse_keychain_kdl(&raw)?)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err.into()),
    }
}

pub fn parse_keychain_kdl(raw: &str) -> Result<LobbyKeychainRecord, Box<dyn std::error::Error>> {
    let doc: KdlDocument = raw.parse()?;
    let root = doc
        .nodes()
        .iter()
        .find(|node| node.name().value() == "keychain")
        .ok_or("missing keychain node")?;
    let active = root
        .get("active")
        .and_then(|value| value.as_integer())
        .unwrap_or(0) as usize;
    let handle = root
        .get("handle")
        .and_then(|value| value.as_string())
        .map(ToString::to_string);
    let identities = doc
        .nodes()
        .iter()
        .filter(|node| node.name().value() == "identity")
        .map(|node| KeychainIdentityRecord {
            npub: node
                .get("npub")
                .and_then(|value| value.as_string())
                .unwrap_or_default()
                .to_string(),
            nsec: node
                .get("nsec")
                .and_then(|value| value.as_string())
                .unwrap_or_default()
                .to_string(),
            identity_type: node
                .get("type")
                .and_then(|value| value.as_string())
                .unwrap_or("local")
                .to_string(),
            created: node
                .get("created")
                .and_then(|value| value.as_string())
                .unwrap_or_default()
                .to_string(),
        })
        .collect();
    Ok(LobbyKeychainRecord {
        active,
        handle,
        identities,
    })
}

pub fn render_keychain_kdl(record: &LobbyKeychainRecord) -> String {
    let handle = record
        .handle
        .as_deref()
        .map(|handle| format!(" handle=\"{handle}\""))
        .unwrap_or_default();
    let mut out = format!("keychain active={}{}\n", record.active, handle);
    for identity in &record.identities {
        out.push_str(&format!(
            "identity npub=\"{}\" nsec=\"{}\" type=\"{}\" created=\"{}\"\n",
            identity.npub, identity.nsec, identity.identity_type, identity.created
        ));
    }
    out
}
