use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostConfig {
    pub games_root: PathBuf,
    pub relay_url: String,
    pub invite_relay_host: String,
    pub identity_path: PathBuf,
    pub sysop_contact_npub: String,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("failed to parse KDL: {0}")]
    ParseError(String),
    #[error("missing required field: {0}")]
    MissingField(String),
    #[error("invalid field value: {0}")]
    InvalidValue(String),
}

impl HostConfig {
    pub fn load(path: &PathBuf) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    pub fn parse(content: &str) -> Result<Self, ConfigError> {
        let doc: kdl::KdlDocument = match content.parse() {
            Ok(d) => d,
            Err(e) => return Err(ConfigError::ParseError(e.to_string())),
        };

        let host = doc
            .get("host")
            .ok_or_else(|| ConfigError::MissingField("host".to_string()))?;

        let games_root = string_field(host, "games-root")
            .map(|s| PathBuf::from(s.to_string()))
            .ok_or_else(|| ConfigError::MissingField("games-root".to_string()))?;

        let relay_url = string_field(host, "relay-url")
            .map(|s| s.to_string())
            .ok_or_else(|| ConfigError::MissingField("relay-url".to_string()))?;

        let invite_relay_host = string_field(host, "invite-relay-host")
            .map(|s| s.to_string())
            .ok_or_else(|| ConfigError::MissingField("invite-relay-host".to_string()))?;

        let identity_path = string_field(host, "identity-path")
            .map(|s| PathBuf::from(s.to_string()))
            .ok_or_else(|| ConfigError::MissingField("identity-path".to_string()))?;

        let sysop_contact_npub = string_field(host, "sysop-contact-npub")
            .map(|s| s.to_string())
            .ok_or_else(|| ConfigError::MissingField("sysop-contact-npub".to_string()))?;

        Ok(HostConfig {
            games_root,
            relay_url,
            invite_relay_host,
            identity_path,
            sysop_contact_npub,
        })
    }

    pub fn default_config_path() -> PathBuf {
        PathBuf::from("/etc/nc-host/host.kdl")
    }
}

fn string_field<'a>(node: &'a kdl::KdlNode, name: &str) -> Option<&'a str> {
    node.get(name)
        .and_then(|entry| entry.value().as_string())
        .or_else(|| {
            node.children()
                .and_then(|children| children.get(name))
                .and_then(|child| child.get(0))
                .and_then(|entry| entry.value().as_string())
        })
}

pub fn generate_default_config() -> String {
    r#"host {
    games-root "/var/lib/nc-host/games"
    relay-url "wss://relay.example.com"
    invite-relay-host "relay.example.com"
    identity-path "/etc/nc-host/host.nsec"
    sysop-contact-npub "npub1..."
}
"#
    .to_string()
}
