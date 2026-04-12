use nostr_sdk::{Event, ToBech32};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateRequest {
    pub request_id: String,
    pub game_id: String,
    pub player_pubkey: String,
    pub last_turn: Option<u32>,
    pub last_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameState {
    pub game_id: String,
    pub turn: u32,
    pub year: u32,
    pub player_seat: u32,
    pub player_name: String,
    pub state_hash: String,
    pub state: serde_json::Value,
    pub queued_mail: Vec<serde_json::Value>,
    pub report_blocks: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateDelta {
    pub game_id: String,
    pub turn: u32,
    pub base_hash: String,
    pub state_hash: String,
    pub deltas: StateDeltas,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct StateDeltas {
    #[serde(default)]
    pub planets: Vec<serde_json::Value>,
    #[serde(default)]
    pub fleets: Vec<serde_json::Value>,
    #[serde(default)]
    pub events: Vec<serde_json::Value>,
}

pub fn parse_state_request(event: &Event) -> Option<StateRequest> {
    let player_pubkey = event.pubkey.to_bech32().ok()?;
    let mut request_id = None;
    let mut game_id = None;
    let mut last_turn = None;
    let mut last_hash = None;

    let content: serde_json::Value = serde_json::from_str(&event.content)
        .ok()
        .unwrap_or_default();
    if let Some(obj) = content.as_object() {
        last_turn = obj
            .get("last_turn")
            .and_then(|v: &serde_json::Value| v.as_u64())
            .map(|v| v as u32);
        last_hash = obj
            .get("last_hash")
            .and_then(|v: &serde_json::Value| v.as_str())
            .map(String::from);
    }

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

    Some(StateRequest {
        request_id: request_id?,
        game_id: game_id?,
        player_pubkey,
        last_turn,
        last_hash,
    })
}

pub fn build_state_response_tags(state: &GameState) -> Vec<(&'static str, String)> {
    vec![
        ("d", format!("state-{}", state.turn)),
        ("game-id", state.game_id.clone()),
        ("turn", state.turn.to_string()),
        ("year", state.year.to_string()),
        ("player-seat", state.player_seat.to_string()),
        ("player-name", state.player_name.clone()),
        ("hash", state.state_hash.clone()),
    ]
}

pub fn build_delta_response_tags(delta: &StateDelta) -> Vec<(&'static str, String)> {
    vec![
        ("d", format!("delta-{}", delta.turn)),
        ("game-id", delta.game_id.clone()),
        ("turn", delta.turn.to_string()),
        ("base-hash", delta.base_hash.clone()),
        ("hash", delta.state_hash.clone()),
    ]
}
