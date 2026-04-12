//! KDL serialization for `identity.kdl`, plus path resolution.

use std::fs;
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};

use nostr_sdk::{Keys, ToBech32};

use super::HostIdentity;

/// Resolve the identity file path.
///
/// Resolution order:
/// 1. System-level: `/etc/nc-gate/identity.kdl` (if the directory exists and
///    the process can write to it — i.e., we're running as root / in deployment).
/// 2. User-level: `~/.local/share/nc-gate/identity.kdl` (default for dev /
///    single-user installs).
///
/// Callers can also pass an explicit path via the `--identity` flag (future
/// work); this function is for the default lookup only.
pub fn identity_path() -> PathBuf {
    let system = PathBuf::from("/etc/nc-gate");
    if system.exists() {
        return system.join("identity.kdl");
    }
    // XDG_DATA_HOME or ~/.local/share
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
            PathBuf::from(home).join(".local").join("share")
        });
    base.join("nc-gate").join("identity.kdl")
}

/// Load the daemon identity from `path`.
pub fn load_identity(path: &Path) -> Result<HostIdentity, Box<dyn std::error::Error>> {
    let text =
        fs::read_to_string(path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
    parse_identity_str(&text)
        .map_err(|err| format!("invalid identity at {}: {err}", path.display()).into())
}

/// Save a daemon identity to `path` atomically.
///
/// The parent directory is created if it does not exist.
pub fn save_identity(
    path: &Path,
    keys: &Keys,
    created: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("cannot create directory {}: {err}", parent.display()))?;
    }
    let content = render_identity(keys, created)?;
    let tmp = path.with_extension("kdl.tmp");
    {
        let mut f = fs::File::create(&tmp)
            .map_err(|err| format!("cannot create temp file {}: {err}", tmp.display()))?;
        f.write_all(content.as_bytes())
            .map_err(|err| format!("write error {}: {err}", tmp.display()))?;
        f.flush()
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

/// Parse an identity from KDL source text.
pub fn parse_identity_str(text: &str) -> Result<HostIdentity, String> {
    let document: kdl::KdlDocument = text
        .parse()
        .map_err(|err| format!("KDL parse error: {err}"))?;
    let node = document.get("daemon").ok_or("missing `daemon` node")?;
    let nsec = prop_str(node, "nsec")?;
    let created = prop_str(node, "created")?;
    let keys = Keys::parse(&nsec).map_err(|err| format!("invalid nsec: {err}"))?;
    Ok(HostIdentity { keys, created })
}

/// Render an identity to a KDL string.
pub fn render_identity(keys: &Keys, created: &str) -> Result<String, Box<dyn std::error::Error>> {
    let nsec = keys
        .secret_key()
        .to_bech32()
        .map_err(|err| format!("nsec bech32: {err}"))?;
    Ok(format!(
        "daemon nsec=\"{}\" created=\"{}\"\n",
        kdl_escape(&nsec),
        kdl_escape(created),
    ))
}

// --- KDL helpers ---

fn prop_str(node: &kdl::KdlNode, name: &str) -> Result<String, String> {
    node.get(name)
        .and_then(|v| v.as_string())
        .map(str::to_string)
        .ok_or_else(|| format!("missing or non-string property `{name}`"))
}

fn kdl_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
