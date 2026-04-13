use nc_data::hosted::{HostedStore, store_player_message};
use nc_nostr::player_message::{PlayerMessage, build_player_message_tags};

pub fn store_message(
    store: &HostedStore,
    game_id: &str,
    message: &PlayerMessage,
    sender_pubkey: &str,
    recipient_pubkey: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    store_player_message(
        store.connection(),
        &message.message_id,
        game_id,
        sender_pubkey,
        message.sender_empire_id,
        &message.sender_empire_name,
        recipient_pubkey,
        message.recipient_empire_id,
        &message.recipient_empire_name,
        &message.body,
    )?;
    Ok(())
}

pub fn enqueue_message(
    store: &HostedStore,
    game_id: &str,
    recipient_pubkey: &str,
    message: &PlayerMessage,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = serde_json::to_string(message)?;
    let tags = build_player_message_tags(message)
        .into_iter()
        .map(|(key, value)| vec![key.to_string(), value])
        .collect();
    crate::game::outbox::enqueue_encrypted_event(
        store.connection(),
        game_id,
        recipient_pubkey,
        30523,
        &content,
        tags,
    )?;
    Ok(())
}
