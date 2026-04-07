//! Seat-claim request/error helpers re-exported from `nc-nostr`.

use nostr_sdk::{Client, Keys, PublicKey};

pub use nc_nostr::claim::{
    ParseSeatClaimError, SeatClaimErrorPayload, SeatClaimRequest, parse_seat_claim_request,
};

pub async fn publish_seat_claim_error(
    client: &Client,
    gate_keys: &Keys,
    player_pubkey: &PublicKey,
    nonce: &str,
    code: &str,
    message: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let payload = SeatClaimErrorPayload {
        error: code.to_string(),
        message: message.to_string(),
    };
    let event = nc_nostr::claim::build_seat_claim_error_event(
        gate_keys,
        player_pubkey,
        nonce,
        &payload,
    )?;
    let event_id = event.id.to_hex();
    client.send_event(&event).await?;
    Ok(event_id)
}

pub fn seat_claim_error_payload(code: &str, message: &str) -> String {
    SeatClaimErrorPayload {
        error: code.to_string(),
        message: message.to_string(),
    }
    .to_json()
}
