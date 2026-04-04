//! KDL serialization for `config.kdl`, plus path resolution.

use std::fs;
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};

use super::{AuthKeysMethod, DEFAULT_NC_GAME_PATH, GateConfig};

/// Resolve the config file path.
///
/// Resolution order:
/// 1. System-level: `/etc/nc-gate/config.kdl` (if the directory exists).
/// 2. User-level: `~/.config/nc-gate/config.kdl` (XDG config home).
pub fn config_path() -> PathBuf {
    let system = PathBuf::from("/etc/nc-gate");
    if system.exists() {
        return system.join("config.kdl");
    }
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
            PathBuf::from(home).join(".config")
        });
    base.join("nc-gate").join("config.kdl")
}

/// Load the gate config from `path`.
pub fn load_config(path: &Path) -> Result<GateConfig, Box<dyn std::error::Error>> {
    let text =
        fs::read_to_string(path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
    parse_config_str(&text)
        .map_err(|err| format!("invalid config at {}: {err}", path.display()).into())
}

/// Save the gate config to `path` atomically.
pub fn save_config(path: &Path, config: &GateConfig) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("cannot create directory {}: {err}", parent.display()))?;
    }
    let content = render_config(config);
    let tmp = path.with_extension("kdl.tmp");
    {
        let mut file = fs::File::create(&tmp)
            .map_err(|err| format!("cannot create temp file {}: {err}", tmp.display()))?;
        file.write_all(content.as_bytes())
            .map_err(|err| format!("write error {}: {err}", tmp.display()))?;
        file.flush()
            .map_err(|err| format!("flush error {}: {err}", tmp.display()))?;
    }
    fs::rename(&tmp, path).map_err(|err| {
        format!(
            "cannot rename {} -> {}: {err}",
            tmp.display(),
            path.display()
        )
    })?;
    Ok(())
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
    let nc_game_path = PathBuf::from(
        opt_top_string(&document, "nc-game-path")?
            .unwrap_or_else(|| DEFAULT_NC_GAME_PATH.to_string()),
    );
    let nc_game_log_file = opt_top_string(&document, "nc-game-log-file")?.map(PathBuf::from);
    let nc_game_log_level = opt_top_string(&document, "nc-game-log-level")?
        .map(|value| {
            nc_log::LogLevel::parse(&value)
                .map_err(|err| format!("nc-game-log-level rejected: {err}"))
        })
        .transpose()?;

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

    Ok(GateConfig {
        relay,
        ssh_host,
        ssh_port,
        ssh_user,
        nc_game_path,
        nc_game_log_file,
        nc_game_log_level,
        auth_keys_method,
        auth_keys_path,
        key_ttl,
        games,
    })
}

/// Render a `GateConfig` back to KDL source text.
pub fn render_config(config: &GateConfig) -> String {
    let auth_keys_method = match config.auth_keys_method {
        AuthKeysMethod::Command => "command",
        AuthKeysMethod::File => "file",
    };

    let mut out = String::new();
    out.push_str(&format!("relay \"{}\"\n", kdl_escape(&config.relay)));
    out.push_str(&format!("ssh-host \"{}\"\n", kdl_escape(&config.ssh_host)));
    out.push_str(&format!("ssh-port {}\n", config.ssh_port));
    out.push_str(&format!("ssh-user \"{}\"\n", kdl_escape(&config.ssh_user)));
    if config.nc_game_path != PathBuf::from(DEFAULT_NC_GAME_PATH) {
        out.push_str(&format!(
            "nc-game-path \"{}\"\n",
            kdl_escape(&config.nc_game_path.display().to_string())
        ));
    }
    if let Some(log_file) = &config.nc_game_log_file {
        out.push_str(&format!(
            "nc-game-log-file \"{}\"\n",
            kdl_escape(&log_file.display().to_string())
        ));
    }
    if let Some(log_level) = config.nc_game_log_level {
        let level = match log_level {
            nc_log::LogLevel::Error => "error",
            nc_log::LogLevel::Warn => "warn",
            nc_log::LogLevel::Info => "info",
            nc_log::LogLevel::Debug => "debug",
            nc_log::LogLevel::Trace => "trace",
        };
        out.push_str(&format!("nc-game-log-level \"{}\"\n", level));
    }
    out.push_str(&format!("auth-keys-method \"{}\"\n", auth_keys_method));
    out.push_str(&format!(
        "auth-keys-path \"{}\"\n",
        kdl_escape(&config.auth_keys_path.display().to_string())
    ));
    out.push_str(&format!("key-ttl {}\n", config.key_ttl));
    for game in &config.games {
        out.push_str(&format!(
            "game \"{}\"\n",
            kdl_escape(&game.display().to_string())
        ));
    }
    out
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

fn kdl_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
