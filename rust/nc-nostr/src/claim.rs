use crate::json::{escape_json_string, extract_str};
use crate::private_payload::{decrypt_private_json_from_event, encrypt_private_text};
use crate::tags::tag_content;
use crate::timing::is_event_stale;
use nostr_sdk::nips::nip44;
use nostr_sdk::nips::nip44::Version;
use nostr_sdk::{Event, EventBuilder, Keys, Kind, PublicKey, Tag};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeatClaimRequest {
    pub nonce: String,
    pub player_pubkey: String,
    pub invite_code: String,
    pub game_id: Option<String>,
    pub handle: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SeatClaimRequestPayload {
    pub invite: String,
    pub handle: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SeatClaimStatus {
    Claimed,
    InvalidInvite,
    AlreadyClaimed,
}

impl SeatClaimStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Claimed => "claimed",
            Self::InvalidInvite => "invalid_invite",
            Self::AlreadyClaimed => "already_claimed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SeatClaimResultPayload {
    pub nonce: String,
    pub game_id: Option<String>,
    pub status: SeatClaimStatus,
    pub message: String,
    pub seat: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseSeatClaimError {
    WrongKind(u16),
    InvalidSignature,
    Stale,
    MissingNonce,
    InvalidPayload,
    MissingInviteCode,
}

impl std::fmt::Display for ParseSeatClaimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongKind(kind) => write!(f, "expected kind 30510, got {kind}"),
            Self::InvalidSignature => write!(f, "event signature invalid"),
            Self::Stale => write!(f, "event is too old (replay prevention)"),
            Self::MissingNonce => write!(f, "missing or empty `d` tag (nonce)"),
            Self::InvalidPayload => write!(f, "invalid encrypted claim payload"),
            Self::MissingInviteCode => write!(f, "missing invite code"),
        }
    }
}

impl std::error::Error for ParseSeatClaimError {}

pub fn parse_seat_claim_request(
    secret_key: &nostr_sdk::SecretKey,
    event: &Event,
) -> Result<SeatClaimRequest, ParseSeatClaimError> {
    if event.kind.as_u16() != 30510 {
        return Err(ParseSeatClaimError::WrongKind(event.kind.as_u16()));
    }
    if !event.verify_signature() {
        return Err(ParseSeatClaimError::InvalidSignature);
    }
    if is_event_stale(event) {
        return Err(ParseSeatClaimError::Stale);
    }
    let nonce = tag_content(&event.tags, "d")
        .filter(|value| !value.is_empty())
        .ok_or(ParseSeatClaimError::MissingNonce)?
        .to_string();
    let payload: SeatClaimRequestPayload = decrypt_private_json_from_event(secret_key, event)
        .map_err(|_| ParseSeatClaimError::InvalidPayload)?;
    let invite_code = payload.invite.trim().to_ascii_lowercase();
    if invite_code.is_empty() {
        return Err(ParseSeatClaimError::MissingInviteCode);
    }
    Ok(SeatClaimRequest {
        nonce,
        player_pubkey: event.pubkey.to_hex(),
        invite_code,
        game_id: tag_content(&event.tags, "game-id")
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        handle: payload.handle.filter(|value| !value.trim().is_empty()),
    })
}

pub fn build_seat_claim_request_event(
    player_keys: &Keys,
    gate_pubkey: &PublicKey,
    nonce: &str,
    invite_code: &str,
    game_id: Option<&str>,
    handle: Option<&str>,
) -> Result<Event, Box<dyn std::error::Error + Send + Sync>> {
    let mut tags = vec![
        Tag::parse(["d", nonce])?,
        Tag::parse(["p", &gate_pubkey.to_hex()])?,
    ];
    if let Some(game_id) = game_id {
        tags.push(Tag::parse(["game-id", game_id])?);
    }
    let content = encrypt_private_text(
        player_keys,
        gate_pubkey,
        &serde_json::to_string(&SeatClaimRequestPayload {
            invite: invite_code.trim().to_ascii_lowercase(),
            handle: handle
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
        })?,
    )?;
    Ok(EventBuilder::new(Kind::Custom(30510), content)
        .tags(tags)
        .sign_with_keys(player_keys)?)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeatClaimErrorPayload {
    pub error: String,
    pub message: String,
}

impl SeatClaimErrorPayload {
    pub fn to_json(&self) -> String {
        let error = escape_json_string(&self.error);
        let message = escape_json_string(&self.message);
        format!(r#"{{"error":"{error}","message":"{message}"}}"#)
    }
}

pub fn parse_seat_claim_error(json: &str) -> Result<SeatClaimErrorPayload, String> {
    let error = extract_str(json, "error")?;
    let message = extract_str(json, "message")?;
    Ok(SeatClaimErrorPayload { error, message })
}

pub fn build_seat_claim_error_event(
    gate_keys: &Keys,
    player_pubkey: &PublicKey,
    nonce: &str,
    payload: &SeatClaimErrorPayload,
) -> Result<Event, Box<dyn std::error::Error + Send + Sync>> {
    let plaintext = payload.to_json();
    let encrypted = nip44::encrypt(
        gate_keys.secret_key(),
        player_pubkey,
        &plaintext,
        Version::V2,
    )?;
    let tags = vec![
        Tag::parse(["d", nonce])?,
        Tag::parse(["p", &player_pubkey.to_hex()])?,
    ];
    Ok(EventBuilder::new(Kind::Custom(30511), encrypted)
        .tags(tags)
        .sign_with_keys(gate_keys)?)
}

pub fn build_seat_claim_result_tags(
    payload: &SeatClaimResultPayload,
) -> Vec<(&'static str, String)> {
    let mut tags = vec![
        ("d", payload.nonce.clone()),
        ("status", payload.status.as_str().to_string()),
    ];

    if let Some(game_id) = &payload.game_id {
        tags.push(("game-id", game_id.clone()));
    }

    tags
}
