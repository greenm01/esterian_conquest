//! Config file I/O: load, save, parse, render for `~/.config/ec/config.kdl`.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use kdl::KdlDocument;

use super::{ConnectConfig, ServerBookmark};

/// Default SSH port used when a server bookmark omits `port`.
const DEFAULT_PORT: u16 = 22;

// ---------------------------------------------------------------------------
// Path resolution
// ---------------------------------------------------------------------------

/// Return the platform-appropriate config file path:
///   `~/.config/ec/config.kdl` (Linux/macOS XDG)
///   `%APPDATA%\ec\config.kdl` (Windows)
pub fn config_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .map(|h| h.join(".config"))
            .unwrap_or_else(|| PathBuf::from("."))
    });
    base.join("ec").join("config.kdl")
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
                config.relay = Some(url);
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
            // Unknown nodes are silently ignored for forward compatibility.
            _ => {}
        }
    }

    Ok(config)
}

/// Render a `ConnectConfig` to a KDL string.
pub fn render_config(config: &ConnectConfig) -> String {
    let mut out = String::new();
    if let Some(relay) = &config.relay {
        out.push_str(&format!("relay \"{}\"\n", kdl_escape(relay)));
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
    out
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn kdl_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
