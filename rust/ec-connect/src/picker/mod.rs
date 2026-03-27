//! Game picker TUI.
//!
//! Entry point: `run_picker(keys, npub, gate_npub_fn)`.
//!
//! The picker is a minimal ratatui application that shows the player's
//! joined games and provides navigation to connect, join new games, or
//! manage identity.  All async work (handshake, bridge) is run via a
//! tokio runtime that is created once and reused across sessions.
//!
//! Module layout:
//!   mod.rs   — public entry point and shared state types
//!   render.rs — ratatui draw functions
//!   event.rs  — key event handling and state transitions

pub mod event;
pub mod render;

use std::io;

use crossterm::event::EnableMouseCapture;
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use nostr_sdk::Keys;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::cache::{load_cache, GameCache};
use crate::connect::resolve::{resolve_invite, resolve_server};
use crate::connect::session::{run_session, DisambigMode, SessionOutcome};

// ── State ─────────────────────────────────────────────────────────────────────

/// Which screen is being shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    /// Main game list.
    GameList,
    /// Inline join-code input (bottom bar replaced with prompt).
    JoinPrompt,
    /// Identity info overlay.
    IdentityOverlay,
}

/// Full picker application state.
pub struct PickerState {
    /// Sorted game list (refreshed after each session).
    pub cache: GameCache,
    /// Currently selected row index.
    pub selected: usize,
    /// Which screen is active.
    pub screen: Screen,
    /// Text being typed in the join prompt.
    pub join_input: String,
    /// Status message shown at the bottom (cleared on next key).
    pub status_msg: Option<String>,
    /// The player's active npub (for display and gate queries).
    pub npub: String,
    /// Count of identities in the wallet (for identity overlay).
    pub identity_count: usize,
    /// Active identity type ("local" or "imported").
    pub identity_type: String,
    /// Whether the user has requested quit.
    pub quit: bool,
}

impl PickerState {
    pub fn new(
        cache: GameCache,
        npub: String,
        identity_count: usize,
        identity_type: String,
    ) -> Self {
        PickerState {
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

    /// Reload the cache from disk and clamp selection.
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

// ── Public entry point ────────────────────────────────────────────────────────

/// Run the picker TUI.  Blocks until the user quits.
///
/// `keys`          — active identity's Nostr keypair  
/// `npub`          — active identity's npub string (for display + SSH username)  
/// `gate_npub`     — gate's Nostr public key (required for handshake)  
/// `identity_count`— total identities in wallet  
/// `identity_type` — "local" or "imported"  
pub fn run_picker(
    keys: Keys,
    npub: String,
    gate_npub: String,
    identity_count: usize,
    identity_type: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let cache = load_cache().unwrap_or_else(|_| GameCache::empty());
    let mut state = PickerState::new(cache, npub, identity_count, identity_type);

    // Set up terminal.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Build tokio runtime for async session calls.
    let rt = tokio::runtime::Runtime::new()?;

    let result = run_loop(&mut terminal, &mut state, &keys, &gate_npub, &rt);

    // Restore terminal unconditionally.
    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture,
    );
    let _ = terminal.show_cursor();

    result
}

// ── Main event loop ───────────────────────────────────────────────────────────

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut PickerState,
    keys: &Keys,
    gate_npub: &str,
    rt: &tokio::runtime::Runtime,
) -> Result<(), Box<dyn std::error::Error>> {
    use crossterm::event::{self, Event, KeyEventKind};
    use std::time::Duration;

    loop {
        terminal.draw(|f| render::draw(f, state))?;

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        state.status_msg = None;

        match state.screen {
            Screen::GameList => {
                handle_game_list_key(key.code, state, keys, gate_npub, rt)?;
            }
            Screen::JoinPrompt => {
                handle_join_prompt_key(key.code, state, keys, gate_npub, rt)?;
            }
            Screen::IdentityOverlay => {
                // Any key dismisses the overlay.
                state.screen = Screen::GameList;
            }
        }

        if state.quit {
            break;
        }
    }

    Ok(())
}

// ── Key handlers ──────────────────────────────────────────────────────────────

fn handle_game_list_key(
    code: crossterm::event::KeyCode,
    state: &mut PickerState,
    keys: &Keys,
    gate_npub: &str,
    rt: &tokio::runtime::Runtime,
) -> Result<(), Box<dyn std::error::Error>> {
    use crossterm::event::KeyCode;

    let game_count = state.cache.sorted().len();

    match code {
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
            state.quit = true;
        }
        KeyCode::Char('i') | KeyCode::Char('I') => {
            state.screen = Screen::IdentityOverlay;
        }
        KeyCode::Char('j') | KeyCode::Char('J') => {
            state.screen = Screen::JoinPrompt;
            state.join_input.clear();
        }
        KeyCode::Up => {
            if state.selected > 0 {
                state.selected -= 1;
            }
        }
        KeyCode::Down => {
            if game_count > 0 && state.selected < game_count - 1 {
                state.selected += 1;
            }
        }
        KeyCode::Enter => {
            if game_count == 0 {
                state.status_msg = Some("No games yet. Press J to join a game.".into());
                return Ok(());
            }
            connect_selected(state, keys, gate_npub, rt)?;
        }
        _ => {}
    }
    Ok(())
}

fn handle_join_prompt_key(
    code: crossterm::event::KeyCode,
    state: &mut PickerState,
    keys: &Keys,
    gate_npub: &str,
    rt: &tokio::runtime::Runtime,
) -> Result<(), Box<dyn std::error::Error>> {
    use crossterm::event::KeyCode;

    match code {
        KeyCode::Esc => {
            state.screen = Screen::GameList;
            state.join_input.clear();
        }
        KeyCode::Backspace => {
            state.join_input.pop();
        }
        KeyCode::Char(c) => {
            state.join_input.push(c);
        }
        KeyCode::Enter => {
            let code = state.join_input.trim().to_string();
            if code.is_empty() {
                state.screen = Screen::GameList;
                return Ok(());
            }
            state.screen = Screen::GameList;
            join_with_code(state, &code, keys, gate_npub, rt)?;
        }
        _ => {}
    }
    Ok(())
}

// ── Session actions ───────────────────────────────────────────────────────────

/// Connect to the currently selected game.
fn connect_selected(
    state: &mut PickerState,
    keys: &Keys,
    gate_npub: &str,
    rt: &tokio::runtime::Runtime,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::config::load_config;
    use crate::config::ConnectConfig;

    let sorted = state.cache.sorted();
    let Some(game) = sorted.get(state.selected).copied() else {
        return Ok(());
    };

    let config = load_config().unwrap_or_else(|_| ConnectConfig::empty());
    let server_str = format!("{}:{}", game.server, game.port);
    let mut target = resolve_server(&server_str, &config)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    target.game_id = Some(game.id.clone());

    let outcome = rt.block_on(run_session(
        keys,
        target,
        &state.npub,
        gate_npub,
        DisambigMode::Prompt,
    ));

    state.refresh_cache();

    match outcome {
        SessionOutcome::Done { .. } => {}
        SessionOutcome::Error(msg) => {
            state.status_msg = Some(format!("Error: {msg}"));
        }
        SessionOutcome::Timeout => {
            state.status_msg = Some("Handshake timed out.".into());
        }
    }

    Ok(())
}

/// Join a game using an invite code entered in the prompt.
fn join_with_code(
    state: &mut PickerState,
    code: &str,
    keys: &Keys,
    gate_npub: &str,
    rt: &tokio::runtime::Runtime,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::config::load_config;
    use crate::config::ConnectConfig;

    let config = load_config().unwrap_or_else(|_| ConnectConfig::empty());
    let target = match resolve_invite(code, &config) {
        Ok(t) => t,
        Err(e) => {
            state.status_msg = Some(format!("Invalid invite code: {e}"));
            return Ok(());
        }
    };

    let outcome = rt.block_on(run_session(
        keys,
        target,
        &state.npub,
        gate_npub,
        DisambigMode::Prompt,
    ));

    state.refresh_cache();
    // After joining, select the newly added game (top of sorted list).
    state.selected = 0;

    match outcome {
        SessionOutcome::Done { .. } => {}
        SessionOutcome::Error(msg) => {
            state.status_msg = Some(format!("Error: {msg}"));
        }
        SessionOutcome::Timeout => {
            state.status_msg = Some("Handshake timed out.".into());
        }
    }

    Ok(())
}
