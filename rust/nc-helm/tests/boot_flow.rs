mod support;

use nc_helm::{App, BootSnapshot, Effect, Msg, Route};

use crate::support::key;

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
        let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char(ch))));
    }
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Tab)));
    for ch in "hunter2".chars() {
        let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char(ch))));
    }
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Tab)));
    for ch in "hunter2".chars() {
        let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char(ch))));
    }
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Tab)));
    let effects = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Enter)));
    assert!(matches!(
        effects.as_slice(),
        [Effect::CreateIdentity { handle, password, relay_url }]
        if handle == "captain" && password == "hunter2" && relay_url == "ws://127.0.0.1:8080"
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
