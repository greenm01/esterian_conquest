mod support;

use nc_client::cache::ClientCache;
use nc_helm::{App, Effect, LobbySnapshot, Msg, Route, SandboxReleaseSuccess};

use crate::support::{
    alt_key, dummy_session, key, league_my_game_row, left_click, my_game_row, sandbox_open_game_row,
};

#[test]
fn lobby_help_closes_on_q_or_escape_and_reopens_on_question_mark() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    match &app.model().route {
        Route::Lobby(lobby) => assert!(!lobby.help_open),
        other => panic!("expected lobby route, got {other:?}"),
    }
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('?'))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Enter)));
    match &app.model().route {
        Route::Lobby(lobby) => assert!(lobby.help_open),
        other => panic!("expected lobby route, got {other:?}"),
    }
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('q'))));
    match &app.model().route {
        Route::Lobby(lobby) => assert!(!lobby.help_open),
        other => panic!("expected lobby route, got {other:?}"),
    }
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('?'))));
    match &app.model().route {
        Route::Lobby(lobby) => assert!(lobby.help_open),
        other => panic!("expected lobby route, got {other:?}"),
    }
}

#[test]
fn lobby_help_clicking_close_tag_closes_popup() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('?'))));

    let row = 11usize;
    let line = app.view().plain_line(row);
    let tag_offset = line.find("┐ [X] ┌").expect("close tag should render");
    let tag_col = line[..tag_offset].chars().count();
    let _ = app.dispatch(Msg::Mouse(left_click(tag_col + 3, row)));

    match &app.model().route {
        Route::Lobby(lobby) => assert!(!lobby.help_open),
        other => panic!("expected lobby route, got {other:?}"),
    }
}

#[test]
fn lobby_help_clicking_body_does_not_close_popup() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('?'))));
    let _ = app.dispatch(Msg::Mouse(left_click(24, 14)));

    match &app.model().route {
        Route::Lobby(lobby) => assert!(lobby.help_open),
        other => panic!("expected lobby route, got {other:?}"),
    }
}

#[test]
fn lobby_update_populates_games_and_notices() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::LobbyUpdated(Ok(LobbySnapshot {
        cache: ClientCache::empty(),
        my_games: Vec::new(),
        open_games: vec![sandbox_open_game_row()],
        notices: vec!["sysop: sandbox reset tonight".to_string()],
    })));
    assert_eq!(app.model().open_games.len(), 1);
    assert_eq!(app.model().notices.len(), 1);
    match &app.model().route {
        Route::Lobby(lobby) => assert!(lobby.status.is_none()),
        other => panic!("expected lobby route, got {other:?}"),
    }
}

#[test]
fn lock_action_disconnects_transport_and_returns_to_locked_route() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Esc)));
    let effects = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('l'))));
    assert!(matches!(effects.as_slice(), [Effect::DisconnectTransport]));
    match &app.model().route {
        Route::MatrixLocked => {}
        other => panic!("expected locked route, got {other:?}"),
    }
    assert!(app.model().session.is_none());
    assert!(app.model().open_games.is_empty());
    assert!(app.model().my_games.is_empty());
    assert!(app.model().notices.is_empty());
}

#[test]
fn any_key_from_matrix_lock_opens_unlock_gate() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Esc)));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('l'))));

    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Enter)));
    match &app.model().route {
        Route::Locked(locked) => {
            assert!(locked.password_input.is_empty());
            assert!(locked.resume_session);
            assert!(locked.status.is_none());
        }
        other => panic!("expected locked route, got {other:?}"),
    }
}

#[test]
fn escape_from_resume_unlock_returns_to_matrix_lock() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Esc)));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('l'))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Enter)));

    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Esc)));
    match &app.model().route {
        Route::MatrixLocked => {}
        other => panic!("expected matrix lock route, got {other:?}"),
    }
    assert!(!app.model().should_quit);
}

#[test]
fn letter_shortcuts_switch_lobby_tabs() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Esc)));
    let buffer = app.view();
    assert!(buffer.plain_line(4).contains("MY GAMES"));

    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('o'))));
    let buffer = app.view();
    assert!(buffer.plain_line(4).contains("OPEN GAMES"));

    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('c'))));
    let buffer = app.view();
    assert!(buffer.plain_line(4).contains("COMMS"));

    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('s'))));
    let buffer = app.view();
    assert!(buffer.plain_line(4).contains("SETTINGS"));
}

#[test]
fn left_clicking_tab_strip_switches_lobby_tabs() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Esc)));

    let row = 2usize;
    let line = app.view().plain_line(row);
    let tag_offset = line
        .find("[Open Games]")
        .expect("open games tab should render");
    let tag_col = line[..tag_offset].chars().count();
    let _ = app.dispatch(Msg::Mouse(left_click(tag_col + 1, row)));

    let buffer = app.view();
    assert!(buffer.plain_line(4).contains("OPEN GAMES"));
}

#[test]
fn settings_relay_edit_emits_save_and_reconnect_effects() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Esc)));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('s'))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('r'))));
    for _ in 0.."ws://127.0.0.1:8080".chars().count() {
        let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Backspace)));
    }
    for ch in "ws://relay.example".chars() {
        let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char(ch))));
    }
    let effects = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Enter)));
    assert!(matches!(
        effects.as_slice(),
        [
            Effect::SaveRelayUrl { relay_url: saved },
            Effect::DisconnectTransport,
            Effect::ConnectTransport { relay_url: connected, .. }
        ] if saved == "ws://relay.example" && connected == "ws://relay.example"
    ));
}

#[test]
fn alt_q_quits_the_lobby() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Esc)));

    let effects = app.dispatch(Msg::Key(alt_key(nc_helm::KeyCode::Char('q'))));
    assert!(matches!(effects.as_slice(), [Effect::Quit]));
    assert!(app.model().should_quit);
}

#[test]
fn enter_on_sandbox_open_game_opens_confirm_then_y_joins() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::LobbyUpdated(Ok(LobbySnapshot {
        cache: ClientCache::empty(),
        my_games: Vec::new(),
        open_games: vec![sandbox_open_game_row()],
        notices: Vec::new(),
    })));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('o'))));

    let effects = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Enter)));
    assert!(effects.is_empty());
    match &app.model().route {
        Route::SandboxJoinConfirm(row) => assert_eq!(row.game_id, "phase-sapling-awful"),
        other => panic!("expected sandbox confirm route, got {other:?}"),
    }

    let effects = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('y'))));
    assert!(matches!(
        effects.as_slice(),
        [Effect::JoinSandboxGame { row, .. }] if row.game_id == "phase-sapling-awful"
    ));
}

#[test]
fn alt_r_emits_refresh_effect_from_lobby() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Esc)));

    let effects = app.dispatch(Msg::Key(alt_key(nc_helm::KeyCode::Char('r'))));
    assert!(matches!(effects.as_slice(), [Effect::RefreshLobby]));
    match &app.model().route {
        Route::Lobby(lobby) => {
            assert_eq!(lobby.status.as_deref(), Some("Refreshing hosted lobby..."));
        }
        other => panic!("expected lobby route, got {other:?}"),
    }
}

#[test]
fn alt_d_on_sandbox_my_game_opens_confirm_then_y_releases() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::LobbyUpdated(Ok(LobbySnapshot {
        cache: ClientCache::empty(),
        my_games: vec![my_game_row("joined")],
        open_games: vec![sandbox_open_game_row()],
        notices: Vec::new(),
    })));

    let effects = app.dispatch(Msg::Key(alt_key(nc_helm::KeyCode::Char('d'))));
    assert!(effects.is_empty());
    match &app.model().route {
        Route::SandboxDeleteConfirm(row) => assert_eq!(row.game_id, "phase-sapling-awful"),
        other => panic!("expected sandbox delete confirm route, got {other:?}"),
    }

    let effects = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('y'))));
    assert!(matches!(
        effects.as_slice(),
        [Effect::ReleaseSandboxGame { row }] if row.game_id == "phase-sapling-awful"
    ));
    match &app.model().route {
        Route::Lobby(lobby) => {
            assert_eq!(lobby.status.as_deref(), Some("Releasing sandbox seat..."));
        }
        other => panic!("expected lobby route, got {other:?}"),
    }
}

#[test]
fn alt_d_on_non_sandbox_my_game_is_ignored() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::LobbyUpdated(Ok(LobbySnapshot {
        cache: ClientCache::empty(),
        my_games: vec![league_my_game_row("joined")],
        open_games: vec![sandbox_open_game_row()],
        notices: Vec::new(),
    })));

    let effects = app.dispatch(Msg::Key(alt_key(nc_helm::KeyCode::Char('d'))));
    assert!(effects.is_empty());
    match &app.model().route {
        Route::Lobby(_) => {}
        other => panic!("expected lobby route, got {other:?}"),
    }
}

#[test]
fn sandbox_release_success_removes_row_and_returns_to_lobby() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::LobbyUpdated(Ok(LobbySnapshot {
        cache: ClientCache::empty(),
        my_games: vec![my_game_row("joined")],
        open_games: vec![sandbox_open_game_row()],
        notices: Vec::new(),
    })));
    let _ = app.dispatch(Msg::Key(alt_key(nc_helm::KeyCode::Char('d'))));

    let effects = app.dispatch(Msg::SandboxReleased(Ok(SandboxReleaseSuccess {
        game_id: "phase-sapling-awful".to_string(),
        cache: ClientCache::empty(),
    })));
    assert!(matches!(
        effects.as_slice(),
        [Effect::SaveClientCache { .. }]
    ));
    assert!(app.model().my_games.is_empty());
    match &app.model().route {
        Route::Lobby(lobby) => {
            assert_eq!(
                lobby.status.as_deref(),
                Some("Sandbox removed from My Games.")
            );
        }
        other => panic!("expected lobby route, got {other:?}"),
    }
}

#[test]
fn enter_on_joined_my_game_emits_open_hosted_game_effect() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::LobbyUpdated(Ok(LobbySnapshot {
        cache: ClientCache::empty(),
        my_games: vec![my_game_row("joined")],
        open_games: vec![sandbox_open_game_row()],
        notices: Vec::new(),
    })));

    let effects = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Enter)));
    assert!(matches!(
        effects.as_slice(),
        [Effect::OpenHostedGame { row, .. }] if row.game_id == "phase-sapling-awful"
    ));
}

#[test]
fn enter_on_requested_my_game_stays_in_lobby_with_status() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::LobbyUpdated(Ok(LobbySnapshot {
        cache: ClientCache::empty(),
        my_games: vec![my_game_row("requested")],
        open_games: vec![sandbox_open_game_row()],
        notices: Vec::new(),
    })));

    let effects = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Enter)));
    assert!(effects.is_empty());
    match &app.model().route {
        Route::Lobby(lobby) => assert_eq!(
            lobby.status.as_deref(),
            Some("Join request is still waiting for nc-host approval.")
        ),
        other => panic!("expected lobby route, got {other:?}"),
    }
}
