use crate::lobby::publish::EventPublisher;
use nc_nostr::invite_request::{InviteDecision, InviteDecisionPayload};

pub async fn publish_invite_decision(
    publisher: &EventPublisher,
    player_pubkey: &str,
    request_id: &str,
    game_id: &str,
    decision: InviteDecision,
    message: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let payload = InviteDecisionPayload {
        request_id: request_id.to_string(),
        game_id: game_id.to_string(),
        decision,
        message: message.to_string(),
    };

    let content = serde_json::to_string(&payload)?;

    let tag_refs: Vec<(&str, &str)> = vec![
        ("d", request_id),
        ("game-id", game_id),
    ];

    publisher
        .publish_encrypted(player_pubkey, 30515, &content, tag_refs)
        .await?;

    tracing::info!(
        "Published encrypted invite decision {} to {}",
        request_id,
        player_pubkey
    );

    Ok(())
}
