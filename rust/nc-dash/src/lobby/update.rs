use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::hosted::dashboard::build_hosted_dash_app;
use super::state::{
    FirstRunField, HostedGameView, LobbyApp, LobbyFocus, LobbyNetworkStatus, LobbyRoute,
    LobbyStatusTone,
};
use crate::theme;

pub fn apply_key(app: &mut LobbyApp, key: KeyEvent) {
    match app.state.route {
        LobbyRoute::FirstRun => handle_first_run_key(app, key),
        LobbyRoute::Locked => handle_locked_key(app, key),
        LobbyRoute::ComposeInvite => handle_compose_key(app, key),
        LobbyRoute::ComposeThread => handle_compose_thread_key(app, key),
        LobbyRoute::EditHandle => handle_edit_handle_key(app, key),
        LobbyRoute::Settings => handle_settings_key(app, key),
        LobbyRoute::ThemePicker => handle_theme_picker_key(app, key),
        LobbyRoute::HostedGame => handle_hosted_game_key(app, key),
        LobbyRoute::SubmitTurn => handle_submit_turn_key(app, key),
        LobbyRoute::Home => handle_home_key(app, key),
    }
}

fn handle_home_key(app: &mut LobbyApp, key: KeyEvent) {
    if app.state.show_help {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('?') => app.state.show_help = false,
            _ => {}
        }
        return;
    }
    if !matches!(key.code, KeyCode::Char('?')) {
        clear_status(app);
    }

    match key.code {
        KeyCode::Esc | KeyCode::Char('q' | 'Q')
            if matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT) =>
        {
            app.should_quit = true;
        }
        KeyCode::Enter => {
            if app.state.focus == LobbyFocus::OpenGames {
                app.state.compose_message_input.clear();
                app.state.route = LobbyRoute::ComposeInvite;
            } else if app.state.focus == LobbyFocus::JoinedGames {
                open_or_claim_selected_game(app);
            }
        }
        KeyCode::Tab => app.state.focus = app.state.focus.next(),
        KeyCode::BackTab => app.state.focus = app.state.focus.prev(),
        KeyCode::Up | KeyCode::Char('k') => move_selection(app, -1),
        KeyCode::Down | KeyCode::Char('j') => move_selection(app, 1),
        KeyCode::Char('n' | 'N') => {
            app.state.compose_message_input.clear();
            app.state.route = LobbyRoute::ComposeInvite;
        }
        KeyCode::Char('m' | 'M') => {
            app.state.compose_message_input.clear();
            app.state.route = LobbyRoute::ComposeThread;
        }
        KeyCode::Char('s' | 'S') => open_settings(app),
        KeyCode::Char('r' | 'R') => refresh_lobby(app),
        KeyCode::Char('?') => app.state.show_help = true,
        _ => {}
    }
}

fn handle_first_run_key(app: &mut LobbyApp, key: KeyEvent) {
    if handle_single_line_paste(app, key, |state| match state.first_run_field {
        FirstRunField::Handle => &mut state.first_run_handle_input,
        FirstRunField::Password => &mut state.first_run_password_input,
        FirstRunField::Confirm => &mut state.first_run_confirm_input,
    }) {
        return;
    }

    match key.code {
        KeyCode::Esc | KeyCode::Char('q' | 'Q')
            if matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT) =>
        {
            app.should_quit = true;
        }
        KeyCode::Up => app.state.first_run_field = app.state.first_run_field.prev(),
        KeyCode::Down => app.state.first_run_field = app.state.first_run_field.next(),
        KeyCode::Backspace => match app.state.first_run_field {
            FirstRunField::Handle => {
                app.state.first_run_handle_input.pop();
            }
            FirstRunField::Password => {
                app.state.first_run_password_input.pop();
            }
            FirstRunField::Confirm => {
                app.state.first_run_confirm_input.pop();
            }
        },
        KeyCode::Enter => {
            if app.state.first_run_field != FirstRunField::Confirm {
                app.state.first_run_field = app.state.first_run_field.next();
            } else {
                match app.transport.create_identity(
                    &app.state.first_run_handle_input,
                    &app.state.first_run_password_input,
                    &app.state.first_run_confirm_input,
                ) {
                    Ok(loaded) => {
                        app.state.apply_loaded(loaded);
                        app.state.unlock_password_input.clear();
                        app.state.first_run_password_input.clear();
                        app.state.first_run_confirm_input.clear();
                        app.state.route = LobbyRoute::Home;
                    }
                    Err(err) => set_status(app, LobbyStatusTone::Error, err),
                }
            }
        }
        KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            match app.state.first_run_field {
                FirstRunField::Handle => app.state.first_run_handle_input.push(ch),
                FirstRunField::Password => app.state.first_run_password_input.push(ch),
                FirstRunField::Confirm => app.state.first_run_confirm_input.push(ch),
            }
        }
        _ => {}
    }
}

fn handle_locked_key(app: &mut LobbyApp, key: KeyEvent) {
    if handle_single_line_paste(app, key, |state| &mut state.unlock_password_input) {
        return;
    }

    match key.code {
        KeyCode::Esc | KeyCode::Char('q' | 'Q')
            if matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT) =>
        {
            app.should_quit = true;
        }
        KeyCode::Backspace => {
            app.state.unlock_password_input.pop();
        }
        KeyCode::Enter => match app.transport.unlock(&app.state.unlock_password_input) {
            Ok(loaded) => {
                app.state.apply_loaded(loaded);
                app.state.unlock_password_input.clear();
                app.state.route = LobbyRoute::Home;
            }
            Err(err) => set_status(app, LobbyStatusTone::Error, err),
        },
        KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.state.unlock_password_input.push(ch);
        }
        _ => {}
    }
}

fn handle_compose_key(app: &mut LobbyApp, key: KeyEvent) {
    if handle_single_line_paste(app, key, |state| &mut state.compose_message_input) {
        return;
    }

    match key.code {
        KeyCode::Esc => app.state.route = LobbyRoute::Home,
        KeyCode::Backspace => {
            app.state.compose_message_input.pop();
        }
        KeyCode::Enter => {
            let Some(row) = app.state.selected_open_game().cloned() else {
                set_status(
                    app,
                    LobbyStatusTone::Error,
                    "no recruiting game selected".to_string(),
                );
                app.state.route = LobbyRoute::Home;
                return;
            };
            match app
                .transport
                .send_invite_request(&row, &app.state.compose_message_input)
            {
                Ok(loaded) => {
                    app.state.apply_loaded(loaded);
                    app.state.compose_message_input.clear();
                    app.state.route = LobbyRoute::Home;
                }
                Err(err) => set_status(app, LobbyStatusTone::Error, err),
            }
        }
        KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.state.compose_message_input.push(ch);
        }
        _ => {}
    }
}

fn handle_compose_thread_key(app: &mut LobbyApp, key: KeyEvent) {
    if handle_single_line_paste(app, key, |state| &mut state.compose_message_input) {
        return;
    }

    match key.code {
        KeyCode::Esc => app.state.route = LobbyRoute::Home,
        KeyCode::Backspace => {
            app.state.compose_message_input.pop();
        }
        KeyCode::Enter => {
            let Some((game_id, daemon_pubkey)) = selected_thread_target(app) else {
                set_status(
                    app,
                    LobbyStatusTone::Error,
                    "select an open or joined game first".to_string(),
                );
                app.state.route = LobbyRoute::Home;
                return;
            };
            match app.transport.send_thread_message(
                &game_id,
                &daemon_pubkey,
                &app.state.compose_message_input,
            ) {
                Ok(loaded) => {
                    app.state.apply_loaded(loaded);
                    app.state.compose_message_input.clear();
                    app.state.route = LobbyRoute::Home;
                }
                Err(err) => set_status(app, LobbyStatusTone::Error, err),
            }
        }
        KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.state.compose_message_input.push(ch);
        }
        _ => {}
    }
}

fn handle_edit_handle_key(app: &mut LobbyApp, key: KeyEvent) {
    if handle_single_line_paste(app, key, |state| &mut state.edit_handle_input) {
        return;
    }

    match key.code {
        KeyCode::Esc => app.state.route = app.state.edit_handle_return_route,
        KeyCode::Backspace => {
            app.state.edit_handle_input.pop();
        }
        KeyCode::Enter => match app.transport.save_handle(&app.state.edit_handle_input) {
            Ok(loaded) => {
                app.state.apply_loaded(loaded);
                app.state.route = app.state.edit_handle_return_route;
            }
            Err(err) => set_status(app, LobbyStatusTone::Error, err),
        },
        KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.state.edit_handle_input.push(ch);
        }
        _ => {}
    }
}

fn handle_hosted_game_key(app: &mut LobbyApp, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.state.route = LobbyRoute::Home,
        KeyCode::Char('r' | 'R') => refresh_hosted_game(app),
        KeyCode::Char('t' | 'T') => open_submit_turn(app),
        _ => {
            if let Some(hosted) = app.state.hosted_game.as_mut() {
                hosted.dashboard.dispatch_key_event(key);
                if hosted.dashboard.should_quit {
                    app.should_quit = true;
                }
            }
        }
    }
}

fn handle_settings_key(app: &mut LobbyApp, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => cancel_settings(app),
        KeyCode::Up | KeyCode::Char('k') => {
            app.state.settings_selected = app.state.settings_selected.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.state.settings_selected = (app.state.settings_selected + 1).min(5);
        }
        KeyCode::Char('s' | 'S') => save_settings(app),
        KeyCode::Char(' ') | KeyCode::Enter => match app.state.settings_selected {
            0 => {
                app.state.edit_handle_input = app.state.player_handle.clone().unwrap_or_default();
                app.state.edit_handle_return_route = LobbyRoute::Settings;
                app.state.route = LobbyRoute::EditHandle;
            }
            1 => {
                app.state.settings_draft.follow_mouse_on_map =
                    !app.state.settings_draft.follow_mouse_on_map;
            }
            2 => {
                app.state.settings_draft.dense_empty_sector_dots =
                    !app.state.settings_draft.dense_empty_sector_dots;
            }
            3 => open_theme_picker(app),
            4 => save_settings(app),
            5 => cancel_settings(app),
            _ => {}
        },
        _ => {}
    }
}

fn handle_theme_picker_key(app: &mut LobbyApp, key: KeyEvent) {
    let themes = app.state.available_themes();
    if themes.is_empty() {
        app.state.route = LobbyRoute::Settings;
        return;
    }

    match key.code {
        KeyCode::Esc => {
            let _ = theme::apply_theme_key(&app.state.theme_original_key);
            app.state.settings_draft.theme_key = app.state.theme_original_key.clone();
            app.state.route = LobbyRoute::Settings;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.state.theme_selected > 0 {
                app.state.theme_selected -= 1;
                preview_selected_theme(app, &themes);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.state.theme_selected + 1 < themes.len() {
                app.state.theme_selected += 1;
                preview_selected_theme(app, &themes);
            }
        }
        KeyCode::Enter => {
            if let Some(entry) = themes.get(app.state.theme_selected) {
                app.state.settings_draft.theme_key = entry.key.clone();
                let _ = theme::apply_theme_key(&entry.key);
            }
            app.state.route = LobbyRoute::Settings;
        }
        _ => {}
    }
}

fn open_submit_turn(app: &mut LobbyApp) {
    let Some(hosted) = app.state.hosted_game.as_mut() else {
        app.state.route = LobbyRoute::Home;
        return;
    };
    hosted.submit_input = hosted.dashboard.hosted_turn_text().unwrap_or_default();
    hosted.submit_status = if hosted.submit_input.is_empty() {
        Some("No staged hosted orders yet.".to_string())
    } else {
        Some("Review the staged turn.kdl, then press Enter to send 30522.".to_string())
    };
    app.state.route = LobbyRoute::SubmitTurn;
}

fn handle_submit_turn_key(app: &mut LobbyApp, key: KeyEvent) {
    if handle_submit_turn_paste(app, key) {
        return;
    }

    match key.code {
        KeyCode::Esc => app.state.route = LobbyRoute::HostedGame,
        KeyCode::Backspace => {
            if let Some(hosted) = app.state.hosted_game.as_mut() {
                hosted.submit_input.pop();
            }
        }
        KeyCode::Enter => {
            let Some(hosted) = app.state.hosted_game.as_ref() else {
                app.state.route = LobbyRoute::Home;
                return;
            };
            let row = hosted.row.clone();
            let turn = hosted.snapshot.turn;
            let commands = hosted.submit_input.clone();
            if commands.trim().is_empty() {
                if let Some(hosted) = app.state.hosted_game.as_mut() {
                    hosted.submit_status = Some("No staged hosted orders to submit.".to_string());
                }
                app.state.route = LobbyRoute::HostedGame;
                return;
            }
            match app.transport.submit_turn(&row, turn, &commands) {
                Ok(loaded) => {
                    if let Some(hosted) = app.state.hosted_game.as_mut() {
                        hosted.submit_status =
                            Some("Turn submitted; check inbox for receipt.".to_string());
                        hosted.submit_input.clear();
                    }
                    app.state.apply_loaded(loaded);
                    app.state.route = LobbyRoute::HostedGame;
                }
                Err(err) => {
                    if let Some(hosted) = app.state.hosted_game.as_mut() {
                        hosted.submit_status = Some(err);
                    }
                    app.state.route = LobbyRoute::HostedGame;
                }
            }
        }
        KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let Some(hosted) = app.state.hosted_game.as_mut() {
                hosted.submit_input.push(ch);
            }
        }
        _ => {}
    }
}

fn refresh_lobby(app: &mut LobbyApp) {
    match app.transport.refresh() {
        Ok(loaded) => app.state.apply_loaded(loaded),
        Err(err) => set_network_error(app, err),
    }
}

fn open_settings(app: &mut LobbyApp) {
    app.state.settings_draft = app.state.settings.clone();
    app.state.settings_selected = 0;
    clear_status(app);
    app.state.route = LobbyRoute::Settings;
}

fn cancel_settings(app: &mut LobbyApp) {
    app.state.settings_draft = app.state.settings.clone();
    let _ = theme::apply_theme_key(&app.state.settings.theme_key);
    app.state.route = LobbyRoute::Home;
}

fn save_settings(app: &mut LobbyApp) {
    match super::storage::settings::save_settings_to(&app.state.settings_draft, &app.settings_path) {
        Ok(()) => {
            app.state.settings = app.state.settings_draft.clone();
            let _ = theme::apply_theme_key(&app.state.settings.theme_key);
            set_status(
                app,
                LobbyStatusTone::Success,
                "Saved local settings".to_string(),
            );
            app.state.route = LobbyRoute::Home;
        }
        Err(err) => {
            set_status(app, LobbyStatusTone::Error, format!("Save failed: {err}"));
        }
    }
}

fn open_theme_picker(app: &mut LobbyApp) {
    let themes = app.state.available_themes();
    if themes.is_empty() {
        return;
    }
    app.state.theme_original_key = app.state.settings_draft.theme_key.clone();
    app.state.theme_selected = themes
        .iter()
        .position(|entry| entry.key == app.state.settings_draft.theme_key)
        .unwrap_or(0);
    preview_selected_theme(app, &themes);
    app.state.route = LobbyRoute::ThemePicker;
}

fn preview_selected_theme(app: &mut LobbyApp, themes: &[crate::theme::ThemeCatalogEntry]) {
    if let Some(entry) = themes.get(app.state.theme_selected) {
        app.state.settings_draft.theme_key = entry.key.clone();
        let _ = theme::apply_theme_key(&entry.key);
    }
}

fn refresh_hosted_game(app: &mut LobbyApp) {
    let Some(row) = app
        .state
        .hosted_game
        .as_ref()
        .map(|hosted| hosted.row.clone())
    else {
        app.state.route = LobbyRoute::Home;
        return;
    };
    match app.transport.open_game(&row) {
        Ok(snapshot) => {
            let submit_status = Some("Hosted dashboard refreshed from nc-host.".to_string());
            match build_hosted_dash_app(&snapshot, app.geometry) {
                Ok(dashboard) => {
                    if let Some(hosted) = app.state.hosted_game.as_mut() {
                        hosted.snapshot = snapshot;
                        hosted.dashboard = dashboard;
                        hosted.submit_status = submit_status;
                    }
                }
                Err(err) => {
                    if let Some(hosted) = app.state.hosted_game.as_mut() {
                        hosted.submit_status =
                            Some(format!("Hosted dashboard refresh failed: {err}"));
                    }
                }
            }
        }
        Err(err) => {
            if let Some(hosted) = app.state.hosted_game.as_mut() {
                hosted.submit_status = Some(err);
            }
        }
    }
}

fn open_or_claim_selected_game(app: &mut LobbyApp) {
    let Some(row) = app.state.selected_joined_game().cloned() else {
        set_status(
            app,
            LobbyStatusTone::Error,
            "no hosted game selected".to_string(),
        );
        return;
    };
    if row.status == "approved" {
        match app.transport.claim_invite(&row) {
            Ok(loaded) => app.state.apply_loaded(loaded),
            Err(err) => set_status(app, LobbyStatusTone::Error, err),
        }
        return;
    }
    match app.transport.open_game(&row) {
        Ok(snapshot) => match build_hosted_dash_app(&snapshot, app.geometry) {
            Ok(dashboard) => {
                app.state.hosted_game = Some(HostedGameView {
                    row,
                    snapshot,
                    dashboard,
                    submit_input: String::new(),
                    submit_status: Some("Hosted dashboard loaded from nc-host.".to_string()),
                });
                app.state.route = LobbyRoute::HostedGame;
            }
            Err(err) => {
                set_status(
                    app,
                    LobbyStatusTone::Error,
                    format!("Unable to build hosted dashboard: {err}"),
                );
            }
        },
        Err(err) => set_status(app, LobbyStatusTone::Error, err),
    }
}

fn move_selection(app: &mut LobbyApp, delta: isize) {
    let len = match app.state.focus {
        LobbyFocus::JoinedGames => app.state.joined_games.len(),
        LobbyFocus::Inbox => app.state.inbox.len(),
        LobbyFocus::OpenGames => app.state.open_games.len(),
        LobbyFocus::Notices => app.state.notices.len(),
        LobbyFocus::Thread => app.state.visible_thread_messages().len(),
    };
    let selection = match app.state.focus {
        LobbyFocus::JoinedGames => &mut app.state.joined_selected,
        LobbyFocus::Inbox => &mut app.state.inbox_selected,
        LobbyFocus::OpenGames => &mut app.state.open_selected,
        LobbyFocus::Notices => &mut app.state.notices_selected,
        LobbyFocus::Thread => &mut app.state.thread_selected,
    };
    if len == 0 {
        *selection = 0;
        return;
    }
    let next = (*selection as isize + delta).clamp(0, len.saturating_sub(1) as isize);
    *selection = next as usize;
}

fn selected_thread_target(app: &LobbyApp) -> Option<(String, String)> {
    let game_id = app.state.thread_context_game_id()?;
    app.state
        .joined_games
        .iter()
        .find(|row| row.game_id == game_id)
        .map(|row| (row.game_id.clone(), row.daemon_pubkey.clone()))
        .or_else(|| {
            app.state
                .open_games
                .iter()
                .find(|row| row.game_id == game_id)
                .map(|row| (row.game_id.clone(), row.daemon_pubkey.clone()))
        })
}

fn handle_single_line_paste(
    app: &mut LobbyApp,
    key: KeyEvent,
    select: impl FnOnce(&mut super::state::LobbyState) -> &mut String,
) -> bool {
    let Some(text) = read_clipboard_text(app, key) else {
        return false;
    };
    let field = select(&mut app.state);
    field.extend(sanitize_single_line_paste(&text));
    true
}

fn handle_submit_turn_paste(app: &mut LobbyApp, key: KeyEvent) -> bool {
    let Some(text) = read_clipboard_text(app, key) else {
        return false;
    };
    let Some(hosted) = app.state.hosted_game.as_mut() else {
        return false;
    };
    hosted
        .submit_input
        .push_str(&sanitize_multiline_paste(&text));
    true
}

fn read_clipboard_text(app: &mut LobbyApp, key: KeyEvent) -> Option<String> {
    if !is_paste_shortcut(key) {
        return None;
    }
    match app.clipboard.get_text() {
        Ok(Some(text)) => Some(text),
        Ok(None) => {
            set_status(
                app,
                LobbyStatusTone::Error,
                "Clipboard is unavailable.".to_string(),
            );
            None
        }
        Err(err) => {
            set_status(
                app,
                LobbyStatusTone::Error,
                format!("Clipboard paste failed: {err}"),
            );
            None
        }
    }
}

pub(crate) fn set_network_error(app: &mut LobbyApp, err: String) {
    app.state.network_status = LobbyNetworkStatus::Error;
    set_status(app, LobbyStatusTone::Error, err);
}

pub(crate) fn set_status(app: &mut LobbyApp, tone: LobbyStatusTone, message: String) {
    app.state.status_tone = tone;
    app.state.status_message = Some(message);
}

pub(crate) fn clear_status(app: &mut LobbyApp) {
    app.state.status_message = None;
    app.state.status_tone = LobbyStatusTone::Info;
}

fn is_paste_shortcut(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Insert) && key.modifiers.contains(KeyModifiers::SHIFT)
        || matches!(key.code, KeyCode::Char('v' | 'V'))
            && key.modifiers.contains(KeyModifiers::CONTROL)
            && !key.modifiers.contains(KeyModifiers::ALT)
}

fn sanitize_single_line_paste(text: &str) -> impl Iterator<Item = char> + '_ {
    text.chars()
        .filter(|ch| !matches!(ch, '\r' | '\n' | '\u{7f}'))
}

fn sanitize_multiline_paste(text: &str) -> String {
    text.chars()
        .filter(|ch| !matches!(ch, '\u{7f}'))
        .collect::<String>()
}
