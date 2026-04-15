use crate::private_payload::decrypt_private_json_from_event;
use crate::pubkeys::event_pubkey_hex;
use crate::state_sync::GameState;
use nostr_sdk::{Event, SecretKey};
use serde::{Deserialize, Serialize};

pub const FIRST_JOIN_NAME_MAX_CHARS: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstJoinSetupRequest {
    pub request_id: String,
    pub game_id: String,
    pub player_pubkey: String,
    pub empire_name: String,
    pub homeworld_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstJoinSetupRequestPayload {
    pub empire_name: String,
    pub homeworld_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirstJoinSetupStatus {
    Accepted,
    Rejected,
}

impl FirstJoinSetupStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstJoinSetupResult {
    pub request_id: String,
    pub game_id: String,
    pub status: FirstJoinSetupStatus,
    pub message: String,
    pub state: Option<GameState>,
}

pub fn parse_first_join_setup_request(
    secret_key: &SecretKey,
    event: &Event,
) -> Option<FirstJoinSetupRequest> {
    let player_pubkey = event_pubkey_hex(event);
    let mut request_id = None;
    let mut game_id = None;
    let payload: FirstJoinSetupRequestPayload =
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

    Some(FirstJoinSetupRequest {
        request_id: request_id?,
        game_id: game_id?,
        player_pubkey,
        empire_name: payload.empire_name,
        homeworld_name: payload.homeworld_name,
    })
}

pub fn build_first_join_setup_result_tags(
    result: &FirstJoinSetupResult,
) -> Vec<(&'static str, String)> {
    vec![
        ("d", result.request_id.clone()),
        ("game-id", result.game_id.clone()),
        ("status", result.status.as_str().to_string()),
    ]
}
