use crate::private_payload::decrypt_private_json_from_event;
use crate::pubkeys::{event_pubkey_hex, event_pubkey_npub};
use nostr_sdk::{Event, SecretKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactMessage {
    pub message_id: String,
    pub sender_pubkey: String,
    pub sender_npub: String,
    pub sender_label: Option<String>,
    pub body: String,
    pub created_at: i64,
}

pub fn decrypt_contact_message(secret_key: &SecretKey, event: &Event) -> Option<ContactMessage> {
    let mut message: ContactMessage = decrypt_private_json_from_event(secret_key, event).ok()?;
    if message.message_id.trim().is_empty() {
        message.message_id = extract_tag(event, "d")?;
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

fn extract_tag(event: &Event, key: &str) -> Option<String> {
    for tag in event.tags.iter() {
        let values = tag.clone().to_vec();
        if values.first().map(String::as_str) == Some(key) && values.len() >= 2 {
            return Some(values[1].clone());
        }
    }
    None
}
