use crate::game::effects::GameEffects;
use nc_data::hosted::HostedStore;
use nc_nostr::state_sync::StateRequest;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

pub struct StateSync {
    pub game_id: String,
    pub store: Arc<HostedStore>,
    pub games_root: PathBuf,
}

impl StateSync {
    pub fn new(game_id: String, store: Arc<HostedStore>, games_root: PathBuf) -> Self {
        Self {
            game_id,
            store,
            games_root,
        }
    }

    pub fn handle_state_request(&self, request: &StateRequest) -> GameEffects {
        tracing::info!(
            "Processing state request for game {} turn {:?} from {}",
            request.game_id,
            request.last_turn,
            request.player_pubkey
        );

        let _settings = match nc_data::hosted::get_settings(self.store.connection(), &self.game_id)
        {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to load game settings: {}", e);
                return GameEffects::QueueEvent {
                    recipient_pubkey: request.player_pubkey.clone(),
                    kind: 30520,
                    content: serde_json::to_string(&json!({
                        "error": "Failed to load game state"
                    }))
                    .unwrap_or_default(),
                    tags: vec![
                        ("game-id".to_string(), self.game_id.clone()),
                        ("error".to_string(), "game_not_found".to_string()),
                    ],
                    encrypt: true,
                };
            }
        };

        let seat = match nc_data::hosted::get_seat_by_pubkey(
            self.store.connection(),
            &self.game_id,
            &request.player_pubkey,
        ) {
            Ok(Some(s)) => s,
            Ok(None) => {
                return GameEffects::QueueEvent {
                    recipient_pubkey: request.player_pubkey.clone(),
                    kind: 30520,
                    content: serde_json::to_string(&json!({
                        "error": "No claimed seat in this game"
                    }))
                    .unwrap_or_default(),
                    tags: vec![
                        ("game-id".to_string(), self.game_id.clone()),
                        ("error".to_string(), "not_a_player".to_string()),
                    ],
                    encrypt: true,
                };
            }
            Err(e) => {
                tracing::error!("Failed to lookup seat: {}", e);
                return GameEffects::InvalidEvent {
                    reason: format!("Database error: {}", e),
                };
            }
        };

        let current_turn = 0;
        let current_year = 3000;

        let state_hash = blake3::hash(
            format!("{}:{}:{}", self.game_id, current_turn, seat.seat_number).as_bytes(),
        )
        .to_hex()
        .to_string();

        let state_payload = json!({
            "game_id": self.game_id,
            "turn": current_turn,
            "year": current_year,
            "player_seat": seat.seat_number,
            "player_name": format!("Player {}", seat.seat_number),
            "state_hash": state_hash,
            "state": Value::Null,
            "queued_mail": Vec::<Value>::new(),
            "report_blocks": Vec::<Value>::new(),
        });

        GameEffects::QueueEvent {
            recipient_pubkey: request.player_pubkey.clone(),
            kind: 30520,
            content: serde_json::to_string(&state_payload).unwrap_or_default(),
            tags: vec![
                ("game-id".to_string(), self.game_id.clone()),
                ("turn".to_string(), current_turn.to_string()),
                ("year".to_string(), current_year.to_string()),
                ("player-seat".to_string(), seat.seat_number.to_string()),
                ("hash".to_string(), state_hash),
            ],
            encrypt: true,
        }
    }
}
