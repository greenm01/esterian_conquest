use crate::private_payload::decrypt_private_json_from_event;
use crate::pubkeys::event_pubkey_hex;
use nostr_sdk::{Event, SecretKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandleCheckRequest {
    pub request_id: String,
    pub player_pubkey: String,
    pub handle: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandleCheckRequestPayload {
    pub handle: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandleCheckStatus {
    Available,
    OwnedBySelf,
    Taken,
}

impl HandleCheckStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::OwnedBySelf => "owned_by_self",
            Self::Taken => "taken",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandleCheckResult {
    pub request_id: String,
    pub handle: String,
    pub status: HandleCheckStatus,
    pub message: String,
}

pub fn parse_handle_check_request(
    secret_key: &SecretKey,
    event: &Event,
) -> Option<HandleCheckRequest> {
    let player_pubkey = event_pubkey_hex(event);
    let mut request_id = None;
    let payload: HandleCheckRequestPayload =
        decrypt_private_json_from_event(secret_key, event).ok()?;

    for tag in event.tags.iter() {
        let values = tag.clone().to_vec();
        let Some(kind) = values.first().map(String::as_str) else {
            continue;
        };
        if kind == "d" && values.len() >= 2 {
            request_id = Some(values[1].clone());
        }
    }

    let handle = payload.handle.trim();
    if handle.is_empty() {
        return None;
    }

    Some(HandleCheckRequest {
        request_id: request_id?,
        player_pubkey,
        handle: handle.to_string(),
    })
}

pub fn build_handle_check_result_tags(result: &HandleCheckResult) -> Vec<(&'static str, String)> {
    vec![
        ("d", result.request_id.clone()),
        ("status", result.status.as_str().to_string()),
    ]
}
