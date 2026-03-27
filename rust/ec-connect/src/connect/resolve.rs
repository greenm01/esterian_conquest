//! Invite code parsing and server resolution.
//!
//! Invite codes are two-word slugs optionally followed by a server suffix:
//!
//! ```text
//! velvet-mountain
//! velvet-mountain@play.example.com
//! velvet-mountain@play.example.com:2222
//! ```
//!
//! The `SERVER` argument for direct mode can be a hostname, `hostname:port`,
//! or the name of a bookmark in the player's config file.
//!
//! Resolution produces a [`ResolvedTarget`] that contains the server
//! coordinates and relay URL needed to start a Nostr session handshake.
//! Gate npub discovery is deferred to the handshake step (queried via
//! kind-30500 events on the relay).

use crate::config::ConnectConfig;

// ── Constants ────────────────────────────────────────────────────────────────

/// Default SSH port used when none is specified.
pub const DEFAULT_SSH_PORT: u16 = 22;

/// Private-use and loopback address ranges used for TLS detection.
/// Traffic to these addresses uses `ws://`; all others use `wss://`.
const PRIVATE_PREFIXES: &[&str] = &["localhost", "127.", "10.", "192.168.", "::1", "[::1]"];

// ── Types ────────────────────────────────────────────────────────────────────

/// Parsed invite code before relay/server resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedInviteCode {
    /// The two-word slug (e.g. `"velvet-mountain"`).
    pub words: String,
    /// Optional server host and port extracted from the `@host[:port]` suffix.
    pub server: Option<(String, u16)>,
}

/// Fully resolved connection target, ready for the handshake step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTarget {
    /// Server hostname or IP.
    pub server_host: String,
    /// SSH port on the server.
    pub server_port: u16,
    /// Nostr relay WebSocket URL.
    pub relay_url: String,
    /// Raw invite code words, if this was an invite-code resolution.
    pub invite_code: Option<String>,
    /// Game ID hint from the invite code, if available (currently None until
    /// the handshake step queries the relay for a matching 30500 event).
    pub game_id: Option<String>,
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Parse an invite code string.
///
/// Returns `Err` if the code is empty or structurally invalid (e.g. the port
/// suffix is not a valid `u16`).
pub fn parse_invite_code(code: &str) -> Result<ParsedInviteCode, String> {
    let code = code.trim();
    if code.is_empty() {
        return Err("invite code must not be empty".into());
    }

    if let Some(at) = code.find('@') {
        let words = code[..at].to_string();
        if words.is_empty() {
            return Err("invite code words must not be empty".into());
        }
        let host_part = &code[at + 1..];
        let (host, port) = split_host_port(host_part)?;
        Ok(ParsedInviteCode {
            words,
            server: Some((host, port)),
        })
    } else {
        Ok(ParsedInviteCode {
            words: code.to_string(),
            server: None,
        })
    }
}

/// Resolve an invite code to a [`ResolvedTarget`].
///
/// Server coordinates are taken from (in priority order):
/// 1. The `@host[:port]` suffix in the invite code itself.
/// 2. The `default_server` bookmark in `config`.
/// 3. Returns `Err` if neither is available.
///
/// Relay URL is derived from the resolved server host unless the config
/// provides an explicit `relay` field.
pub fn resolve_invite(code: &str, config: &ConnectConfig) -> Result<ResolvedTarget, String> {
    let parsed = parse_invite_code(code)?;

    let (server_host, server_port) = resolve_server_coords(
        parsed.server.as_ref().map(|(h, p)| (h.as_str(), *p)),
        config,
    )?;

    let relay_url = pick_relay_url(&server_host, config);

    Ok(ResolvedTarget {
        server_host,
        server_port,
        relay_url,
        invite_code: Some(parsed.words),
        game_id: None,
    })
}

/// Resolve a server argument (bookmark name, hostname, or `hostname:port`) to
/// a [`ResolvedTarget`].
///
/// Lookup order:
/// 1. Bookmark name in `config.servers`.
/// 2. `hostname:port` literal.
/// 3. Plain hostname (uses `DEFAULT_SSH_PORT`).
pub fn resolve_server(server: &str, config: &ConnectConfig) -> Result<ResolvedTarget, String> {
    let server = server.trim();
    if server.is_empty() {
        return Err("server must not be empty".into());
    }

    // 1. Try bookmark lookup first.
    if let Some(bm) = config.server(server) {
        let relay_url = pick_relay_url(&bm.host, config);
        return Ok(ResolvedTarget {
            server_host: bm.host.clone(),
            server_port: bm.port,
            relay_url,
            invite_code: None,
            game_id: None,
        });
    }

    // 2. Parse as host[:port].
    let (host, port) = split_host_port(server)?;
    let relay_url = pick_relay_url(&host, config);
    Ok(ResolvedTarget {
        server_host: host,
        server_port: port,
        relay_url,
        invite_code: None,
        game_id: None,
    })
}

/// Derive a WebSocket relay URL from a server hostname.
///
/// Uses `ws://` for localhost and RFC-1918/loopback addresses; `wss://` for
/// everything else.  The relay is assumed to be on port 7777, following the
/// ec-gate default.
pub fn derive_relay_url(host: &str) -> String {
    let scheme = if is_private_host(host) { "ws" } else { "wss" };
    format!("{scheme}://{host}:7777")
}

// ── Private helpers ──────────────────────────────────────────────────────────

/// Pick the relay URL to use: config-explicit relay first, otherwise derive
/// from the server host.
fn pick_relay_url(server_host: &str, config: &ConnectConfig) -> String {
    if let Some(relay) = &config.relay {
        relay.clone()
    } else {
        derive_relay_url(server_host)
    }
}

/// Resolve server coordinates from an optional inline `(host, port)` pair
/// (from the invite code) or from the config's default server bookmark.
fn resolve_server_coords(
    inline: Option<(&str, u16)>,
    config: &ConnectConfig,
) -> Result<(String, u16), String> {
    if let Some((host, port)) = inline {
        return Ok((host.to_string(), port));
    }

    // Fall back to the default bookmark.
    if let Some(name) = &config.default_server {
        if let Some(bm) = config.server(name) {
            return Ok((bm.host.clone(), bm.port));
        }
        return Err(format!(
            "default server bookmark '{name}' not found in config"
        ));
    }

    Err("no server specified: include a @host suffix in the invite code or set a default server in config".into())
}

/// Split `host` or `host:port` into `(host, port)`.
///
/// For IPv6 literals the caller should pass the bracket-wrapped form
/// `[::1]:2222`; the brackets are preserved in the returned host string.
fn split_host_port(s: &str) -> Result<(String, u16), String> {
    // IPv6 bracket form: `[::1]` or `[::1]:port`
    if s.starts_with('[') {
        if let Some(close) = s.find(']') {
            let host = s[..=close].to_string();
            let rest = &s[close + 1..];
            if rest.is_empty() {
                return Ok((host, DEFAULT_SSH_PORT));
            }
            if let Some(port_str) = rest.strip_prefix(':') {
                let port = parse_port(port_str)?;
                return Ok((host, port));
            }
            return Err(format!("invalid host:port '{s}'"));
        }
        return Err(format!("unmatched '[' in address '{s}'"));
    }

    // Regular host or host:port. Count colons to distinguish IPv6 literals
    // from host:port.
    let colon_count = s.chars().filter(|&c| c == ':').count();
    if colon_count == 0 {
        return Ok((s.to_string(), DEFAULT_SSH_PORT));
    }
    if colon_count == 1 {
        let idx = s.find(':').unwrap();
        let host = s[..idx].to_string();
        let port = parse_port(&s[idx + 1..])?;
        return Ok((host, port));
    }
    // Multiple colons without brackets — treat as bare IPv6, no port.
    Ok((s.to_string(), DEFAULT_SSH_PORT))
}

fn parse_port(s: &str) -> Result<u16, String> {
    s.parse::<u16>()
        .map_err(|_| format!("invalid port '{s}': must be 0–65535"))
}

fn is_private_host(host: &str) -> bool {
    let h = host.trim_matches(|c| c == '[' || c == ']');
    PRIVATE_PREFIXES.iter().any(|prefix| h.starts_with(prefix)) || is_172_16_private(h)
}

/// Detect 172.16.0.0/12 (172.16.x.x – 172.31.x.x).
fn is_172_16_private(host: &str) -> bool {
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() < 2 {
        return false;
    }
    let Ok(a) = parts[0].parse::<u8>() else {
        return false;
    };
    let Ok(b) = parts[1].parse::<u8>() else {
        return false;
    };
    a == 172 && (16..=31).contains(&b)
}
