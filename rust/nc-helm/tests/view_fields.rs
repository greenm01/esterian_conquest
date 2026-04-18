use nc_helm::{App, BackgroundMode, BootSnapshot, Msg};

#[test]
fn first_run_input_track_uses_full_cell_backgrounds() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::BootLoaded(Ok(BootSnapshot {
        has_keychain: false,
        relay_url: Some("ws://127.0.0.1:8080".to_string()),
    })));
    let buffer = app.view();
    let row = buffer.row(19);
    assert_eq!(row[31].style.bg_mode, BackgroundMode::Cell);
    assert_eq!(row[45].style.bg_mode, BackgroundMode::Cell);
}

#[test]
fn locked_view_uses_inline_masked_password_text() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::BootLoaded(Ok(BootSnapshot {
        has_keychain: true,
        relay_url: Some("ws://127.0.0.1:8080".to_string()),
    })));
    let buffer = app.view();
    let mut found_password = false;
    let mut found_relay = false;
    for row in 0..buffer.height() {
        let line = buffer.plain_line(row);
        found_password |= line.contains("Password: ");
        found_relay |= line.contains("Relay:");
    }
    assert!(found_password, "unlock screen should render an inline masked password row");
    assert!(!found_relay, "unlock screen should not render the relay line");
}
