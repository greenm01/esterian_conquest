use std::time::Duration;

use nostr_sdk::nips::nip44;
use nostr_sdk::{Client, EventBuilder, Keys, Kind, PublicKey, Tag, Timestamp};
use serde::{Deserialize, Serialize};

use crate::connect::handshake::{HandshakeResult, run_handshake};
use crate::connect::live_response::{
    build_response_filter, is_matching_response_event, wait_for_matching_response,
};
use crate::connect::resolve::ResolvedTarget;
use crate::connect::ssh_key::EphemeralKeypair;
use ec_nostr::nonce::random_nonce_hex;

pub const SESSION_STATE_TIMEOUT_SECS: u64 = 10;

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

pub async fn fetch_session_state(
    player_keys: &Keys,
    target: &ResolvedTarget,
    gate_npub: &str,
    game_id: &str,
) -> Result<SessionStatePayload, String> {
    let gate_pubkey = PublicKey::parse(gate_npub).map_err(|e| format!("gate key: {e}"))?;
    let nonce = random_nonce_hex();

    let client = Client::new(player_keys.clone());
    client
        .add_relay(&target.relay_url)
        .await
        .map_err(|e| format!("add relay: {e}"))?;
    client.connect().await;

    let response_kinds = [Kind::Custom(30508), Kind::Custom(30509)];
    let response_filter = build_response_filter(
        &gate_pubkey,
        &player_keys.public_key(),
        response_kinds,
        Timestamp::now() - Duration::from_secs(60),
    );
    let mut notifications = client.notifications();
    let subscription_id = client
        .subscribe(response_filter, None)
        .await
        .map_err(|e| format!("subscribe: {e}"))?
        .val;

    let publish_result =
        publish_session_state_request(&client, player_keys, &gate_pubkey, &nonce, game_id).await;
    if let Err(err) = publish_result {
        client.unsubscribe(&subscription_id).await;
        client.disconnect().await;
        return Err(err.to_string());
    }

    let timeout = Duration::from_secs(SESSION_STATE_TIMEOUT_SECS);
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
        let plaintext = nip44::decrypt(player_keys.secret_key(), &event.pubkey, &event.content)
            .map_err(|e| format!("decrypt session state payload: {e}"))?;

        return match event.kind.as_u16() {
            30508 => serde_json::from_str::<SessionStatePayload>(&plaintext)
                .map_err(|e| format!("parse session state: {e}")),
            30509 => {
                let err = serde_json::from_str::<SessionStateErrorPayload>(&plaintext)
                    .map_err(|e| format!("parse session state error: {e}"))?;
                Err(format!("{}: {}", err.error, err.message))
            }
            other => Err(format!("unexpected session state response kind: {other}")),
        };
    }

    Err("session state refresh timed out (no response from server)".into())
}

pub async fn fetch_game_metadata(
    player_keys: &Keys,
    target: &ResolvedTarget,
    gate_npub: &str,
    game_id: &str,
) -> Result<SessionStatePayload, String> {
    match fetch_session_state(player_keys, target, gate_npub, game_id).await {
        Ok(payload) => Ok(payload),
        Err(_) => fetch_game_metadata_via_handshake(player_keys, target, gate_npub, game_id).await,
    }
}

async fn publish_session_state_request(
    client: &Client,
    player_keys: &Keys,
    gate_pubkey: &PublicKey,
    nonce: &str,
    game_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let tags = vec![
        Tag::parse(["d", nonce])?,
        Tag::parse(["p", &gate_pubkey.to_hex()])?,
        Tag::parse(["game-id", game_id])?,
    ];
    let event = EventBuilder::new(Kind::Custom(30507), "")
        .tags(tags)
        .sign_with_keys(player_keys)?;
    client.send_event(&event).await?;
    Ok(())
}

async fn fetch_game_metadata_via_handshake(
    player_keys: &Keys,
    target: &ResolvedTarget,
    gate_npub: &str,
    game_id: &str,
) -> Result<SessionStatePayload, String> {
    let mut retry_target = target.clone();
    retry_target.game_id = Some(game_id.to_string());
    let keypair = EphemeralKeypair::generate();

    match run_handshake(
        player_keys,
        &retry_target,
        &keypair,
        Some(game_id),
        gate_npub,
    )
    .await
    {
        Ok(HandshakeResult::Ready(payload)) => Ok(SessionStatePayload {
            game_id: payload.game_id,
            game_name: payload.game_name,
            seat: payload.seat,
            player_name: payload.player_name,
        }),
        Ok(HandshakeResult::Error(err)) => Err(format!("{}: {}", err.error, err.message)),
        Ok(HandshakeResult::Timeout) => Err("game info refresh timed out.".to_string()),
        Err(err) => Err(format!("game info refresh failed: {err}")),
    }
}
