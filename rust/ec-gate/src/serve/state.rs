use ec_nostr::tags::tag_content;
use ec_nostr::timing::is_event_stale;
use nostr_sdk::nips::nip44;
use nostr_sdk::nips::nip44::Version;
use nostr_sdk::{Client, Event, EventBuilder, Keys, Kind, PublicKey, Tag};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct SessionStateRequest {
    pub nonce: String,
    pub player_pubkey: String,
    pub game_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    WrongKind(u16),
    InvalidSignature,
    Stale,
    MissingNonce,
    MissingGameId,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::WrongKind(kind) => write!(f, "expected kind 30507, got {kind}"),
            ParseError::InvalidSignature => write!(f, "event signature invalid"),
            ParseError::Stale => write!(f, "event is too old (replay prevention)"),
            ParseError::MissingNonce => write!(f, "missing or empty `d` tag (nonce)"),
            ParseError::MissingGameId => write!(f, "missing or empty `game-id` tag"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStatePayload {
    pub game_id: String,
    pub game_name: String,
    pub seat: u32,
    pub player_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStateErrorPayload {
    pub error: String,
    pub message: String,
}

pub fn parse_session_state_request(event: &Event) -> Result<SessionStateRequest, ParseError> {
    let kind_u16 = event.kind.as_u16();
    if kind_u16 != 30507 {
        return Err(ParseError::WrongKind(kind_u16));
    }
    if !event.verify_signature() {
        return Err(ParseError::InvalidSignature);
    }

    if is_event_stale(event) {
        return Err(ParseError::Stale);
    }

    let nonce = tag_content(&event.tags, "d")
        .filter(|s| !s.is_empty())
        .ok_or(ParseError::MissingNonce)?
        .to_string();
    let game_id = tag_content(&event.tags, "game-id")
        .filter(|s| !s.is_empty())
        .ok_or(ParseError::MissingGameId)?
        .to_string();

    Ok(SessionStateRequest {
        nonce,
        player_pubkey: event.pubkey.to_hex(),
        game_id,
    })
}

pub async fn publish_session_state(
    client: &Client,
    gate_keys: &Keys,
    player_pubkey: &PublicKey,
    request_nonce: &str,
    payload: &SessionStatePayload,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let plaintext = serde_json::to_string(payload)?;
    let encrypted = nip44::encrypt(
        gate_keys.secret_key(),
        player_pubkey,
        &plaintext,
        Version::V2,
    )?;
    let tags = vec![
        Tag::parse(["d", request_nonce])?,
        Tag::parse(["p", &player_pubkey.to_hex()])?,
    ];
    let event = EventBuilder::new(Kind::Custom(30508), encrypted)
        .tags(tags)
        .sign_with_keys(gate_keys)?;
    client.send_event(&event).await?;
    Ok(event.id.to_hex())
}

pub async fn publish_session_state_error(
    client: &Client,
    gate_keys: &Keys,
    player_pubkey: &PublicKey,
    request_nonce: &str,
    error: &str,
    message: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let payload = SessionStateErrorPayload {
        error: error.to_string(),
        message: message.to_string(),
    };
    let plaintext = serde_json::to_string(&payload)?;
    let encrypted = nip44::encrypt(
        gate_keys.secret_key(),
        player_pubkey,
        &plaintext,
        Version::V2,
    )?;
    let tags = vec![
        Tag::parse(["d", request_nonce])?,
        Tag::parse(["p", &player_pubkey.to_hex()])?,
    ];
    let event = EventBuilder::new(Kind::Custom(30509), encrypted)
        .tags(tags)
        .sign_with_keys(gate_keys)?;
    client.send_event(&event).await?;
    Ok(event.id.to_hex())
}
