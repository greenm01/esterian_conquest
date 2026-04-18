mod support;

use nc_helm::{App, Effect, GameRow, LobbySnapshot, Msg, Route};

use crate::support::{alt_key, dummy_session, key, left_click};

#[test]
fn lobby_help_closes_on_q_or_escape_and_reopens_on_question_mark() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    match &app.model().route {
        Route::Lobby(lobby) => assert!(lobby.help_open),
        other => panic!("expected lobby route, got {other:?}"),
    }
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

    let row = 12usize;
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
        games: vec![GameRow {
            game_id: "phase-sapling-awful".to_string(),
            name: "Phase Sapling".to_string(),
            host: "daemon".to_string(),
            tier: "sandbox".to_string(),
            seats: "1/4".to_string(),
            when: "Y3001 T1".to_string(),
        }],
        notices: vec!["sysop: sandbox reset tonight".to_string()],
    })));
    assert_eq!(app.model().games.len(), 1);
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
        Route::Locked(locked) => {
            assert_eq!(locked.status.as_deref(), Some("Session locked."));
            assert!(locked.password_input.is_empty());
        }
        other => panic!("expected locked route, got {other:?}"),
    }
    assert!(app.model().session.is_none());
    assert!(app.model().games.is_empty());
    assert!(app.model().notices.is_empty());
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
