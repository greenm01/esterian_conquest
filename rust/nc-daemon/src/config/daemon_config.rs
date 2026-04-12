use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonConfig {
    pub games_root: PathBuf,
    pub relay_url: String,
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

impl DaemonConfig {
    pub fn load(path: &PathBuf) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    pub fn parse(content: &str) -> Result<Self, ConfigError> {
        let doc: kdl::KdlDocument = match content.parse() {
            Ok(d) => d,
            Err(e) => return Err(ConfigError::ParseError(e.to_string())),
        };

        let daemon = doc
            .get("daemon")
            .ok_or_else(|| ConfigError::MissingField("daemon".to_string()))?;

        let games_root = daemon
            .get("games-root")
            .and_then(|e| e.value().as_string())
            .map(|s| PathBuf::from(s.to_string()))
            .ok_or_else(|| ConfigError::MissingField("games-root".to_string()))?;

        let relay_url = daemon
            .get("relay-url")
            .and_then(|e| e.value().as_string())
            .map(|s| s.to_string())
            .ok_or_else(|| ConfigError::MissingField("relay-url".to_string()))?;

        let identity_path = daemon
            .get("identity-path")
            .and_then(|e| e.value().as_string())
            .map(|s| PathBuf::from(s.to_string()))
            .ok_or_else(|| ConfigError::MissingField("identity-path".to_string()))?;

        let sysop_contact_npub = daemon
            .get("sysop-contact-npub")
            .and_then(|e| e.value().as_string())
            .map(|s| s.to_string())
            .ok_or_else(|| ConfigError::MissingField("sysop-contact-npub".to_string()))?;

        Ok(DaemonConfig {
            games_root,
            relay_url,
            identity_path,
            sysop_contact_npub,
        })
    }

    pub fn default_config_path() -> PathBuf {
        PathBuf::from("/etc/nc-daemon/daemon.kdl")
    }
}

pub fn generate_default_config() -> String {
    r#"daemon {
    games-root "/var/lib/nc-daemon/games"
    relay-url "wss://relay.example.com"
    identity-path "/etc/nc-daemon/daemon.nsec"
    sysop-contact-npub "npub1..."
}
"#
    .to_string()
}
