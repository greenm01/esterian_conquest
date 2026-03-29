use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use ec_ui::buffer::PlayfieldBuffer;
use ec_ui::theme::classic;

use crate::connect::game_discovery::discover_game_for_invite;
use crate::connect::public_join::prepare_public_join;
use crate::connect::session::{
    DisambigMode, PreparedSession, SessionOutcome, SessionPreparation, finish_prepared_session,
    prepare_session,
};

use super::flows::apply_session_outcome;
use super::layout::{PLAYFIELD_WIDTH, truncate};
use super::overlay::PickerOverlay;
use super::state::{ConnectDisplay, ConnectOrigin, PickerSession, PickerState, Screen};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingConnectRequest {
    pub origin: ConnectOrigin,
    pub target: crate::connect::resolve::ResolvedTarget,
    pub gate_npub: String,
    pub display: ConnectDisplay,
}

pub struct ActiveConnect {
    rx: Receiver<ConnectTaskResult>,
}

enum ConnectTaskResult {
    Outcome {
        request: PendingConnectRequest,
        outcome: SessionOutcome,
    },
    Prepared {
        request: PendingConnectRequest,
        prepared: PreparedSession,
    },
}

pub fn queue_connect_request(state: &mut PickerState, request: PendingConnectRequest) {
    state.overlay = Some(
        if matches!(request.origin, ConnectOrigin::JoinPrompt)
            && request.gate_npub.trim().is_empty()
            && request.target.invite_code.is_some()
        {
            PickerOverlay::ClaimingInvite {
                lines: request.display.lines.clone(),
            }
        } else {
            PickerOverlay::Connecting {
                lines: request.display.lines.clone(),
            }
        },
    );
    state.pending_connect = Some(request);
}

pub fn start_pending_connect(
    state: &mut PickerState,
    picker_session: &mut PickerSession,
    maps_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if state.active_connect.is_some() {
        return Ok(());
    }
    let Some(request) = state.pending_connect.take() else {
        return Ok(());
    };
    let keys = picker_session.keys.clone();
    let npub = picker_session.npub.clone();
    let maps_root = maps_root.to_path_buf();
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(err) => {
                let _ = tx.send(ConnectTaskResult::Outcome {
                    request,
                    outcome: SessionOutcome::Error(format!("unable to start runtime: {err}")),
                });
                return;
            }
        };
        let result = rt.block_on(run_connect_task(&keys, &npub, maps_root, request));
        let _ = tx.send(result);
    });
    state.active_connect = Some(ActiveConnect { rx });
    Ok(())
}

pub fn poll_active_connect(
    state: &mut PickerState,
    session: &mut ec_ui::session::TerminalSession,
    rt: &tokio::runtime::Runtime,
    picker_session: &PickerSession,
) -> Result<bool, Box<dyn std::error::Error>> {
    let Some(active) = state.active_connect.as_ref() else {
        return Ok(false);
    };
    let result = match active.rx.try_recv() {
        Ok(result) => result,
        Err(TryRecvError::Empty) => return Ok(false),
        Err(TryRecvError::Disconnected) => {
            state.active_connect = None;
            state.overlay = None;
            state.show_error("connection attempt ended unexpectedly");
            return Ok(false);
        }
    };
    state.active_connect = None;

    match result {
        ConnectTaskResult::Prepared { request, prepared } => {
            let outcome = run_suspended(session, || {
                rt.block_on(finish_prepared_session(prepared, &picker_session.npub))
            })?;
            apply_connect_outcome(state, request, outcome)?;
            Ok(true)
        }
        ConnectTaskResult::Outcome { request, outcome } => {
            apply_connect_outcome(state, request, outcome)?;
            Ok(false)
        }
    }
}

pub fn cancel_active_connect(state: &mut PickerState) {
    state.pending_connect = None;
    state.active_connect = None;
    state.overlay = None;
}

fn apply_connect_outcome(
    state: &mut PickerState,
    request: PendingConnectRequest,
    outcome: SessionOutcome,
) -> Result<(), Box<dyn std::error::Error>> {
    match request.origin {
        ConnectOrigin::GameList => {
            state.overlay = None;
            state.refresh_cache();
            apply_session_outcome(state, outcome, Some((request.target, request.gate_npub)));
        }
        ConnectOrigin::JoinPrompt => match outcome {
            SessionOutcome::Done { notice, .. } => {
                state.overlay = None;
                state.refresh_cache();
                state.join_input.clear();
                state.screen = Screen::GameList;
                state.selected = 0;
                if let Some(notice) = notice
                    .filter(|message| !message.trim().is_empty())
                    .filter(|message| message != "For Griffith and glory.")
                {
                    state.show_notice(notice);
                }
            }
            SessionOutcome::NeedsDisambiguation { games } => {
                state.overlay = None;
                state.screen = Screen::GameSelect {
                    games,
                    selected: 0,
                    server_host: request.target.server_host,
                    server_port: request.target.server_port,
                    relay_url: request.target.relay_url,
                    gate_npub: request.gate_npub,
                };
            }
            SessionOutcome::Error(msg) => {
                state.show_error(msg);
            }
            SessionOutcome::Timeout => {
                state.show_error("Connection timed out.");
            }
        },
        ConnectOrigin::GameSelect => match outcome {
            SessionOutcome::Done { .. } => {
                state.overlay = None;
                state.screen = Screen::GameList;
                state.refresh_cache();
                apply_session_outcome(state, outcome, Some((request.target, request.gate_npub)));
            }
            SessionOutcome::NeedsDisambiguation { .. } => {
                state.overlay = None;
                apply_session_outcome(state, outcome, Some((request.target, request.gate_npub)));
            }
            SessionOutcome::Error(msg) => {
                state.overlay = None;
                state.show_error(msg);
            }
            SessionOutcome::Timeout => {
                state.overlay = None;
                state.show_error("handshake timed out.");
            }
        },
        ConnectOrigin::GameRelayPrompt { index } => match outcome {
            SessionOutcome::Done { .. } | SessionOutcome::NeedsDisambiguation { .. } => {
                super::flows::persist_cached_game_relay(state, index, &request.target.relay_url)?;
                state.relay_input.clear();
                state.overlay = None;
                state.refresh_cache();
                apply_session_outcome(state, outcome, Some((request.target, request.gate_npub)));
            }
            SessionOutcome::Error(msg) => {
                state.overlay = Some(PickerOverlay::GameRelayPrompt {
                    index,
                    action: super::relay::RelayPromptAction::Connect,
                    error: Some(msg),
                });
            }
            SessionOutcome::Timeout => {
                state.overlay = Some(PickerOverlay::GameRelayPrompt {
                    index,
                    action: super::relay::RelayPromptAction::Connect,
                    error: Some("handshake timed out.".to_string()),
                });
            }
        },
    }
    Ok(())
}

pub fn render_connecting_popup(buffer: &mut PlayfieldBuffer, lines: &[String]) {
    let mut popup_lines = lines.to_vec();
    popup_lines.push("Esc/Q: cancel".to_string());
    render_status_popup(buffer, "CONNECTING TO GAME", &popup_lines);
}

pub fn render_status_popup(buffer: &mut PlayfieldBuffer, title: &str, lines: &[String]) {
    let content_width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let width = (content_width + 4)
        .max(title.chars().count() + 4)
        .min(PLAYFIELD_WIDTH.saturating_sub(8));
    let height = (lines.len() + 2) as u16;
    let popup =
        super::overlay::draw_modal_frame(buffer, title, width, height, classic::table_body_style());
    let mut row = popup.y as usize + 1;
    let col = popup.x as usize + 2;
    for line in lines {
        buffer.write_text_clipped(
            row,
            col,
            &truncate(line, popup.width.saturating_sub(4) as usize),
            classic::table_body_style(),
        );
        row += 1;
    }
    buffer.clear_cursor();
}

impl PendingConnectRequest {
    fn disambig_mode(&self) -> DisambigMode {
        DisambigMode::Picker
    }
}

fn run_suspended<T>(
    session: &mut ec_ui::session::TerminalSession,
    action: impl FnOnce() -> T,
) -> Result<T, Box<dyn std::error::Error>> {
    session.suspend_for_bridge()?;
    let result = action();
    session.resume_after_bridge()?;
    Ok(result)
}

async fn run_connect_task(
    keys: &nostr_sdk::Keys,
    npub: &str,
    maps_root: PathBuf,
    mut request: PendingConnectRequest,
) -> ConnectTaskResult {
    let preparation = if matches!(request.origin, ConnectOrigin::JoinPrompt)
        && request.gate_npub.trim().is_empty()
        && request.target.invite_code.is_some()
    {
        match prepare_public_join(
            keys,
            request.target.clone(),
            npub,
            request.disambig_mode(),
            &maps_root,
        )
        .await
        {
            Ok(preparation) => preparation,
            Err(err) => {
                return ConnectTaskResult::Outcome {
                    request,
                    outcome: SessionOutcome::Error(err.to_string()),
                };
            }
        }
    } else {
        if request.gate_npub.trim().is_empty() {
            if let Some(invite_code) = request.target.invite_code.clone() {
                match discover_game_for_invite(keys, &request.target, &invite_code).await {
                    Ok(discovered) => {
                        request.gate_npub = discovered.gate_npub;
                        request.target.server_host = discovered.ssh_host;
                        request.target.server_port = discovered.ssh_port;
                        request.target.game_id.get_or_insert(discovered.game_id);
                    }
                    Err(err) => {
                        return ConnectTaskResult::Outcome {
                            request,
                            outcome: SessionOutcome::Error(err),
                        };
                    }
                }
            }
        }

        prepare_session(
            keys,
            request.target.clone(),
            npub,
            &request.gate_npub,
            request.disambig_mode(),
            &maps_root,
        )
        .await
    };

    match preparation {
        SessionPreparation::Ready(prepared) => ConnectTaskResult::Prepared { request, prepared },
        SessionPreparation::Outcome(outcome) => ConnectTaskResult::Outcome { request, outcome },
    }
}
