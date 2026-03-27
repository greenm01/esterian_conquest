//! KDL serialization for `config.kdl`, plus path resolution.

use std::fs;
use std::path::{Path, PathBuf};

use super::{AuthKeysMethod, DEFAULT_EC_GAME_PATH, GateConfig};

/// Resolve the config file path.
///
/// Resolution order:
/// 1. System-level: `/etc/ec-gate/config.kdl` (if the directory exists).
/// 2. User-level: `~/.config/ec-gate/config.kdl` (XDG config home).
pub fn config_path() -> PathBuf {
    let system = PathBuf::from("/etc/ec-gate");
    if system.exists() {
        return system.join("config.kdl");
    }
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
            PathBuf::from(home).join(".config")
        });
    base.join("ec-gate").join("config.kdl")
}

/// Load the gate config from `path`.
pub fn load_config(path: &Path) -> Result<GateConfig, Box<dyn std::error::Error>> {
    let text =
        fs::read_to_string(path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
    parse_config_str(&text)
        .map_err(|err| format!("invalid config at {}: {err}", path.display()).into())
}

/// Parse a `GateConfig` from KDL source text.
pub fn parse_config_str(text: &str) -> Result<GateConfig, String> {
    let document: kdl::KdlDocument = text
        .parse()
        .map_err(|err| format!("KDL parse error: {err}"))?;

    let relay = top_string(&document, "relay")?;
    let ssh_host = top_string(&document, "ssh-host")?;
    let ssh_port = top_u16(&document, "ssh-port")?;
    let ssh_user = top_string(&document, "ssh-user")?;
    let ec_game_path = PathBuf::from(
        opt_top_string(&document, "ec-game-path")?
            .unwrap_or_else(|| DEFAULT_EC_GAME_PATH.to_string()),
    );

    let auth_keys_method_str = top_string(&document, "auth-keys-method")?;
    let auth_keys_method = match auth_keys_method_str.as_str() {
        "command" => AuthKeysMethod::Command,
        "file" => AuthKeysMethod::File,
        other => return Err(format!("unknown auth-keys-method: `{other}`")),
    };

    let auth_keys_path = PathBuf::from(top_string(&document, "auth-keys-path")?);
    let key_ttl = top_u64(&document, "key-ttl")?;

    let games = document
        .nodes()
        .iter()
        .filter(|n| n.name().value() == "game")
        .map(|n| {
            n.entries()
                .first()
                .and_then(|e| e.value().as_string())
                .map(|s| PathBuf::from(s.to_string()))
                .ok_or_else(|| "game node must have a string argument".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;

    if games.is_empty() {
        return Err("config must include at least one `game` directory".to_string());
    }

    Ok(GateConfig {
        relay,
        ssh_host,
        ssh_port,
        ssh_user,
        ec_game_path,
        auth_keys_method,
        auth_keys_path,
        key_ttl,
        games,
    })
}

fn opt_top_string(doc: &kdl::KdlDocument, name: &str) -> Result<Option<String>, String> {
    Ok(doc
        .get(name)
        .map(|node| {
            node.entries()
                .first()
                .and_then(|e| e.value().as_string())
                .map(str::to_string)
                .ok_or_else(|| format!("`{name}` must have a string argument"))
        })
        .transpose()?)
}

// --- KDL helpers ---

/// Extract the first positional string argument from a top-level node.
fn top_string(doc: &kdl::KdlDocument, name: &str) -> Result<String, String> {
    let node = doc
        .get(name)
        .ok_or_else(|| format!("missing `{name}` node"))?;
    node.entries()
        .first()
        .and_then(|e| e.value().as_string())
        .map(str::to_string)
        .ok_or_else(|| format!("`{name}` must have a string argument"))
}

/// Extract the first positional integer argument as `u16`.
fn top_u16(doc: &kdl::KdlDocument, name: &str) -> Result<u16, String> {
    let node = doc
        .get(name)
        .ok_or_else(|| format!("missing `{name}` node"))?;
    let n = node
        .entries()
        .first()
        .and_then(|e| e.value().as_integer())
        .ok_or_else(|| format!("`{name}` must have an integer argument"))?;
    u16::try_from(n).map_err(|_| format!("`{name}` value {n} is out of range for u16"))
}

/// Extract the first positional integer argument as `u64`.
fn top_u64(doc: &kdl::KdlDocument, name: &str) -> Result<u64, String> {
    let node = doc
        .get(name)
        .ok_or_else(|| format!("missing `{name}` node"))?;
    let n = node
        .entries()
        .first()
        .and_then(|e| e.value().as_integer())
        .ok_or_else(|| format!("`{name}` must have an integer argument"))?;
    u64::try_from(n).map_err(|_| format!("`{name}` value {n} is out of range for u64"))
}
