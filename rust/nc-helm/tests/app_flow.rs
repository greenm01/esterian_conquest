use nc_client::keychain::{Keychain, active_identity_npub, now_iso8601, push_new_identity};
use nc_helm::{
    App, BootSnapshot, Effect, GameRow, KeyCode, KeyEvent, KeyModifiers, LobbySnapshot, Msg, Route,
    StoredSession,
};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn dummy_session(handle: &str) -> StoredSession {
    let mut keychain = Keychain::empty();
    push_new_identity(&mut keychain, now_iso8601(), Some(handle.to_string()))
        .expect("new identity");
    let active_npub = active_identity_npub(&keychain).expect("npub");
    let active = keychain.active_identity().expect("active identity").clone();
    StoredSession {
        keychain,
        active_npub,
        active_nsec: active.nsec.clone(),
        active_handle: active.handle.clone(),
    }
}

#[test]
fn boot_without_keychain_enters_first_run() {
    let (mut app, effects) = App::new(None);
    assert!(matches!(effects.as_slice(), [Effect::LoadBoot]));
    let _ = app.dispatch(Msg::BootLoaded(Ok(BootSnapshot {
        has_keychain: false,
        relay_url: Some("ws://relay.example".to_string()),
    })));
    match &app.model().route {
        Route::FirstRun(first_run) => {
            assert_eq!(first_run.relay_input, "ws://relay.example");
        }
        other => panic!("expected first run route, got {other:?}"),
    }
}

#[test]
fn first_run_submit_emits_create_identity_effect() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::BootLoaded(Ok(BootSnapshot {
        has_keychain: false,
        relay_url: Some("ws://127.0.0.1:8080".to_string()),
    })));
    for ch in "captain".chars() {
        let _ = app.dispatch(Msg::Key(key(KeyCode::Char(ch))));
    }
    let _ = app.dispatch(Msg::Key(key(KeyCode::Tab)));
    for ch in "hunter2".chars() {
        let _ = app.dispatch(Msg::Key(key(KeyCode::Char(ch))));
    }
    let _ = app.dispatch(Msg::Key(key(KeyCode::Tab)));
    for ch in "hunter2".chars() {
        let _ = app.dispatch(Msg::Key(key(KeyCode::Char(ch))));
    }
    let _ = app.dispatch(Msg::Key(key(KeyCode::Tab)));
    let effects = app.dispatch(Msg::Key(key(KeyCode::Enter)));
    assert!(matches!(
        effects.as_slice(),
        [Effect::CreateIdentity { handle, password, relay_url }]
        if handle == "captain" && password == "hunter2" && relay_url == "ws://127.0.0.1:8080"
    ));
}

#[test]
fn lobby_help_closes_on_any_key_and_reopens_on_question_mark() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    match &app.model().route {
        Route::Lobby(lobby) => assert!(lobby.help_open),
        other => panic!("expected lobby route, got {other:?}"),
    }
    let _ = app.dispatch(Msg::Key(key(KeyCode::Enter)));
    match &app.model().route {
        Route::Lobby(lobby) => assert!(!lobby.help_open),
        other => panic!("expected lobby route, got {other:?}"),
    }
    let _ = app.dispatch(Msg::Key(key(KeyCode::Char('?'))));
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
}

#[test]
fn lock_action_disconnects_transport_and_returns_to_locked_route() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(KeyCode::Enter)));
    let effects = app.dispatch(Msg::Key(key(KeyCode::Char('l'))));
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
fn tab_switching_changes_rendered_lobby_content() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(KeyCode::Enter)));
    let buffer = app.view();
    assert!(buffer.plain_line(5).contains("HOME"));

    let _ = app.dispatch(Msg::Key(key(KeyCode::Tab)));
    let buffer = app.view();
    assert!(buffer.plain_line(5).contains("OPEN GAMES"));

    let _ = app.dispatch(Msg::Key(key(KeyCode::Tab)));
    let buffer = app.view();
    assert!(buffer.plain_line(5).contains("COMMS"));

    let _ = app.dispatch(Msg::Key(key(KeyCode::Tab)));
    let buffer = app.view();
    assert!(buffer.plain_line(5).contains("SETTINGS"));
}

#[test]
fn settings_relay_edit_emits_save_and_reconnect_effects() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(KeyCode::Enter)));
    let _ = app.dispatch(Msg::Key(key(KeyCode::Char('4'))));
    let _ = app.dispatch(Msg::Key(key(KeyCode::Char('r'))));
    for _ in 0.."ws://127.0.0.1:8080".chars().count() {
        let _ = app.dispatch(Msg::Key(key(KeyCode::Backspace)));
    }
    for ch in "ws://relay.example".chars() {
        let _ = app.dispatch(Msg::Key(key(KeyCode::Char(ch))));
    }
    let effects = app.dispatch(Msg::Key(key(KeyCode::Enter)));
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
fn relay_saved_updates_model_and_boot_snapshot() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::BootLoaded(Ok(BootSnapshot {
        has_keychain: false,
        relay_url: Some("ws://old-relay".to_string()),
    })));
    let _ = app.dispatch(Msg::RelaySaved(Ok("ws://new-relay".to_string())));
    assert_eq!(app.model().relay_url, "ws://new-relay");
}
