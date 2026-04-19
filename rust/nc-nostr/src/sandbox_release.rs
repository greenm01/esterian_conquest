use crate::private_payload::decrypt_private_json_from_event;
use crate::pubkeys::event_pubkey_hex;
use nostr_sdk::{Event, SecretKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxReleaseRequest {
    pub request_id: String,
    pub game_id: String,
    pub player_pubkey: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxReleaseRequestPayload {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxReleaseStatus {
    Accepted,
    Rejected,
}

impl SandboxReleaseStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxReleaseResult {
    pub request_id: String,
    pub game_id: String,
    pub status: SandboxReleaseStatus,
    pub message: String,
}

pub fn parse_sandbox_release_request(
    secret_key: &SecretKey,
    event: &Event,
) -> Option<SandboxReleaseRequest> {
    let player_pubkey = event_pubkey_hex(event);
    let mut request_id = None;
    let mut game_id = None;
    let _: SandboxReleaseRequestPayload =
        decrypt_private_json_from_event(secret_key, event).ok()?;

    for tag in event.tags.iter() {
        let values = tag.clone().to_vec();
        let Some(kind) = values.first().map(String::as_str) else {
            continue;
        };
        match kind {
            "d" if values.len() >= 2 => request_id = Some(values[1].clone()),
            "game-id" if values.len() >= 2 => game_id = Some(values[1].clone()),
            _ => {}
        }
    }

    Some(SandboxReleaseRequest {
        request_id: request_id?,
        game_id: game_id?,
        player_pubkey,
    })
}

pub fn build_sandbox_release_result_tags(
    result: &SandboxReleaseResult,
) -> Vec<(&'static str, String)> {
    vec![
        ("d", result.request_id.clone()),
        ("game-id", result.game_id.clone()),
        ("status", result.status.as_str().to_string()),
    ]
}
