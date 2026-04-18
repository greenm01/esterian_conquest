use nc_client::password::validate_new_password;
use nc_nostr::hosted::relay_url_to_invite_host;

use super::{
    Effect, GameRow, LobbyTab, Model, Msg, NetworkState, Route, active_session_from_stored,
    append_text, bootstrap_route, field_string_mut, handle_help_click, is_printable_key,
    lobby_route,
};
use crate::input::{KeyCode, MouseButton, MouseEventKind};
use crate::ScreenGeometry;

pub fn update(model: &mut Model, msg: Msg) -> Vec<Effect> {
    match msg {
        Msg::Resize(geometry) => {
            model.geometry = geometry;
            Vec::new()
        }
        Msg::FocusChanged(focused) => {
            model.window_focused = focused;
            Vec::new()
        }
        Msg::BootLoaded(result) => handle_boot_loaded(model, result),
        Msg::IdentityCreated(result) => handle_identity_created(model, result),
        Msg::Unlocked(result) => handle_unlocked(model, result),
        Msg::LobbyUpdated(result) => handle_lobby_updated(model, result),
        Msg::RelaySaved(result) => handle_relay_saved(model, result),
        Msg::Key(key) => handle_key(model, key),
        Msg::TextInput(text) => handle_text_input(model, &text),
        Msg::Mouse(mouse) => handle_mouse(model, mouse),
    }
}

fn handle_boot_loaded(
    model: &mut Model,
    result: Result<crate::storage::BootSnapshot, String>,
) -> Vec<Effect> {
    match result {
        Ok(snapshot) => {
            if !model.relay_overridden {
                model.relay_url = snapshot
                    .relay_url
                    .clone()
                    .unwrap_or_else(|| model.relay_url.clone());
            }
            model.route = bootstrap_route(&snapshot, model.relay_url.clone());
            Vec::new()
        }
        Err(err) => {
            model.route = Route::FatalError(err);
            Vec::new()
        }
    }
}

fn handle_identity_created(
    model: &mut Model,
    result: Result<crate::storage::StoredSession, String>,
) -> Vec<Effect> {
    match result {
        Ok(stored) => {
            let relay_url = model.relay_url.clone();
            let session = active_session_from_stored(stored, current_password(model));
            let nsec = session.active_nsec.clone();
            model.session = Some(session);
            model.route = lobby_route(Some("Identity created.".to_string()), relay_url.clone());
            model.network = NetworkState::Connecting;
            vec![Effect::ConnectTransport { relay_url, nsec }]
        }
        Err(err) => {
            if let Route::FirstRun(first_run) = &mut model.route {
                first_run.status = Some(err);
            }
            Vec::new()
        }
    }
}

fn handle_unlocked(
    model: &mut Model,
    result: Result<crate::storage::StoredSession, String>,
) -> Vec<Effect> {
    match result {
        Ok(stored) => {
            let relay_url = model.relay_url.clone();
            let session = active_session_from_stored(stored, current_password(model));
            let nsec = session.active_nsec.clone();
            model.session = Some(session);
            model.route = lobby_route(Some("Keychain unlocked.".to_string()), relay_url.clone());
            model.network = NetworkState::Connecting;
            vec![Effect::ConnectTransport { relay_url, nsec }]
        }
        Err(err) => {
            if let Route::Locked(locked) = &mut model.route {
                locked.status = Some(err);
                locked.password_input.clear();
            }
            Vec::new()
        }
    }
}

fn handle_lobby_updated(
    model: &mut Model,
    result: Result<crate::transport::LobbySnapshot, String>,
) -> Vec<Effect> {
    if !matches!(model.route, Route::Lobby(_)) {
        return Vec::new();
    }
    match result {
        Ok(snapshot) => {
            model.network = NetworkState::Synced;
            model.games = snapshot.games;
            model.notices = snapshot.notices;
            if let Route::Lobby(lobby) = &mut model.route {
                if lobby.selected_game >= model.games.len() {
                    lobby.selected_game = model.games.len().saturating_sub(1);
                }
                lobby.status = Some(format!(
                    "Catalog synced: {} games, {} notices.",
                    model.games.len(),
                    model.notices.len()
                ));
            }
        }
        Err(err) => {
            model.network = NetworkState::Error;
            if let Route::Lobby(lobby) = &mut model.route {
                lobby.status = Some(err);
            }
        }
    }
    Vec::new()
}

fn handle_relay_saved(model: &mut Model, result: Result<String, String>) -> Vec<Effect> {
    match result {
        Ok(relay_url) => {
            model.relay_url = relay_url.clone();
            if let Route::Lobby(lobby) = &mut model.route {
                lobby.relay_draft = relay_url;
                lobby.status = Some("Relay setting saved.".to_string());
            }
        }
        Err(err) => {
            if let Route::Lobby(lobby) = &mut model.route {
                lobby.status = Some(err);
            }
        }
    }
    Vec::new()
}

fn handle_key(model: &mut Model, key: crate::input::KeyEvent) -> Vec<Effect> {
    match model.route {
        Route::Boot(_) => Vec::new(),
        Route::FatalError(_) => match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                model.should_quit = true;
                vec![Effect::Quit]
            }
            _ => Vec::new(),
        },
        Route::FirstRun(_) => handle_first_run_key(model, key),
        Route::Locked(_) => handle_locked_key(model, key),
        Route::Lobby(_) => handle_lobby_key(model, key),
    }
}

fn handle_mouse(model: &mut Model, mouse: crate::input::MouseEvent) -> Vec<Effect> {
    if mouse.kind == MouseEventKind::Down(MouseButton::Left)
        && handle_help_click(model, mouse.position)
    {
        return Vec::new();
    }

    if let Route::Lobby(lobby) = &mut model.route {
        if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
            let row = mouse.position.row.as_usize();
            if (7..(7 + model.games.len())).contains(&row) {
                let index = row - 7;
                lobby.active_tab = LobbyTab::OpenGames;
                lobby.selected_game = index.min(model.games.len().saturating_sub(1));
            }
        }
    }
    Vec::new()
}

fn handle_text_input(model: &mut Model, text: &str) -> Vec<Effect> {
    match &mut model.route {
        Route::FirstRun(first_run) => {
            append_text(field_string_mut(first_run), text);
            Vec::new()
        }
        Route::Locked(locked) => {
            append_text(&mut locked.password_input, text);
            Vec::new()
        }
        Route::Lobby(lobby) if lobby.active_tab == LobbyTab::Settings && lobby.editing_relay => {
            append_text(&mut lobby.relay_draft, text);
            Vec::new()
        }
        _ => Vec::new(),
    }
}

fn handle_first_run_key(model: &mut Model, key: crate::input::KeyEvent) -> Vec<Effect> {
    let Route::FirstRun(first_run) = &mut model.route else {
        return Vec::new();
    };
    match key.code {
        KeyCode::Tab => {
            first_run.active_field = first_run.active_field.next();
            Vec::new()
        }
        KeyCode::BackTab => {
            first_run.active_field = first_run.active_field.previous();
            Vec::new()
        }
        KeyCode::Backspace => {
            field_string_mut(first_run).pop();
            Vec::new()
        }
        KeyCode::Enter => {
            let handle = first_run.handle_input.trim().to_string();
            let password = first_run.password_input.clone();
            let confirm = first_run.confirm_input.clone();
            let relay = first_run.relay_input.trim().to_string();
            if handle.is_empty() {
                first_run.status = Some("Handle cannot be empty.".to_string());
                return Vec::new();
            }
            if let Err(err) = validate_new_password(&password, &confirm) {
                first_run.status = Some(err);
                return Vec::new();
            }
            if let Err(err) = relay_url_to_invite_host(&relay) {
                first_run.status = Some(err);
                return Vec::new();
            }
            model.relay_url = relay.clone();
            first_run.status = Some("Creating local identity...".to_string());
            vec![Effect::CreateIdentity {
                handle,
                password,
                relay_url: relay,
            }]
        }
        KeyCode::Esc => {
            model.should_quit = true;
            vec![Effect::Quit]
        }
        _ => {
            if let Some(ch) = is_printable_key(key) {
                field_string_mut(first_run).push(ch);
            }
            Vec::new()
        }
    }
}

fn handle_locked_key(model: &mut Model, key: crate::input::KeyEvent) -> Vec<Effect> {
    let Route::Locked(locked) = &mut model.route else {
        return Vec::new();
    };
    match key.code {
        KeyCode::Backspace => {
            locked.password_input.pop();
            Vec::new()
        }
        KeyCode::Enter => {
            let password = locked.password_input.clone();
            if password.is_empty() {
                locked.status = Some("Password cannot be empty.".to_string());
                return Vec::new();
            }
            locked.status = Some("Unlocking local keychain...".to_string());
            vec![Effect::Unlock { password }]
        }
        KeyCode::Esc => {
            model.should_quit = true;
            vec![Effect::Quit]
        }
        _ => {
            if let Some(ch) = is_printable_key(key) {
                locked.password_input.push(ch);
            }
            Vec::new()
        }
    }
}

fn handle_lobby_key(model: &mut Model, key: crate::input::KeyEvent) -> Vec<Effect> {
    let Route::Lobby(lobby) = &mut model.route else {
        return Vec::new();
    };
    if lobby.help_open {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char(_) | KeyCode::Tab | KeyCode::BackTab => {
                lobby.help_open = false;
            }
            _ => {}
        }
        return Vec::new();
    }

    if lobby.active_tab == LobbyTab::Settings && lobby.editing_relay {
        match key.code {
            KeyCode::Enter => {
                let relay_url = lobby.relay_draft.trim().to_string();
                if let Err(err) = relay_url_to_invite_host(&relay_url) {
                    lobby.status = Some(err);
                    return Vec::new();
                }
                lobby.editing_relay = false;
                model.relay_url = relay_url.clone();
                model.network = if model.session.is_some() {
                    NetworkState::Connecting
                } else {
                    NetworkState::Idle
                };
                let mut effects = vec![Effect::SaveRelayUrl {
                    relay_url: relay_url.clone(),
                }];
                if let Some(session) = &model.session {
                    effects.push(Effect::DisconnectTransport);
                    effects.push(Effect::ConnectTransport {
                        relay_url,
                        nsec: session.active_nsec.clone(),
                    });
                }
                return effects;
            }
            KeyCode::Esc => {
                lobby.editing_relay = false;
                lobby.relay_draft = model.relay_url.clone();
                lobby.status = Some("Relay edit cancelled.".to_string());
                return Vec::new();
            }
            KeyCode::Backspace => {
                lobby.relay_draft.pop();
                return Vec::new();
            }
            _ => {
                if let Some(ch) = is_printable_key(key) {
                    lobby.relay_draft.push(ch);
                }
                return Vec::new();
            }
        }
    }

    match key.code {
        KeyCode::Tab => {
            lobby.active_tab = lobby.active_tab.next();
            Vec::new()
        }
        KeyCode::Up => {
            lobby.selected_game = lobby.selected_game.saturating_sub(1);
            Vec::new()
        }
        KeyCode::Down => {
            let max_index = model.games.len().saturating_sub(1);
            lobby.selected_game = (lobby.selected_game + 1).min(max_index);
            Vec::new()
        }
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
            model.should_quit = true;
            vec![Effect::Quit]
        }
        KeyCode::Char('l') | KeyCode::Char('L') => {
            model.session = None;
            model.network = NetworkState::Idle;
            model.games.clear();
            model.notices.clear();
            model.route = Route::Locked(super::LockedModel {
                password_input: String::new(),
                status: Some("Session locked.".to_string()),
            });
            vec![Effect::DisconnectTransport]
        }
        KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::Char('H') => {
            lobby.help_open = true;
            Vec::new()
        }
        KeyCode::Char('1') => {
            lobby.active_tab = LobbyTab::Home;
            Vec::new()
        }
        KeyCode::Char('2') => {
            lobby.active_tab = LobbyTab::OpenGames;
            Vec::new()
        }
        KeyCode::Char('3') => {
            lobby.active_tab = LobbyTab::Comms;
            Vec::new()
        }
        KeyCode::Char('4') => {
            lobby.active_tab = LobbyTab::Settings;
            Vec::new()
        }
        KeyCode::Char('r') | KeyCode::Char('R') if lobby.active_tab == LobbyTab::Settings => {
            lobby.editing_relay = true;
            lobby.relay_draft = model.relay_url.clone();
            lobby.status = Some("Editing relay URL. Enter saves, Esc cancels.".to_string());
            Vec::new()
        }
        _ => Vec::new(),
    }
}

fn current_password(model: &Model) -> String {
    match &model.route {
        Route::FirstRun(first_run) => first_run.password_input.clone(),
        Route::Locked(locked) => locked.password_input.clone(),
        _ => model
            .session
            .as_ref()
            .map(|session| session.password.clone())
            .unwrap_or_default(),
    }
}

#[allow(dead_code)]
fn _assert_screen_geometry_send(_: ScreenGeometry) {}

#[allow(dead_code)]
fn _assert_row_send(_: GameRow) {}
