use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, poll, read};

use ec_ui::paint::render_to_stdout;
use ec_ui::session::TerminalSession;

use crate::cache::{GameCache, load_cache};
use crate::launcher::run_password_gate_in_session;

use super::input::{
    handle_game_list_key, handle_game_select_key, handle_identity_overlay_key,
    handle_join_prompt_key, handle_wallet_key,
};
use super::layout::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};
use super::overlay::handle_overlay_key;
use super::session::load_picker_session;
use super::state::{PickerSession, PickerState, Screen};

pub fn run_picker_in_session(
    picker_session: PickerSession,
    gate_npub: String,
    maps_root: PathBuf,
    lock_timeout_minutes: u16,
    mut session: TerminalSession,
) -> Result<(), Box<dyn std::error::Error>> {
    let cache = load_cache().unwrap_or_else(|_| GameCache::empty());
    let mut state = PickerState::new(cache);
    let rt = tokio::runtime::Runtime::new()?;
    let mut picker_session = Some(picker_session);
    let result = run_loop(
        &mut state,
        &mut picker_session,
        &gate_npub,
        &maps_root,
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
    maps_root: &Path,
    lock_timeout_minutes: u16,
    rt: &tokio::runtime::Runtime,
    session: &mut TerminalSession,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut last_activity = Instant::now();

    loop {
        let (term_width, term_height) = crossterm::terminal::size().unwrap_or((80, 25));
        let too_small = usize::from(term_width) < PLAYFIELD_WIDTH
            || usize::from(term_height) < PLAYFIELD_HEIGHT;
        let buffer =
            super::render::render_buffer(state, picker_session.as_ref(), term_width, term_height);
        render_to_stdout(&buffer)?;

        let wait = next_wait_duration(state, lock_timeout_minutes, last_activity);
        if !poll(wait)? {
            if matches!(state.screen, Screen::Locked) {
                state.matrix.advance();
                continue;
            }
            if should_lock_for_idle(lock_timeout_minutes, last_activity) {
                lock_picker(state, picker_session);
            }
            continue;
        }

        let Event::Key(key) = read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
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
            if let Some(unlocked) = unlock_picker_session(session)? {
                *picker_session = Some(unlocked);
                state.screen = Screen::GameList;
                last_activity = Instant::now();
            }
            continue;
        }

        let text_entry = matches!(
            state.screen,
            Screen::JoinPrompt | Screen::WalletAliasPrompt | Screen::WalletImportPrompt
        );
        if is_manual_lock_key(key, text_entry) {
            lock_picker(state, picker_session);
            continue;
        }

        last_activity = Instant::now();

        if state.overlay.is_some() {
            handle_overlay_key(key, state);
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
                handle_game_list_key(key, state, session_state, gate_npub, maps_root, rt, session)?;
            }
            Screen::JoinPrompt => {
                let session_state = picker_session
                    .as_mut()
                    .ok_or("picker session missing while unlocked")?;
                handle_join_prompt_key(
                    key,
                    state,
                    session_state,
                    gate_npub,
                    maps_root,
                    rt,
                    session,
                )?;
            }
            Screen::IdentityOverlay => handle_identity_overlay_key(key, state),
            Screen::WalletList | Screen::WalletAliasPrompt | Screen::WalletImportPrompt => {
                let session_state = picker_session
                    .as_mut()
                    .ok_or("picker session missing while unlocked")?;
                handle_wallet_key(key, state, session_state)?;
            }
            Screen::GameSelect { .. } => {
                let session_state = picker_session
                    .as_mut()
                    .ok_or("picker session missing while unlocked")?;
                handle_game_select_key(key, state, session_state, maps_root, rt, session)?;
            }
            Screen::Locked => {}
        }

        if state.quit {
            break;
        }
    }

    Ok(())
}

fn next_wait_duration(
    state: &PickerState,
    lock_timeout_minutes: u16,
    last_activity: Instant,
) -> Duration {
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

fn lock_picker(state: &mut PickerState, picker_session: &mut Option<PickerSession>) {
    *picker_session = None;
    state.overlay = None;
    state.screen = Screen::Locked;
    state.join_input.clear();
    state.alias_input.clear();
    state.import_input.clear();
    state.matrix.reset();
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
