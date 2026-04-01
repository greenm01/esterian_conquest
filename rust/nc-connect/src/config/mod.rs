//! Player config types and I/O.
//!
//! The config file lives at `~/.config/nc/config.kdl` and is entirely
//! optional.  A missing file produces a default `ConnectConfig` with no
//! bookmarks, no relays, and no default server.
//!
//! Format:
//! ```kdl
//! relay "wss://relay.example.com" default=true status="ok"
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
    config_path, load_config, load_config_from, save_config, save_config_to, seed_default_relay,
    seed_default_relay_at, update_relay_result, update_relay_result_at,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayStatus {
    Unknown,
    Ok,
    Timeout,
    ConnectFailed,
    ProtocolError,
}

impl RelayStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Ok => "ok",
            Self::Timeout => "timeout",
            Self::ConnectFailed => "connect-failed",
            Self::ProtocolError => "protocol-error",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "unknown" => Some(Self::Unknown),
            "ok" => Some(Self::Ok),
            "timeout" => Some(Self::Timeout),
            "connect-failed" => Some(Self::ConnectFailed),
            "protocol-error" => Some(Self::ProtocolError),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayEntry {
    pub url: String,
    pub is_default: bool,
    pub status: RelayStatus,
    pub last_error: Option<String>,
    pub last_checked: Option<String>,
}

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
    /// Backward-compatible alias for the default relay URL.
    pub relay: Option<String>,
    /// Known relay entries with optional cached health.
    pub relays: Vec<RelayEntry>,
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
            relays: Vec::new(),
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

    pub fn default_relay_url(&self) -> Option<&str> {
        self.relays
            .iter()
            .find(|relay| relay.is_default)
            .map(|relay| relay.url.as_str())
            .or(self.relay.as_deref())
    }

    pub fn relay_entry(&self, url: &str) -> Option<&RelayEntry> {
        self.relays.iter().find(|relay| relay.url == url)
    }

    pub fn relay_entry_mut(&mut self, url: &str) -> Option<&mut RelayEntry> {
        self.relays.iter_mut().find(|relay| relay.url == url)
    }

    pub fn upsert_relay(&mut self, url: String) -> &mut RelayEntry {
        if let Some(index) = self.relays.iter().position(|relay| relay.url == url) {
            return &mut self.relays[index];
        }
        self.relays.push(RelayEntry {
            url,
            is_default: false,
            status: RelayStatus::Unknown,
            last_error: None,
            last_checked: None,
        });
        self.relays.last_mut().expect("relay pushed")
    }

    pub fn set_default_relay(&mut self, url: &str) {
        for relay in &mut self.relays {
            relay.is_default = relay.url == url;
        }
        self.relay = Some(url.to_string());
        if self.relay_entry(url).is_none() {
            let relay = self.upsert_relay(url.to_string());
            relay.is_default = true;
        }
    }

    pub fn remove_relay(&mut self, url: &str) -> bool {
        let Some(index) = self.relays.iter().position(|relay| relay.url == url) else {
            return false;
        };
        self.relays.remove(index);
        if self.relay.as_deref() == Some(url) {
            self.relay = None;
        }
        self.normalize_relays();
        true
    }

    pub fn normalize_relays(&mut self) {
        if self.relays.is_empty() {
            self.relay = self
                .relay
                .as_deref()
                .and_then(|value| validate_relay_url(value).ok().flatten());
            if let Some(url) = self.relay.clone() {
                let relay = self.upsert_relay(url.clone());
                relay.is_default = true;
            }
            return;
        }

        let mut saw_default = false;
        for relay in &mut self.relays {
            if relay.is_default {
                if saw_default {
                    relay.is_default = false;
                } else {
                    saw_default = true;
                }
            }
        }

        if !saw_default {
            if let Some(default_url) = self
                .relay
                .as_deref()
                .and_then(|value| validate_relay_url(value).ok().flatten())
            {
                self.set_default_relay(&default_url);
            } else if let Some(first_url) = self.relays.first().map(|relay| relay.url.clone()) {
                self.set_default_relay(&first_url);
            }
        } else {
            self.relay = self
                .relays
                .iter()
                .find(|relay| relay.is_default)
                .map(|relay| relay.url.clone());
        }
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
