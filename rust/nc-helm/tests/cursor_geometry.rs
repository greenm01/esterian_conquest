mod support;

use nc_helm::{App, BootSnapshot, Msg, Point};

use crate::support::{dummy_session, key, view_cursor};

#[test]
fn first_run_cursor_starts_at_handle_origin_and_advances_with_input() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::BootLoaded(Ok(BootSnapshot {
        has_keychain: false,
        relay_url: Some("ws://127.0.0.1:8080".to_string()),
    })));
    assert_eq!(view_cursor(&app), Point::from_usize(31, 16));

    for ch in "captain".chars() {
        let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char(ch))));
    }
    let buffer = app.view();
    assert!(buffer.plain_line(16).contains("captain"));
    assert_eq!(
        buffer.cursor().expect("cursor should be set"),
        Point::from_usize(38, 16)
    );
}

#[test]
fn first_run_tab_moves_cursor_to_password_origin() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::BootLoaded(Ok(BootSnapshot {
        has_keychain: false,
        relay_url: Some("ws://127.0.0.1:8080".to_string()),
    })));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Tab)));
    assert_eq!(view_cursor(&app), Point::from_usize(31, 18));
}

#[test]
fn locked_cursor_starts_at_password_origin() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::BootLoaded(Ok(BootSnapshot {
        has_keychain: true,
        relay_url: Some("ws://127.0.0.1:8080".to_string()),
    })));
    assert_eq!(view_cursor(&app), Point::from_usize(35, 18));
}

#[test]
fn settings_relay_edit_cursor_tracks_the_relay_draft() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Enter)));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('4'))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('r'))));
    assert_eq!(view_cursor(&app), Point::from_usize(39, 8));
}
