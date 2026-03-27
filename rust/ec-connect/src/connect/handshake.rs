//! Nostr session handshake client.
//!
//! This module handles the full async handshake sequence:
//!
//! 1. Connect to the Nostr relay.
//! 2. Subscribe to kind 30502 (SessionReady) and 30503 (SessionError)
//!    filtered to the player's own pubkey.
//! 3. Publish a 30501 SessionRequest with the ephemeral SSH public key.
//! 4. Wait (up to `HANDSHAKE_TIMEOUT_SECS`) for a matching response.
//! 5. Decrypt and parse the NIP-44 encrypted payload.
//! 6. Disconnect from the relay.
//!
//! Payload types (`SessionReadyPayload`, `SessionErrorPayload`,
//! `GameEntry`) are plain data structs.  JSON parsing is done by hand
//! using a minimal helper so we avoid pulling in `serde_json`.

use std::time::Duration;

use nostr_sdk::nips::nip44;
use nostr_sdk::{Client, EventBuilder, Filter, Keys, Kind, PublicKey, Tag};
use rand::RngCore;
use rand::rngs::OsRng;

use crate::connect::resolve::ResolvedTarget;
use crate::connect::ssh_key::EphemeralKeypair;

// ── Constants ─────────────────────────────────────────────────────────────────

/// How long to wait for a 30502/30503 response before giving up.
pub const HANDSHAKE_TIMEOUT_SECS: u64 = 15;

// ── Payload types ─────────────────────────────────────────────────────────────

/// Decrypted payload from a 30502 SessionReady event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionReadyPayload {
    pub game_id: String,
    pub ssh_host: String,
    pub ssh_port: u16,
    /// SSH username to authenticate as. Empty string if not present (old gate version).
    pub ssh_user: String,
    /// SSH server host-key fingerprint for verification, e.g. `"SHA256:…"`.
    /// Empty string if not present (old gate version).
    pub host_fingerprint: String,
    pub game_name: String,
    pub seat: u32,
    pub player_name: String,
}

/// One entry in a `multiple_games` error list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameEntry {
    pub game_id: String,
    pub name: String,
    pub seat: u32,
}

/// Decrypted payload from a 30503 SessionError event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionErrorPayload {
    pub error: String,
    pub message: String,
    /// Only present when `error == "multiple_games"`.
    pub games: Vec<GameEntry>,
}

/// Outcome of a completed handshake attempt.
#[derive(Debug)]
pub enum HandshakeResult {
    Ready(SessionReadyPayload),
    Error(SessionErrorPayload),
    Timeout,
}

// ── Public async entry point ──────────────────────────────────────────────────

/// Run the full Nostr session handshake and return the result.
///
/// # Arguments
///
/// - `player_keys` — the player's Nostr keypair (from wallet)
/// - `target` — resolved server + relay coordinates
/// - `keypair` — ephemeral SSH keypair for this session
/// - `game_id` — optional game-id hint (from cache) to include in 30501
/// - `gate_npub` — gate's Nostr public key as an npub or hex string; must
///   be resolved before calling (e.g. from a prior 30500 query or cache)
pub async fn run_handshake(
    player_keys: &Keys,
    target: &ResolvedTarget,
    keypair: &EphemeralKeypair,
    game_id: Option<&str>,
    gate_npub: &str,
) -> Result<HandshakeResult, Box<dyn std::error::Error + Send + Sync>> {
    // Parse gate pubkey.
    let gate_pubkey = PublicKey::parse(gate_npub)?;

    // Generate session nonce: 32 random bytes as lowercase hex.
    let nonce = random_nonce_hex();

    // Build the client.
    let client = Client::new(player_keys.clone());
    client.add_relay(&target.relay_url).await?;
    client.connect().await;

    // Subscribe to 30502 and 30503 events addressed to us.
    let response_filter = Filter::new()
        .kinds([Kind::Custom(30502), Kind::Custom(30503)])
        .pubkey(player_keys.public_key());
    client.subscribe(response_filter, None).await?;

    // Publish the 30501 SessionRequest.
    publish_session_request(
        &client,
        player_keys,
        &gate_pubkey,
        &nonce,
        keypair,
        target.invite_code.as_deref(),
        game_id,
    )
    .await?;

    // Wait for the matching response.
    let timeout = Duration::from_secs(HANDSHAKE_TIMEOUT_SECS);
    let events = client
        .fetch_events(
            Filter::new()
                .kinds([Kind::Custom(30502), Kind::Custom(30503)])
                .pubkey(player_keys.public_key()),
            timeout,
        )
        .await?;

    client.disconnect().await;

    // Find the event whose `d` tag matches our nonce.
    for event in events.iter() {
        let d = tag_value(event.tags.iter(), "d");
        if d.as_deref() != Some(nonce.as_str()) {
            continue;
        }

        let kind = event.kind.as_u16();
        let plaintext = nip44::decrypt(player_keys.secret_key(), &event.pubkey, &event.content)?;

        if kind == 30502 {
            let payload = parse_session_ready(&plaintext)?;
            return Ok(HandshakeResult::Ready(payload));
        } else if kind == 30503 {
            let payload = parse_session_error(&plaintext)?;
            return Ok(HandshakeResult::Error(payload));
        }
    }

    Ok(HandshakeResult::Timeout)
}

// ── Event construction ────────────────────────────────────────────────────────

async fn publish_session_request(
    client: &Client,
    player_keys: &Keys,
    gate_pubkey: &PublicKey,
    nonce: &str,
    keypair: &EphemeralKeypair,
    invite_code: Option<&str>,
    game_id: Option<&str>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut tags = vec![
        Tag::parse(["d", nonce])?,
        Tag::parse(["p", &gate_pubkey.to_hex()])?,
        Tag::parse(["ssh-pubkey", &keypair.openssh_pubkey_string()])?,
    ];
    if let Some(gid) = game_id {
        tags.push(Tag::parse(["game-id", gid])?);
    }

    let content = invite_code.unwrap_or("");
    let event = EventBuilder::new(Kind::Custom(30501), content)
        .tags(tags)
        .sign_with_keys(player_keys)?;

    client.send_event(&event).await?;
    Ok(())
}

// ── Payload parsing ───────────────────────────────────────────────────────────

/// Parse a 30502 SessionReady JSON payload.
///
/// Expected shape (all fields required except `host_fingerprint`,
/// `player_name`):
/// ```json
/// {"game_id":"...","ssh_host":"...","ssh_port":22,"ssh_user":"...",
///  "host_fingerprint":"...",
///  "game_name":"...","seat":2,"player_name":"..."}
/// ```
pub fn parse_session_ready(json: &str) -> Result<SessionReadyPayload, String> {
    let game_id = extract_str(json, "game_id")?;
    let ssh_host = extract_str(json, "ssh_host")?;
    let ssh_port = extract_u32(json, "ssh_port")
        .map(|v| v as u16)
        .ok_or("missing or invalid ssh_port")?;
    let ssh_user = extract_str(json, "ssh_user").unwrap_or_default();
    let host_fingerprint = extract_str(json, "host_fingerprint").unwrap_or_default();
    let game_name = extract_str(json, "game_name")?;
    let seat = extract_u32(json, "seat").ok_or("missing or invalid seat")?;
    let player_name = extract_str(json, "player_name").unwrap_or_default();

    Ok(SessionReadyPayload {
        game_id,
        ssh_host,
        ssh_port,
        ssh_user,
        host_fingerprint,
        game_name,
        seat,
        player_name,
    })
}

/// Parse a 30503 SessionError JSON payload.
///
/// Minimal expected shape:
/// ```json
/// {"error":"invalid_code","message":"..."}
/// ```
/// Optional for `multiple_games`:
/// ```json
/// {"error":"multiple_games","message":"...","games":[...]}
/// ```
pub fn parse_session_error(json: &str) -> Result<SessionErrorPayload, String> {
    let error = extract_str(json, "error")?;
    let message = extract_str(json, "message")?;
    let games = if error == "multiple_games" {
        parse_game_entries(json)
    } else {
        Vec::new()
    };
    Ok(SessionErrorPayload {
        error,
        message,
        games,
    })
}

// ── Minimal JSON field extraction ─────────────────────────────────────────────

/// Extract a quoted string field value by key.
///
/// Handles basic JSON escapes `\\` and `\"` within the value.
fn extract_str(json: &str, key: &str) -> Result<String, String> {
    let needle = format!("\"{}\"", key);
    let key_pos = json
        .find(&needle)
        .ok_or_else(|| format!("missing field '{key}'"))?;
    let after_key = &json[key_pos + needle.len()..];
    // Skip whitespace and colon.
    let colon_pos = after_key
        .find(':')
        .ok_or_else(|| format!("malformed field '{key}'"))?;
    let after_colon = after_key[colon_pos + 1..].trim_start();
    if !after_colon.starts_with('"') {
        return Err(format!("field '{key}' is not a string"));
    }
    let inner = &after_colon[1..];
    let mut value = String::new();
    let mut chars = inner.chars();
    loop {
        match chars.next() {
            None => return Err(format!("unterminated string for field '{key}'")),
            Some('"') => break,
            Some('\\') => match chars.next() {
                Some('"') => value.push('"'),
                Some('\\') => value.push('\\'),
                Some('n') => value.push('\n'),
                Some('r') => value.push('\r'),
                Some('t') => value.push('\t'),
                Some(c) => {
                    value.push('\\');
                    value.push(c);
                }
                None => return Err(format!("truncated escape in field '{key}'")),
            },
            Some(c) => value.push(c),
        }
    }
    Ok(value)
}

/// Extract a numeric (unsigned integer) field value by key.
fn extract_u32(json: &str, key: &str) -> Option<u32> {
    let needle = format!("\"{}\"", key);
    let key_pos = json.find(&needle)?;
    let after_key = &json[key_pos + needle.len()..];
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();
    let end = after_colon
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(after_colon.len());
    after_colon[..end].parse().ok()
}

/// Parse the `games` array in a `multiple_games` error payload.
///
/// Each element has the shape:
/// `{"game_id":"...","name":"...","seat":N}`
fn parse_game_entries(json: &str) -> Vec<GameEntry> {
    let mut entries = Vec::new();
    // Find the games array.
    let Some(arr_start) = json.find("\"games\"") else {
        return entries;
    };
    let after = &json[arr_start + 7..]; // skip `"games"`
    let Some(bracket) = after.find('[') else {
        return entries;
    };
    let arr_body = &after[bracket + 1..];

    // Walk through `{...}` objects.
    let mut remaining = arr_body;
    while let Some(obj_start) = remaining.find('{') {
        let body = &remaining[obj_start + 1..];
        let Some(obj_end) = body.find('}') else {
            break;
        };
        let obj = &body[..obj_end];
        // Wrap in braces for reuse of extract_str/extract_u32.
        let wrapped = format!("{{{obj}}}");
        let game_id = extract_str(&wrapped, "game_id").unwrap_or_default();
        let name = extract_str(&wrapped, "name").unwrap_or_default();
        let seat = extract_u32(&wrapped, "seat").unwrap_or(0);
        if !game_id.is_empty() {
            entries.push(GameEntry {
                game_id,
                name,
                seat,
            });
        }
        remaining = &body[obj_end + 1..];
    }
    entries
}

// ── Nonce + tag helpers ───────────────────────────────────────────────────────

/// Generate a 32-byte random session nonce as a lowercase hex string.
pub fn random_nonce_hex() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Extract the content (index-1 value) of the first tag with the given name.
fn tag_value<'a>(mut tags: impl Iterator<Item = &'a nostr_sdk::Tag>, name: &str) -> Option<String> {
    tags.find_map(|t| {
        if t.kind().as_str() == name {
            t.content().map(str::to_string)
        } else {
            None
        }
    })
}
