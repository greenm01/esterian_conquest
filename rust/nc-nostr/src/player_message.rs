use crate::private_payload::decrypt_private_json_from_event;
use crate::pubkeys::event_pubkey_hex;
use nostr_sdk::{Event, SecretKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerMessageRequest {
    pub message_id: String,
    pub game_id: String,
    pub sender_pubkey: String,
    pub recipient_empire_id: u8,
    pub body: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerMessage {
    pub message_id: String,
    pub game_id: String,
    pub sender_empire_id: u8,
    pub sender_empire_name: String,
    pub recipient_empire_id: u8,
    pub recipient_empire_name: String,
    pub body: String,
    pub created_at: i64,
}

pub fn parse_player_message_request(
    secret_key: &SecretKey,
    event: &Event,
) -> Option<PlayerMessageRequest> {
    let mut request: PlayerMessageRequest =
        decrypt_private_json_from_event(secret_key, event).ok()?;
    if request.message_id.trim().is_empty() {
        request.message_id = extract_tag(event, "d")?;
    }
    if request.game_id.trim().is_empty() {
        request.game_id = extract_tag(event, "game-id")?;
    }
    if request.sender_pubkey.trim().is_empty() {
        request.sender_pubkey = event_pubkey_hex(event);
    }
    if request.created_at == 0 {
        request.created_at = i64::try_from(event.created_at.as_secs()).ok()?;
    }
    Some(request)
}

pub fn decrypt_player_message(secret_key: &SecretKey, event: &Event) -> Option<PlayerMessage> {
    let mut message: PlayerMessage = decrypt_private_json_from_event(secret_key, event).ok()?;
    if message.message_id.trim().is_empty() {
        message.message_id = extract_tag(event, "d")?;
    }
    if message.game_id.trim().is_empty() {
        message.game_id = extract_tag(event, "game-id")?;
    }
    if message.created_at == 0 {
        message.created_at = i64::try_from(event.created_at.as_secs()).ok()?;
    }
    Some(message)
}

pub fn build_player_message_tags(message: &PlayerMessage) -> Vec<(&'static str, String)> {
    vec![
        ("d", message.message_id.clone()),
        ("game-id", message.game_id.clone()),
    ]
}

fn extract_tag(event: &Event, key: &str) -> Option<String> {
    for tag in event.tags.iter() {
        let values = tag.clone().to_vec();
        if values.first().map(String::as_str) == Some(key) && values.len() >= 2 {
            return Some(values[1].clone());
        }
    }
    None
}
