use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nc_client::hosted::store::HostedDraftStatus;
use nc_client::password::validate_new_password;
use nc_client::paths::data_root;
use nc_nostr::first_join::FIRST_JOIN_NAME_MAX_CHARS;
use std::fs;
use std::time::Instant;

use super::hosted::dashboard::{build_hosted_dash_app, replay_hosted_draft};
use super::state::{
    FirstJoinSetupField, FirstJoinSetupView, FirstRunField, GateResetAction, HostedGameView,
    KeychainGateMode, LobbyApp, LobbyNetworkStatus, LobbyRoute, LobbyStatusTone, LobbyTab,
    ThreadPaneFocus,
};
use super::storage::settings::{LOCK_TIMEOUT_OPTIONS, lock_timeout_label};
use crate::theme;

const INVALID_GATE_ENTRY_MESSAGE: &str = "invalid entry, try again.";

pub fn apply_key(app: &mut LobbyApp, key: KeyEvent) {
    if app.state.show_manual {
        app.state.show_manual = false;
        app.popup_position = None;
        return;
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        match key.code {
            KeyCode::Char('q' | 'Q') => {
                if app.state.route == LobbyRoute::HostedGame {
                    dispatch_hosted_dashboard_key(app, key);
                    return;
                }
                if app.state.route == LobbyRoute::Home {
                    app.state.quit_confirm_return_route = app.state.route;
                    open_popup_route(app, LobbyRoute::QuitConfirm);
                }
                return;
            }
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
        LobbyRoute::FirstJoinSetup => handle_first_join_setup_key(app, key),
        LobbyRoute::QuitConfirm => handle_quit_confirm_key(app, key),
        LobbyRoute::ComposeInvite => handle_compose_key(app, key),
        LobbyRoute::SandboxJoinConfirm => handle_sandbox_join_confirm_key(app, key),
        LobbyRoute::SandboxJoinUnavailable => handle_sandbox_join_unavailable_key(app, key),
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
        KeyCode::Enter => match app.state.active_tab {
            LobbyTab::OpenGames => activate_selected_open_game(app),
            LobbyTab::MyGames => open_or_claim_selected_game(app),
            _ => {}
        },
        KeyCode::Up => move_selection(app, -1),
        KeyCode::Down => move_selection(app, 1),
        KeyCode::Char('J' | 'n' | 'N') => {
            activate_selected_open_game(app);
        }
        KeyCode::Char('H' | 'h') => {
            open_manual_popup(app);
        }
        KeyCode::Char('s' | 'S') => open_settings(app),
        KeyCode::Char('r' | 'R') => refresh_lobby(app),
        KeyCode::Char('?') => {
            app.state.show_help = true;
            app.state.show_manual = false;
            app.popup_position = None;
        }
        _ => {}
    }
}

fn handle_quit_confirm_key(app: &mut LobbyApp, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y' | 'Y') => app.should_quit = true,
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('n' | 'N') => {
            let route = app.state.quit_confirm_return_route;
            close_popup_route(app, route);
        }
        _ => {}
    }
}

fn handle_sandbox_join_confirm_key(app: &mut LobbyApp, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y' | 'Y') => join_selected_sandbox_game(app),
        _ => {
            app.state.sandbox_join_target = None;
            app.state.sandbox_join_notice = None;
            close_popup_route(app, LobbyRoute::Home);
        }
    }
}

fn handle_sandbox_join_unavailable_key(app: &mut LobbyApp, _key: KeyEvent) {
    app.state.sandbox_join_target = None;
    app.state.sandbox_join_notice = None;
    close_popup_route(app, LobbyRoute::Home);
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
            if app.state.route != LobbyRoute::Home {
                enter_home(app);
                app.state.thread_pane_focus = ThreadPaneFocus::Chat;
            }
        }
        KeyCode::Char('?') => {
            app.state.show_help = true;
            app.state.show_manual = false;
            app.popup_position = None;
        }
        KeyCode::Char('H' | 'h')
            if app.state.route == LobbyRoute::Home
                && app.state.thread_pane_focus != ThreadPaneFocus::Chat =>
        {
            open_manual_popup(app);
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
            app.state.show_manual = false;
            app.popup_position = None;
        }
        KeyCode::Char('a' | 'A') => {
            app.state.add_contact_input.clear();
            open_popup_route(app, LobbyRoute::AddContact);
        }
        KeyCode::Char('b' | 'B') => toggle_picker_contact_block(app),
        KeyCode::Char('d' | 'D') | KeyCode::Delete => toggle_picker_contact_hidden(app),
        KeyCode::Up => move_contact_picker_selection(app, -1),
        KeyCode::Down => move_contact_picker_selection(app, 1),
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
    if app.gate_reset_deadline.is_some() {
        return;
    }
    if handle_single_line_paste(app, key, |state| match state.first_run_field {
        FirstRunField::Handle => &mut state.first_run_handle_input,
        FirstRunField::Password => &mut state.first_run_password_input,
        FirstRunField::Confirm => &mut state.first_run_confirm_input,
    }) {
        return;
    }

    match key.code {
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
                if validate_new_password(
                    &app.state.first_run_password_input,
                    &app.state.first_run_confirm_input,
                )
                .is_err()
                {
                    set_gate_error(
                        app,
                        GateResetAction::FirstRunRetry,
                        INVALID_GATE_ENTRY_MESSAGE.to_string(),
                    );
                    return;
                }
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
                        app.state.auto_open_manual_after_onboarding = true;
                        enter_home(app);
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
    if app.gate_reset_deadline.is_some() {
        return;
    }
    if handle_single_line_paste(app, key, |state| &mut state.unlock_password_input) {
        return;
    }

    match key.code {
        KeyCode::Esc => {
            if app.state.gate_mode == KeychainGateMode::ResumeSession {
                app.state.unlock_password_input.clear();
                app.state.status_message = None;
                app.clear_gate_reset();
                app.state.route = LobbyRoute::MatrixLocked;
            }
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
                if route == LobbyRoute::Home {
                    enter_home(app);
                } else {
                    app.state.route = route;
                }
            }
            Err(err) if err == INVALID_GATE_ENTRY_MESSAGE => {
                set_gate_error(app, GateResetAction::UnlockRetry, err);
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

fn handle_first_join_setup_key(app: &mut LobbyApp, key: KeyEvent) {
    if handle_first_join_setup_paste(app, key) {
        trim_first_join_setup_inputs(app);
        return;
    }

    match key.code {
        KeyCode::Esc => cancel_first_join_setup(app),
        KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
            if let Some(setup) = app.state.first_join_setup.as_mut() {
                setup.active_field = setup.active_field.next();
                setup.status = None;
            }
        }
        KeyCode::Backspace => {
            if let Some(setup) = app.state.first_join_setup.as_mut() {
                match setup.active_field {
                    FirstJoinSetupField::Empire => {
                        setup.empire_input.pop();
                    }
                    FirstJoinSetupField::Homeworld => {
                        setup.homeworld_input.pop();
                    }
                }
                setup.status = None;
            }
        }
        KeyCode::Enter => {
            if !validate_first_join_setup_field(app) {
                return;
            }
            let Some(active_field) = app
                .state
                .first_join_setup
                .as_ref()
                .map(|setup| setup.active_field)
            else {
                return;
            };
            if active_field == FirstJoinSetupField::Empire {
                if let Some(setup) = app.state.first_join_setup.as_mut() {
                    setup.active_field = FirstJoinSetupField::Homeworld;
                    setup.status = None;
                }
            } else {
                submit_first_join_setup(app);
            }
        }
        KeyCode::Char(ch)
            if !key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::ALT) =>
        {
            if let Some(setup) = app.state.first_join_setup.as_mut() {
                let input = match setup.active_field {
                    FirstJoinSetupField::Empire => &mut setup.empire_input,
                    FirstJoinSetupField::Homeworld => &mut setup.homeworld_input,
                };
                if input.chars().count() < FIRST_JOIN_NAME_MAX_CHARS {
                    input.push(ch);
                }
                setup.status = None;
            }
        }
        _ => {}
    }
}

fn handle_first_join_setup_paste(app: &mut LobbyApp, key: KeyEvent) -> bool {
    let Some(text) = read_clipboard_text(app, key) else {
        return false;
    };
    let Some(setup) = app.state.first_join_setup.as_mut() else {
        return false;
    };
    let input = match setup.active_field {
        FirstJoinSetupField::Empire => &mut setup.empire_input,
        FirstJoinSetupField::Homeworld => &mut setup.homeworld_input,
    };
    input.extend(sanitize_single_line_paste(&text));
    true
}

fn handle_hosted_game_key(app: &mut LobbyApp, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            if app
                .state
                .hosted_game
                .as_ref()
                .is_some_and(|hosted| hosted.dashboard.is_at_root_surface())
            {
                dispatch_hosted_dashboard_key(
                    app,
                    KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT),
                );
            } else {
                dispatch_hosted_dashboard_key(app, key);
            }
        }
        KeyCode::Char('r' | 'R') => refresh_hosted_game(app),
        KeyCode::Char('t' | 'T') => open_submit_turn(app),
        _ => dispatch_hosted_dashboard_key(app, key),
    }
}

fn dispatch_hosted_dashboard_key(app: &mut LobbyApp, key: KeyEvent) {
    if let Some(hosted) = app.state.hosted_game.as_mut() {
        hosted.dashboard.dispatch_key_event(key);
        if hosted.dashboard.should_quit {
            app.should_quit = true;
        }
    }
    sync_hosted_dashboard_draft(app);
}

pub(crate) fn sync_hosted_dashboard_draft(app: &mut LobbyApp) {
    let Some(hosted) = app.state.hosted_game.as_ref() else {
        return;
    };
    let Some(draft) = hosted.dashboard.hosted_turn_draft.clone() else {
        return;
    };
    let game_id = hosted.row.game_id.clone();
    let base_hash = hosted.snapshot.state_hash.clone();
    let has_commands = hosted.dashboard.hosted_turn_text().is_some();
    let result = if has_commands {
        app.transport
            .save_hosted_draft(&game_id, &base_hash, &draft, HostedDraftStatus::Local)
    } else {
        app.transport.clear_hosted_draft(&game_id)
    };
    if let Err(err) = result {
        if let Some(hosted) = app.state.hosted_game.as_mut() {
            hosted.submit_status = Some(format!("Failed to persist hosted draft: {err}"));
        }
    }
}

fn handle_settings_key(app: &mut LobbyApp, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => cancel_settings(app),
        KeyCode::Up => {
            app.state.settings_selected = app.state.settings_selected.saturating_sub(1);
        }
        KeyCode::Down => {
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
        KeyCode::Up => {
            if app.state.theme_selected > 0 {
                app.state.theme_selected -= 1;
                preview_selected_theme(app, &themes);
            }
        }
        KeyCode::Down => {
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
        enter_home(app);
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
                enter_home(app);
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
            enter_home(app);
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
        enter_home(app);
        return;
    };
    match app.transport.open_game(&row) {
        Ok(snapshot) => match build_hosted_dash_app_with_local_draft(app, &row, &snapshot) {
            Ok((dashboard, submit_status)) => {
                let submit_input = dashboard.hosted_turn_text().unwrap_or_default();
                if let Some(hosted) = app.state.hosted_game.as_mut() {
                    hosted.snapshot = snapshot;
                    hosted.dashboard = dashboard;
                    hosted.submit_input = submit_input;
                    hosted.submit_status = submit_status
                        .or_else(|| Some("Hosted dashboard refreshed from nc-host.".to_string()));
                }
            }
            Err(err) => {
                if let Some(hosted) = app.state.hosted_game.as_mut() {
                    hosted.submit_status = Some(format!("Hosted dashboard refresh failed: {err}"));
                }
            }
        },
        Err(err) => {
            if let Some(loaded) = err.loaded {
                app.state.apply_loaded(loaded);
            }
            if let Some(hosted) = app.state.hosted_game.as_mut() {
                hosted.submit_status = Some(err.message);
            }
        }
    }
}

pub(crate) fn activate_selected_open_game(app: &mut LobbyApp) {
    let Some(row) = app.state.selected_open_game().cloned() else {
        set_status(
            app,
            LobbyStatusTone::Error,
            "no hosted game selected".to_string(),
        );
        return;
    };
    if row.game_tier.eq_ignore_ascii_case("sandbox") {
        if let Some(joined_row) = app
            .state
            .joined_games
            .iter()
            .find(|joined| joined.game_id == row.game_id && joined.status == "joined")
            .cloned()
        {
            open_joined_game(app, joined_row);
            return;
        }
        app.state.sandbox_join_target = Some(row);
        app.state.sandbox_join_notice = None;
        open_popup_route(app, LobbyRoute::SandboxJoinConfirm);
        return;
    }
    app.state.compose_message_input.clear();
    open_popup_route(app, LobbyRoute::ComposeInvite);
}

fn join_selected_sandbox_game(app: &mut LobbyApp) {
    let Some(row) = app.state.sandbox_join_target.clone() else {
        close_popup_route(app, LobbyRoute::Home);
        return;
    };
    match app.transport.join_sandbox_game(&row) {
        Ok(super::transport::LobbySandboxJoinResult::Joined(success)) => {
            let row = success
                .loaded
                .joined_games
                .iter()
                .find(|joined| joined.game_id == success.snapshot.game_id)
                .cloned()
                .unwrap_or_else(|| {
                    let mut joined = super::models::JoinedGameRow::new(
                        &row.game_id,
                        "joined",
                        &row.game,
                        &row.host,
                        &row.relay_url,
                        &row.daemon_pubkey,
                        Some(success.snapshot.player_seat as u8),
                        &format!("Y{} T{}", success.snapshot.year, success.snapshot.turn),
                    );
                    joined.game_tier = row.game_tier.clone();
                    joined.host_contact_npub = row.host_contact_npub.clone();
                    joined.last_turn = Some(success.snapshot.turn);
                    joined.last_hash = Some(success.snapshot.state_hash.clone());
                    joined
                });
            app.state.apply_loaded(success.loaded);
            app.state.sandbox_join_target = None;
            app.state.sandbox_join_notice = None;
            open_hosted_dashboard(app, row, success.snapshot);
        }
        Ok(super::transport::LobbySandboxJoinResult::Full(message)) => {
            app.state.sandbox_join_notice = Some(message);
            open_popup_route(app, LobbyRoute::SandboxJoinUnavailable);
        }
        Err(err) => {
            app.state.sandbox_join_target = None;
            app.state.sandbox_join_notice = None;
            close_popup_route(app, LobbyRoute::Home);
            set_status(app, LobbyStatusTone::Error, err);
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
            "expired" => "Your sandbox seat is no longer active. Rejoin from Open Games.",
            _ => "This game is not ready to open from the lobby.",
        };
        set_status(app, LobbyStatusTone::Info, message.to_string());
        return;
    }
    open_joined_game(app, row);
}

fn open_joined_game(app: &mut LobbyApp, row: super::models::JoinedGameRow) {
    match app.transport.open_game(&row) {
        Ok(snapshot) => open_hosted_dashboard(app, row, snapshot),
        Err(err) => {
            if let Some(loaded) = err.loaded {
                app.state.apply_loaded(loaded);
            }
            let tone = if err.code == Some(nc_nostr::state_sync::StateErrorCode::NotAPlayer) {
                LobbyStatusTone::Info
            } else {
                LobbyStatusTone::Error
            };
            set_status(app, tone, err.message);
        }
    }
}

fn open_hosted_dashboard(
    app: &mut LobbyApp,
    row: super::models::JoinedGameRow,
    snapshot: nc_nostr::state_sync::GameState,
) {
    maybe_dump_hosted_snapshot(app, &snapshot);
    if let Some(setup) = first_join_setup_view(row.clone(), &snapshot) {
        app.state.first_join_setup = Some(setup);
        open_popup_route(app, LobbyRoute::FirstJoinSetup);
        return;
    }
    app.state.first_join_setup = None;
    match build_hosted_dash_app_with_local_draft(app, &row, &snapshot) {
        Ok((dashboard, submit_status)) => {
            let submit_input = dashboard.hosted_turn_text().unwrap_or_default();
            app.state.hosted_game = Some(HostedGameView {
                row,
                snapshot,
                dashboard,
                submit_input,
                submit_status: submit_status
                    .or_else(|| Some("Hosted dashboard loaded from nc-host.".to_string())),
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
    }
}

fn maybe_dump_hosted_snapshot(app: &LobbyApp, snapshot: &nc_nostr::state_sync::GameState) {
    if !app.diagnostic_mode {
        return;
    }
    let path = data_root().join("last-hosted-snapshot.json");
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_vec_pretty(snapshot) {
        let _ = fs::write(path, json);
    }
}

fn build_hosted_dash_app_with_local_draft(
    app: &mut LobbyApp,
    row: &super::models::JoinedGameRow,
    snapshot: &nc_nostr::state_sync::GameState,
) -> Result<(crate::app::state::DashApp, Option<String>), String> {
    let mut dashboard =
        build_hosted_dash_app(snapshot, app.geometry).map_err(|err| err.to_string())?;
    let Some(cached_draft) = app
        .transport
        .load_hosted_draft(&row.game_id)
        .map_err(|err| err.to_string())?
    else {
        return Ok((dashboard, None));
    };
    if snapshot.turn > cached_draft.turn {
        app.transport
            .clear_hosted_draft(&row.game_id)
            .map_err(|err| err.to_string())?;
        return Ok((
            dashboard,
            Some("Cleared staged local orders from an earlier turn.".to_string()),
        ));
    }
    if snapshot.turn != cached_draft.turn {
        app.transport
            .save_hosted_draft(
                &row.game_id,
                &cached_draft.base_hash,
                &cached_draft.draft,
                HostedDraftStatus::Conflict,
            )
            .map_err(|err| err.to_string())?;
        return Ok((
            dashboard,
            Some("Local staged orders are out of sync and need review.".to_string()),
        ));
    }
    match replay_hosted_draft(&mut dashboard, &cached_draft.draft) {
        Ok(()) => {
            let message = match cached_draft.status {
                HostedDraftStatus::SubmittedPending => {
                    "Hosted dashboard loaded from nc-host. Reapplied pending submitted orders."
                }
                HostedDraftStatus::Conflict => {
                    "Hosted dashboard loaded from nc-host. Reapplied local orders marked for review."
                }
                HostedDraftStatus::Local => {
                    "Hosted dashboard loaded from nc-host. Reapplied staged local orders."
                }
            };
            Ok((dashboard, Some(message.to_string())))
        }
        Err(err) => {
            app.transport
                .save_hosted_draft(
                    &row.game_id,
                    &cached_draft.base_hash,
                    &cached_draft.draft,
                    HostedDraftStatus::Conflict,
                )
                .map_err(|save_err| save_err.to_string())?;
            Ok((
                dashboard,
                Some(format!(
                    "Local staged orders no longer apply cleanly and need review: {err}"
                )),
            ))
        }
    }
}

pub(crate) fn reload_hosted_dashboard_from_cached_snapshot(app: &mut LobbyApp) -> bool {
    let Some((row, current_hash)) = app
        .state
        .hosted_game
        .as_ref()
        .map(|hosted| (hosted.row.clone(), hosted.snapshot.state_hash.clone()))
    else {
        return false;
    };
    let Ok(Some(snapshot)) = app.transport.load_cached_hosted_snapshot(&row.game_id) else {
        return false;
    };
    if snapshot.state_hash == current_hash {
        return false;
    }
    match build_hosted_dash_app_with_local_draft(app, &row, &snapshot) {
        Ok((dashboard, submit_status)) => {
            let submit_input = dashboard.hosted_turn_text().unwrap_or_default();
            if let Some(hosted) = app.state.hosted_game.as_mut() {
                hosted.snapshot = snapshot;
                hosted.dashboard = dashboard;
                hosted.submit_input = submit_input;
                hosted.submit_status = submit_status
                    .or_else(|| Some("Hosted dashboard synchronized from nc-host.".to_string()));
            }
            true
        }
        Err(err) => {
            if let Some(hosted) = app.state.hosted_game.as_mut() {
                hosted.submit_status = Some(format!("Hosted dashboard sync failed: {err}"));
            }
            false
        }
    }
}

fn first_join_setup_view(
    row: super::models::JoinedGameRow,
    snapshot: &nc_nostr::state_sync::GameState,
) -> Option<FirstJoinSetupView> {
    let empire_pending = snapshot.state.player.empire_name == "In Civil Disorder";
    let homeworld = snapshot
        .state
        .owned_planets
        .iter()
        .find(|planet| {
            planet.planet_index == usize::from(snapshot.state.player.homeworld_planet_index)
        })
        .or_else(|| snapshot.state.owned_planets.first())?;
    let homeworld_pending = homeworld.name == "Not Named Yet";
    if !empire_pending && !homeworld_pending {
        return None;
    }
    Some(FirstJoinSetupView {
        row,
        empire_input: if empire_pending {
            String::new()
        } else {
            snapshot.state.player.empire_name.clone()
        },
        homeworld_input: if homeworld_pending {
            String::new()
        } else {
            homeworld.name.clone()
        },
        active_field: if empire_pending {
            FirstJoinSetupField::Empire
        } else {
            FirstJoinSetupField::Homeworld
        },
        status: None,
        homeworld_coords: homeworld.coords,
        present_production: u16::from(homeworld.current_production),
        potential_production: homeworld.potential_production,
    })
}

fn validate_first_join_setup_field(app: &mut LobbyApp) -> bool {
    let Some(setup) = app.state.first_join_setup.as_mut() else {
        return false;
    };
    let value = match setup.active_field {
        FirstJoinSetupField::Empire => setup.empire_input.trim(),
        FirstJoinSetupField::Homeworld => setup.homeworld_input.trim(),
    };
    if value.is_empty() {
        setup.status = Some(match setup.active_field {
            FirstJoinSetupField::Empire => {
                "Empire names need at least one visible character.".to_string()
            }
            FirstJoinSetupField::Homeworld => {
                "Homeworld names need at least one visible character.".to_string()
            }
        });
        return false;
    }
    true
}

fn trim_first_join_setup_inputs(app: &mut LobbyApp) {
    let Some(setup) = app.state.first_join_setup.as_mut() else {
        return;
    };
    if setup.empire_input.chars().count() > FIRST_JOIN_NAME_MAX_CHARS {
        setup.empire_input = setup
            .empire_input
            .chars()
            .take(FIRST_JOIN_NAME_MAX_CHARS)
            .collect();
    }
    if setup.homeworld_input.chars().count() > FIRST_JOIN_NAME_MAX_CHARS {
        setup.homeworld_input = setup
            .homeworld_input
            .chars()
            .take(FIRST_JOIN_NAME_MAX_CHARS)
            .collect();
    }
    setup.status = None;
}

fn submit_first_join_setup(app: &mut LobbyApp) {
    let Some(setup) = app.state.first_join_setup.as_ref() else {
        return;
    };
    let row = setup.row.clone();
    let empire_name = setup.empire_input.trim().to_string();
    let homeworld_name = setup.homeworld_input.trim().to_string();
    match app
        .transport
        .complete_first_join_setup(&row, &empire_name, &homeworld_name)
    {
        Ok(snapshot) => {
            app.state.first_join_setup = None;
            open_hosted_dashboard(app, row, snapshot);
        }
        Err(err) => {
            if let Some(setup) = app.state.first_join_setup.as_mut() {
                setup.status = Some(err);
            }
        }
    }
}

fn cancel_first_join_setup(app: &mut LobbyApp) {
    app.state.first_join_setup = None;
    close_popup_route(app, LobbyRoute::Home);
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
    enter_home(app);
    app.state.thread_pane_focus = ThreadPaneFocus::Chat;
    activate_current_comms(app);
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
    app.clear_gate_reset();
    app.state.route = route;
    app.popup_position = None;
}

fn close_popup_route(app: &mut LobbyApp, route: LobbyRoute) {
    app.clear_gate_reset();
    if route == LobbyRoute::Home {
        enter_home(app);
    } else {
        app.state.route = route;
        app.popup_position = None;
    }
}

fn open_manual_popup(app: &mut LobbyApp) {
    app.state.show_help = false;
    app.state.show_manual = true;
    app.state.manual_seen_this_session = true;
    app.popup_position = None;
}

pub(crate) fn maybe_open_home_manual(app: &mut LobbyApp) {
    if app.state.route != LobbyRoute::Home || app.state.show_resume_sync_overlay {
        return;
    }
    if app.state.manual_seen_this_session {
        app.state.auto_open_manual_after_onboarding = false;
        return;
    }
    app.state.auto_open_manual_after_onboarding = false;
    open_manual_popup(app);
}

pub(crate) fn close_active_popup(app: &mut LobbyApp) {
    if app.state.show_manual {
        app.state.show_manual = false;
        app.popup_position = None;
        return;
    }
    if app.state.show_help {
        app.state.show_help = false;
        app.popup_position = None;
        return;
    }

    match app.state.route {
        LobbyRoute::QuitConfirm => {
            let route = app.state.quit_confirm_return_route;
            close_popup_route(app, route);
        }
        LobbyRoute::ComposeInvite => close_popup_route(app, LobbyRoute::Home),
        LobbyRoute::SandboxJoinConfirm | LobbyRoute::SandboxJoinUnavailable => {
            app.state.sandbox_join_target = None;
            app.state.sandbox_join_notice = None;
            close_popup_route(app, LobbyRoute::Home);
        }
        LobbyRoute::EditHandle => {
            let route = app.state.edit_handle_return_route;
            close_popup_route(app, route);
        }
        LobbyRoute::FirstJoinSetup => cancel_first_join_setup(app),
        LobbyRoute::Settings => cancel_settings(app),
        LobbyRoute::ThemePicker => {
            let _ = theme::apply_theme_key(&app.state.theme_original_key);
            app.state.settings_draft.theme_key = app.state.theme_original_key.clone();
            close_popup_route(app, LobbyRoute::Settings);
        }
        LobbyRoute::SubmitTurn => {
            app.state.route = LobbyRoute::HostedGame;
            app.popup_position = None;
        }
        LobbyRoute::ContactPicker => close_popup_route(app, LobbyRoute::Home),
        LobbyRoute::AddContact => close_popup_route(app, LobbyRoute::ContactPicker),
        _ => {}
    }
}

fn enter_home(app: &mut LobbyApp) {
    app.clear_gate_reset();
    app.state.route = LobbyRoute::Home;
    app.popup_position = None;
    maybe_open_home_manual(app);
}

pub(crate) fn set_network_error(app: &mut LobbyApp, err: String) {
    app.state.network_status = LobbyNetworkStatus::Error;
    set_status(app, LobbyStatusTone::Error, err);
}

pub(crate) fn set_status(app: &mut LobbyApp, tone: LobbyStatusTone, message: String) {
    app.clear_gate_reset();
    app.state.status_tone = tone;
    app.state.status_message = Some(message);
}

pub(crate) fn clear_status(app: &mut LobbyApp) {
    app.clear_gate_reset();
    app.state.status_message = None;
    app.state.status_tone = LobbyStatusTone::Info;
}

fn set_gate_error(app: &mut LobbyApp, action: GateResetAction, message: String) {
    app.schedule_gate_reset(action, Instant::now(), message);
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
    use super::{apply_key, ctrl_key_for_tests, maybe_open_home_manual};
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

    #[test]
    fn home_manual_auto_opens_first_time_home_is_eligible() {
        let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));

        maybe_open_home_manual(&mut app);

        assert!(app.state.show_manual);
        assert!(app.state.manual_seen_this_session);
    }

    #[test]
    fn home_manual_auto_opens_once_after_onboarding() {
        let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
        app.state.auto_open_manual_after_onboarding = true;

        maybe_open_home_manual(&mut app);

        assert!(app.state.show_manual);
        assert!(app.state.manual_seen_this_session);
        assert!(!app.state.auto_open_manual_after_onboarding);

        app.state.show_manual = false;
        maybe_open_home_manual(&mut app);

        assert!(!app.state.show_manual);
    }
}

#[cfg(test)]
fn ctrl_key_for_tests(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL)
}
