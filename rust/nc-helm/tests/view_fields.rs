use nc_helm::{App, BackgroundMode, BootSnapshot, Msg};

#[test]
fn first_run_input_track_uses_text_band_backgrounds() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::BootLoaded(Ok(BootSnapshot {
        has_keychain: false,
        relay_url: Some("ws://127.0.0.1:8080".to_string()),
    })));
    let buffer = app.view();
    let row = buffer.row(16);
    assert_eq!(row[31].style.bg_mode, BackgroundMode::TextBand);
    assert_eq!(row[45].style.bg_mode, BackgroundMode::TextBand);
}
