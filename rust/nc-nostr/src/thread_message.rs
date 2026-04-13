use crate::private_payload::decrypt_private_json_from_event;
use crate::pubkeys::{event_pubkey_hex, event_pubkey_npub};
use nostr_sdk::{Event, SecretKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SenderRole {
    Player,
    Sysop,
}

impl SenderRole {
    pub fn as_str(self) -> &'static str {
        match self {
            SenderRole::Player => "player",
            SenderRole::Sysop => "sysop",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SysopThreadMessage {
    pub message_id: String,
    pub game_id: String,
    pub sender_role: SenderRole,
    pub sender_pubkey: String,
    pub sender_npub: String,
    pub sender_handle: Option<String>,
    pub body: String,
    pub created_at: i64,
}

pub fn decrypt_thread_message(secret_key: &SecretKey, event: &Event) -> Option<SysopThreadMessage> {
    let mut message: SysopThreadMessage = decrypt_private_json_from_event(secret_key, event).ok()?;
    if message.message_id.trim().is_empty() {
        message.message_id = extract_tag(event, "d")?;
    }
    if message.game_id.trim().is_empty() {
        message.game_id = extract_tag(event, "game-id")?;
    }
    if message.sender_pubkey.trim().is_empty() {
        message.sender_pubkey = event_pubkey_hex(event);
    }
    if message.sender_npub.trim().is_empty() {
        message.sender_npub = event_pubkey_npub(event)?;
    }
    if message.created_at == 0 {
        message.created_at = i64::try_from(event.created_at.as_secs()).ok()?;
    }
    Some(message)
}

pub fn build_thread_message_tags(message: &SysopThreadMessage) -> Vec<(&'static str, String)> {
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
