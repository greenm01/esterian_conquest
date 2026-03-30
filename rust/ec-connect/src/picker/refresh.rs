use std::time::{Duration, Instant};

use ec_ui::buffer::PlayfieldBuffer;

use crate::cache::save_cache;
use crate::connect::resolve::ResolvedTarget;
use crate::connect::session_state::fetch_game_metadata;

use super::connecting::render_status_popup;
use super::overlay::PickerOverlay;
use super::state::{PickerSession, PickerState};

const REFRESH_POPUP_MIN_DWELL: Duration = Duration::from_millis(1000);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingRefreshRequest {
    pub game_id: String,
    pub target: ResolvedTarget,
    pub gate_npub: String,
    pub lines: Vec<String>,
    pub execute_after: Instant,
}

impl PendingRefreshRequest {
    pub fn from_game(
        name: &str,
        target: ResolvedTarget,
        gate_npub: String,
        game_id: String,
    ) -> Self {
        Self {
            game_id,
            lines: vec![
                format!("Game: {name}"),
                format!("Server: {}:{}", target.server_host, target.server_port),
                format!("Relay: {}", target.relay_url),
                "Refreshing game info...".to_string(),
            ],
            target,
            gate_npub,
            execute_after: Instant::now() + REFRESH_POPUP_MIN_DWELL,
        }
    }

    pub fn is_ready(&self) -> bool {
        Instant::now() >= self.execute_after
    }

    pub fn remaining_until_execute(&self) -> Duration {
        self.execute_after.saturating_duration_since(Instant::now())
    }
}

pub fn queue_refresh_request(state: &mut PickerState, request: PendingRefreshRequest) {
    state.overlay = Some(PickerOverlay::RefreshingGame {
        lines: request.lines.clone(),
    });
    state.pending_refresh = Some(request);
}

pub fn execute_pending_refresh(
    state: &mut PickerState,
    picker_session: &PickerSession,
    rt: &tokio::runtime::Runtime,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(request) = state.pending_refresh.take() else {
        return Ok(());
    };
    state.overlay = None;

    match rt.block_on(fetch_game_metadata(
        &picker_session.keys,
        &request.target,
        &request.gate_npub,
        &request.game_id,
    )) {
        Ok(payload) => {
            let Some(existing) = state
                .cache
                .games
                .iter()
                .find(|game| game.id == request.game_id)
                .cloned()
            else {
                state.refresh_cache();
                return Ok(());
            };

            let mut updated = existing;
            updated.name = payload.game_name;
            updated.player_name =
                Some(payload.player_name).filter(|value| !value.trim().is_empty());
            updated.seat = payload.seat;
            updated.relay_url = Some(request.target.relay_url);
            updated.gate_npub = request.gate_npub;
            state.cache.upsert(updated);
            save_cache(&state.cache)?;
            state.refresh_cache();
        }
        Err(err) => {
            state.show_error(format!("unable to refresh game info: {err}"));
        }
    }

    Ok(())
}

pub fn render_refreshing_popup(buffer: &mut PlayfieldBuffer, lines: &[String]) {
    render_status_popup(buffer, "REFRESHING GAME", lines);
}
