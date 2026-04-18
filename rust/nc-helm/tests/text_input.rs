mod support;

use nc_helm::{App, BootSnapshot, Msg, Route};

use crate::support::{dummy_session, key};

#[test]
fn text_input_updates_unlock_password_field() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::BootLoaded(Ok(BootSnapshot {
        has_keychain: true,
        relay_url: Some("ws://127.0.0.1:8080".to_string()),
    })));
    let _ = app.dispatch(Msg::TextInput("é".to_string()));
    match &app.model().route {
        Route::Locked(locked) => assert_eq!(locked.password_input, "é"),
        other => panic!("expected locked route, got {other:?}"),
    }
}

#[test]
fn text_input_updates_settings_relay_draft() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Esc)));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('s'))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('r'))));
    let _ = app.dispatch(Msg::TextInput("relay".to_string()));
    let buffer = app.view();
    assert!(buffer.plain_line(7).contains("relay"));
}
