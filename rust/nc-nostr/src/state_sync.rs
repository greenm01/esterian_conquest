use crate::private_payload::decrypt_private_json_from_event;
use nostr_sdk::{Event, SecretKey, ToBech32};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateRequest {
    pub request_id: String,
    pub game_id: String,
    pub player_pubkey: String,
    pub last_turn: Option<u32>,
    pub last_hash: Option<String>,
    pub handle: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateRequestPayload {
    pub last_turn: Option<u32>,
    pub last_hash: Option<String>,
    pub handle: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameState {
    pub game_id: String,
    pub turn: u32,
    pub year: u32,
    pub player_seat: u32,
    pub player_name: String,
    pub state_hash: String,
    pub state: HostedStatePayload,
    pub queued_mail: Vec<HostedQueuedMail>,
    pub report_blocks: Vec<HostedReportBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    pub planets: Vec<HostedOwnedPlanet>,
    #[serde(default)]
    pub fleets: Vec<HostedOwnedFleet>,
    #[serde(default)]
    pub events: Vec<HostedReportBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedStatePayload {
    pub player: HostedPlayerState,
    pub starmap: HostedStarmapState,
    pub owned_planets: Vec<HostedOwnedPlanet>,
    pub owned_fleets: Vec<HostedOwnedFleet>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedPlayerState {
    pub seat: u8,
    pub empire_name: String,
    pub handle: Option<String>,
    pub mode: String,
    pub tax_rate: u8,
    pub planet_count: u8,
    pub starbase_count: u8,
    pub homeworld_planet_index: u16,
    pub last_run_year: u16,
    pub diplomacy: Vec<HostedDiplomacyState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedDiplomacyState {
    pub empire_id: u8,
    pub relation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedStarmapState {
    pub map_width: u8,
    pub map_height: u8,
    pub viewer_empire_id: u8,
    pub year: u16,
    pub worlds: Vec<HostedWorldState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedWorldState {
    pub planet_index: usize,
    pub coords: [u8; 2],
    pub intel_tier: String,
    pub known_name: Option<String>,
    pub known_owner_empire_id: Option<u8>,
    pub known_owner_empire_name: Option<String>,
    pub known_potential_production: Option<u16>,
    pub known_armies: Option<u8>,
    pub known_ground_batteries: Option<u8>,
    pub known_starbase_count: Option<u8>,
    pub known_current_production: Option<u8>,
    pub known_stored_points: Option<u16>,
    pub known_docked_summary: Option<String>,
    pub known_orbit_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedOwnedPlanet {
    pub planet_index: usize,
    pub name: String,
    pub coords: [u8; 2],
    pub potential_production: u16,
    pub current_production: u8,
    pub stored_points: u16,
    pub armies: u8,
    pub ground_batteries: u8,
    pub starbase_count: u8,
    pub stardock: Vec<HostedStardockSlot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedStardockSlot {
    pub slot: usize,
    pub kind: String,
    pub count: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedOwnedFleet {
    pub fleet_id: u8,
    pub local_slot: u8,
    pub coords: [u8; 2],
    pub target_coords: [u8; 2],
    pub order: String,
    pub order_summary: String,
    pub rules_of_engagement: u8,
    pub current_speed: u8,
    pub max_speed: u8,
    pub ships: HostedFleetShips,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedFleetShips {
    pub scout: u16,
    pub battleship: u16,
    pub cruiser: u16,
    pub destroyer: u16,
    pub transport: u16,
    pub army: u16,
    pub etac: u16,
    pub total_starships: u16,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedQueuedMail {
    pub sender_empire_id: u8,
    pub recipient_empire_id: u8,
    pub year: u16,
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedReportBlock {
    pub viewer_empire_id: u8,
    pub block_index: usize,
    pub decoded_text: String,
}

pub fn parse_state_request(secret_key: &SecretKey, event: &Event) -> Option<StateRequest> {
    let player_pubkey = event.pubkey.to_bech32().ok()?;
    let mut request_id = None;
    let mut game_id = None;
    let payload: StateRequestPayload = decrypt_private_json_from_event(secret_key, event).ok()?;

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
        last_turn: payload.last_turn,
        last_hash: payload.last_hash,
        handle: payload.handle.filter(|value| !value.trim().is_empty()),
    })
}

pub fn build_state_response_tags(state: &GameState) -> Vec<(&'static str, String)> {
    vec![
        ("d", format!("state-{}", state.turn)),
        ("game-id", state.game_id.clone()),
        ("turn", state.turn.to_string()),
        ("year", state.year.to_string()),
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
