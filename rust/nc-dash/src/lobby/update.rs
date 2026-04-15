use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::hosted::dashboard::build_hosted_dash_app;
use super::state::{
    FirstRunField, HostedGameView, KeychainGateMode, LobbyApp, LobbyNetworkStatus, LobbyRoute,
    LobbyStatusTone, LobbyTab, ThreadPaneFocus,
};
use super::storage::settings::{LOCK_TIMEOUT_OPTIONS, lock_timeout_label};
use crate::theme;

pub fn apply_key(app: &mut LobbyApp, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::ALT) {
        match key.code {
            KeyCode::Char('l' | 'L') => {
                app.enter_session_lock();
                return;
            }
            KeyCode::Char('a' | 'A')
                if matches!(
                    app.state.route,
                    LobbyRoute::Home | LobbyRoute::ContactPicker
                ) =>
            {
                open_popup_route(app, LobbyRoute::ContactPicker);
                return;
            }
            _ => {}
        }
    }
    match app.state.route {
        LobbyRoute::FirstRun => handle_first_run_key(app, key),
        LobbyRoute::MatrixLocked => handle_matrix_locked_key(app, key),
        LobbyRoute::Locked => handle_locked_key(app, key),
        LobbyRoute::ComposeInvite => handle_compose_key(app, key),
        LobbyRoute::EditHandle => handle_edit_handle_key(app, key),
        LobbyRoute::Settings => handle_settings_key(app, key),
        LobbyRoute::ThemePicker => handle_theme_picker_key(app, key),
        LobbyRoute::HostedGame => handle_hosted_game_key(app, key),
        LobbyRoute::SubmitTurn => handle_submit_turn_key(app, key),
        LobbyRoute::Home => handle_home_key(app, key),
        LobbyRoute::GameInboxThread | LobbyRoute::ComposeThread => handle_comms_key(app, key),
        LobbyRoute::ContactPicker => handle_contact_picker_key(app, key),
        LobbyRoute::AddContact => handle_add_contact_key(app, key),
    }
}

fn handle_matrix_locked_key(app: &mut LobbyApp, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q' | 'Q')
            if app.state.gate_mode == KeychainGateMode::Startup
                && matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT) =>
        {
            app.should_quit = true;
        }
        _ => app.begin_unlock_prompt(),
    }
}

fn handle_home_key(app: &mut LobbyApp, key: KeyEvent) {
    if app.state.show_help {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('?') => {
                app.state.show_help = false;
                app.popup_position = None;
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Tab => {
            app.state.active_tab = app.state.active_tab.next();
            app.state.sync_default_contact_selection();
            return;
        }
        KeyCode::BackTab => {
            app.state.active_tab = app.state.active_tab.prev();
            app.state.sync_default_contact_selection();
            return;
        }
        _ => {}
    }

    if app.state.active_tab == LobbyTab::Comms {
        handle_comms_key(app, key);
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
        KeyCode::Enter => match app.state.active_tab {
            LobbyTab::OpenGames => {
                app.state.compose_message_input.clear();
                open_popup_route(app, LobbyRoute::ComposeInvite);
            }
            LobbyTab::MyGames => open_or_claim_selected_game(app),
            _ => {}
        },
        KeyCode::Up | KeyCode::Char('k') => move_selection(app, -1),
        KeyCode::Down | KeyCode::Char('j') => move_selection(app, 1),
        KeyCode::Char('J' | 'n' | 'N') => {
            app.state.compose_message_input.clear();
            open_popup_route(app, LobbyRoute::ComposeInvite);
        }
        KeyCode::Char('s' | 'S') => open_settings(app),
        KeyCode::Char('r' | 'R') => refresh_lobby(app),
        KeyCode::Char('?') => {
            app.state.show_help = true;
            app.popup_position = None;
        }
        _ => {}
    }
}

fn handle_comms_key(app: &mut LobbyApp, key: KeyEvent) {
    if app.state.show_help {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('?') => {
                app.state.show_help = false;
                app.popup_position = None;
            }
            _ => {}
        }
        return;
    }
    if !matches!(key.code, KeyCode::Char('?')) {
        clear_status(app);
    }

    match key.code {
        KeyCode::Esc => {
            if app.state.route == LobbyRoute::Home {
                app.should_quit = true;
            } else {
                app.state.route = LobbyRoute::Home;
                app.state.thread_pane_focus = ThreadPaneFocus::Chat;
                app.popup_position = None;
            }
        }
        KeyCode::Char('?') => {
            app.state.show_help = true;
            app.popup_position = None;
        }
        KeyCode::Tab => cycle_comms_focus(app, 1),
        KeyCode::BackTab => cycle_comms_focus(app, -1),
        KeyCode::Left => cycle_comms_focus(app, -1),
        KeyCode::Right => cycle_comms_focus(app, 1),
        KeyCode::Up => move_comms_selection(app, -1),
        KeyCode::Down => move_comms_selection(app, 1),
        KeyCode::Enter => handle_comms_enter(app),
        KeyCode::Delete if app.state.thread_pane_focus != ThreadPaneFocus::Chat => {
            hide_active_conversation(app);
        }
        KeyCode::Backspace
            if app.state.thread_pane_focus == ThreadPaneFocus::Chat
                && active_comms_writable(app) =>
        {
            app.state.compose_message_input.pop();
        }
        KeyCode::Char('h') if app.state.thread_pane_focus != ThreadPaneFocus::Chat => {
            cycle_comms_focus(app, -1);
        }
        KeyCode::Char('l') if app.state.thread_pane_focus != ThreadPaneFocus::Chat => {
            cycle_comms_focus(app, 1);
        }
        KeyCode::Char('k') if app.state.thread_pane_focus != ThreadPaneFocus::Chat => {
            move_comms_selection(app, -1);
        }
        KeyCode::Char('j') if app.state.thread_pane_focus != ThreadPaneFocus::Chat => {
            move_comms_selection(app, 1);
        }
        KeyCode::Char(ch)
            if app.state.thread_pane_focus == ThreadPaneFocus::Chat
                && !key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::ALT) =>
        {
            if active_comms_writable(app) {
                app.state.compose_message_input.push(ch);
            }
        }
        KeyCode::Insert | KeyCode::Char('v' | 'V')
            if app.state.thread_pane_focus == ThreadPaneFocus::Chat
                && active_comms_writable(app) =>
        {
            let _ = handle_single_line_paste(app, key, |state| &mut state.compose_message_input);
        }
        _ => {}
    }
}

fn handle_contact_picker_key(app: &mut LobbyApp, key: KeyEvent) {
    if app.state.show_help {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('?') => {
                app.state.show_help = false;
                app.popup_position = None;
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Esc => close_popup_route(app, LobbyRoute::Home),
        KeyCode::Char('?') => {
            app.state.show_help = true;
            app.popup_position = None;
        }
        KeyCode::Char('a' | 'A') => {
            app.state.add_contact_input.clear();
            open_popup_route(app, LobbyRoute::AddContact);
        }
        KeyCode::Char('b' | 'B') => toggle_picker_contact_block(app),
        KeyCode::Char('d' | 'D') | KeyCode::Delete => toggle_picker_contact_hidden(app),
        KeyCode::Up | KeyCode::Char('k') => move_contact_picker_selection(app, -1),
        KeyCode::Down | KeyCode::Char('j') => move_contact_picker_selection(app, 1),
        KeyCode::Enter => select_picker_contact(app),
        _ => {}
    }
}

fn handle_add_contact_key(app: &mut LobbyApp, key: KeyEvent) {
    if handle_single_line_paste(app, key, |state| &mut state.add_contact_input) {
        return;
    }

    match key.code {
        KeyCode::Esc => close_popup_route(app, LobbyRoute::ContactPicker),
        KeyCode::Backspace => {
            app.state.add_contact_input.pop();
        }
        KeyCode::Enter => submit_added_contact(app),
        KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.state.add_contact_input.push(ch);
        }
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
        KeyCode::Esc => {
            if app.state.gate_mode == KeychainGateMode::ResumeSession {
                app.state.unlock_password_input.clear();
                app.state.status_message = None;
                app.state.route = LobbyRoute::MatrixLocked;
            } else {
                app.should_quit = true;
            }
        }
        KeyCode::Char('q' | 'Q')
            if matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT) =>
        {
            app.should_quit = true;
        }
        KeyCode::Backspace => {
            app.state.unlock_password_input.pop();
        }
        KeyCode::Enter => match app.transport.unlock(&app.state.unlock_password_input) {
            Ok(loaded) => {
                let resume_session = app.state.gate_mode == KeychainGateMode::ResumeSession;
                app.state.apply_loaded(loaded);
                app.state.unlock_password_input.clear();
                app.last_activity_at = std::time::Instant::now();
                app.state.show_resume_sync_overlay = resume_session
                    && app.transport.has_session()
                    && app.state.network_status != LobbyNetworkStatus::NoRelay
                    && app.state.network_status != LobbyNetworkStatus::Synced;
                let route = if resume_session {
                    app.state.unlock_return_route
                } else {
                    LobbyRoute::Home
                };
                app.state.gate_mode = KeychainGateMode::Startup;
                app.state.unlock_return_route = LobbyRoute::Home;
                app.state.route = route;
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
        KeyCode::Esc => close_popup_route(app, LobbyRoute::Home),
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
                close_popup_route(app, LobbyRoute::Home);
                return;
            };
            match app
                .transport
                .send_invite_request(&row, &app.state.compose_message_input)
            {
                Ok(loaded) => {
                    app.state.apply_loaded(loaded);
                    app.state.compose_message_input.clear();
                    close_popup_route(app, LobbyRoute::Home);
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
        KeyCode::Esc => {
            let route = app.state.edit_handle_return_route;
            close_popup_route(app, route);
        }
        KeyCode::Backspace => {
            app.state.edit_handle_input.pop();
        }
        KeyCode::Enter => match app.transport.save_handle(&app.state.edit_handle_input) {
            Ok(loaded) => {
                app.state.apply_loaded(loaded);
                let route = app.state.edit_handle_return_route;
                close_popup_route(app, route);
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
            app.state.settings_selected = (app.state.settings_selected + 1).min(6);
        }
        KeyCode::Char('s' | 'S') => save_settings(app),
        KeyCode::Char(' ') | KeyCode::Enter => match app.state.settings_selected {
            0 => {
                app.state.edit_handle_input = app.state.player_handle.clone().unwrap_or_default();
                app.state.edit_handle_return_route = LobbyRoute::Settings;
                open_popup_route(app, LobbyRoute::EditHandle);
            }
            1 => cycle_lock_timeout(app),
            2 => {
                app.state.settings_draft.follow_mouse_on_map =
                    !app.state.settings_draft.follow_mouse_on_map;
            }
            3 => {
                app.state.settings_draft.dense_empty_sector_dots =
                    !app.state.settings_draft.dense_empty_sector_dots;
            }
            4 => open_theme_picker(app),
            5 => save_settings(app),
            6 => cancel_settings(app),
            _ => {}
        },
        _ => {}
    }
}

fn handle_theme_picker_key(app: &mut LobbyApp, key: KeyEvent) {
    let themes = app.state.available_themes();
    if themes.is_empty() {
        close_popup_route(app, LobbyRoute::Settings);
        return;
    }

    match key.code {
        KeyCode::Esc => {
            let _ = theme::apply_theme_key(&app.state.theme_original_key);
            app.state.settings_draft.theme_key = app.state.theme_original_key.clone();
            close_popup_route(app, LobbyRoute::Settings);
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
            close_popup_route(app, LobbyRoute::Settings);
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
    app.popup_position = None;
}

fn handle_submit_turn_key(app: &mut LobbyApp, key: KeyEvent) {
    if handle_submit_turn_paste(app, key) {
        return;
    }

    match key.code {
        KeyCode::Esc => {
            app.state.route = LobbyRoute::HostedGame;
            app.popup_position = None;
        }
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
                app.popup_position = None;
                return;
            }
            match app.transport.submit_turn(&row, turn, &commands) {
                Ok(loaded) => {
                    if let Some(hosted) = app.state.hosted_game.as_mut() {
                        hosted.submit_status =
                            Some("Turn submitted; check hosted status for receipt.".to_string());
                        hosted.submit_input.clear();
                    }
                    app.state.apply_loaded(loaded);
                    app.state.route = LobbyRoute::HostedGame;
                    app.popup_position = None;
                }
                Err(err) => {
                    if let Some(hosted) = app.state.hosted_game.as_mut() {
                        hosted.submit_status = Some(err);
                    }
                    app.state.route = LobbyRoute::HostedGame;
                    app.popup_position = None;
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
    open_popup_route(app, LobbyRoute::Settings);
}

fn activate_current_comms(app: &mut LobbyApp) {
    let Some(active) = app.state.active_comms_row() else {
        return;
    };
    if active.hidden {
        restore_active_conversation(app);
    }
    if let super::models::CommsConversationKey::Direct { contact_npub } = active.key {
        mark_direct_contact_read(app, &contact_npub);
    }
}

fn submit_added_contact(app: &mut LobbyApp) {
    match app
        .transport
        .add_direct_contact(&app.state.add_contact_input)
    {
        Ok((loaded, npub)) => {
            app.state.apply_loaded(loaded);
            app.state.add_contact_input.clear();
            app.state
                .set_active_comms(super::models::CommsConversationKey::Direct {
                    contact_npub: npub,
                });
            app.state.active_tab = LobbyTab::Comms;
            app.state.thread_pane_focus = ThreadPaneFocus::Chat;
            activate_current_comms(app);
            app.state.route = LobbyRoute::Home;
            app.popup_position = None;
        }
        Err(err) => set_status(app, LobbyStatusTone::Error, err),
    }
}

fn submit_active_comms_message(app: &mut LobbyApp) {
    let Some(active) = app.state.active_comms_row() else {
        set_status(
            app,
            LobbyStatusTone::Error,
            "No conversation selected.".to_string(),
        );
        return;
    };
    match active.key {
        super::models::CommsConversationKey::Announcements => {
            set_status(
                app,
                LobbyStatusTone::Info,
                "Broadcast is read-only.".to_string(),
            );
        }
        super::models::CommsConversationKey::Direct { contact_npub } => {
            match app
                .transport
                .send_direct_message(&contact_npub, &app.state.compose_message_input)
            {
                Ok(loaded) => {
                    app.state.apply_loaded(loaded);
                    app.state.compose_message_input.clear();
                    app.state.comms_scroll = 0;
                    app.state.thread_scroll = 0;
                }
                Err(err) => set_status(app, LobbyStatusTone::Error, err),
            }
        }
        super::models::CommsConversationKey::GameMail { .. } => {
            set_status(
                app,
                LobbyStatusTone::Info,
                "Anonymous game mail now lives in-game.".to_string(),
            );
        }
    }
}

fn mark_direct_contact_read(app: &mut LobbyApp, contact_npub: &str) {
    match app.transport.mark_direct_contact_read(contact_npub) {
        Ok(loaded) => app.state.apply_loaded(loaded),
        Err(err) if err == "keychain is locked" => {
            if let Some(contact) = app
                .state
                .direct_contacts
                .iter_mut()
                .find(|contact| contact.npub == contact_npub)
            {
                contact.unread_count = 0;
            }
        }
        Err(err) => set_status(app, LobbyStatusTone::Error, err),
    }
}

fn toggle_active_contact_block(app: &mut LobbyApp) {
    let Some(contact) = app.state.active_direct_contact().cloned() else {
        set_status(
            app,
            LobbyStatusTone::Info,
            "Blocking only applies to direct contacts.".to_string(),
        );
        return;
    };
    match app
        .transport
        .set_direct_contact_blocked(&contact.npub, !contact.blocked)
    {
        Ok(loaded) => app.state.apply_loaded(loaded),
        Err(err) if err == "keychain is locked" => {
            if let Some(entry) = app
                .state
                .direct_contacts
                .iter_mut()
                .find(|entry| entry.npub == contact.npub)
            {
                entry.blocked = !entry.blocked;
                if entry.blocked {
                    entry.unread_count = 0;
                }
            }
        }
        Err(err) => set_status(app, LobbyStatusTone::Error, err),
    }
}

fn hide_active_conversation(app: &mut LobbyApp) {
    let Some(contact) = app.state.active_direct_contact().cloned() else {
        set_status(
            app,
            LobbyStatusTone::Info,
            "Only direct contacts can be hidden locally.".to_string(),
        );
        return;
    };
    match app.transport.set_direct_contact_hidden(&contact.npub, true) {
        Ok(loaded) => {
            app.state.apply_loaded(loaded);
            app.state.thread_pane_focus = ThreadPaneFocus::Threads;
            app.state.comms_scroll = 0;
            app.state.thread_scroll = 0;
            app.state.sync_active_comms_selection();
        }
        Err(err) if err == "keychain is locked" => {
            if let Some(entry) = app
                .state
                .direct_contacts
                .iter_mut()
                .find(|entry| entry.npub == contact.npub)
            {
                entry.hidden = true;
                entry.unread_count = 0;
            }
            app.state.sync_active_comms_selection();
            app.state.thread_pane_focus = ThreadPaneFocus::Threads;
            app.state.comms_scroll = 0;
            app.state.thread_scroll = 0;
        }
        Err(err) => set_status(app, LobbyStatusTone::Error, err),
    }
}

fn restore_active_conversation(app: &mut LobbyApp) {
    let Some(contact) = app.state.active_direct_contact().cloned() else {
        return;
    };
    if !contact.hidden {
        return;
    }
    match app
        .transport
        .set_direct_contact_hidden(&contact.npub, false)
    {
        Ok(loaded) => app.state.apply_loaded(loaded),
        Err(err) if err == "keychain is locked" => {
            if let Some(entry) = app
                .state
                .direct_contacts
                .iter_mut()
                .find(|entry| entry.npub == contact.npub)
            {
                entry.hidden = false;
            }
        }
        Err(err) => set_status(app, LobbyStatusTone::Error, err),
    }
}

fn cycle_lock_timeout(app: &mut LobbyApp) {
    let current = app.state.settings_draft.lock_timeout_minutes;
    let next = LOCK_TIMEOUT_OPTIONS
        .iter()
        .copied()
        .find(|minutes| *minutes > current)
        .unwrap_or(LOCK_TIMEOUT_OPTIONS[0]);
    app.state.settings_draft.lock_timeout_minutes = next;
    set_status(
        app,
        LobbyStatusTone::Info,
        format!("Idle lock set to {}", lock_timeout_label(next)),
    );
}

fn cancel_settings(app: &mut LobbyApp) {
    app.state.settings_draft = app.state.settings.clone();
    let _ = theme::apply_theme_key(&app.state.settings.theme_key);
    close_popup_route(app, LobbyRoute::Home);
}

fn save_settings(app: &mut LobbyApp) {
    match super::storage::settings::save_settings_to(&app.state.settings_draft, &app.settings_path)
    {
        Ok(()) => {
            app.state.settings = app.state.settings_draft.clone();
            let _ = theme::apply_theme_key(&app.state.settings.theme_key);
            set_status(
                app,
                LobbyStatusTone::Success,
                "Saved local settings".to_string(),
            );
            close_popup_route(app, LobbyRoute::Home);
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
    open_popup_route(app, LobbyRoute::ThemePicker);
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
    if row.status != "joined" {
        let message = match row.status.as_str() {
            "requested" => "Join request is still waiting for nc-host approval.",
            "rejected" => "Join request was rejected. Select the game in Games to request again.",
            _ => "This game is not ready to open from the lobby.",
        };
        set_status(app, LobbyStatusTone::Info, message.to_string());
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
                app.popup_position = None;
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
    let previous_context = app.state.preferred_game_context_id().map(str::to_string);
    let len = match app.state.active_tab {
        LobbyTab::MyGames => app.state.joined_games.len(),
        LobbyTab::OpenGames => app.state.open_games.len(),
        LobbyTab::Comms => app.state.comms_hotlist_rows().len(),
    };
    let selection = match app.state.active_tab {
        LobbyTab::MyGames => &mut app.state.joined_selected,
        LobbyTab::OpenGames => &mut app.state.open_selected,
        LobbyTab::Comms => &mut app.state.comms_selected,
    };
    if len == 0 {
        *selection = 0;
        return;
    }
    let next = (*selection as isize + delta).clamp(0, len.saturating_sub(1) as isize);
    *selection = next as usize;
    if app.state.active_tab == LobbyTab::Comms {
        if let Some(row) = app.state.selected_comms_hotlist() {
            app.state.set_active_comms(row.key);
        }
    }
    reset_context_dependent_views(app, previous_context);
}

fn move_comms_selection(app: &mut LobbyApp, delta: isize) {
    if app.state.thread_pane_focus == ThreadPaneFocus::Chat {
        if delta < 0 {
            app.state.comms_scroll = app.state.comms_scroll.saturating_sub((-delta) as usize);
        } else {
            app.state.comms_scroll = app.state.comms_scroll.saturating_add(delta as usize);
        }
        app.state.thread_scroll = app.state.comms_scroll;
        return;
    }
    let rows = if app.state.thread_pane_focus == ThreadPaneFocus::New {
        app.state.comms_unread_rows()
    } else {
        app.state.comms_sidebar_rows()
    };
    if rows.is_empty() {
        return;
    }
    let selected = if app.state.thread_pane_focus == ThreadPaneFocus::New {
        app.state
            .comms_new_selected
            .min(rows.len().saturating_sub(1))
    } else {
        app.state
            .active_comms_row()
            .and_then(|active| rows.iter().position(|row| row.key == active.key))
            .unwrap_or(0)
    };
    let next = (selected as isize + delta).clamp(0, rows.len().saturating_sub(1) as isize) as usize;
    if app.state.thread_pane_focus == ThreadPaneFocus::New {
        app.state.comms_new_selected = next;
    } else {
        app.state.set_active_comms(rows[next].key.clone());
    }
}

fn cycle_comms_focus(app: &mut LobbyApp, delta: isize) {
    let order = [
        ThreadPaneFocus::Chat,
        ThreadPaneFocus::New,
        ThreadPaneFocus::Threads,
    ];
    let current = order
        .iter()
        .position(|focus| *focus == app.state.thread_pane_focus)
        .unwrap_or(0) as isize;
    let next = (current + delta).rem_euclid(order.len() as isize) as usize;
    app.state.thread_pane_focus = order[next];
}

fn handle_comms_enter(app: &mut LobbyApp) {
    match app.state.thread_pane_focus {
        ThreadPaneFocus::Chat => {
            if active_comms_writable(app) && !app.state.compose_message_input.trim().is_empty() {
                submit_active_comms_message(app);
            }
        }
        ThreadPaneFocus::New => {
            let Some(row) = app
                .state
                .comms_unread_rows()
                .get(app.state.comms_new_selected)
                .cloned()
            else {
                return;
            };
            app.state.set_active_comms(row.key);
            app.state.thread_pane_focus = ThreadPaneFocus::Chat;
            activate_current_comms(app);
        }
        ThreadPaneFocus::Threads => {
            activate_current_comms(app);
            app.state.thread_pane_focus = ThreadPaneFocus::Chat;
        }
    }
}

fn active_comms_writable(app: &LobbyApp) -> bool {
    app.state
        .active_comms_row()
        .is_some_and(|row| !row.read_only)
}

fn move_contact_picker_selection(app: &mut LobbyApp, delta: isize) {
    let contacts = app.state.selectable_direct_contacts();
    if contacts.is_empty() {
        app.state.contact_picker_selected = 0;
        return;
    }
    let next = (app.state.contact_picker_selected as isize + delta)
        .clamp(0, contacts.len().saturating_sub(1) as isize) as usize;
    app.state.contact_picker_selected = next;
}

fn select_picker_contact(app: &mut LobbyApp) {
    let Some((_, contact)) = app
        .state
        .selectable_direct_contacts()
        .get(app.state.contact_picker_selected)
        .cloned()
    else {
        return;
    };
    let npub = contact.npub.clone();
    if contact.hidden {
        match app.transport.set_direct_contact_hidden(&npub, false) {
            Ok(loaded) => app.state.apply_loaded(loaded),
            Err(err) => {
                set_status(app, LobbyStatusTone::Error, err);
                return;
            }
        }
    }
    app.state
        .set_active_comms(super::models::CommsConversationKey::Direct { contact_npub: npub });
    app.state.active_tab = LobbyTab::Comms;
    app.state.route = LobbyRoute::Home;
    app.state.thread_pane_focus = ThreadPaneFocus::Chat;
    activate_current_comms(app);
    app.popup_position = None;
}

fn toggle_picker_contact_block(app: &mut LobbyApp) {
    let Some((index, _)) = app
        .state
        .selectable_direct_contacts()
        .get(app.state.contact_picker_selected)
        .cloned()
    else {
        return;
    };
    app.state.contact_selected = index;
    toggle_active_contact_block(app);
}

fn toggle_picker_contact_hidden(app: &mut LobbyApp) {
    let Some((index, contact)) = app
        .state
        .selectable_direct_contacts()
        .get(app.state.contact_picker_selected)
        .cloned()
    else {
        return;
    };
    let hidden = contact.hidden;
    let npub = contact.npub.clone();
    app.state.contact_selected = index;
    if hidden {
        match app.transport.set_direct_contact_hidden(&npub, false) {
            Ok(loaded) => app.state.apply_loaded(loaded),
            Err(err) => set_status(app, LobbyStatusTone::Error, err),
        }
    } else {
        hide_active_conversation(app);
    }
}

pub(crate) fn reset_context_dependent_views(app: &mut LobbyApp, previous_context: Option<String>) {
    let current = app.state.preferred_game_context_id().map(str::to_string);
    if current != previous_context {
        app.state.reset_thread_view();
        app.state.reset_game_inbox_view();
        app.state.sync_default_contact_selection();
        app.state.sync_active_comms_selection();
    }
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
        Some(text) => Some(text),
        None => {
            set_status(
                app,
                LobbyStatusTone::Error,
                "Clipboard is unavailable.".to_string(),
            );
            None
        }
    }
}

fn open_popup_route(app: &mut LobbyApp, route: LobbyRoute) {
    app.state.route = route;
    app.popup_position = None;
}

fn close_popup_route(app: &mut LobbyApp, route: LobbyRoute) {
    app.state.route = route;
    app.popup_position = None;
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

#[cfg(test)]
mod tests {
    use super::{apply_key, ctrl_key_for_tests};
    use crate::geometry::ScreenGeometry;
    use crate::lobby::LobbyApp;
    use crate::lobby::state::{LobbyRoute, LobbyStatusTone};

    #[test]
    fn unavailable_clipboard_sets_nonfatal_status_message() {
        let mut app = LobbyApp::new_for_tests(LobbyRoute::FirstRun, ScreenGeometry::new(120, 40));
        app.clipboard.disable_for_tests();

        apply_key(&mut app, ctrl_key_for_tests('v'));

        assert_eq!(app.state.status_tone, LobbyStatusTone::Error);
        assert_eq!(
            app.state.status_message.as_deref(),
            Some("Clipboard is unavailable.")
        );
        assert_eq!(app.state.first_run_handle_input, "");
    }
}

#[cfg(test)]
fn ctrl_key_for_tests(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL)
}
