use crate::game::effects::GameEffects;
use nc_data::hosted::HostedStore;
use nostr_sdk::{Event, PublicKey};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug)]
pub enum RoutingError {
    UnknownGame(String),
    StoreError(String),
    InvalidEvent(String),
    NotAddressedToHost,
}

pub struct RoutedEvent {
    pub game_id: String,
    pub store: Arc<HostedStore>,
    pub event: Event,
}

pub fn route_event(
    event: Event,
    games_root: &PathBuf,
    host_pubkey: &PublicKey,
) -> Result<RoutedEvent, RoutingError> {
    let host_hex = host_pubkey.to_hex();

    let addressed_to_host = event.tags.iter().any(|tag| {
        let values = tag.clone().to_vec();
        values.get(0) == Some(&"p".to_string()) && values.get(1) == Some(&host_hex)
    });

    if !addressed_to_host {
        return Err(RoutingError::NotAddressedToHost);
    }

    let game_id = extract_game_id(&event)
        .ok_or_else(|| RoutingError::InvalidEvent("missing game-id tag".to_string()))?;

    let game_dir = games_root.join(&game_id);
    let db_path = game_dir.join("hosted.db");

    if !db_path.exists() {
        return Err(RoutingError::UnknownGame(game_id));
    }

    let store = HostedStore::open(&db_path).map_err(|e| RoutingError::StoreError(e.to_string()))?;

    Ok(RoutedEvent {
        game_id,
        store: Arc::new(store),
        event,
    })
}

fn extract_game_id(event: &Event) -> Option<String> {
    for tag in event.tags.iter() {
        let values = tag.clone().to_vec();
        if let Some(kind) = values.first() {
            if kind == "game-id" && values.len() >= 2 {
                return Some(values[1].clone());
            }
        }
    }
    None
}

pub fn process_event(routed: &RoutedEvent) -> Vec<GameEffects> {
    let kind: u16 = routed.event.kind.into();

    match kind {
        30507 => {
            if let Some(req) = nc_nostr::state_sync::parse_state_request(&routed.event) {
                vec![GameEffects::HandleStateRequest { request: req }]
            } else {
                vec![GameEffects::InvalidEvent {
                    reason: "failed to parse StateRequest".to_string(),
                }]
            }
        }
        30513 => {
            if let Some(req) = nc_nostr::invite_request::parse_invite_request(&routed.event) {
                vec![GameEffects::HandleInviteRequest {
                    request: req,
                    game_id: routed.game_id.clone(),
                }]
            } else {
                vec![GameEffects::InvalidEvent {
                    reason: "failed to parse InviteRequest".to_string(),
                }]
            }
        }
        30510 => {
            match nc_nostr::claim::parse_seat_claim_request(&routed.event) {
                Ok(req) => vec![GameEffects::HandleSeatClaim {
                    request: req,
                    game_id: routed.game_id.clone(),
                }],
                Err(err) => vec![GameEffects::InvalidEvent {
                    reason: format!("failed to parse SeatClaimRequest: {}", err),
                }],
            }
        }
        30522 => {
            if let Some(cmds) = nc_nostr::turn_commands::parse_turn_commands(&routed.event) {
                vec![GameEffects::HandleTurnCommands {
                    commands: cmds,
                    game_id: routed.game_id.clone(),
                }]
            } else {
                vec![GameEffects::InvalidEvent {
                    reason: "failed to parse TurnCommands".to_string(),
                }]
            }
        }

        _ => {
            vec![GameEffects::InvalidEvent {
                reason: format!("unsupported event kind: {}", kind),
            }]
        }
    }
}
