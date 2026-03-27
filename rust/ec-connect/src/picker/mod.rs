pub mod event;
pub mod render;

use std::path::{Path, PathBuf};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, poll, read};
use nostr_sdk::Keys;

use ec_ui::paint::render_to_stdout;
use ec_ui::session::TerminalSession;

use crate::cache::{GameCache, load_cache};
use crate::connect::handshake::GameEntry;
use crate::connect::map_fetch::fetch_map_bundle;
use crate::connect::resolve::{resolve_invite, resolve_server};
use crate::connect::session::{DisambigMode, SessionOutcome, run_session};
use crate::map_store::save_map_bundle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    GameList,
    JoinPrompt,
    IdentityOverlay,
    GameSelect {
        games: Vec<GameEntry>,
        selected: usize,
        server_host: String,
        server_port: u16,
        relay_url: String,
        gate_npub: String,
    },
}

pub struct PickerState {
    pub cache: GameCache,
    pub selected: usize,
    pub screen: Screen,
    pub join_input: String,
    pub status_msg: Option<String>,
    pub npub: String,
    pub identity_count: usize,
    pub identity_type: String,
    pub quit: bool,
}

impl PickerState {
    pub fn new(
        cache: GameCache,
        npub: String,
        identity_count: usize,
        identity_type: String,
    ) -> Self {
        Self {
            cache,
            selected: 0,
            screen: Screen::GameList,
            join_input: String::new(),
            status_msg: None,
            npub,
            identity_count,
            identity_type,
            quit: false,
        }
    }

    pub fn refresh_cache(&mut self) {
        if let Ok(cache) = load_cache() {
            self.cache = cache;
        }
        let len = self.cache.sorted().len();
        if self.selected >= len && len > 0 {
            self.selected = len - 1;
        }
    }
}

pub fn run_picker(
    keys: Keys,
    npub: String,
    gate_npub: String,
    identity_count: usize,
    identity_type: String,
    maps_root: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let cache = load_cache().unwrap_or_else(|_| GameCache::empty());
    let mut state = PickerState::new(cache, npub, identity_count, identity_type);
    let mut session = TerminalSession::enter_picker()?;
    let rt = tokio::runtime::Runtime::new()?;
    let result = run_loop(&mut state, &keys, &gate_npub, &maps_root, &rt, &mut session);
    let _ = session.restore();
    result
}

fn run_loop(
    state: &mut PickerState,
    keys: &Keys,
    gate_npub: &str,
    maps_root: &Path,
    rt: &tokio::runtime::Runtime,
    session: &mut TerminalSession,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::time::Duration;

    loop {
        let (width, height) = crossterm::terminal::size().unwrap_or((80, 25));
        let buffer = render::render_buffer(state, width, height);
        render_to_stdout(&buffer)?;

        if !poll(Duration::from_millis(250))? {
            continue;
        }
        let Event::Key(key) = read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        state.status_msg = None;

        match state.screen {
            Screen::GameList => {
                handle_game_list_key(key, state, keys, gate_npub, maps_root, rt, session)?;
            }
            Screen::JoinPrompt => {
                handle_join_prompt_key(key, state, keys, gate_npub, maps_root, rt, session)?;
            }
            Screen::IdentityOverlay => {
                state.screen = Screen::GameList;
            }
            Screen::GameSelect { .. } => {
                handle_game_select_key(key, state, keys, maps_root, rt, session)?;
            }
        }

        if state.quit {
            break;
        }
    }

    Ok(())
}

fn handle_game_list_key(
    key: KeyEvent,
    state: &mut PickerState,
    keys: &Keys,
    gate_npub: &str,
    maps_root: &Path,
    rt: &tokio::runtime::Runtime,
    session: &mut TerminalSession,
) -> Result<(), Box<dyn std::error::Error>> {
    let game_count = state.cache.sorted().len();
    match key {
        KeyEvent {
            code: KeyCode::Char('q' | 'Q'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        }
        | KeyEvent {
            code: KeyCode::Esc, ..
        } => state.quit = true,
        KeyEvent {
            code: KeyCode::Char('i' | 'I'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => state.screen = Screen::IdentityOverlay,
        KeyEvent {
            code: KeyCode::Char('n' | 'N'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => {
            state.screen = Screen::JoinPrompt;
            state.join_input.clear();
        }
        KeyEvent {
            code: KeyCode::Char('m' | 'M'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => redownload_selected_maps(state, keys, gate_npub, maps_root, rt)?,
        KeyEvent {
            code: KeyCode::Char('j'),
            modifiers: KeyModifiers::NONE,
            ..
        }
        | KeyEvent {
            code: KeyCode::Down,
            ..
        } => move_selection(&mut state.selected, 1, game_count),
        KeyEvent {
            code: KeyCode::Char('k'),
            modifiers: KeyModifiers::NONE,
            ..
        }
        | KeyEvent {
            code: KeyCode::Up, ..
        } => move_selection(&mut state.selected, -1, game_count),
        KeyEvent {
            code: KeyCode::Char('d'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }
        | KeyEvent {
            code: KeyCode::PageDown,
            ..
        } => move_selection(&mut state.selected, 10, game_count),
        KeyEvent {
            code: KeyCode::Char('u'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }
        | KeyEvent {
            code: KeyCode::PageUp,
            ..
        } => move_selection(&mut state.selected, -10, game_count),
        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => {
            if game_count == 0 {
                state.status_msg = Some("No games yet. Press N to join a game.".into());
            } else {
                connect_selected(state, keys, gate_npub, maps_root, rt, session)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_join_prompt_key(
    key: KeyEvent,
    state: &mut PickerState,
    keys: &Keys,
    gate_npub: &str,
    maps_root: &Path,
    rt: &tokio::runtime::Runtime,
    session: &mut TerminalSession,
) -> Result<(), Box<dyn std::error::Error>> {
    match key {
        KeyEvent {
            code: KeyCode::Esc, ..
        } => {
            state.screen = Screen::GameList;
            state.join_input.clear();
        }
        KeyEvent {
            code: KeyCode::Char('q' | 'Q'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } if state.join_input.is_empty() => {
            state.screen = Screen::GameList;
        }
        KeyEvent {
            code: KeyCode::Backspace,
            ..
        } => {
            state.join_input.pop();
        }
        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => {
            let code = state.join_input.trim().to_string();
            state.screen = Screen::GameList;
            if code.is_empty() {
                return Ok(());
            }
            join_with_code(state, &code, keys, gate_npub, maps_root, rt, session)?;
        }
        KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => {
            state.join_input.push(c);
        }
        _ => {}
    }
    Ok(())
}

fn handle_game_select_key(
    key: KeyEvent,
    state: &mut PickerState,
    keys: &Keys,
    maps_root: &Path,
    rt: &tokio::runtime::Runtime,
    session: &mut TerminalSession,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::connect::resolve::ResolvedTarget;

    let Screen::GameSelect {
        ref games,
        ref mut selected,
        ref server_host,
        server_port,
        ref relay_url,
        ref gate_npub,
    } = state.screen
    else {
        return Ok(());
    };
    let game_count = games.len();

    match key {
        KeyEvent {
            code: KeyCode::Esc, ..
        }
        | KeyEvent {
            code: KeyCode::Char('q' | 'Q'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => state.screen = Screen::GameList,
        KeyEvent {
            code: KeyCode::Char('j'),
            modifiers: KeyModifiers::NONE,
            ..
        }
        | KeyEvent {
            code: KeyCode::Down,
            ..
        } => move_selection(selected, 1, game_count),
        KeyEvent {
            code: KeyCode::Char('k'),
            modifiers: KeyModifiers::NONE,
            ..
        }
        | KeyEvent {
            code: KeyCode::Up, ..
        } => move_selection(selected, -1, game_count),
        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => {
            if game_count == 0 {
                state.screen = Screen::GameList;
                return Ok(());
            }
            let chosen = games[*selected].game_id.clone();
            let target = ResolvedTarget {
                server_host: server_host.clone(),
                server_port,
                relay_url: relay_url.clone(),
                invite_code: None,
                game_id: Some(chosen),
            };
            let gate = gate_npub.clone();
            let npub = state.npub.clone();
            state.screen = Screen::GameList;

            let outcome = run_suspended(session, || {
                rt.block_on(run_session(
                    keys,
                    target,
                    &npub,
                    &gate,
                    DisambigMode::Prompt,
                    maps_root,
                ))
            })?;
            state.refresh_cache();
            apply_session_outcome(state, outcome, None);
        }
        _ => {}
    }
    Ok(())
}

fn connect_selected(
    state: &mut PickerState,
    keys: &Keys,
    gate_npub: &str,
    maps_root: &Path,
    rt: &tokio::runtime::Runtime,
    session: &mut TerminalSession,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::config::ConnectConfig;
    use crate::config::load_config;

    let sorted = state.cache.sorted();
    let Some(game) = sorted.get(state.selected).copied() else {
        return Ok(());
    };

    let config = load_config().unwrap_or_else(|_| ConnectConfig::empty());
    let server_str = format!("{}:{}", game.server, game.port);
    let mut target = resolve_server(&server_str, &config)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    target.game_id = Some(game.id.clone());
    let effective_gate: String = if !game.gate_npub.is_empty() {
        game.gate_npub.clone()
    } else {
        gate_npub.to_string()
    };
    drop(sorted);

    let outcome = run_suspended(session, || {
        rt.block_on(run_session(
            keys,
            target.clone(),
            &state.npub,
            &effective_gate,
            DisambigMode::Picker,
            maps_root,
        ))
    })?;
    state.refresh_cache();
    apply_session_outcome(state, outcome, Some((target, effective_gate)));
    Ok(())
}

fn join_with_code(
    state: &mut PickerState,
    code: &str,
    keys: &Keys,
    gate_npub: &str,
    maps_root: &Path,
    rt: &tokio::runtime::Runtime,
    session: &mut TerminalSession,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::config::ConnectConfig;
    use crate::config::load_config;

    let config = load_config().unwrap_or_else(|_| ConnectConfig::empty());
    let target = match resolve_invite(code, &config) {
        Ok(t) => t,
        Err(e) => {
            state.status_msg = Some(format!("Invalid invite code: {e}"));
            return Ok(());
        }
    };

    let outcome = run_suspended(session, || {
        rt.block_on(run_session(
            keys,
            target.clone(),
            &state.npub,
            gate_npub,
            DisambigMode::Picker,
            maps_root,
        ))
    })?;
    state.refresh_cache();
    state.selected = 0;
    apply_session_outcome(state, outcome, Some((target, gate_npub.to_string())));
    Ok(())
}

fn redownload_selected_maps(
    state: &mut PickerState,
    keys: &Keys,
    gate_npub: &str,
    maps_root: &Path,
    rt: &tokio::runtime::Runtime,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::config::ConnectConfig;
    use crate::config::load_config;

    let sorted = state.cache.sorted();
    let Some(game) = sorted.get(state.selected).copied() else {
        state.status_msg = Some("No joined games yet.".into());
        return Ok(());
    };
    let game_id = game.id.clone();
    let game_server = game.server.clone();
    let game_port = game.port;
    let cached_gate_npub = game.gate_npub.clone();

    let effective_gate: String = if !cached_gate_npub.is_empty() {
        cached_gate_npub
    } else if !gate_npub.is_empty() {
        gate_npub.to_string()
    } else {
        state.status_msg =
            Some("Gate key not known for this game. Reconnect once, then try M again.".into());
        return Ok(());
    };

    let config = load_config().unwrap_or_else(|_| ConnectConfig::empty());
    let server_str = format!("{}:{}", game_server, game_port);
    let mut target = match resolve_server(&server_str, &config) {
        Ok(target) => target,
        Err(err) => {
            state.status_msg = Some(format!("Unable to resolve server: {err}"));
            return Ok(());
        }
    };
    target.game_id = Some(game_id.clone());
    drop(sorted);

    match rt.block_on(fetch_map_bundle(keys, &target, &effective_gate, &game_id)) {
        Ok(bundle) => {
            match save_map_bundle(&bundle, &target.server_host, target.server_port, maps_root) {
                Ok(path) => {
                    state.status_msg = Some(format!("Maps saved to {}", path.display()));
                }
                Err(err) => {
                    state.status_msg = Some(format!("Unable to save maps: {err}"));
                }
            }
        }
        Err(err) => {
            state.status_msg = Some(format!("Unable to download maps: {err}"));
        }
    }

    Ok(())
}

fn move_selection(selected: &mut usize, delta: isize, game_count: usize) {
    if game_count == 0 {
        *selected = 0;
        return;
    }
    let current = *selected as isize;
    let max = game_count.saturating_sub(1) as isize;
    *selected = (current + delta).clamp(0, max) as usize;
}

fn run_suspended<T>(
    session: &mut TerminalSession,
    action: impl FnOnce() -> T,
) -> Result<T, Box<dyn std::error::Error>> {
    session.suspend_for_bridge()?;
    let result = action();
    session.resume_after_bridge()?;
    Ok(result)
}

fn apply_session_outcome(
    state: &mut PickerState,
    outcome: SessionOutcome,
    retry_context: Option<(crate::connect::resolve::ResolvedTarget, String)>,
) {
    match outcome {
        SessionOutcome::Done { notice, .. } => {
            if let Some(msg) = notice {
                state.status_msg = Some(msg);
            }
        }
        SessionOutcome::Error(msg) => {
            state.status_msg = Some(format!("Error: {msg}"));
        }
        SessionOutcome::Timeout => {
            state.status_msg = Some("Handshake timed out.".into());
        }
        SessionOutcome::NeedsDisambiguation { games } => {
            if let Some((target, gate_npub)) = retry_context {
                state.screen = Screen::GameSelect {
                    games,
                    selected: 0,
                    server_host: target.server_host,
                    server_port: target.server_port,
                    relay_url: target.relay_url,
                    gate_npub,
                };
            } else {
                state.status_msg = Some("Unexpected disambiguation error.".into());
            }
        }
    }
}
