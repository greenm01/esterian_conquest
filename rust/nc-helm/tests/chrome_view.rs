mod support;

use nc_helm::{App, BootSnapshot, Msg};

use crate::support::{dummy_session, key};

#[test]
fn first_run_view_uses_unicode_centered_box_chrome() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::BootLoaded(Ok(BootSnapshot {
        has_keychain: false,
        relay_url: Some("ws://127.0.0.1:8080".to_string()),
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
    assert!(header.contains("Nostrian Conquest <v"));
    assert!(!header.contains("beta"));
    assert_eq!(header.find("Nostrian Conquest <v"), Some(1));
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

    let buffer = app.view();
    assert!(buffer.plain_line(12).contains("┐ HELP ┌"));
    assert!(buffer.plain_line(12).contains("┐ [X] ┌"));
}
