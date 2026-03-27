//! Session request type parsed from a 30501 Nostr event.
//!
//! This module only handles parsing. Routing and provisioning live in later
//! steps (steps 6–8).

use std::time::{SystemTime, UNIX_EPOCH};

use nostr_sdk::Event;

/// Maximum age of a 30501 SessionRequest event before it is rejected.
///
/// This prevents replay attacks using captured events.
pub const MAX_EVENT_AGE_SECS: u64 = 60;

/// A parsed 30501 SessionRequest from a player.
#[derive(Debug, Clone)]
pub struct SessionRequest {
    /// Session nonce from the `d` tag. Used to correlate response events.
    pub nonce: String,
    /// Player's Nostr public key (hex, 64 chars).
    pub player_pubkey: String,
    /// Ephemeral SSH public key, OpenSSH `ssh-ed25519` format.
    pub ssh_pubkey: String,
    /// Invite code from the event content, if present (first-time join).
    pub invite_code: Option<String>,
    /// Game ID from the `game-id` tag, if present (returning player disambiguation).
    pub game_id: Option<String>,
}

/// Errors that can occur when parsing a 30501 event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Wrong event kind (not 30501).
    WrongKind(u16),
    /// Event signature is invalid.
    InvalidSignature,
    /// Event is too old (replay prevention).
    Stale,
    /// Required `d` tag (nonce) is missing or empty.
    MissingNonce,
    /// Required `ssh-pubkey` tag is missing or empty.
    MissingSshPubkey,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::WrongKind(k) => write!(f, "expected kind 30501, got {k}"),
            ParseError::InvalidSignature => write!(f, "event signature invalid"),
            ParseError::Stale => write!(f, "event is too old (replay prevention)"),
            ParseError::MissingNonce => write!(f, "missing or empty `d` tag (nonce)"),
            ParseError::MissingSshPubkey => write!(f, "missing or empty `ssh-pubkey` tag"),
        }
    }
}

/// Parse and validate a raw 30501 event.
///
/// Checks:
/// - kind == 30501
/// - signature valid
/// - `created_at` within `MAX_EVENT_AGE_SECS` of now
/// - `d` tag present and non-empty
/// - `ssh-pubkey` tag present and non-empty
///
/// The `p` tag (gate npub target) and optional `game-id` tag are extracted
/// without validation — the caller is responsible for confirming the `p` tag
/// matches this daemon's public key.
pub fn parse_session_request(event: &Event) -> Result<SessionRequest, ParseError> {
    // Kind check.
    let kind_u16 = event.kind.as_u16();
    if kind_u16 != 30501 {
        return Err(ParseError::WrongKind(kind_u16));
    }

    // Signature check.
    if !event.verify_signature() {
        return Err(ParseError::InvalidSignature);
    }

    // Staleness check.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let event_ts = event.created_at.as_secs();
    let age = now.saturating_sub(event_ts);
    if age > MAX_EVENT_AGE_SECS {
        return Err(ParseError::Stale);
    }

    // Extract `d` tag (nonce).
    let nonce = tag_content(&event.tags, "d")
        .filter(|s| !s.is_empty())
        .ok_or(ParseError::MissingNonce)?
        .to_string();

    // Extract `ssh-pubkey` tag.
    let ssh_pubkey = tag_content(&event.tags, "ssh-pubkey")
        .filter(|s| !s.is_empty())
        .ok_or(ParseError::MissingSshPubkey)?
        .to_string();

    // Extract player pubkey.
    let player_pubkey = event.pubkey.to_hex();

    // Extract invite code from content (empty string → None).
    let invite_code = {
        let c = event.content.trim().to_string();
        if c.is_empty() { None } else { Some(c) }
    };

    // Extract optional `game-id` tag.
    let game_id = tag_content(&event.tags, "game-id")
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    Ok(SessionRequest {
        nonce,
        player_pubkey,
        ssh_pubkey,
        invite_code,
        game_id,
    })
}

// --- helpers ---

/// Find the first tag with the given name and return its content (index 1).
fn tag_content<'a>(tags: &'a nostr_sdk::event::Tags, name: &str) -> Option<&'a str> {
    tags.iter().find_map(|t| {
        if t.kind().as_str() == name {
            t.content()
        } else {
            None
        }
    })
}
