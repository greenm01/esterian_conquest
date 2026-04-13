use crate::support::pubkeys::short_pubkey;
use nc_data::hosted::HostedStore;
use nc_nostr::invite_request::{InviteDecision, InviteDecisionPayload, build_invite_decision_tags};

pub fn enqueue_invite_decision(
    store: &HostedStore,
    game_id: &str,
    player_pubkey: &str,
    request_id: &str,
    decision: InviteDecision,
    message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let payload = InviteDecisionPayload {
        request_id: request_id.to_string(),
        game_id: game_id.to_string(),
        decision,
        message: message.to_string(),
    };

    let content = serde_json::to_string(&payload)?;

    let tags = build_invite_decision_tags(&payload)
        .into_iter()
        .map(|(key, value)| vec![key.to_string(), value])
        .collect();

    crate::game::outbox::enqueue_encrypted_event(
        store.connection(),
        game_id,
        player_pubkey,
        30515,
        &content,
        tags,
    )?;

    tracing::info!(
        "Queued encrypted invite decision {} to {}",
        request_id,
        short_pubkey(player_pubkey)
    );

    Ok(())
}
