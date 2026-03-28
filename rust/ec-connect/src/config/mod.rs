//! Player config types and I/O.
//!
//! The config file lives at `~/.config/ec/config.kdl` and is entirely
//! optional.  A missing file produces a default `ConnectConfig` with no
//! bookmarks, no relay, and no default server.
//!
//! Format:
//! ```kdl
//! relay "wss://relay.example.com"
//! server "friday" host="play.example.com" port=22
//! server "local"  host="localhost"        port=2222
//! default "friday"
//! maps-dir "/path/to/maps"
//! lock-timeout-minutes 5
//! ```

use std::path::PathBuf;
use url::Url;

pub mod io;

pub use io::{
    config_path, load_config, load_config_from, save_config, save_config_to,
    seed_default_relay, seed_default_relay_at,
};

/// A named server bookmark.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerBookmark {
    /// Short name used on the command line (e.g. `"friday"`).
    pub name: String,
    /// Hostname or IP address.
    pub host: String,
    /// SSH port (defaults to 22 when omitted in the config file).
    pub port: u16,
}

/// The player's local configuration.
#[derive(Debug, Clone)]
pub struct ConnectConfig {
    /// Default Nostr relay URL for session handshakes, if configured.
    pub relay: Option<String>,
    /// Named server bookmarks.
    pub servers: Vec<ServerBookmark>,
    /// Name of the default server bookmark, if configured.
    pub default_server: Option<String>,
    /// Optional override for where downloaded starmap bundles are stored.
    pub maps_dir: Option<PathBuf>,
    /// Optional idle-lock timeout override in minutes.
    pub lock_timeout_minutes: Option<u16>,
}

impl ConnectConfig {
    /// Return an empty default config (no relay, no bookmarks, no default).
    pub fn empty() -> Self {
        ConnectConfig {
            relay: None,
            servers: Vec::new(),
            default_server: None,
            maps_dir: None,
            lock_timeout_minutes: None,
        }
    }

    /// Look up a bookmark by name.
    pub fn server(&self, name: &str) -> Option<&ServerBookmark> {
        self.servers.iter().find(|s| s.name == name)
    }

    pub fn effective_lock_timeout_minutes(&self) -> u16 {
        self.lock_timeout_minutes.unwrap_or(5)
    }
}

pub fn validate_relay_url(input: &str) -> Result<Option<String>, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let parsed = Url::parse(trimmed)
        .map_err(|_| "relay URL must be a valid ws:// or wss:// URL".to_string())?;
    match parsed.scheme() {
        "ws" | "wss" => {}
        _ => return Err("relay URL must start with ws:// or wss://".to_string()),
    }
    if parsed.host_str().is_none() {
        return Err("relay URL must include a host".to_string());
    }
    Ok(Some(trimmed.to_string()))
}
