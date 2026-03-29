//! Invite code parsing and server resolution.
//!
//! Invite codes are accepted in three forms:
//!
//! ```text
//! ecinv1...                                  bech32 (relay URL embedded)
//! velvet-mountain                            plain slug (uses config relay)
//! velvet-mountain@play.example.com           plain slug + inline host
//! velvet-mountain@play.example.com:2222      plain slug + inline host:port
//! ```
//!
//! The `SERVER` argument for direct mode can be a hostname, `hostname:port`,
//! or the name of a bookmark in the player's config file.
//!
//! Resolution produces a [`ResolvedTarget`] that contains the server
//! coordinates and relay URL needed to start a Nostr session handshake.

use ec_nostr::invite::{decode_invite, is_bech32_invite};

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
    /// Relay URL embedded in a bech32 invite (overrides derived relay).
    pub relay_url: Option<String>,
    /// Game ID hint embedded in a bech32 invite (skips 30500 lookup).
    pub game_id: Option<String>,
    /// Gate public key hint embedded in a bech32 invite (skips 30500 discovery).
    /// Stored as hex string.
    pub gate_npub: Option<String>,
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
    /// Game ID hint (from bech32 invite or cache).
    pub game_id: Option<String>,
    /// Gate public key hint (from bech32 invite) — skips 30500 discovery.
    /// Stored as hex string.
    pub gate_npub: Option<String>,
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Parse an invite code string.
///
/// Accepts bech32 (`ecinv1...`), plain slugs (`velvet-mountain`), and
/// slug-with-host (`velvet-mountain@play.example.com[:port]`).
///
/// Returns `Err` if the code is empty or structurally invalid.
pub fn parse_invite_code(code: &str) -> Result<ParsedInviteCode, String> {
    let code = code.trim();
    if code.is_empty() {
        return Err("invite code must not be empty".into());
    }

    // Bech32 invite: self-contained with embedded relay URL.
    if is_bech32_invite(code) {
        let payload = decode_invite(code).map_err(|e| format!("invalid bech32 invite: {e}"))?;
        let gate_npub = payload
            .gate_npub
            .map(|bytes| bytes.iter().map(|b| format!("{b:02x}")).collect());
        return Ok(ParsedInviteCode {
            words: payload.words,
            server: Some((payload.ssh_host, payload.ssh_port)),
            relay_url: Some(payload.relay_url),
            game_id: payload.game_id,
            gate_npub,
        });
    }

    // Plain slug with optional @host[:port] suffix.
    if let Some(at) = code.find('@') {
        let raw_words = &code[..at];
        if raw_words.is_empty() {
            return Err("invite code words must not be empty".into());
        }
        let words = validate_and_normalize_words(raw_words)?;
        let host_part = &code[at + 1..];
        let (host, port) = split_host_port(host_part)?;
        Ok(ParsedInviteCode {
            words,
            server: Some((host, port)),
            relay_url: None,
            game_id: None,
            gate_npub: None,
        })
    } else {
        let words = validate_and_normalize_words(code)?;
        Ok(ParsedInviteCode {
            words,
            server: None,
            relay_url: None,
            game_id: None,
            gate_npub: None,
        })
    }
}

/// Validate that `s` is a `word-word` invite code slug (exactly two runs of
/// one or more ASCII lowercase letters joined by a single hyphen), normalize
/// to lowercase, and return the canonical form.
fn validate_and_normalize_words(s: &str) -> Result<String, String> {
    let lower = s.to_lowercase();
    // Must contain exactly one hyphen.
    let hyphen_count = lower.chars().filter(|&c| c == '-').count();
    if hyphen_count != 1 {
        return Err(format!(
            "invite code must be two words joined by a single hyphen (got '{s}')"
        ));
    }
    let idx = lower.find('-').unwrap();
    let left = &lower[..idx];
    let right = &lower[idx + 1..];
    if left.is_empty() || right.is_empty() {
        return Err(format!("invite code words must not be empty (got '{s}')"));
    }
    if !left.chars().all(|c| c.is_ascii_lowercase())
        || !right.chars().all(|c| c.is_ascii_lowercase())
    {
        return Err(format!(
            "invite code words must contain only lowercase letters (got '{s}')"
        ));
    }
    Ok(lower)
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

    // For bech32 invites all coordinates are embedded — no config needed.
    // For plain invites, derive relay from the server host or config.
    let (server_host, server_port, relay_url) = if let Some(ref relay) = parsed.relay_url {
        let (host, port) = resolve_server_coords(
            parsed.server.as_ref().map(|(h, p)| (h.as_str(), *p)),
            config,
        )?;
        (host, port, relay.clone())
    } else {
        let (host, port) = resolve_server_coords(
            parsed.server.as_ref().map(|(h, p)| (h.as_str(), *p)),
            config,
        )?;
        let relay = pick_relay_url(&host, config);
        (host, port, relay)
    };

    Ok(ResolvedTarget {
        server_host,
        server_port,
        relay_url,
        invite_code: Some(parsed.words),
        game_id: parsed.game_id,
        gate_npub: parsed.gate_npub,
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
            gate_npub: None,
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
        gate_npub: None,
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
    if let Some(relay) = config.default_relay_url() {
        relay.to_string()
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
