use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::state::{LobbyApp, LobbyFocus, LobbyRoute};
use super::transport::LobbyTransport;

pub fn apply_key(app: &mut LobbyApp, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q' | 'Q')
            if matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT) =>
        {
            match app.state.route {
                LobbyRoute::Home | LobbyRoute::FirstRun | LobbyRoute::Locked => {
                    app.should_quit = true;
                }
                _ => app.state.route = LobbyRoute::Home,
            }
        }
        KeyCode::Enter => handle_enter(app),
        KeyCode::Tab => {
            if app.state.route == LobbyRoute::Home {
                app.state.focus = app.state.focus.next();
            }
        }
        KeyCode::BackTab => {
            if app.state.route == LobbyRoute::Home {
                app.state.focus = app.state.focus.prev();
            }
        }
        KeyCode::Up | KeyCode::Char('k') => move_selection(app, -1),
        KeyCode::Down | KeyCode::Char('j') => move_selection(app, 1),
        KeyCode::Char('n' | 'N') if app.state.route == LobbyRoute::Home => {
            app.state.route = LobbyRoute::ComposeInvite;
        }
        KeyCode::Char('h' | 'H') if app.state.route == LobbyRoute::Home => {
            app.state.route = LobbyRoute::EditHandle;
        }
        _ => {}
    }
}

fn handle_enter(app: &mut LobbyApp) {
    match app.state.route {
        LobbyRoute::FirstRun => {
            app.state.status_message = Some(
                "Onboarding is stubbed. The lobby shell is active with placeholder data."
                    .to_string(),
            );
            app.state.route = LobbyRoute::Home;
        }
        LobbyRoute::Locked => {
            app.state.status_message = Some(
                "Unlock is stubbed. The lobby shell is active with placeholder data.".to_string(),
            );
            app.state.route = LobbyRoute::Home;
        }
        LobbyRoute::ComposeInvite => {
            app.state.status_message = Some(format!(
                "Invite request stub queued via {}.",
                app.transport.status_label()
            ));
            app.state.route = LobbyRoute::Home;
        }
        LobbyRoute::EditHandle => {
            app.state.status_message = Some("Handle edit stub saved locally.".to_string());
            app.state.route = LobbyRoute::Home;
        }
        LobbyRoute::Home => {
            if app.state.focus == LobbyFocus::OpenGames {
                app.state.route = LobbyRoute::ComposeInvite;
            }
        }
    }
}

fn move_selection(app: &mut LobbyApp, delta: isize) {
    if app.state.route != LobbyRoute::Home {
        return;
    }
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
