use std::path::Path;

use ec_ui::buffer::PlayfieldBuffer;
use ec_ui::theme::classic;

use crate::connect::game_discovery::discover_game_for_invite;
use crate::connect::public_join::run_public_join;
use crate::connect::session::{DisambigMode, SessionOutcome, run_session};

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

pub fn execute_pending_connect(
    state: &mut PickerState,
    picker_session: &mut PickerSession,
    maps_root: &Path,
    rt: &tokio::runtime::Runtime,
    session: &mut ec_ui::session::TerminalSession,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(mut request) = state.pending_connect.take() else {
        return Ok(());
    };

    let outcome = if matches!(request.origin, ConnectOrigin::JoinPrompt)
        && request.gate_npub.trim().is_empty()
        && request.target.invite_code.is_some()
    {
        run_suspended(session, || {
            rt.block_on(run_public_join(
                &picker_session.keys,
                request.target.clone(),
                &picker_session.npub,
                request.disambig_mode(),
                maps_root,
            ))
        })?
        .map_err(|err| -> Box<dyn std::error::Error> { err })?
    } else {
        if request.gate_npub.trim().is_empty() {
            if let Some(invite_code) = request.target.invite_code.clone() {
                match rt.block_on(discover_game_for_invite(
                    &picker_session.keys,
                    &request.target,
                    &invite_code,
                )) {
                    Ok(discovered) => {
                        request.gate_npub = discovered.gate_npub;
                        request.target.game_id.get_or_insert(discovered.game_id);
                    }
                    Err(err) => {
                        state.overlay = None;
                        state.screen = Screen::JoinPrompt;
                        state.show_error(err);
                        return Ok(());
                    }
                }
            }
        }

        run_suspended(session, || {
            rt.block_on(run_session(
                &picker_session.keys,
                request.target.clone(),
                &picker_session.npub,
                &request.gate_npub,
                request.disambig_mode(),
                maps_root,
            ))
        })?
    };

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
                state.overlay = None;
                state.screen = Screen::JoinPrompt;
                state.show_error(msg);
            }
            SessionOutcome::Timeout => {
                state.overlay = None;
                state.screen = Screen::JoinPrompt;
                state.show_error("handshake timed out.");
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
    render_status_popup(buffer, "CONNECTING TO GAME", lines);
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
        match self.origin {
            ConnectOrigin::GameSelect => DisambigMode::Prompt,
            _ => DisambigMode::Picker,
        }
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
