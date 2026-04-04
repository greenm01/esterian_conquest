//! SSH key provisioning: write and expire ephemeral authorized-key entries.
//!
//! When a session request is approved, `provision_key` writes a short-lived
//! `command=`-restricted entry to the authorized keys store.  The caller
//! publishes 30502 SessionReady afterward.  A background reaper task calls
//! `reap_expired_keys` on an interval to clear stale entries.
//!
//! Two storage methods are supported, matching `auth-keys-method` in config:
//!
//! * **`Command`** — each key is a separate file in `auth_keys_path/`.  The
//!   `nc-gate-keys` helper prints all files in that directory; sshd calls it
//!   via `AuthorizedKeysCommand`.
//!
//! * **`File`** — append/remove from `authorized_keys` directly.  Each entry
//!   is bracketed by `# nc-gate-begin <key_id>` / `# nc-gate-end <key_id>`
//!   marker comments for surgical removal.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::RngCore;
use rand::SeedableRng;

use crate::config::{AuthKeysMethod, GateConfig};
use crate::serve::routing::ResolvedSeat;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A handle to one provisioned ephemeral key entry.
#[derive(Debug, Clone)]
pub struct ProvisionedKey {
    /// Unique identifier for this key entry (hex timestamp + random bytes).
    pub key_id: String,
    /// Database-backed session token passed to the forced nc-game command.
    pub session_token: String,
    /// The full authorized-keys entry text (single line for `Command` method;
    /// bracketed block for `File` method).
    pub entry: String,
    /// Unix timestamp after which this key is considered expired.
    pub expires_at: u64,
}

/// Errors from provisioning operations.
#[derive(Debug)]
pub enum ProvisionError {
    Io(std::io::Error),
    Other(String),
}

impl std::fmt::Display for ProvisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProvisionError::Io(e) => write!(f, "I/O error: {e}"),
            ProvisionError::Other(s) => write!(f, "{s}"),
        }
    }
}

impl From<std::io::Error> for ProvisionError {
    fn from(e: std::io::Error) -> Self {
        ProvisionError::Io(e)
    }
}

impl std::error::Error for ProvisionError {}

// ---------------------------------------------------------------------------
// Public functions
// ---------------------------------------------------------------------------

/// Provision an ephemeral authorized-key entry for the given seat.
///
/// Writes the entry to the configured store and returns a `ProvisionedKey`
/// handle.  The caller should publish 30502 SessionReady after this returns
/// `Ok`.
pub fn provision_key(
    config: &GateConfig,
    seat: &ResolvedSeat,
    ssh_pubkey: &str,
    game_dir: &std::path::Path,
    session_token: &str,
    hosted_invite_code: Option<&str>,
) -> Result<ProvisionedKey, ProvisionError> {
    let key_id = new_key_id();
    let now = unix_now();
    let expires_at = now + config.key_ttl;

    // The service user shell is constrained to /bin/bash or /bin/sh by the
    // installer. Use `exec` so the shell is replaced by nc-game; when the
    // hosted session ends, sshd closes the connection cleanly instead of
    // dropping the player to an interactive shell prompt.
    let mut command = String::new();
    if let Some(log_file) = config.nc_game_log_file.as_deref() {
        command.push_str("env ");
        command.push_str("NC_GAME_LOG_FILE=");
        command.push_str(&shell_quote(&log_file.display().to_string()));
        command.push(' ');
        if let Some(log_level) = config.nc_game_log_level {
            command.push_str("NC_GAME_LOG_LEVEL=");
            command.push_str(&shell_quote(match log_level {
                nc_log::LogLevel::Error => "error",
                nc_log::LogLevel::Warn => "warn",
                nc_log::LogLevel::Info => "info",
                nc_log::LogLevel::Debug => "debug",
                nc_log::LogLevel::Trace => "trace",
            }));
            command.push(' ');
        }
    }
    command.push_str("exec ");
    command.push_str(&shell_quote(&config.nc_game_path.display().to_string()));
    command.push_str(" --dir ");
    command.push_str(&shell_quote(&game_dir.display().to_string()));
    command.push_str(" --player ");
    command.push_str(&seat.player.to_string());
    command.push_str(" --session-token ");
    command.push_str(&shell_quote(session_token));
    if let Some(invite_code) = hosted_invite_code.filter(|value| !value.trim().is_empty()) {
        command.push_str(" --hosted-invite-code ");
        command.push_str(&shell_quote(invite_code.trim()));
    }
    let restrictions = "no-port-forwarding,no-X11-forwarding,no-agent-forwarding";
    let key_line = format!(r#"command="{command}",{restrictions} {ssh_pubkey}"#);

    match config.auth_keys_method {
        AuthKeysMethod::Command => {
            write_command_entry(config, &key_id, &key_line, expires_at)?;
        }
        AuthKeysMethod::File => {
            write_file_entry(config, &key_id, &key_line, expires_at)?;
        }
    }

    Ok(ProvisionedKey {
        key_id,
        session_token: session_token.to_string(),
        entry: key_line,
        expires_at,
    })
}

pub fn new_session_token() -> String {
    let mut rng = rand::rngs::StdRng::from_entropy();
    let mut buf = [0u8; 16];
    rng.fill_bytes(&mut buf);
    buf.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Remove a provisioned key entry by its key ID.
pub fn remove_key(config: &GateConfig, key_id: &str) -> Result<(), ProvisionError> {
    match config.auth_keys_method {
        AuthKeysMethod::Command => remove_command_entry(config, key_id),
        AuthKeysMethod::File => remove_file_entry(config, key_id),
    }
}

/// Scan the authorized keys store and remove all expired entries.
///
/// Returns the number of entries removed.
pub fn reap_expired_keys(config: &GateConfig) -> Result<usize, ProvisionError> {
    match config.auth_keys_method {
        AuthKeysMethod::Command => reap_command_entries(config),
        AuthKeysMethod::File => reap_file_entries(config),
    }
}

// ---------------------------------------------------------------------------
// Command method: one file per key in auth_keys_path/
// ---------------------------------------------------------------------------
//
// Each file is named `<key_id>.key` and contains two lines:
//   expires=<unix_timestamp>
//   <authorized-keys-line>
//
// The `nc-gate-keys` helper reads all `.key` files and prints only the
// authorized-keys line of non-expired entries.

fn key_file_path(config: &GateConfig, key_id: &str) -> PathBuf {
    config.auth_keys_path.join(format!("{key_id}.key"))
}

fn write_command_entry(
    config: &GateConfig,
    key_id: &str,
    key_line: &str,
    expires_at: u64,
) -> Result<(), ProvisionError> {
    fs::create_dir_all(&config.auth_keys_path)?;
    let path = key_file_path(config, key_id);
    // Atomic write: write to `.tmp` then rename.
    let tmp_path = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp_path)?;
        writeln!(f, "expires={expires_at}")?;
        writeln!(f, "{key_line}")?;
    }
    fs::rename(&tmp_path, &path)?;
    Ok(())
}

fn remove_command_entry(config: &GateConfig, key_id: &str) -> Result<(), ProvisionError> {
    let path = key_file_path(config, key_id);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(ProvisionError::Io(e)),
    }
}

fn reap_command_entries(config: &GateConfig) -> Result<usize, ProvisionError> {
    let now = unix_now();
    let dir = &config.auth_keys_path;

    // If the directory does not yet exist there is nothing to reap.
    if !dir.exists() {
        return Ok(0);
    }

    let mut removed = 0usize;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("key") {
            continue;
        }
        let contents = fs::read_to_string(&path).unwrap_or_default();
        let expires_at = parse_expires_from_command_file(&contents);
        if expires_at <= now {
            match fs::remove_file(&path) {
                Ok(()) => removed += 1,
                Err(_) => {} // best-effort
            }
        }
    }
    Ok(removed)
}

/// Parse the `expires=<ts>` line from a `.key` file.  Returns 0 on failure
/// (treats malformed entries as already expired).
fn parse_expires_from_command_file(contents: &str) -> u64 {
    contents
        .lines()
        .find_map(|l| l.strip_prefix("expires="))
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// File method: single authorized_keys with marker comments
// ---------------------------------------------------------------------------
//
// Each ephemeral entry is bracketed:
//   # nc-gate-begin <key_id> expires=<ts>
//   <authorized-keys-line>
//   # nc-gate-end <key_id>

const BEGIN_PREFIX: &str = "# nc-gate-begin ";
const END_PREFIX: &str = "# nc-gate-end ";

fn write_file_entry(
    config: &GateConfig,
    key_id: &str,
    key_line: &str,
    expires_at: u64,
) -> Result<(), ProvisionError> {
    let path = &config.auth_keys_path;
    // Ensure parent directory exists.
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let block =
        format!("{BEGIN_PREFIX}{key_id} expires={expires_at}\n{key_line}\n{END_PREFIX}{key_id}\n");

    // Atomic append via read-modify-write with a sibling `.tmp`.
    let existing = fs::read_to_string(path).unwrap_or_default();
    let new_contents = format!("{existing}{block}");
    atomic_write(path, &new_contents)?;
    Ok(())
}

fn remove_file_entry(config: &GateConfig, key_id: &str) -> Result<(), ProvisionError> {
    let path = &config.auth_keys_path;
    if !path.exists() {
        return Ok(());
    }
    let existing = fs::read_to_string(path)?;
    let stripped = remove_block_for_id(&existing, key_id);
    atomic_write(path, &stripped)?;
    Ok(())
}

fn reap_file_entries(config: &GateConfig) -> Result<usize, ProvisionError> {
    let path = &config.auth_keys_path;
    if !path.exists() {
        return Ok(0);
    }
    let now = unix_now();
    let existing = fs::read_to_string(path)?;
    let (cleaned, removed) = reap_expired_blocks(&existing, now);
    if removed > 0 {
        atomic_write(path, &cleaned)?;
    }
    Ok(removed)
}

// ---------------------------------------------------------------------------
// File method helpers
// ---------------------------------------------------------------------------

/// Remove all `nc-gate` blocks for a given key ID.
fn remove_block_for_id(contents: &str, key_id: &str) -> String {
    let begin_marker = format!("{BEGIN_PREFIX}{key_id}");
    let end_marker = format!("{END_PREFIX}{key_id}");
    strip_blocks(
        contents,
        |begin_line| begin_line.starts_with(&begin_marker),
        &end_marker,
    )
}

/// Remove all `nc-gate` blocks whose `expires=` timestamp has passed.
fn reap_expired_blocks(contents: &str, now: u64) -> (String, usize) {
    strip_blocks_counting(
        contents,
        |begin_line| {
            // begin_line: "# nc-gate-begin <key_id> expires=<ts>"
            let expires = begin_line
                .split_whitespace()
                .find_map(|tok| tok.strip_prefix("expires="))
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0);
            expires <= now
        },
        END_PREFIX,
    )
}

/// Generic block stripper.  `should_strip(begin_line)` decides whether to
/// drop the entire block.  Blocks are delineated by lines starting with
/// `BEGIN_PREFIX` and `end_prefix`.
fn strip_blocks(contents: &str, should_strip: impl Fn(&str) -> bool, end_prefix: &str) -> String {
    strip_blocks_counting(contents, should_strip, end_prefix).0
}

/// Like `strip_blocks` but counts the number of blocks removed.
fn strip_blocks_counting(
    contents: &str,
    mut should_strip: impl FnMut(&str) -> bool,
    end_prefix: &str,
) -> (String, usize) {
    let mut out = String::with_capacity(contents.len());
    let mut skipping = false;
    let mut removed = 0usize;

    for line in contents.lines() {
        if line.starts_with(BEGIN_PREFIX) {
            if should_strip(line) {
                skipping = true;
            } else {
                out.push_str(line);
                out.push('\n');
            }
        } else if line.starts_with(end_prefix) {
            if skipping {
                skipping = false;
                removed += 1;
            } else {
                out.push_str(line);
                out.push('\n');
            }
        } else if !skipping {
            out.push_str(line);
            out.push('\n');
        }
    }
    (out, removed)
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Generate a unique key ID: 8-hex-digit timestamp + 8 random hex digits.
fn new_key_id() -> String {
    let ts = unix_now() & 0xFFFF_FFFF;
    let mut rng = rand::rngs::StdRng::from_entropy();
    let mut buf = [0u8; 4];
    rng.fill_bytes(&mut buf);
    let rand_part = u32::from_be_bytes(buf);
    format!("{ts:08x}{rand_part:08x}")
}

/// Current Unix timestamp in seconds.
fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    let mut quoted = String::from("'");
    for ch in value.chars() {
        if ch == '\'' {
            quoted.push_str("'\"'\"'");
        } else {
            quoted.push(ch);
        }
    }
    quoted.push('\'');
    quoted
}

/// Write `contents` to `path` atomically via a `.tmp` sibling.
fn atomic_write(path: &std::path::Path, contents: &str) -> Result<(), ProvisionError> {
    let tmp = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(contents.as_bytes())?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}
