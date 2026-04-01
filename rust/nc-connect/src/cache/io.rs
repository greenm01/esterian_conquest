//! Cache file I/O: load, save, parse, render for `~/.local/share/nc/cache.kdl`.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use kdl::KdlDocument;

use crate::paths::data_root;

use super::{CachedGame, CachedGameStatus, GameCache};

// ---------------------------------------------------------------------------
// Path resolution
// ---------------------------------------------------------------------------

/// Return the platform-appropriate cache file path:
///   `~/.local/share/nc/cache.kdl` (Linux/macOS XDG)
///   `%APPDATA%\nc\cache.kdl` (Windows)
pub fn cache_path() -> PathBuf {
    data_root().join("cache.kdl")
}

// ---------------------------------------------------------------------------
// Load / save
// ---------------------------------------------------------------------------

/// Load the cache from the default path.
///
/// Returns `Ok(GameCache::empty())` when the file does not exist.
pub fn load_cache() -> Result<GameCache, Box<dyn std::error::Error>> {
    load_cache_from(&cache_path())
}

/// Load the cache from a specific path.
///
/// Returns `Ok(GameCache::empty())` when the file does not exist.
pub fn load_cache_from(path: &std::path::Path) -> Result<GameCache, Box<dyn std::error::Error>> {
    match fs::read_to_string(path) {
        Ok(text) => parse_cache_str(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(GameCache::empty()),
        Err(e) => Err(e.into()),
    }
}

/// Save the cache to the default path.
pub fn save_cache(cache: &GameCache) -> Result<(), Box<dyn std::error::Error>> {
    save_cache_to(cache, &cache_path())
}

/// Save the cache to a specific path.
pub fn save_cache_to(
    cache: &GameCache,
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = render_cache(cache);
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

/// Parse a KDL cache document.
pub fn parse_cache_str(kdl: &str) -> Result<GameCache, Box<dyn std::error::Error>> {
    let doc: KdlDocument = kdl.parse()?;
    let mut cache = GameCache::empty();

    for node in doc.nodes() {
        if node.name().value() != "game" {
            // Unknown nodes are ignored.
            continue;
        }

        let id = req_str(node, "id", "game")?;
        let name = req_str(node, "name", "game")?;
        let player_name = node
            .get("player-name")
            .and_then(|v| v.as_string())
            .map(str::to_string)
            .filter(|value| !value.is_empty());
        let server = req_str(node, "server", "game")?;

        let port = node
            .get("port")
            .and_then(|v| v.as_integer())
            .map(|p| u16::try_from(p).map_err(|_| format!("port out of range: {p}")))
            .transpose()?
            .unwrap_or(22);

        let relay_url = node
            .get("relay-url")
            .and_then(|v| v.as_string())
            .map(str::to_string)
            .filter(|value| !value.is_empty());

        let seat = node
            .get("seat")
            .and_then(|v| v.as_integer())
            .map(|s| u32::try_from(s).map_err(|_| format!("seat out of range: {s}")))
            .transpose()?
            .ok_or("game node missing required `seat` property")?;

        let npub = req_str(node, "npub", "game")?;
        let gate_npub = node
            .get("gate-npub")
            .and_then(|v| v.as_string())
            .map(str::to_string)
            .unwrap_or_default();
        let status = node
            .get("status")
            .and_then(|v| v.as_string())
            .map(CachedGameStatus::parse)
            .unwrap_or(CachedGameStatus::Joined);
        let invite_code = node
            .get("invite-code")
            .and_then(|v| v.as_string())
            .map(str::to_string)
            .filter(|value| !value.is_empty());
        let joined = req_str(node, "joined", "game")?;
        let last_connected = node
            .get("last-connected")
            .and_then(|v| v.as_string())
            .map(str::to_string);

        cache.games.push(CachedGame {
            id,
            name,
            player_name,
            server,
            port,
            relay_url,
            seat,
            npub,
            gate_npub,
            status,
            invite_code,
            joined,
            last_connected,
        });
    }

    Ok(cache)
}

/// Render a `GameCache` to its KDL string.
pub fn render_cache(cache: &GameCache) -> String {
    let mut out = String::new();
    for g in &cache.games {
        out.push_str(&format!(
            "game id=\"{}\" name=\"{}\"",
            kdl_escape(&g.id),
            kdl_escape(&g.name),
        ));
        if let Some(player_name) = g.player_name.as_deref().filter(|value| !value.is_empty()) {
            out.push_str(&format!(" player-name=\"{}\"", kdl_escape(player_name)));
        }
        out.push_str(&format!(
            " server=\"{}\" port={}",
            kdl_escape(&g.server),
            g.port,
        ));
        if let Some(relay_url) = g.relay_url.as_deref().filter(|value| !value.is_empty()) {
            out.push_str(&format!(" relay-url=\"{}\"", kdl_escape(relay_url)));
        }
        out.push_str(&format!(
            " seat={} npub=\"{}\" status=\"{}\" joined=\"{}\"",
            g.seat,
            kdl_escape(&g.npub),
            g.status.as_kdl_value(),
            kdl_escape(&g.joined),
        ));
        if !g.gate_npub.is_empty() {
            out.push_str(&format!(" gate-npub=\"{}\"", kdl_escape(&g.gate_npub)));
        }
        if let Some(invite_code) = g.invite_code.as_deref().filter(|value| !value.is_empty()) {
            out.push_str(&format!(" invite-code=\"{}\"", kdl_escape(invite_code)));
        }
        if let Some(lc) = &g.last_connected {
            out.push_str(&format!(" last-connected=\"{}\"", kdl_escape(lc)));
        }
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn req_str(
    node: &kdl::KdlNode,
    key: &str,
    node_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    node.get(key)
        .and_then(|v| v.as_string())
        .map(str::to_string)
        .ok_or_else(|| format!("{node_name} node missing required `{key}` property").into())
}

fn kdl_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
