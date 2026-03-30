use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::connecting::{PendingConnectRequest, queue_connect_request};
use crate::wallet::io::now_iso8601;
use crate::wallet::push_identity_from_input;

use super::event::{is_back_key, is_escape_key, is_help_key, is_manual_refresh_key};
use super::flows::{
    connect_selected, move_selection, open_maps_download_popup, persist_maps_root,
    queue_selected_game_refresh, redownload_selected_maps,
};
use super::overlay::PickerOverlay;
use super::relay::{
    handle_relay_games_key, handle_relay_list_key, open_relay_list, open_selected_game_relay_prompt,
};
use super::state::{BODY_PAGE, ConnectDisplay, ConnectOrigin, PickerSession, PickerState, Screen};

pub fn handle_game_list_key(
    key: KeyEvent,
    state: &mut PickerState,
    _picker_session: &mut PickerSession,
    gate_npub: &str,
    _rt: &tokio::runtime::Runtime,
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
            state.join_input.clear();
            state.overlay = Some(super::overlay::PickerOverlay::JoinCodePopup { error: None });
        }
        KeyEvent {
            code: KeyCode::Char('m' | 'M'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => {
            if game_count == 0 {
                state.show_error("No joined games yet.");
            } else {
                open_maps_download_popup(state);
            }
        }
        KeyEvent {
            code: KeyCode::Char('r'),
            modifiers: KeyModifiers::NONE,
            ..
        } => open_relay_list(state),
        KeyEvent {
            code: KeyCode::Char('R'),
            modifiers: KeyModifiers::SHIFT,
            ..
        } => open_selected_game_relay_prompt(state),
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

/// Key handler for the [`super::overlay::PickerOverlay::JoinCodePopup`] overlay.
pub fn handle_join_code_popup_key(
    key: KeyEvent,
    state: &mut PickerState,
    gate_npub: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match key {
        // Only Esc cancels — not 'q', since invite text may legitimately contain it.
        key if is_escape_key(key) => {
            state.overlay = None;
            state.join_input.clear();
        }
        KeyEvent {
            code: KeyCode::Backspace,
            ..
        } => {
            state.join_input.pop();
            // Clear any stale error so it doesn't persist after editing.
            state.overlay = Some(super::overlay::PickerOverlay::JoinCodePopup { error: None });
        }
        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => {
            let code = state.join_input.trim().to_string();
            if !code.is_empty() {
                super::flows::join_with_code(state, &code, gate_npub)?;
            }
        }
        KeyEvent {
            code: KeyCode::Char(ch),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => {
            state.join_input.push(ch);
            // Clear any stale error on new input.
            state.overlay = Some(super::overlay::PickerOverlay::JoinCodePopup { error: None });
        }
        _ => {}
    }
    Ok(())
}

pub fn handle_maps_download_popup_key(
    key: KeyEvent,
    state: &mut PickerState,
    picker_session: Option<&PickerSession>,
    gate_npub: &str,
    rt: Option<&tokio::runtime::Runtime>,
) -> Result<(), Box<dyn std::error::Error>> {
    match key {
        key if is_escape_key(key) => {
            state.overlay = None;
            state.maps_input.clear();
            state.maps_input_prefilled = false;
        }
        KeyEvent {
            code: KeyCode::Backspace,
            ..
        } => {
            if state.maps_input_prefilled {
                state.maps_input.clear();
                state.maps_input_prefilled = false;
            } else {
                state.maps_input.pop();
            }
            state.overlay = Some(PickerOverlay::MapsDownloadPrompt { error: None });
        }
        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => {
            let Some(picker_session) = picker_session else {
                return Ok(());
            };
            let Some(rt) = rt else {
                return Ok(());
            };
            match persist_maps_root(state) {
                Ok(_) => {
                    state.overlay = None;
                    state.maps_input.clear();
                    state.maps_input_prefilled = false;
                    redownload_selected_maps(state, &picker_session.keys, gate_npub, rt)?;
                }
                Err(err) => {
                    state.maps_input_prefilled = false;
                    state.overlay = Some(PickerOverlay::MapsDownloadPrompt {
                        error: Some(err.to_string()),
                    });
                }
            }
        }
        KeyEvent {
            code: KeyCode::Char(ch),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => {
            if state.maps_input_prefilled {
                state.maps_input.clear();
                state.maps_input_prefilled = false;
            }
            state.maps_input.push(ch);
            state.overlay = Some(PickerOverlay::MapsDownloadPrompt { error: None });
        }
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

pub fn handle_relay_key(
    key: KeyEvent,
    state: &mut PickerState,
) -> Result<(), Box<dyn std::error::Error>> {
    match state.screen.clone() {
        Screen::RelayList => handle_relay_list_key(key, state),
        Screen::RelayGames { relay_url } => handle_relay_games_key(key, state, &relay_url),
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
