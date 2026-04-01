use std::path::PathBuf;
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, poll, read};

use ec_ui::paint::StdoutRenderer;
use ec_ui::session::TerminalSession;

use crate::cache::{GameCache, load_cache};
use crate::hard_quit::is_hard_quit_key;
use crate::launcher::run_password_gate_in_session;

use super::connecting::{poll_active_connect, start_pending_connect};
use super::input::{
    handle_game_list_key, handle_game_select_key, handle_identity_overlay_key, handle_relay_key,
    handle_keychain_key,
};
use super::overlay::handle_overlay_key;
use super::refresh::execute_pending_refresh;
use super::session::load_picker_session;
use super::state::{PickerSession, PickerState, Screen};
use crate::shell::terminal_fits_outer;

const POST_BRIDGE_RECOVERY_WINDOW: Duration = Duration::from_millis(120);

#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerReadResult {
    Timeout,
    Key(KeyEvent),
}

pub fn run_picker_in_session(
    picker_session: PickerSession,
    gate_npub: String,
    maps_root: PathBuf,
    lock_timeout_minutes: u16,
    mut session: TerminalSession,
) -> Result<(), Box<dyn std::error::Error>> {
    let cache = load_cache().unwrap_or_else(|_| GameCache::empty());
    let mut state = PickerState::new(cache, maps_root);
    let rt = tokio::runtime::Runtime::new()?;
    let mut picker_session = Some(picker_session);
    let result = run_loop(
        &mut state,
        &mut picker_session,
        &gate_npub,
        lock_timeout_minutes,
        &rt,
        &mut session,
    );
    let _ = session.restore();
    result
}

fn run_loop(
    state: &mut PickerState,
    picker_session: &mut Option<PickerSession>,
    gate_npub: &str,
    lock_timeout_minutes: u16,
    rt: &tokio::runtime::Runtime,
    session: &mut TerminalSession,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut last_activity = Instant::now();
    let mut replayed_key = None;
    let mut renderer = StdoutRenderer::new();

    loop {
        let (term_width, term_height) = crossterm::terminal::size().unwrap_or((80, 25));
        let too_small = !terminal_fits_outer(usize::from(term_width), usize::from(term_height));
        let buffer =
            super::render::render_buffer(state, picker_session.as_ref(), term_width, term_height);
        renderer.render(&buffer)?;

        if !too_small {
            if let Some(session_state) = picker_session.as_mut() {
                if let Some(request) = state.pending_refresh.as_ref() {
                    if request.is_ready() {
                        execute_pending_refresh(state, session_state, rt)?;
                        if state.quit {
                            break;
                        }
                        continue;
                    }
                }
                if state.pending_connect.is_some() {
                    start_pending_connect(state, session_state)?;
                }
                if state.active_connect.is_some() {
                    let bridged = poll_active_connect(state, session, rt, session_state)?;
                    if bridged {
                        renderer.reset();
                        replayed_key = capture_post_bridge_key()?;
                        if state.quit {
                            break;
                        }
                        continue;
                    }
                }
            }
        }

        let wait = next_wait_duration(state, lock_timeout_minutes, last_activity);
        let key = match read_picker_key(&mut replayed_key, wait)? {
            PickerReadResult::Timeout => {
                if matches!(state.screen, Screen::Locked) {
                    state.matrix.advance();
                    continue;
                }
                if should_lock_for_idle(lock_timeout_minutes, last_activity) {
                    lock_picker(state, picker_session);
                }
                continue;
            }
            PickerReadResult::Key(key) => key,
        };
        if is_hard_quit_key(key) {
            state.quit = true;
            break;
        }

        if too_small {
            match key.code {
                KeyCode::Esc => state.quit = true,
                KeyCode::Char('q' | 'Q')
                    if matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT) =>
                {
                    state.quit = true;
                }
                _ => {}
            }
            if state.quit {
                break;
            }
            continue;
        }

        if matches!(state.screen, Screen::Locked) {
            let unlocked = unlock_picker_session(session)?;
            renderer.reset();
            if let Some(unlocked) = unlocked {
                *picker_session = Some(unlocked);
                state.screen = Screen::GameList;
                last_activity = Instant::now();
            }
            continue;
        }

        let text_entry = matches!(state.screen, Screen::KeychainAddPrompt)
            || matches!(
                state.overlay,
                Some(super::overlay::PickerOverlay::RelayEditor { .. })
                    | Some(super::overlay::PickerOverlay::GameRelayPrompt { .. })
                    | Some(super::overlay::PickerOverlay::JoinCodePopup { .. })
                    | Some(super::overlay::PickerOverlay::MapsDownloadPrompt { .. })
            );
        if is_manual_lock_key(key, text_entry) {
            lock_picker(state, picker_session);
            continue;
        }

        last_activity = Instant::now();

        if state.overlay.is_some() {
            handle_overlay_key(key, state, picker_session.as_mut(), gate_npub, Some(rt))?;
            if state.quit {
                break;
            }
            continue;
        }

        match state.screen {
            Screen::GameList => {
                let session_state = picker_session
                    .as_mut()
                    .ok_or("picker session missing while unlocked")?;
                handle_game_list_key(key, state, session_state, gate_npub, rt)?;
            }
            Screen::RelayList | Screen::RelayGames { .. } => {
                handle_relay_key(key, state)?;
            }
            Screen::IdentityOverlay => handle_identity_overlay_key(key, state),
            Screen::KeychainList | Screen::KeychainAddPrompt => {
                let session_state = picker_session
                    .as_mut()
                    .ok_or("picker session missing while unlocked")?;
                handle_keychain_key(key, state, session_state)?;
            }
            Screen::GameSelect { .. } => {
                handle_game_select_key(key, state)?;
            }
            Screen::Locked => {}
        }

        if state.quit {
            break;
        }
    }

    Ok(())
}

fn read_picker_key(
    replayed_key: &mut Option<KeyEvent>,
    wait: Duration,
) -> Result<PickerReadResult, Box<dyn std::error::Error>> {
    if let Some(key) = replayed_key.take() {
        return Ok(PickerReadResult::Key(key));
    }

    let deadline = Instant::now() + wait;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(PickerReadResult::Timeout);
        }
        if !poll(remaining)? {
            return Ok(PickerReadResult::Timeout);
        }
        if let Some(key) = classify_picker_event(read()?) {
            return Ok(PickerReadResult::Key(key));
        }
    }
}

fn lock_picker(state: &mut PickerState, picker_session: &mut Option<PickerSession>) {
    *picker_session = None;
    state.overlay = None;
    state.screen = Screen::Locked;
    state.join_input.clear();
    state.maps_input.clear();
    state.maps_input_prefilled = false;
    state.keychain_input.clear();
    state.relay_input.clear();
    state.pending_connect = None;
    state.active_connect = None;
    state.pending_refresh = None;
    state.matrix.reset();
}

fn capture_post_bridge_key() -> Result<Option<KeyEvent>, Box<dyn std::error::Error>> {
    let deadline = Instant::now() + POST_BRIDGE_RECOVERY_WINDOW;
    while Instant::now() < deadline {
        let wait = (deadline - Instant::now()).min(Duration::from_millis(10));
        if !poll(wait)? {
            continue;
        }
        if let Some(key) = post_bridge_recovery_event(read()?) {
            return Ok(Some(key));
        }
    }
    Ok(None)
}

#[doc(hidden)]
pub fn classify_picker_event(event: Event) -> Option<KeyEvent> {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => Some(key),
        _ => None,
    }
}

#[doc(hidden)]
pub fn post_bridge_recovery_event(event: Event) -> Option<KeyEvent> {
    classify_picker_event(event)
}

fn next_wait_duration(
    state: &PickerState,
    lock_timeout_minutes: u16,
    last_activity: Instant,
) -> Duration {
    if let Some(request) = state.pending_refresh.as_ref() {
        return request.remaining_until_execute();
    }
    if state.active_connect.is_some() {
        return Duration::from_millis(50);
    }
    if matches!(state.screen, Screen::Locked) {
        return Duration::from_millis(80);
    }
    if lock_timeout_minutes == 0 {
        return Duration::from_millis(250);
    }
    let timeout = Duration::from_secs(u64::from(lock_timeout_minutes) * 60);
    let elapsed = last_activity.elapsed();
    if elapsed >= timeout {
        Duration::from_millis(1)
    } else {
        (timeout - elapsed).min(Duration::from_millis(250))
    }
}

fn should_lock_for_idle(lock_timeout_minutes: u16, last_activity: Instant) -> bool {
    lock_timeout_minutes != 0
        && last_activity.elapsed() >= Duration::from_secs(u64::from(lock_timeout_minutes) * 60)
}

fn is_manual_lock_key(key: KeyEvent, text_entry: bool) -> bool {
    let alt_l = matches!(
        key,
        KeyEvent {
            code: KeyCode::Char('l' | 'L'),
            modifiers,
            ..
        } if modifiers.contains(KeyModifiers::ALT)
    );
    let plain_l = matches!(
        key,
        KeyEvent {
            code: KeyCode::Char('l' | 'L'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        }
    );
    alt_l || (plain_l && !text_entry)
}

fn unlock_picker_session(
    session: &mut TerminalSession,
) -> Result<Option<PickerSession>, Box<dyn std::error::Error>> {
    let mut error_msg = None;
    loop {
        let Some(password) = run_password_gate_in_session(session, error_msg.take())? else {
            return Ok(None);
        };
        match load_picker_session(password) {
            Ok(session) => return Ok(Some(session)),
            Err(err) => error_msg = Some(format!("Error: {err}")),
        }
    }
}
