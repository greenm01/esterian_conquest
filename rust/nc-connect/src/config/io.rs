//! Config file I/O: load, save, parse, render for `~/.config/nc/config.kdl`.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use kdl::KdlDocument;

use crate::keychain::io::now_iso8601;
use crate::paths::config_root;

use super::{ConnectConfig, RelayStatus, ServerBookmark, validate_relay_url};

/// Default SSH port used when a server bookmark omits `port`.
const DEFAULT_PORT: u16 = 22;

// ---------------------------------------------------------------------------
// Path resolution
// ---------------------------------------------------------------------------

/// Return the platform-appropriate config file path:
///   `~/.config/nc/config.kdl` (Linux/macOS XDG)
///   `%APPDATA%\nc\config.kdl` (Windows)
pub fn config_path() -> PathBuf {
    config_root().join("config.kdl")
}

// ---------------------------------------------------------------------------
// Load / save
// ---------------------------------------------------------------------------

/// Load the config from the default path.
///
/// Returns `Ok(ConnectConfig::empty())` when the file does not exist.
pub fn load_config() -> Result<ConnectConfig, Box<dyn std::error::Error>> {
    load_config_from(&config_path())
}

/// Load the config from a specific path.
///
/// Returns `Ok(ConnectConfig::empty())` when the file does not exist.
pub fn load_config_from(
    path: &std::path::Path,
) -> Result<ConnectConfig, Box<dyn std::error::Error>> {
    match fs::read_to_string(path) {
        Ok(text) => parse_config_str(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ConnectConfig::empty()),
        Err(e) => Err(e.into()),
    }
}

/// Save the config to the default path.
pub fn save_config(config: &ConnectConfig) -> Result<(), Box<dyn std::error::Error>> {
    save_config_to(config, &config_path())
}

/// Save the config to a specific path.
pub fn save_config_to(
    config: &ConnectConfig,
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = render_config(config);
    let tmp = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(text.as_bytes())?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

/// Seed the default relay from a successful join when the config does not
/// already have a valid relay.
pub fn seed_default_relay(relay_url: &str) -> Result<bool, Box<dyn std::error::Error>> {
    seed_default_relay_at(relay_url, &config_path())
}

/// Testable path override for [`seed_default_relay`].
pub fn seed_default_relay_at(
    relay_url: &str,
    path: &std::path::Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    let relay = validate_relay_url(relay_url)
        .map_err(|err| format!("default relay seed rejected: {err}"))?
        .ok_or("default relay seed rejected: relay URL must not be empty")?;

    let mut config = load_config_from(path)?;
    let has_valid_default = config
        .default_relay_url()
        .map(|current| validate_relay_url(current).ok().flatten().is_some())
        .unwrap_or(false);
    if has_valid_default {
        return Ok(false);
    }

    config.set_default_relay(&relay);
    save_config_to(&config, path)?;
    Ok(true)
}

pub fn update_relay_result(
    relay_url: &str,
    status: RelayStatus,
    last_error: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    update_relay_result_at(relay_url, status, last_error, &config_path())
}

pub fn update_relay_result_at(
    relay_url: &str,
    status: RelayStatus,
    last_error: Option<&str>,
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let relay = validate_relay_url(relay_url)
        .map_err(|err| format!("relay update rejected: {err}"))?
        .ok_or("relay update rejected: relay URL must not be empty")?;

    let mut config = load_config_from(path)?;
    let entry = config.upsert_relay(relay);
    entry.status = status;
    entry.last_error = last_error
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    entry.last_checked = Some(now_iso8601());
    config.normalize_relays();
    save_config_to(&config, path)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// KDL parse / render
// ---------------------------------------------------------------------------

/// Parse a KDL config document.
pub fn parse_config_str(kdl: &str) -> Result<ConnectConfig, Box<dyn std::error::Error>> {
    let doc: KdlDocument = kdl.parse()?;
    let mut config = ConnectConfig::empty();

    for node in doc.nodes() {
        match node.name().value() {
            "relay" => {
                let url = node
                    .get(0usize)
                    .and_then(|v| v.as_string())
                    .ok_or("relay node requires a string argument")?
                    .to_string();
                let is_default = node
                    .get("default")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false);
                let status = node
                    .get("status")
                    .and_then(|value| value.as_string())
                    .and_then(RelayStatus::from_str)
                    .unwrap_or(RelayStatus::Unknown);
                let last_error = node
                    .get("last-error")
                    .and_then(|value| value.as_string())
                    .map(str::to_string);
                let last_checked = node
                    .get("checked")
                    .and_then(|value| value.as_string())
                    .map(str::to_string);
                let entry = config.upsert_relay(url.clone());
                entry.is_default = is_default;
                entry.status = status;
                entry.last_error = last_error;
                entry.last_checked = last_checked;
                if is_default {
                    config.relay = Some(url);
                }
            }
            "server" => {
                let name = node
                    .get(0usize)
                    .and_then(|v| v.as_string())
                    .ok_or("server node requires a name argument")?
                    .to_string();
                let host = node
                    .get("host")
                    .and_then(|v| v.as_string())
                    .ok_or("server node requires a `host` property")?
                    .to_string();
                let port = node
                    .get("port")
                    .and_then(|v| v.as_integer())
                    .map(|p| u16::try_from(p).map_err(|_| format!("port out of range: {p}")))
                    .transpose()?
                    .unwrap_or(DEFAULT_PORT);
                config.servers.push(ServerBookmark { name, host, port });
            }
            "default" => {
                let name = node
                    .get(0usize)
                    .and_then(|v| v.as_string())
                    .ok_or("default node requires a string argument")?
                    .to_string();
                config.default_server = Some(name);
            }
            "maps-dir" => {
                let path = node
                    .get(0usize)
                    .and_then(|v| v.as_string())
                    .ok_or("maps-dir node requires a string argument")?;
                config.maps_dir = Some(PathBuf::from(path));
            }
            "lock-timeout-minutes" => {
                let minutes = node
                    .get(0usize)
                    .and_then(|v| v.as_integer())
                    .ok_or("lock-timeout-minutes node requires an integer argument")?;
                config.lock_timeout_minutes = Some(
                    u16::try_from(minutes)
                        .map_err(|_| format!("lock-timeout-minutes out of range: {minutes}"))?,
                );
            }
            "log-file" => {
                let path = node
                    .get(0usize)
                    .and_then(|v| v.as_string())
                    .ok_or("log-file node requires a string argument")?;
                config.log_file = Some(PathBuf::from(path));
            }
            "log-level" => {
                let level = node
                    .get(0usize)
                    .and_then(|v| v.as_string())
                    .ok_or("log-level node requires a string argument")?;
                config.log_level = Some(
                    nc_log::LogLevel::parse(level)
                        .map_err(|err| format!("log-level node rejected: {err}"))?,
                );
            }
            // Unknown nodes are silently ignored for forward compatibility.
            _ => {}
        }
    }

    config.normalize_relays();
    Ok(config)
}

/// Render a `ConnectConfig` to a KDL string.
pub fn render_config(config: &ConnectConfig) -> String {
    let mut out = String::new();
    for relay in &config.relays {
        out.push_str(&format!("relay \"{}\"", kdl_escape(&relay.url)));
        if relay.is_default {
            out.push_str(" default=#true");
        }
        if relay.status != RelayStatus::Unknown {
            out.push_str(&format!(" status=\"{}\"", relay.status.as_str()));
        }
        if let Some(last_error) = relay
            .last_error
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            out.push_str(&format!(" last-error=\"{}\"", kdl_escape(last_error)));
        }
        if let Some(last_checked) = relay
            .last_checked
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            out.push_str(&format!(" checked=\"{}\"", kdl_escape(last_checked)));
        }
        out.push('\n');
    }
    for server in &config.servers {
        out.push_str(&format!(
            "server \"{}\" host=\"{}\" port={}\n",
            kdl_escape(&server.name),
            kdl_escape(&server.host),
            server.port,
        ));
    }
    if let Some(default) = &config.default_server {
        out.push_str(&format!("default \"{}\"\n", kdl_escape(default)));
    }
    if let Some(maps_dir) = &config.maps_dir {
        out.push_str(&format!(
            "maps-dir \"{}\"\n",
            kdl_escape(&maps_dir.to_string_lossy())
        ));
    }
    if let Some(minutes) = config.lock_timeout_minutes {
        out.push_str(&format!("lock-timeout-minutes {minutes}\n"));
    }
    if let Some(log_file) = &config.log_file {
        out.push_str(&format!(
            "log-file \"{}\"\n",
            kdl_escape(&log_file.to_string_lossy())
        ));
    }
    if let Some(log_level) = config.log_level {
        out.push_str(&format!(
            "log-level \"{}\"\n",
            match log_level {
                nc_log::LogLevel::Error => "error",
                nc_log::LogLevel::Warn => "warn",
                nc_log::LogLevel::Info => "info",
                nc_log::LogLevel::Debug => "debug",
                nc_log::LogLevel::Trace => "trace",
            }
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn kdl_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
