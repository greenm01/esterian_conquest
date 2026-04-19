use nc_client::password::validate_new_password;
use nc_nostr::hosted::relay_url_to_invite_host;

use super::{
    Effect, FirstJoinSetupField, HostedGameModel, LOBBY_TAB_ROW, LOCK_TIMEOUT_OPTIONS, LobbyTab,
    Model, Msg, NetworkState, Route, active_session_from_stored, append_text, bootstrap_route,
    field_string_mut, first_join_setup_from_snapshot, handle_help_click, is_printable_key,
    lobby_route, trim_first_join_setup_input,
};
use crate::ScreenGeometry;
use crate::dashboard;
use crate::input::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};

const TABLE_DATA_ROW_START: usize = 7;

pub fn update(model: &mut Model, msg: Msg) -> Vec<Effect> {
    match msg {
        Msg::Resize(geometry) => {
            model.geometry = geometry;
            model
                .matrix_rain
                .reset_for_size(geometry.width(), geometry.height());
            if let Route::HostedGame(hosted) = &mut model.route {
                dashboard::resize_hosted_app(
                    &mut hosted.dashboard,
                    geometry.width(),
                    geometry.height(),
                );
            }
            Vec::new()
        }
        Msg::FocusChanged(focused) => {
            model.window_focused = focused;
            Vec::new()
        }
        Msg::MatrixFrame => {
            if matches!(model.route, Route::MatrixLocked) {
                model.matrix_rain.advance();
            }
            Vec::new()
        }
        Msg::IdleLock => handle_idle_lock(model),
        Msg::BootLoaded(result) => handle_boot_loaded(model, result),
        Msg::IdentityCreated(result) => handle_identity_created(model, result),
        Msg::Unlocked(result) => handle_unlocked(model, result),
        Msg::LobbyUpdated(result) => handle_lobby_updated(model, result),
        Msg::LobbyRefreshed(result) => handle_lobby_refreshed(model, result),
        Msg::SandboxJoined(result) => handle_sandbox_joined(model, result),
        Msg::SandboxReleased(result) => handle_sandbox_released(model, result),
        Msg::HostedGameOpened(result) => handle_hosted_game_opened(model, result),
        Msg::FirstJoinSetupCompleted(result) => handle_first_join_setup_completed(model, result),
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
            model.lock_timeout_minutes = snapshot.lock_timeout_minutes;
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
            let cache = stored.cache.clone();
            let session = active_session_from_stored(stored, current_password(model));
            let nsec = session.active_nsec.clone();
            model.cache = cache.clone();
            model.session = Some(session);
            model.route = lobby_route(Some("Identity created.".to_string()), relay_url.clone());
            model.network = NetworkState::Connecting;
            vec![Effect::ConnectTransport {
                relay_url,
                nsec,
                cache,
            }]
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
            let cache = stored.cache.clone();
            let session = active_session_from_stored(stored, current_password(model));
            let nsec = session.active_nsec.clone();
            model.cache = cache.clone();
            model.session = Some(session);
            model.route = lobby_route(Some("Keychain unlocked.".to_string()), relay_url.clone());
            model.network = NetworkState::Connecting;
            vec![Effect::ConnectTransport {
                relay_url,
                nsec,
                cache,
            }]
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
    match result {
        Ok(snapshot) => {
            apply_lobby_snapshot(model, snapshot);
            if let Route::Lobby(lobby) = &mut model.route {
                lobby.status = None;
            }
            if let Some(session) = &model.session {
                return vec![Effect::SaveClientCache {
                    cache: model.cache.clone(),
                    password: session.password.clone(),
                }];
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

fn handle_lobby_refreshed(
    model: &mut Model,
    result: Result<crate::transport::LobbySnapshot, String>,
) -> Vec<Effect> {
    match result {
        Ok(snapshot) => {
            apply_lobby_snapshot(model, snapshot);
            if let Route::Lobby(lobby) = &mut model.route {
                lobby.status = Some("Hosted lobby refreshed.".to_string());
            }
            if let Some(session) = &model.session {
                return vec![Effect::SaveClientCache {
                    cache: model.cache.clone(),
                    password: session.password.clone(),
                }];
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

fn handle_sandbox_joined(
    model: &mut Model,
    result: Result<crate::transport::SandboxJoinResult, String>,
) -> Vec<Effect> {
    match result {
        Ok(crate::transport::SandboxJoinResult::Joined(success)) => {
            model.cache = success.cache.clone();
            upsert_joined_game(model, success.row.clone());
            open_snapshot_route(
                model,
                success.row,
                success.snapshot,
                Some("Sandbox joined.".to_string()),
            )
        }
        Ok(crate::transport::SandboxJoinResult::Full(message)) => {
            let row = match &model.route {
                Route::SandboxJoinConfirm(row) => row.clone(),
                _ => return Vec::new(),
            };
            model.route = Route::SandboxJoinUnavailable {
                row,
                notice: message,
            };
            Vec::new()
        }
        Err(err) => return_to_lobby_with_status(model, err),
    }
}

fn handle_sandbox_released(
    model: &mut Model,
    result: Result<crate::transport::SandboxReleaseSuccess, String>,
) -> Vec<Effect> {
    match result {
        Ok(success) => {
            model.cache = success.cache;
            model.my_games.retain(|row| row.game_id != success.game_id);
            let mut effects =
                return_to_lobby_with_status(model, "Sandbox removed from My Games.".to_string());
            if let Some(session) = &model.session {
                effects.push(Effect::SaveClientCache {
                    cache: model.cache.clone(),
                    password: session.password.clone(),
                });
            }
            effects
        }
        Err(err) => return_to_lobby_with_status(model, err),
    }
}

fn handle_hosted_game_opened(
    model: &mut Model,
    result: Result<crate::transport::HostedGameOpenResult, String>,
) -> Vec<Effect> {
    match result {
        Ok(crate::transport::HostedGameOpenResult::Opened(success)) => {
            model.cache = success.cache.clone();
            upsert_joined_game(model, success.row.clone());
            open_snapshot_route(model, success.row, success.snapshot, None)
        }
        Ok(crate::transport::HostedGameOpenResult::Expired {
            row,
            cache,
            message,
        }) => {
            model.cache = cache;
            upsert_joined_game(model, row);
            return_to_lobby_with_status(model, message)
        }
        Err(err) => return_to_lobby_with_status(model, err),
    }
}

fn handle_first_join_setup_completed(
    model: &mut Model,
    result: Result<crate::transport::HostedGameOpenSuccess, String>,
) -> Vec<Effect> {
    match result {
        Ok(success) => {
            model.cache = success.cache.clone();
            upsert_joined_game(model, success.row.clone());
            open_snapshot_route(model, success.row, success.snapshot, None)
        }
        Err(err) => {
            if let Route::FirstJoinSetup(setup) = &mut model.route {
                setup.status = Some(err);
            }
            Vec::new()
        }
    }
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
    if is_quit_shortcut(key) {
        model.should_quit = true;
        return vec![Effect::Quit];
    }
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
        Route::MatrixLocked => handle_matrix_locked_key(model, key),
        Route::Locked(_) => handle_locked_key(model, key),
        Route::Lobby(_) => handle_lobby_key(model, key),
        Route::SandboxJoinConfirm(_) => handle_sandbox_join_confirm_key(model, key),
        Route::SandboxJoinUnavailable { .. } => handle_sandbox_join_unavailable_key(model),
        Route::SandboxDeleteConfirm(_) => handle_sandbox_delete_confirm_key(model, key),
        Route::FirstJoinSetup(_) => handle_first_join_setup_key(model, key),
        Route::HostedGame(_) => handle_hosted_game_key(model, key),
    }
}

fn handle_mouse(model: &mut Model, mouse: crate::input::MouseEvent) -> Vec<Effect> {
    if mouse.kind == MouseEventKind::Down(MouseButton::Left)
        && handle_help_click(model, mouse.position)
    {
        return Vec::new();
    }

    match &mut model.route {
        Route::Lobby(lobby) => {
            if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
                return Vec::new();
            }

            let row = mouse.position.row.as_usize();
            let column = mouse.position.column.as_usize();
            if row == LOBBY_TAB_ROW {
                for (tab, start, end) in super::lobby_tab_bounds(model.geometry) {
                    if column >= start && column < end {
                        lobby.active_tab = tab;
                        return Vec::new();
                    }
                }
            }

            if row < TABLE_DATA_ROW_START || row >= model.geometry.height().saturating_sub(4) {
                return Vec::new();
            }

            let index = row.saturating_sub(TABLE_DATA_ROW_START);
            match lobby.active_tab {
                LobbyTab::MyGames if !model.my_games.is_empty() => {
                    lobby.selected_my_game = index.min(model.my_games.len().saturating_sub(1));
                }
                LobbyTab::OpenGames if !model.open_games.is_empty() => {
                    lobby.selected_open_game = index.min(model.open_games.len().saturating_sub(1));
                }
                _ => {}
            }
            Vec::new()
        }
        Route::HostedGame(hosted) => {
            let Some(mapped) = dashboard_mouse_event(mouse) else {
                return Vec::new();
            };
            let _ = dashboard::dispatch_hosted_mouse(&mut hosted.dashboard, mapped);
            Vec::new()
        }
        _ => Vec::new(),
    }
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
        Route::FirstJoinSetup(setup) => {
            let input = match setup.active_field {
                FirstJoinSetupField::Empire => &mut setup.empire_input,
                FirstJoinSetupField::Homeworld => &mut setup.homeworld_input,
            };
            append_text(input, text);
            trim_first_join_setup_input(input);
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

fn handle_matrix_locked_key(model: &mut Model, key: crate::input::KeyEvent) -> Vec<Effect> {
    match key.code {
        _ => {
            model.route = Route::Locked(super::LockedModel {
                password_input: String::new(),
                status: None,
                resume_session: true,
            });
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
            locked.status = None;
            vec![Effect::Unlock { password }]
        }
        KeyCode::Esc => {
            if locked.resume_session {
                model.route = Route::MatrixLocked;
                Vec::new()
            } else {
                model.should_quit = true;
                vec![Effect::Quit]
            }
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
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => lobby.help_open = false,
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
                        cache: model.cache.clone(),
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
            lobby.set_selected_index(lobby.selected_index().saturating_sub(1));
            Vec::new()
        }
        KeyCode::Down => {
            let max_index = match lobby.active_tab {
                LobbyTab::MyGames => model.my_games.len().saturating_sub(1),
                LobbyTab::OpenGames => model.open_games.len().saturating_sub(1),
                _ => 0,
            };
            lobby.set_selected_index((lobby.selected_index() + 1).min(max_index));
            Vec::new()
        }
        KeyCode::Enter => activate_selected_row(model),
        KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::Char('H') => {
            lobby.help_open = true;
            Vec::new()
        }
        KeyCode::Char('r') | KeyCode::Char('R') if key.modifiers.contains(KeyModifiers::ALT) => {
            lobby.status = Some("Refreshing hosted lobby...".to_string());
            model.network = NetworkState::Connecting;
            vec![Effect::RefreshLobby]
        }
        KeyCode::Char('d') | KeyCode::Char('D') if key.modifiers.contains(KeyModifiers::ALT) => {
            open_sandbox_delete_confirm(model)
        }
        KeyCode::Char('m') | KeyCode::Char('M') => {
            lobby.active_tab = LobbyTab::MyGames;
            Vec::new()
        }
        KeyCode::Char('o') | KeyCode::Char('O') => {
            lobby.active_tab = LobbyTab::OpenGames;
            Vec::new()
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            lobby.active_tab = LobbyTab::Comms;
            Vec::new()
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            lobby.active_tab = LobbyTab::Settings;
            Vec::new()
        }
        KeyCode::Char('r') | KeyCode::Char('R') if lobby.active_tab == LobbyTab::Settings => {
            lobby.editing_relay = true;
            lobby.relay_draft = model.relay_url.clone();
            lobby.status = Some("Editing relay URL. Enter saves, Esc cancels.".to_string());
            Vec::new()
        }
        KeyCode::Char('i') | KeyCode::Char('I') if lobby.active_tab == LobbyTab::Settings => {
            let next = cycle_lock_timeout(model.lock_timeout_minutes);
            model.lock_timeout_minutes = next;
            lobby.status = Some(format!(
                "Idle lock timeout set to {}.",
                lock_timeout_label(next)
            ));
            vec![Effect::SaveLockTimeout {
                lock_timeout_minutes: next,
            }]
        }
        KeyCode::Char('l') | KeyCode::Char('L') => lock_session(model),
        KeyCode::Char('j') | KeyCode::Char('J') if lobby.active_tab == LobbyTab::OpenGames => {
            activate_selected_row(model)
        }
        _ => Vec::new(),
    }
}

fn handle_sandbox_join_confirm_key(model: &mut Model, key: crate::input::KeyEvent) -> Vec<Effect> {
    let Route::SandboxJoinConfirm(row) = &model.route else {
        return Vec::new();
    };
    if !matches!(key.code, KeyCode::Char('y') | KeyCode::Char('Y')) {
        return_to_lobby_with_status(model, String::new())
    } else {
        vec![Effect::JoinSandboxGame {
            row: row.clone(),
            password: current_password(model),
            handle: model
                .session
                .as_ref()
                .and_then(|session| session.active_handle.clone()),
        }]
    }
}

fn handle_sandbox_join_unavailable_key(model: &mut Model) -> Vec<Effect> {
    return_to_lobby_with_status(model, String::new())
}

fn handle_sandbox_delete_confirm_key(
    model: &mut Model,
    key: crate::input::KeyEvent,
) -> Vec<Effect> {
    let Route::SandboxDeleteConfirm(row) = &model.route else {
        return Vec::new();
    };
    if matches!(key.code, KeyCode::Char('y') | KeyCode::Char('Y')) {
        let row = row.clone();
        model.route = lobby_route(
            Some("Releasing sandbox seat...".to_string()),
            model.relay_url.clone(),
        );
        vec![Effect::ReleaseSandboxGame { row }]
    } else {
        return_to_lobby_with_status(model, String::new())
    }
}

fn handle_first_join_setup_key(model: &mut Model, key: crate::input::KeyEvent) -> Vec<Effect> {
    let Route::FirstJoinSetup(setup) = &mut model.route else {
        return Vec::new();
    };
    match key.code {
        KeyCode::Tab | KeyCode::BackTab => {
            setup.active_field = setup.active_field.next();
            Vec::new()
        }
        KeyCode::Backspace => {
            match setup.active_field {
                FirstJoinSetupField::Empire => {
                    setup.empire_input.pop();
                }
                FirstJoinSetupField::Homeworld => {
                    setup.homeworld_input.pop();
                }
            }
            Vec::new()
        }
        KeyCode::Esc => {
            return_to_lobby_with_status(model, "First-join setup cancelled.".to_string())
        }
        KeyCode::Enter => {
            let empire_name = setup.empire_input.trim().to_string();
            let homeworld_name = setup.homeworld_input.trim().to_string();
            if empire_name.is_empty() {
                setup.status =
                    Some("Empire names need at least one visible character.".to_string());
                setup.active_field = FirstJoinSetupField::Empire;
                return Vec::new();
            }
            if setup.active_field == FirstJoinSetupField::Empire && homeworld_name.is_empty() {
                setup.active_field = FirstJoinSetupField::Homeworld;
                return Vec::new();
            }
            if homeworld_name.is_empty() {
                setup.status =
                    Some("Homeworld names need at least one visible character.".to_string());
                setup.active_field = FirstJoinSetupField::Homeworld;
                return Vec::new();
            }
            vec![Effect::CompleteFirstJoinSetup {
                row: setup.row.clone(),
                empire_name,
                homeworld_name,
                password: current_password(model),
            }]
        }
        _ => {
            if let Some(ch) = is_printable_key(key) {
                let input = match setup.active_field {
                    FirstJoinSetupField::Empire => &mut setup.empire_input,
                    FirstJoinSetupField::Homeworld => &mut setup.homeworld_input,
                };
                input.push(ch);
                trim_first_join_setup_input(input);
                setup.status = None;
            }
            Vec::new()
        }
    }
}

fn handle_hosted_game_key(model: &mut Model, key: crate::input::KeyEvent) -> Vec<Effect> {
    let Route::HostedGame(hosted) = &mut model.route else {
        return Vec::new();
    };
    if let Some(mapped) = dashboard_key_event(key) {
        dashboard::dispatch_hosted_key(&mut hosted.dashboard, mapped);
        if dashboard::hosted_should_quit(&hosted.dashboard) {
            model.should_quit = true;
            return vec![Effect::Quit];
        }
    }
    Vec::new()
}

fn activate_selected_row(model: &mut Model) -> Vec<Effect> {
    let Some(lobby) = (match &model.route {
        Route::Lobby(lobby) => Some(lobby.clone()),
        _ => None,
    }) else {
        return Vec::new();
    };
    match lobby.active_tab {
        LobbyTab::OpenGames => activate_selected_open_game(model, &lobby),
        LobbyTab::MyGames => open_or_claim_selected_game(model, &lobby),
        _ => Vec::new(),
    }
}

fn activate_selected_open_game(model: &mut Model, lobby: &super::LobbyModel) -> Vec<Effect> {
    let Some(row) = model.open_games.get(lobby.selected_open_game).cloned() else {
        return return_to_lobby_with_status(model, "no hosted game selected".to_string());
    };
    if row.game_tier.eq_ignore_ascii_case("sandbox") {
        if let Some(joined_row) = model
            .my_games
            .iter()
            .find(|joined| joined.game_id == row.game_id && joined.status == "joined")
            .cloned()
        {
            return open_joined_game(model, joined_row);
        }
        model.route = Route::SandboxJoinConfirm(row);
        return Vec::new();
    }
    return_to_lobby_with_status(
        model,
        "League join requests are not wired in nc-helm yet.".to_string(),
    )
}

fn open_or_claim_selected_game(model: &mut Model, lobby: &super::LobbyModel) -> Vec<Effect> {
    let Some(row) = model.my_games.get(lobby.selected_my_game).cloned() else {
        return return_to_lobby_with_status(model, "no hosted game selected".to_string());
    };
    if row.status != "joined" {
        let message = match row.status.as_str() {
            "requested" => "Join request is still waiting for nc-host approval.",
            "rejected" => {
                "Join request was rejected. Select the game in Open Games to request again."
            }
            "expired" => "Your sandbox seat is no longer active. Rejoin from Open Games.",
            _ => "This game is not ready to open from the lobby.",
        };
        return return_to_lobby_with_status(model, message.to_string());
    }
    open_joined_game(model, row)
}

fn open_joined_game(model: &mut Model, row: super::MyGameRow) -> Vec<Effect> {
    vec![Effect::OpenHostedGame {
        row,
        password: current_password(model),
        handle: model
            .session
            .as_ref()
            .and_then(|session| session.active_handle.clone()),
    }]
}

fn open_sandbox_delete_confirm(model: &mut Model) -> Vec<Effect> {
    let Some(lobby) = (match &model.route {
        Route::Lobby(lobby) => Some(lobby.clone()),
        _ => None,
    }) else {
        return Vec::new();
    };
    if lobby.active_tab != LobbyTab::MyGames {
        return Vec::new();
    }
    let Some(row) = model.my_games.get(lobby.selected_my_game).cloned() else {
        return Vec::new();
    };
    if !row.game_tier.eq_ignore_ascii_case("sandbox") {
        return Vec::new();
    }
    model.route = Route::SandboxDeleteConfirm(row);
    Vec::new()
}

fn open_snapshot_route(
    model: &mut Model,
    row: super::MyGameRow,
    snapshot: nc_nostr::state_sync::GameState,
    lobby_status: Option<String>,
) -> Vec<Effect> {
    if let Some(setup) = first_join_setup_from_snapshot(row.clone(), &snapshot) {
        model.route = Route::FirstJoinSetup(setup);
        return Vec::new();
    }
    match dashboard::build_hosted_dash_app(
        &snapshot,
        dashboard::ScreenGeometry::new(model.geometry.width(), model.geometry.height()),
    ) {
        Ok(dashboard) => {
            model.route = Route::HostedGame(HostedGameModel {
                row,
                snapshot,
                dashboard,
                status: None,
            });
            Vec::new()
        }
        Err(err) => return_to_lobby_with_status(
            model,
            lobby_status
                .map(|status| format!("{status} Unable to build hosted dashboard: {err}"))
                .unwrap_or_else(|| format!("Unable to build hosted dashboard: {err}")),
        ),
    }
}

fn return_to_lobby_with_status(model: &mut Model, status: String) -> Vec<Effect> {
    let mut route = super::lobby_route(
        if status.is_empty() {
            None
        } else {
            Some(status)
        },
        model.relay_url.clone(),
    );
    if let Route::Lobby(lobby) = &mut route {
        lobby.selected_my_game = 0;
        lobby.selected_open_game = 0;
    }
    model.route = route;
    Vec::new()
}

fn upsert_joined_game(model: &mut Model, row: super::MyGameRow) {
    if let Some(index) = model
        .my_games
        .iter()
        .position(|existing| existing.game_id == row.game_id)
    {
        model.my_games[index] = row;
    } else {
        model.my_games.push(row);
    }
    model
        .my_games
        .sort_by(|left, right| left.game_id.cmp(&right.game_id));
}

fn apply_lobby_snapshot(model: &mut Model, snapshot: crate::transport::LobbySnapshot) {
    model.network = NetworkState::Synced;
    model.cache = snapshot.cache;
    model.my_games = snapshot.my_games;
    model.open_games = snapshot.open_games;
    model.notices = snapshot.notices;
    if let Route::Lobby(lobby) = &mut model.route {
        lobby.selected_my_game = lobby
            .selected_my_game
            .min(model.my_games.len().saturating_sub(1));
        lobby.selected_open_game = lobby
            .selected_open_game
            .min(model.open_games.len().saturating_sub(1));
    }
}

fn dashboard_key_event(key: crate::input::KeyEvent) -> Option<crate::dashboard::input::KeyEvent> {
    Some(crate::dashboard::input::KeyEvent::new(
        dashboard_key_code(key.code)?,
        dashboard_modifiers(key.modifiers),
    ))
}

fn dashboard_mouse_event(
    mouse: crate::input::MouseEvent,
) -> Option<crate::dashboard::input::MouseEvent> {
    Some(crate::dashboard::input::MouseEvent {
        kind: match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                crate::dashboard::input::MouseEventKind::Down(
                    crate::dashboard::input::MouseButton::Left,
                )
            }
            MouseEventKind::Down(MouseButton::Right) => {
                crate::dashboard::input::MouseEventKind::Down(
                    crate::dashboard::input::MouseButton::Right,
                )
            }
            MouseEventKind::Down(MouseButton::Middle) => {
                crate::dashboard::input::MouseEventKind::Down(
                    crate::dashboard::input::MouseButton::Middle,
                )
            }
            MouseEventKind::Up(MouseButton::Left) => crate::dashboard::input::MouseEventKind::Up(
                crate::dashboard::input::MouseButton::Left,
            ),
            MouseEventKind::Up(MouseButton::Right) => crate::dashboard::input::MouseEventKind::Up(
                crate::dashboard::input::MouseButton::Right,
            ),
            MouseEventKind::Up(MouseButton::Middle) => crate::dashboard::input::MouseEventKind::Up(
                crate::dashboard::input::MouseButton::Middle,
            ),
        },
        column: mouse.position.column.as_usize().min(u16::MAX as usize) as u16,
        row: mouse.position.row.as_usize().min(u16::MAX as usize) as u16,
        modifiers: dashboard_modifiers(mouse.modifiers),
    })
}

fn dashboard_key_code(code: KeyCode) -> Option<crate::dashboard::input::KeyCode> {
    Some(match code {
        KeyCode::Backspace => crate::dashboard::input::KeyCode::Backspace,
        KeyCode::Enter => crate::dashboard::input::KeyCode::Enter,
        KeyCode::Left => crate::dashboard::input::KeyCode::Left,
        KeyCode::Right => crate::dashboard::input::KeyCode::Right,
        KeyCode::Up => crate::dashboard::input::KeyCode::Up,
        KeyCode::Down => crate::dashboard::input::KeyCode::Down,
        KeyCode::Home => crate::dashboard::input::KeyCode::Home,
        KeyCode::End => crate::dashboard::input::KeyCode::End,
        KeyCode::PageUp => crate::dashboard::input::KeyCode::PageUp,
        KeyCode::PageDown => crate::dashboard::input::KeyCode::PageDown,
        KeyCode::Tab => crate::dashboard::input::KeyCode::Tab,
        KeyCode::BackTab => crate::dashboard::input::KeyCode::BackTab,
        KeyCode::Delete => crate::dashboard::input::KeyCode::Delete,
        KeyCode::Esc => crate::dashboard::input::KeyCode::Esc,
        KeyCode::F(n) => crate::dashboard::input::KeyCode::F(n),
        KeyCode::Char(ch) => crate::dashboard::input::KeyCode::Char(ch),
    })
}

fn dashboard_modifiers(modifiers: KeyModifiers) -> crate::dashboard::input::KeyModifiers {
    let mut mapped = crate::dashboard::input::KeyModifiers::empty();
    if modifiers.contains(KeyModifiers::SHIFT) {
        mapped.insert(crate::dashboard::input::KeyModifiers::SHIFT);
    }
    if modifiers.contains(KeyModifiers::CONTROL) {
        mapped.insert(crate::dashboard::input::KeyModifiers::CONTROL);
    }
    if modifiers.contains(KeyModifiers::ALT) {
        mapped.insert(crate::dashboard::input::KeyModifiers::ALT);
    }
    mapped
}

fn handle_idle_lock(model: &mut Model) -> Vec<Effect> {
    if !matches!(model.route, Route::Lobby(_)) || model.lock_timeout_minutes == 0 {
        return Vec::new();
    }
    lock_session(model)
}

fn lock_session(model: &mut Model) -> Vec<Effect> {
    model.session = None;
    model.network = NetworkState::Idle;
    model.my_games.clear();
    model.open_games.clear();
    model.notices.clear();
    model.matrix_rain.reset();
    model.route = Route::MatrixLocked;
    vec![Effect::DisconnectTransport]
}

fn cycle_lock_timeout(current: u16) -> u16 {
    let index = LOCK_TIMEOUT_OPTIONS
        .iter()
        .position(|value| *value == current)
        .unwrap_or(0);
    LOCK_TIMEOUT_OPTIONS[(index + 1) % LOCK_TIMEOUT_OPTIONS.len()]
}

fn lock_timeout_label(value: u16) -> String {
    if value == 0 {
        "Off".to_string()
    } else {
        format!("{value} min")
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

fn is_quit_shortcut(key: crate::input::KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q'))
        && key.modifiers.contains(KeyModifiers::ALT)
}

#[allow(dead_code)]
fn _assert_screen_geometry_send(_: ScreenGeometry) {}
