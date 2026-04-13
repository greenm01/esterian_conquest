use std::time::Duration;

use nc_nostr::claim::build_seat_claim_request_event;
pub use nc_nostr::claim::{SeatClaimErrorPayload, parse_seat_claim_error};
use nc_nostr::nonce::random_nonce_hex;
use nostr_sdk::nips::nip44;
use nostr_sdk::{Client, Filter, Keys, Kind, PublicKey, RelayPoolNotification, Timestamp};
use tokio::sync::broadcast;
use tokio::time::{Instant, sleep_until};

use crate::connect::game_discovery::{DiscoveredGame, parse_game_definition};
use crate::connect::live_response::{build_response_filter, is_matching_response_event};
use crate::connect::resolve::ResolvedTarget;

pub const SEAT_CLAIM_TIMEOUT_SECS: u64 = 15;
const CLAIM_RESUBSCRIBE_DELAYS_SECS: [u64; 3] = [2, 5, 10];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeatClaimResult {
    Claimed(ClaimedSeat),
    Error(SeatClaimErrorPayload),
    Timeout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedSeat {
    pub game_id: String,
    pub game_name: String,
    pub seat: u32,
    pub gate_npub: String,
}

pub async fn claim_seat_and_wait(
    player_keys: &Keys,
    target: &ResolvedTarget,
    invite_code: &str,
    discovered: &DiscoveredGame,
) -> Result<SeatClaimResult, Box<dyn std::error::Error + Send + Sync>> {
    let gate_pubkey = PublicKey::parse(&discovered.gate_npub)?;
    let nonce = random_nonce_hex();

    let client = Client::new(player_keys.clone());
    client.add_relay(&target.relay_url).await?;
    client.connect().await;

    let mut notifications = client.notifications();
    let mut game_subscription_id = subscribe_game_updates(&client, &gate_pubkey).await?;
    let error_kinds = [Kind::Custom(30511)];
    let error_filter = build_response_filter(
        &gate_pubkey,
        &player_keys.public_key(),
        error_kinds,
        Timestamp::now() - Duration::from_secs(60),
    );
    let error_subscription_id = client.subscribe(error_filter, None).await?.val;

    let publish_result = publish_seat_claim_request(
        &client,
        player_keys,
        &gate_pubkey,
        &nonce,
        invite_code,
        Some(&discovered.game_id),
    )
    .await;
    if let Err(err) = publish_result {
        client.unsubscribe(&game_subscription_id).await;
        client.unsubscribe(&error_subscription_id).await;
        client.disconnect().await;
        return Err(err);
    }

    let result = wait_for_claim_result(
        &client,
        &mut notifications,
        &mut game_subscription_id,
        &error_subscription_id,
        player_keys,
        &gate_pubkey,
        &nonce,
        discovered,
    )
    .await?;

    client.unsubscribe(&game_subscription_id).await;
    client.unsubscribe(&error_subscription_id).await;
    client.disconnect().await;

    Ok(result)
}

async fn wait_for_claim_result(
    client: &Client,
    notifications: &mut broadcast::Receiver<RelayPoolNotification>,
    game_subscription_id: &mut nostr_sdk::SubscriptionId,
    error_subscription_id: &nostr_sdk::SubscriptionId,
    player_keys: &Keys,
    gate_pubkey: &PublicKey,
    nonce: &str,
    discovered: &DiscoveredGame,
) -> Result<SeatClaimResult, Box<dyn std::error::Error + Send + Sync>> {
    let deadline = Instant::now() + Duration::from_secs(SEAT_CLAIM_TIMEOUT_SECS);
    let mut next_retry = CLAIM_RESUBSCRIBE_DELAYS_SECS
        .iter()
        .copied()
        .map(|seconds| Instant::now() + Duration::from_secs(seconds))
        .collect::<Vec<_>>();

    loop {
        if Instant::now() >= deadline {
            return Ok(SeatClaimResult::Timeout);
        }

        let retry_at = next_retry.first().copied();
        tokio::select! {
            _ = sleep_until(deadline) => return Ok(SeatClaimResult::Timeout),
            _ = async {
                if let Some(retry_at) = retry_at {
                    sleep_until(retry_at).await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => {
                if retry_at.is_some() {
                    client.unsubscribe(game_subscription_id).await;
                    *game_subscription_id = subscribe_game_updates(client, gate_pubkey).await?;
                    next_retry.remove(0);
                }
            }
            notification = notifications.recv() => {
                match notification {
                    Ok(RelayPoolNotification::Event { subscription_id, event, .. }) => {
                        if subscription_id == *error_subscription_id
                            && is_matching_response_event(
                                &event,
                                &[Kind::Custom(30511)],
                                gate_pubkey,
                                &player_keys.public_key(),
                                nonce,
                            )
                        {
                            let plaintext = nip44::decrypt(
                                player_keys.secret_key(),
                                &event.pubkey,
                                &event.content,
                            )?;
                            return Ok(SeatClaimResult::Error(
                                parse_seat_claim_error(&plaintext)
                                    .map_err(|err| -> Box<dyn std::error::Error + Send + Sync> {
                                        Box::new(std::io::Error::other(err))
                                    })?,
                            ));
                        }

                        if subscription_id == *game_subscription_id
                            && game_definition_claimed(&event, discovered, &player_keys.public_key().to_hex())
                        {
                            return Ok(SeatClaimResult::Claimed(ClaimedSeat {
                                game_id: discovered.game_id.clone(),
                                game_name: discovered.game_name.clone(),
                                seat: discovered.seat,
                                gate_npub: discovered.gate_npub.clone(),
                            }));
                        }
                    }
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => return Ok(SeatClaimResult::Timeout),
                }
            }
        }
    }
}

fn game_definition_claimed(
    event: &nostr_sdk::Event,
    discovered: &DiscoveredGame,
    player_pubkey_hex: &str,
) -> bool {
    let Some(game) = parse_game_definition(event) else {
        return false;
    };
    if game.game_id != discovered.game_id {
        return false;
    }
    game.slots.iter().any(|slot| {
        slot.seat == discovered.seat
            && slot.status == "claimed"
            && slot.player_npub.as_deref() == Some(player_pubkey_hex)
    })
}

async fn subscribe_game_updates(
    client: &Client,
    gate_pubkey: &PublicKey,
) -> Result<nostr_sdk::SubscriptionId, Box<dyn std::error::Error + Send + Sync>> {
    let filter = Filter::new()
        .kinds([Kind::Custom(30500)])
        .author(gate_pubkey.clone())
        .since(Timestamp::now() - Duration::from_secs(3600));
    Ok(client.subscribe(filter, None).await?.val)
}

async fn publish_seat_claim_request(
    client: &Client,
    player_keys: &Keys,
    gate_pubkey: &PublicKey,
    nonce: &str,
    invite_code: &str,
    game_id: Option<&str>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let event = build_seat_claim_request_event(
        player_keys,
        gate_pubkey,
        nonce,
        invite_code,
        game_id,
        None,
    )?;
    client.send_event(&event).await?;
    Ok(())
}
