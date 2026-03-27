//! Gate configuration: typed representation of `config.kdl`.
//!
//! `GateConfig` is the in-memory form loaded from the configuration file. All
//! fields are required in the KDL source; there are no optional fields in this
//! initial implementation.

pub mod io;

use std::path::PathBuf;

pub use io::{config_path, load_config, parse_config_str};

/// How `ec-gate` manages ephemeral SSH authorized keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthKeysMethod {
    /// Write to a file that sshd reads via `AuthorizedKeysCommand`.
    Command,
    /// Write directly to the service user's `authorized_keys` file.
    File,
}

/// Full gate configuration, loaded from `config.kdl`.
#[derive(Debug, Clone)]
pub struct GateConfig {
    /// Nostr relay WebSocket URL.
    pub relay: String,
    /// SSH server hostname sent to players in SessionReady events.
    pub ssh_host: String,
    /// SSH port (default 22).
    pub ssh_port: u16,
    /// Service user for SSH sessions (e.g. `ecgame`).
    pub ssh_user: String,
    /// How ephemeral authorized keys are stored.
    pub auth_keys_method: AuthKeysMethod,
    /// Path to the authorized keys file or directory.
    pub auth_keys_path: PathBuf,
    /// Ephemeral key TTL in seconds.
    pub key_ttl: u64,
    /// Game directories to serve, in config order.
    pub games: Vec<PathBuf>,
}
