use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::connecting::{PendingConnectRequest, queue_connect_request};
use crate::wallet::io::now_iso8601;
use crate::wallet::push_identity_from_input;

use super::event::{is_back_key, is_help_key, is_manual_refresh_key};
use super::flows::{
    connect_selected, join_with_code, move_selection, queue_selected_game_refresh,
    redownload_selected_maps,
};
use super::overlay::PickerOverlay;
use super::relay::open_default_relay_editor;
use super::state::{BODY_PAGE, ConnectDisplay, ConnectOrigin, PickerSession, PickerState, Screen};

pub fn handle_game_list_key(
    key: KeyEvent,
    state: &mut PickerState,
    picker_session: &mut PickerSession,
    gate_npub: &str,
    maps_root: &std::path::Path,
    rt: &tokio::runtime::Runtime,
) -> Result<(), Box<dyn std::error::Error>> {
    let game_count = state.cache.sorted().len();
    if is_help_key(key) {
        state.open_help();
        return Ok(());
    }
    if is_manual_refresh_key(key) && state.can_manual_refresh() {
        state.mark_manual_refresh();
        queue_selected_game_refresh(state, gate_npub)?;
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
            code: KeyCode::Char('r' | 'R'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => {
            if let Err(err) = open_default_relay_editor(state) {
                state.show_error(err.to_string());
            }
        }
        KeyEvent {
            code: KeyCode::Char('d' | 'D'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => {
            if game_count == 0 {
                state.show_error("No joined games yet.");
            } else {
                state.overlay = Some(PickerOverlay::GameDeleteConfirm {
                    index: state.selected,
                    step: 1,
                });
            }
        }
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
                state.show_error("No joined games yet.");
            } else {
                connect_selected(state, gate_npub)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn handle_join_prompt_key(
    key: KeyEvent,
    state: &mut PickerState,
    gate_npub: &str,
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
                join_with_code(state, &code, gate_npub)?;
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
        Screen::WalletAddPrompt => handle_wallet_add_key(key, state, picker_session),
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
            if wallet_len == 0 {
                state.show_error("Wallet has no identities.");
            } else {
                state.alias_input = picker_session
                    .wallet
                    .identities
                    .get(state.wallet_selected)
                    .and_then(|identity| identity.alias.clone())
                    .unwrap_or_default();
                state.overlay = Some(PickerOverlay::WalletDetail {
                    index: state.wallet_selected,
                });
            }
        }
        KeyEvent {
            code: KeyCode::Char('n' | 'N'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => {
            state.wallet_input.clear();
            state.screen = Screen::WalletAddPrompt;
        }
        KeyEvent {
            code: KeyCode::Char('a' | 'A'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => {
            if wallet_len == 0 {
                state.show_error("Wallet has no identities.");
            } else {
                let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                    picker_session.switch_active(state.wallet_selected)?;
                    picker_session.save()?;
                    Ok(())
                })();
                if let Err(err) = result {
                    state.show_error(err.to_string());
                }
            }
        }
        KeyEvent {
            code: KeyCode::Char('d' | 'D'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => {
            if wallet_len <= 1 {
                state.show_error("wallet must keep at least one identity");
            } else {
                state.overlay = Some(PickerOverlay::WalletDeleteConfirm {
                    index: state.wallet_selected,
                    step: 1,
                });
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_wallet_add_key(
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
            state.wallet_input.clear();
            state.screen = Screen::WalletList;
        }
        KeyEvent {
            code: KeyCode::Backspace,
            ..
        } => {
            state.wallet_input.pop();
        }
        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => {
            match push_identity_from_input(
                &mut picker_session.wallet,
                &state.wallet_input,
                now_iso8601(),
            ) {
                Ok(_) => {
                    if let Err(err) = picker_session.save() {
                        state.wallet_input.clear();
                        state.screen = Screen::WalletList;
                        state.show_error(err.to_string());
                        return Ok(());
                    }
                    state.wallet_selected =
                        picker_session.wallet.identities.len().saturating_sub(1);
                    state.wallet_input.clear();
                    state.alias_input.clear();
                    state.screen = Screen::WalletList;
                    state.overlay = Some(PickerOverlay::WalletDetail {
                        index: state.wallet_selected,
                    });
                }
                Err(err) => {
                    state.wallet_input.clear();
                    state.screen = Screen::WalletList;
                    state.show_error(err.to_string());
                }
            }
        }
        KeyEvent {
            code: KeyCode::Char(ch),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => {
            state.wallet_input.push(ch);
        }
        _ => {}
    }
    Ok(())
}

pub fn handle_game_select_key(
    key: KeyEvent,
    state: &mut PickerState,
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
                gate_npub: None,
            };
            let game = games[*selected].clone();
            queue_connect_request(
                state,
                PendingConnectRequest {
                    origin: ConnectOrigin::GameSelect,
                    display: ConnectDisplay::from_game(
                        &format!("{} (Seat {})", game.name, game.seat),
                        &target,
                    ),
                    target,
                    gate_npub: gate_npub.clone(),
                },
            );
        }
        _ => {}
    }
    Ok(())
}
