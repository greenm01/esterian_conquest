use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::state::{FirstRunField, HostedGameView, LobbyApp, LobbyFocus, LobbyRoute};

pub fn apply_key(app: &mut LobbyApp, key: KeyEvent) {
    match app.state.route {
        LobbyRoute::FirstRun => handle_first_run_key(app, key),
        LobbyRoute::Locked => handle_locked_key(app, key),
        LobbyRoute::ComposeInvite => handle_compose_key(app, key),
        LobbyRoute::EditHandle => handle_edit_handle_key(app, key),
        LobbyRoute::HostedGame => handle_hosted_game_key(app, key),
        LobbyRoute::SubmitTurn => handle_submit_turn_key(app, key),
        LobbyRoute::Home => handle_home_key(app, key),
    }
}

fn handle_home_key(app: &mut LobbyApp, key: KeyEvent) {
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
        KeyCode::Char('h' | 'H') => {
            app.state.edit_handle_input = app.state.player_handle.clone().unwrap_or_default();
            app.state.route = LobbyRoute::EditHandle;
        }
        KeyCode::Char('r' | 'R') => refresh_lobby(app),
        _ => {}
    }
}

fn handle_first_run_key(app: &mut LobbyApp, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q' | 'Q')
            if matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT) =>
        {
            app.should_quit = true;
        }
        KeyCode::Tab => {
            app.state.first_run_field = app.state.first_run_field.next();
        }
        KeyCode::Backspace => {
            match app.state.first_run_field {
                FirstRunField::Handle => {
                    app.state.first_run_handle_input.pop();
                }
                FirstRunField::Password => {
                    app.state.first_run_password_input.pop();
                }
                FirstRunField::Confirm => {
                    app.state.first_run_confirm_input.pop();
                }
            }
        }
        KeyCode::Enter => {
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
                Err(err) => app.state.status_message = Some(err),
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
            Err(err) => app.state.status_message = Some(err),
        },
        KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.state.unlock_password_input.push(ch);
        }
        _ => {}
    }
}

fn handle_compose_key(app: &mut LobbyApp, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.state.route = LobbyRoute::Home,
        KeyCode::Backspace => {
            app.state.compose_message_input.pop();
        }
        KeyCode::Enter => {
            let Some(row) = app.state.selected_open_game().cloned() else {
                app.state.status_message = Some("no recruiting game selected".to_string());
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
                Err(err) => app.state.status_message = Some(err),
            }
        }
        KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.state.compose_message_input.push(ch);
        }
        _ => {}
    }
}

fn handle_edit_handle_key(app: &mut LobbyApp, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.state.route = LobbyRoute::Home,
        KeyCode::Backspace => {
            app.state.edit_handle_input.pop();
        }
        KeyCode::Enter => match app.transport.save_handle(&app.state.edit_handle_input) {
            Ok(loaded) => {
                app.state.apply_loaded(loaded);
                app.state.route = LobbyRoute::Home;
            }
            Err(err) => app.state.status_message = Some(err),
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
        KeyCode::Char('t' | 'T') => app.state.route = LobbyRoute::SubmitTurn,
        _ => {}
    }
}

fn handle_submit_turn_key(app: &mut LobbyApp, key: KeyEvent) {
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
            match app.transport.submit_turn(&row, turn, &commands) {
                Ok(loaded) => {
                    if let Some(hosted) = app.state.hosted_game.as_mut() {
                        hosted.submit_status = Some("Turn submitted; check inbox for receipt.".to_string());
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
        Err(err) => app.state.status_message = Some(err),
    }
}

fn refresh_hosted_game(app: &mut LobbyApp) {
    let Some(row) = app.state.hosted_game.as_ref().map(|hosted| hosted.row.clone()) else {
        app.state.route = LobbyRoute::Home;
        return;
    };
    match app.transport.open_game(&row) {
        Ok(snapshot) => {
            if let Some(hosted) = app.state.hosted_game.as_mut() {
                hosted.snapshot = snapshot;
                hosted.submit_status = Some("Hosted snapshot refreshed.".to_string());
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
        app.state.status_message = Some("no hosted game selected".to_string());
        return;
    };
    if row.status == "approved" {
        match app.transport.claim_invite(&row) {
            Ok(loaded) => app.state.apply_loaded(loaded),
            Err(err) => app.state.status_message = Some(err),
        }
        return;
    }
    match app.transport.open_game(&row) {
        Ok(snapshot) => {
            app.state.hosted_game = Some(HostedGameView {
                row,
                snapshot,
                submit_input: String::new(),
                submit_status: Some("Hosted snapshot loaded from nc-host.".to_string()),
            });
            app.state.route = LobbyRoute::HostedGame;
        }
        Err(err) => app.state.status_message = Some(err),
    }
}

fn move_selection(app: &mut LobbyApp, delta: isize) {
    let selection = match app.state.focus {
        LobbyFocus::JoinedGames => &mut app.state.joined_selected,
        LobbyFocus::Inbox => &mut app.state.inbox_selected,
        LobbyFocus::OpenGames => &mut app.state.open_selected,
        LobbyFocus::Notices => &mut app.state.notices_selected,
        LobbyFocus::Thread => &mut app.state.thread_selected,
    };
    let len = match app.state.focus {
        LobbyFocus::JoinedGames => app.state.joined_games.len(),
        LobbyFocus::Inbox => app.state.inbox.len(),
        LobbyFocus::OpenGames => app.state.open_games.len(),
        LobbyFocus::Notices => app.state.notices.len(),
        LobbyFocus::Thread => app.state.thread_messages.len(),
    };
    if len == 0 {
        *selection = 0;
        return;
    }
    let next = (*selection as isize + delta).clamp(0, len.saturating_sub(1) as isize);
    *selection = next as usize;
}
