use nc_data::hosted::{list_thread_messages, list_thread_players, store_thread_message, HostedStore};
use nc_nostr::thread_message::{build_thread_message_tags, SenderRole, SysopThreadMessage};

pub fn store_player_message(
    store: &HostedStore,
    game_id: &str,
    message: &SysopThreadMessage,
) -> Result<(), Box<dyn std::error::Error>> {
    store_thread_message(
        store.connection(),
        &message.message_id,
        game_id,
        &message.sender_npub,
        SenderRole::Player.as_str(),
        &message.sender_npub,
        message.sender_handle.as_deref(),
        &message.body,
    )?;
    Ok(())
}

pub fn enqueue_sysop_message(
    store: &HostedStore,
    game_id: &str,
    player_pubkey: &str,
    sender_pubkey: &str,
    sender_handle: Option<&str>,
    body: &str,
    message_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let payload = SysopThreadMessage {
        message_id: message_id.to_string(),
        game_id: game_id.to_string(),
        sender_role: SenderRole::Sysop,
        sender_npub: sender_pubkey.to_string(),
        sender_handle: sender_handle.map(str::to_string),
        body: body.to_string(),
        created_at: chrono::Utc::now().timestamp(),
    };
    store_thread_message(
        store.connection(),
        &payload.message_id,
        game_id,
        player_pubkey,
        payload.sender_role.as_str(),
        &payload.sender_npub,
        payload.sender_handle.as_deref(),
        &payload.body,
    )?;

    let content = serde_json::to_string(&payload)?;
    let tags = build_thread_message_tags(&payload)
        .into_iter()
        .map(|(key, value)| vec![key.to_string(), value])
        .collect();

    crate::game::outbox::enqueue_encrypted_event(
        store.connection(),
        game_id,
        player_pubkey,
        30517,
        &content,
        tags,
    )?;
    Ok(())
}

pub fn list_players(
    store: &HostedStore,
    game_id: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(list_thread_players(store.connection(), game_id)?)
}

pub fn list_messages(
    store: &HostedStore,
    game_id: &str,
    player_pubkey: &str,
) -> Result<Vec<nc_data::hosted::HostedThreadMessage>, Box<dyn std::error::Error>> {
    Ok(list_thread_messages(store.connection(), game_id, player_pubkey)?)
}
