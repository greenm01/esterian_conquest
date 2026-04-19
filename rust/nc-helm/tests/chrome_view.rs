mod support;

use nc_client::cache::ClientCache;
use nc_helm::{App, BootSnapshot, LobbySnapshot, Msg, ScreenGeometry};

use crate::support::{alt_key, dummy_session, key, my_game_row, sandbox_open_game_row};

#[test]
fn first_run_view_uses_unicode_centered_box_chrome() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::BootLoaded(Ok(BootSnapshot {
        has_keychain: false,
        relay_url: Some("ws://127.0.0.1:8080".to_string()),
        lock_timeout_minutes: 10,
    })));

    let buffer = app.view();
    assert_eq!(buffer.row(7)[16].ch, '╭');
    assert_eq!(buffer.row(7)[83].ch, '╮');
    assert!(buffer.plain_line(7).contains("┐ CREATE IDENTITY ┌"));
}

#[test]
fn lobby_view_uses_unicode_shell_and_panel_titles() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Esc)));

    let buffer = app.view();
    let header = buffer.plain_line(0);
    let tab_strip = "[My Games] [Open Games] [Comms] [Settings]";
    assert_eq!(buffer.row(0)[0].ch, ' ');
    assert!(buffer.has_overlay_text("Nostrian Conquest"));
    assert!(header.contains("<v"));
    assert!(!header.contains("beta"));
    assert!(!header.contains("Nostrian Conquest"));
    assert_eq!(
        header.find("captain"),
        Some((buffer.width() - "captain".len()) / 2)
    );
    assert!(header.trim_end().ends_with("NETWORK: CONNECTING"));
    assert!(buffer.plain_line(2).contains(tab_strip));
    assert_eq!(
        buffer.plain_line(2).find(tab_strip),
        Some((buffer.width() - tab_strip.chars().count()) / 2)
    );
    assert_eq!(buffer.row(4)[1].ch, '╭');
    assert!(buffer.plain_line(4).contains("┐ MY GAMES ┌"));
    assert!(buffer.plain_line(33).contains("┐ COMMANDS ┌"));
    assert!(buffer.plain_line(34).contains("Alt+ Q>uit"));
    assert!(buffer.plain_line(34).contains("R>efresh"));
    assert!(buffer.plain_line(34).contains("D>elete"));
    assert!(buffer.plain_line(34).contains("<?>Keys H>ints"));
    assert!(!buffer.plain_line(26).contains("STATUS"));
}

#[test]
fn command_panel_highlights_hotkeys() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Esc)));

    let buffer = app.view();
    let line = buffer.plain_line(34);
    let q_offset = line.find("Q>uit").expect("command rail should render");
    let base_offset = line
        .find("Alt+ ")
        .expect("command rail prefix should render");
    let q_col = line[..q_offset].chars().count();
    let a_col = line[..base_offset].chars().count();
    assert_ne!(
        buffer.row(34)[a_col].style.fg,
        buffer.row(34)[q_col].style.fg
    );
    assert_eq!(
        buffer.row(34)[a_col].style.bg,
        buffer.row(34)[q_col].style.bg
    );
}

#[test]
fn help_popup_uses_left_help_tag_and_right_close_tag() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('?'))));

    let buffer = app.view();
    assert!(buffer.plain_line(11).contains("┐ HELP ┌"));
    assert!(buffer.plain_line(11).contains("┐ [X] ┌"));
}

#[test]
fn sandbox_delete_confirm_popup_renders_copy() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::LobbyUpdated(Ok(LobbySnapshot {
        cache: ClientCache::empty(),
        my_games: vec![my_game_row("joined")],
        open_games: vec![sandbox_open_game_row()],
        notices: Vec::new(),
    })));
    let _ = app.dispatch(Msg::Key(alt_key(nc_helm::KeyCode::Char('d'))));

    let buffer = app.view();
    let lines = (0..buffer.height())
        .map(|row| buffer.plain_line(row))
        .collect::<Vec<_>>();
    assert!(lines.iter().any(|line| line.contains("DELETE SANDBOX")));
    assert!(
        lines
            .iter()
            .any(|line| line.contains("Release this sandbox seat"))
    );
}

#[test]
fn matrix_locked_route_renders_rain_without_lock_panel_copy() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Esc)));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('l'))));

    let buffer = app.view();
    let allowed = "ΑΒΓΔΕΖΗΘΙΚΛΜΝΞΟΠΡΣΤΥΦΧΨΩ+#%*";
    let glyph = (0..buffer.height())
        .find_map(|row| {
            buffer
                .plain_line(row)
                .chars()
                .find(|ch| allowed.contains(*ch))
        })
        .expect("matrix rain glyph");

    assert!(allowed.contains(glyph));
    for row in 0..buffer.height() {
        let line = buffer.plain_line(row);
        assert!(!line.contains("SESSION LOCKED"));
        assert!(!line.contains("Matrix lock is active."));
    }
}

#[test]
fn undersized_lobby_view_falls_back_instead_of_panicking() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Resize(ScreenGeometry::new(10, 5)));

    let buffer = app.view();
    assert!(buffer.plain_line(0).contains("Window"));
    assert!(buffer.plain_line(1).contains("Minimum"));
}

#[test]
fn undersized_first_run_view_falls_back_instead_of_panicking() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::BootLoaded(Ok(BootSnapshot {
        has_keychain: false,
        relay_url: Some("ws://127.0.0.1:8080".to_string()),
        lock_timeout_minutes: 10,
    })));
    let _ = app.dispatch(Msg::Resize(ScreenGeometry::new(10, 5)));

    let buffer = app.view();
    assert!(buffer.plain_line(0).contains("Window"));
}

#[test]
fn undersized_locked_view_falls_back_instead_of_panicking() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::BootLoaded(Ok(BootSnapshot {
        has_keychain: true,
        relay_url: Some("ws://127.0.0.1:8080".to_string()),
        lock_timeout_minutes: 10,
    })));
    let _ = app.dispatch(Msg::Resize(ScreenGeometry::new(10, 5)));

    let buffer = app.view();
    assert!(buffer.plain_line(0).contains("Window"));
}
