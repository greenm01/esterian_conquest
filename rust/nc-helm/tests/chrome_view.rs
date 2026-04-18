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
    assert_eq!(buffer.row(10)[16].ch, '╭');
    assert_eq!(buffer.row(10)[83].ch, '╮');
    assert!(buffer.plain_line(10).contains("┐ CREATE IDENTITY ┌"));
}

#[test]
fn lobby_view_uses_unicode_shell_and_panel_titles() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Enter)));

    let buffer = app.view();
    assert_eq!(buffer.row(0)[0].ch, '╭');
    assert_eq!(buffer.row(0)[99].ch, '╮');
    assert!(buffer.plain_line(0).contains("┐ nc-helm ┌"));
    assert!(buffer.plain_line(3).contains("┐ 1 HOME ┌"));
    assert_eq!(buffer.row(5)[2].ch, '╭');
    assert!(buffer.plain_line(5).contains("┐ HOME ┌"));
    assert!(buffer.plain_line(25).contains("┐ STATUS ┌"));
}

#[test]
fn help_popup_uses_left_help_tag_and_right_close_tag() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));

    let buffer = app.view();
    assert!(buffer.plain_line(12).contains("┐ HELP ┌"));
    assert!(buffer.plain_line(12).contains("┐ [X] ┌"));
}
