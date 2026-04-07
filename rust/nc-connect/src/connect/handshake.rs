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
//! Payload types live in `nc-nostr`; this module owns the async relay flow.

use std::time::Duration;

use nc_nostr::nonce::random_nonce_hex;
use nc_nostr::session::build_session_request_event;
pub use nc_nostr::session::{
    GameEntry, SessionErrorPayload, SessionReadyPayload, SessionUiMode, parse_session_error,
    parse_session_ready,
};
use nostr_sdk::nips::nip44;
use nostr_sdk::{Client, Keys, Kind, PublicKey, Timestamp};

use crate::connect::live_response::{
    build_response_filter, is_matching_response_event, wait_for_matching_response,
};
use crate::connect::resolve::ResolvedTarget;
use crate::connect::ssh_key::EphemeralKeypair;

// ── Constants ─────────────────────────────────────────────────────────────────

/// How long to wait for a 30502/30503 response before giving up.
pub const HANDSHAKE_TIMEOUT_SECS: u64 = 15;

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
/// - `player_keys` — the player's Nostr keypair (from keychain)
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

    let response_kinds = [Kind::Custom(30502), Kind::Custom(30503)];
    let response_filter = build_response_filter(
        &gate_pubkey,
        &player_keys.public_key(),
        response_kinds,
        Timestamp::now() - Duration::from_secs(60),
    );
    let mut notifications = client.notifications();
    let subscription_id = client.subscribe(response_filter, None).await?.val;

    // Publish the 30501 SessionRequest.
    let publish_result = publish_session_request(
        &client,
        player_keys,
        &gate_pubkey,
        &nonce,
        keypair,
        target.invite_code.as_deref(),
        game_id,
    )
    .await;
    if let Err(err) = publish_result {
        client.unsubscribe(&subscription_id).await;
        client.disconnect().await;
        return Err(err);
    }

    // Wait for the matching response.
    let timeout = Duration::from_secs(HANDSHAKE_TIMEOUT_SECS);
    let event =
        wait_for_matching_response(&mut notifications, &subscription_id, timeout, |event| {
            is_matching_response_event(
                event,
                &response_kinds,
                &gate_pubkey,
                &player_keys.public_key(),
                &nonce,
            )
        })
        .await;

    client.unsubscribe(&subscription_id).await;
    client.disconnect().await;

    if let Some(event) = event {
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
    let event = build_session_request_event(
        player_keys,
        gate_pubkey,
        nonce,
        &keypair.openssh_pubkey_string(),
        invite_code,
        game_id,
    )?;
    client.send_event(&event).await?;
    Ok(())
}
