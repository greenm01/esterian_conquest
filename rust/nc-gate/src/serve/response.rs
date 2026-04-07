//! 30502 SessionReady and 30503 SessionError publishing built on `nc-nostr`.

use nc_nostr::session::{
    GameEntry as SessionGameEntry, SessionErrorPayload, build_session_error_event,
    build_session_ready_event,
};
pub use nc_nostr::session::{SessionReadyPayload, SessionUiMode};
use nostr_sdk::{Client, Keys, PublicKey};

use crate::config::GateConfig;
use crate::serve::provision::ProvisionedKey;
use crate::serve::routing::{ResolvedSeat, RouteError};

pub async fn publish_session_ready(
    client: &Client,
    gate_keys: &Keys,
    player_pubkey: &PublicKey,
    session_nonce: &str,
    config: &GateConfig,
    seat: &ResolvedSeat,
    player_name: &str,
    _provisioned: &ProvisionedKey,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let payload = SessionReadyPayload {
        game_id: seat.game_id.clone(),
        ssh_host: config.ssh_host.clone(),
        ssh_port: config.ssh_port,
        ssh_user: config.ssh_user.clone(),
        host_fingerprint: String::new(),
        game_name: seat.game_name.clone(),
        seat: seat.player as u32,
        player_name: player_name.to_string(),
        session_ui: SessionUiMode::ClassicNcGame,
    };
    let event = build_session_ready_event(gate_keys, player_pubkey, session_nonce, &payload)?;
    client.send_event(&event).await?;
    Ok(event.id.to_hex())
}

fn session_error_payload_struct(error: &RouteError) -> SessionErrorPayload {
    match error {
        RouteError::MultipleGames(games) => SessionErrorPayload {
            error: "multiple_games".to_string(),
            message: "Your identity is in multiple games on this server.".to_string(),
            games: games
                .iter()
                .map(|game| SessionGameEntry {
                    game_id: game.game_id.clone(),
                    name: game.game_name.clone(),
                    seat: game.player as u32,
                })
                .collect(),
        },
        _ => SessionErrorPayload {
            error: error.error_code().to_string(),
            message: error.to_string(),
            games: Vec::new(),
        },
    }
}

pub fn session_error_payload(error: &RouteError) -> String {
    session_error_payload_struct(error).to_json()
}

pub fn session_error_payload_code_message(code: &str, message: &str) -> String {
    SessionErrorPayload {
        error: code.to_string(),
        message: message.to_string(),
        games: Vec::new(),
    }
    .to_json()
}

pub async fn publish_session_error(
    client: &Client,
    gate_keys: &Keys,
    player_pubkey: &PublicKey,
    session_nonce: &str,
    error: &RouteError,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let payload = session_error_payload_struct(error);
    publish_session_error_payload(client, gate_keys, player_pubkey, session_nonce, &payload).await
}

pub async fn publish_session_error_message(
    client: &Client,
    gate_keys: &Keys,
    player_pubkey: &PublicKey,
    session_nonce: &str,
    code: &str,
    message: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let payload = SessionErrorPayload {
        error: code.to_string(),
        message: message.to_string(),
        games: Vec::new(),
    };
    publish_session_error_payload(client, gate_keys, player_pubkey, session_nonce, &payload).await
}

async fn publish_session_error_payload(
    client: &Client,
    gate_keys: &Keys,
    player_pubkey: &PublicKey,
    session_nonce: &str,
    payload: &SessionErrorPayload,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let event = build_session_error_event(gate_keys, player_pubkey, session_nonce, payload)?;
    client.send_event(&event).await?;
    Ok(event.id.to_hex())
}
