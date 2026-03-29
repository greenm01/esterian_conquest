//! 30510 SeatClaimRequest and 30511 SeatClaimError helpers.

use ec_nostr::json::escape_json_string;
use ec_nostr::tags::tag_content;
use ec_nostr::timing::is_event_stale;
use nostr_sdk::nips::nip44;
use nostr_sdk::nips::nip44::Version;
use nostr_sdk::{Client, Event, EventBuilder, Keys, PublicKey, Tag};

/// Parsed 30510 seat-claim request.
#[derive(Debug, Clone)]
pub struct SeatClaimRequest {
    pub nonce: String,
    pub player_pubkey: String,
    pub invite_code: String,
    pub game_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseSeatClaimError {
    WrongKind(u16),
    InvalidSignature,
    Stale,
    MissingNonce,
    MissingInviteCode,
}

impl std::fmt::Display for ParseSeatClaimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongKind(kind) => write!(f, "expected kind 30510, got {kind}"),
            Self::InvalidSignature => write!(f, "event signature invalid"),
            Self::Stale => write!(f, "event is too old (replay prevention)"),
            Self::MissingNonce => write!(f, "missing or empty `d` tag (nonce)"),
            Self::MissingInviteCode => write!(f, "missing invite code"),
        }
    }
}

pub fn parse_seat_claim_request(event: &Event) -> Result<SeatClaimRequest, ParseSeatClaimError> {
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
    let invite_code = event.content.trim().to_ascii_lowercase();
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
    })
}

pub async fn publish_seat_claim_error(
    client: &Client,
    gate_keys: &Keys,
    player_pubkey: &PublicKey,
    nonce: &str,
    code: &str,
    message: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let plaintext = seat_claim_error_payload(code, message);
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
    let event = EventBuilder::new(nostr_sdk::Kind::Custom(30511), encrypted)
        .tags(tags)
        .sign_with_keys(gate_keys)?;
    client.send_event(&event).await?;
    Ok(event.id.to_hex())
}

pub fn seat_claim_error_payload(code: &str, message: &str) -> String {
    let code = escape_json_string(code);
    let message = escape_json_string(message);
    format!(r#"{{"error":"{code}","message":"{message}"}}"#)
}
