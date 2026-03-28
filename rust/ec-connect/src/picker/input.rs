use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::wallet::io::now_iso8601;
use crate::wallet::{push_imported_identity, push_new_identity};

use super::event::{is_back_key, is_help_key};
use super::flows::{
    apply_session_outcome, connect_selected, join_with_code, move_selection,
    redownload_selected_maps,
};
use super::state::{BODY_PAGE, PickerSession, PickerState, Screen};

pub fn handle_game_list_key(
    key: KeyEvent,
    state: &mut PickerState,
    picker_session: &mut PickerSession,
    gate_npub: &str,
    maps_root: &std::path::Path,
    rt: &tokio::runtime::Runtime,
    session: &mut ec_ui::session::TerminalSession,
) -> Result<(), Box<dyn std::error::Error>> {
    let game_count = state.cache.sorted().len();
    if is_help_key(key) {
        state.open_help();
        return Ok(());
    }
    match key {
        key if is_back_key(key) => state.request_quit(),
        KeyEvent {
            code: KeyCode::Char('i' | 'I'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => state.screen = Screen::IdentityOverlay,
        KeyEvent {
            code: KeyCode::Char('w' | 'W'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => state.screen = Screen::WalletList,
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
        } => redownload_selected_maps(state, &picker_session.keys, gate_npub, maps_root, rt)?,
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
        } => move_selection(&mut state.selected, BODY_PAGE, game_count),
        KeyEvent {
            code: KeyCode::Char('u'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }
        | KeyEvent {
            code: KeyCode::PageUp,
            ..
        } => move_selection(&mut state.selected, -BODY_PAGE, game_count),
        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => {
            if game_count == 0 {
                state.show_notice("No joined games yet.");
            } else {
                connect_selected(state, picker_session, gate_npub, maps_root, rt, session)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn handle_join_prompt_key(
    key: KeyEvent,
    state: &mut PickerState,
    picker_session: &mut PickerSession,
    gate_npub: &str,
    maps_root: &std::path::Path,
    rt: &tokio::runtime::Runtime,
    session: &mut ec_ui::session::TerminalSession,
) -> Result<(), Box<dyn std::error::Error>> {
    if is_help_key(key) {
        state.open_help();
        return Ok(());
    }
    match key {
        key if is_back_key(key) => {
            state.screen = Screen::GameList;
            state.join_input.clear();
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
            if !code.is_empty() {
                join_with_code(
                    state,
                    &code,
                    picker_session,
                    gate_npub,
                    maps_root,
                    rt,
                    session,
                )?;
            }
        }
        KeyEvent {
            code: KeyCode::Char(ch),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => state.join_input.push(ch),
        _ => {}
    }
    Ok(())
}

pub fn handle_identity_overlay_key(key: KeyEvent, state: &mut PickerState) {
    if is_help_key(key) {
        state.open_help();
    } else {
        state.screen = Screen::GameList;
    }
}

pub fn handle_wallet_key(
    key: KeyEvent,
    state: &mut PickerState,
    picker_session: &mut PickerSession,
) -> Result<(), Box<dyn std::error::Error>> {
    match state.screen {
        Screen::WalletList => handle_wallet_list_key(key, state, picker_session),
        Screen::WalletAliasPrompt => handle_wallet_alias_key(key, state, picker_session),
        Screen::WalletImportPrompt => handle_wallet_import_key(key, state, picker_session),
        _ => Ok(()),
    }
}

fn handle_wallet_list_key(
    key: KeyEvent,
    state: &mut PickerState,
    picker_session: &mut PickerSession,
) -> Result<(), Box<dyn std::error::Error>> {
    let wallet_len = picker_session.wallet.identities.len();
    if is_help_key(key) {
        state.open_help();
        return Ok(());
    }
    match key {
        key if is_back_key(key) => state.screen = Screen::GameList,
        KeyEvent {
            code: KeyCode::Char('j'),
            modifiers: KeyModifiers::NONE,
            ..
        }
        | KeyEvent {
            code: KeyCode::Down,
            ..
        } => move_selection(&mut state.wallet_selected, 1, wallet_len),
        KeyEvent {
            code: KeyCode::Char('k'),
            modifiers: KeyModifiers::NONE,
            ..
        }
        | KeyEvent {
            code: KeyCode::Up, ..
        } => move_selection(&mut state.wallet_selected, -1, wallet_len),
        KeyEvent {
            code: KeyCode::Char('d'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }
        | KeyEvent {
            code: KeyCode::PageDown,
            ..
        } => move_selection(&mut state.wallet_selected, BODY_PAGE, wallet_len),
        KeyEvent {
            code: KeyCode::Char('u'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }
        | KeyEvent {
            code: KeyCode::PageUp,
            ..
        } => move_selection(&mut state.wallet_selected, -BODY_PAGE, wallet_len),
        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => {
            let npub = picker_session.switch_active(state.wallet_selected)?;
            picker_session.save()?;
            state.show_notice(format!(
                "Active identity: {}",
                super::render::short_npub(&npub)
            ));
        }
        KeyEvent {
            code: KeyCode::Char('n' | 'N'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => {
            let npub = push_new_identity(&mut picker_session.wallet, now_iso8601())?;
            picker_session.save()?;
            state.wallet_selected = picker_session.wallet.identities.len().saturating_sub(1);
            state.show_notice(format!(
                "Created identity: {}",
                super::render::short_npub(&npub)
            ));
        }
        KeyEvent {
            code: KeyCode::Char('i' | 'I'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => {
            state.import_input.clear();
            state.screen = Screen::WalletImportPrompt;
        }
        KeyEvent {
            code: KeyCode::Char('a' | 'A'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => {
            state.alias_input = picker_session
                .wallet
                .identities
                .get(state.wallet_selected)
                .and_then(|identity| identity.alias.clone())
                .unwrap_or_default();
            state.screen = Screen::WalletAliasPrompt;
        }
        _ => {}
    }
    Ok(())
}

fn handle_wallet_alias_key(
    key: KeyEvent,
    state: &mut PickerState,
    picker_session: &mut PickerSession,
) -> Result<(), Box<dyn std::error::Error>> {
    if is_help_key(key) {
        state.open_help();
        return Ok(());
    }
    match key {
        key if is_back_key(key) => {
            state.alias_input.clear();
            state.screen = Screen::WalletList;
        }
        KeyEvent {
            code: KeyCode::Backspace,
            ..
        } => {
            state.alias_input.pop();
        }
        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => {
            let alias = state.alias_input.trim().to_string();
            if let Some(identity) = picker_session
                .wallet
                .identities
                .get_mut(state.wallet_selected)
            {
                identity.alias = if alias.is_empty() { None } else { Some(alias) };
            }
            picker_session.save()?;
            state.alias_input.clear();
            state.screen = Screen::WalletList;
            state.show_notice("Alias updated.");
        }
        KeyEvent {
            code: KeyCode::Char(ch),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => {
            if state.alias_input.chars().count() < 20 {
                state.alias_input.push(ch);
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_wallet_import_key(
    key: KeyEvent,
    state: &mut PickerState,
    picker_session: &mut PickerSession,
) -> Result<(), Box<dyn std::error::Error>> {
    if is_help_key(key) {
        state.open_help();
        return Ok(());
    }
    match key {
        key if is_back_key(key) => {
            state.import_input.clear();
            state.screen = Screen::WalletList;
        }
        KeyEvent {
            code: KeyCode::Backspace,
            ..
        } => {
            state.import_input.pop();
        }
        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => {
            let input = state.import_input.trim().to_string();
            if input.is_empty() {
                state.show_error("nsec cannot be empty.");
            } else {
                match push_imported_identity(&mut picker_session.wallet, &input, now_iso8601()) {
                    Ok(npub) => {
                        picker_session.save()?;
                        state.wallet_selected =
                            picker_session.wallet.identities.len().saturating_sub(1);
                        state.import_input.clear();
                        state.screen = Screen::WalletList;
                        state.show_notice(format!(
                            "Imported identity: {}",
                            super::render::short_npub(&npub)
                        ));
                    }
                    Err(err) => state.show_error(err.to_string()),
                }
            }
        }
        KeyEvent {
            code: KeyCode::Char(ch),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => state.import_input.push(ch),
        _ => {}
    }
    Ok(())
}

pub fn handle_game_select_key(
    key: KeyEvent,
    state: &mut PickerState,
    picker_session: &mut PickerSession,
    maps_root: &std::path::Path,
    rt: &tokio::runtime::Runtime,
    session: &mut ec_ui::session::TerminalSession,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::connect::resolve::ResolvedTarget;

    if is_help_key(key) {
        state.open_help();
        return Ok(());
    }

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
        key if is_back_key(key) => state.screen = Screen::GameList,
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
            code: KeyCode::Char('d'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }
        | KeyEvent {
            code: KeyCode::PageDown,
            ..
        } => move_selection(selected, BODY_PAGE, game_count),
        KeyEvent {
            code: KeyCode::Char('u'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }
        | KeyEvent {
            code: KeyCode::PageUp,
            ..
        } => move_selection(selected, -BODY_PAGE, game_count),
        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => {
            if game_count == 0 {
                state.screen = Screen::GameList;
                return Ok(());
            }
            let target = ResolvedTarget {
                server_host: server_host.clone(),
                server_port,
                relay_url: relay_url.clone(),
                invite_code: None,
                game_id: Some(games[*selected].game_id.clone()),
            };
            let gate = gate_npub.clone();
            state.screen = Screen::GameList;
            let outcome = {
                session.suspend_for_bridge()?;
                let outcome = rt.block_on(crate::connect::session::run_session(
                    &picker_session.keys,
                    target,
                    &picker_session.npub,
                    &gate,
                    crate::connect::session::DisambigMode::Prompt,
                    maps_root,
                ));
                session.resume_after_bridge()?;
                outcome
            };
            state.refresh_cache();
            apply_session_outcome(state, outcome, None);
        }
        _ => {}
    }
    Ok(())
}
