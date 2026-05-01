mod support;

use nc_client::cache::ClientCache;
use nc_helm::{App, BootSnapshot, LobbySnapshot, Msg, ScreenGeometry};

use crate::support::{alt_key, dummy_session, key, my_game_row, sandbox_open_game_row};

fn find_line(buffer: &nc_helm::PlayfieldBuffer, needle: &str) -> usize {
    (0..buffer.height())
        .find(|&row| buffer.plain_line(row).contains(needle))
        .unwrap_or_else(|| panic!("expected line containing {needle:?}"))
}

fn modal_bounds(
    buffer: &nc_helm::PlayfieldBuffer,
    title_needle: &str,
) -> (usize, usize, usize, usize) {
    let top = find_line(buffer, title_needle);
    let line = buffer.plain_line(top);
    let chars = line.chars().collect::<Vec<_>>();
    let title_byte = line.find(title_needle).expect("modal title");
    let title_col = line[..title_byte].chars().count();
    let left = chars[..=title_col]
        .iter()
        .rposition(|ch| *ch == '┌')
        .expect("modal left border");
    let right = chars
        .iter()
        .rposition(|ch| *ch == '┐')
        .expect("modal right border");
    let bottom = (top + 1..buffer.height())
        .find(|&row| buffer.row(row)[left].ch == '└' && buffer.row(row)[right].ch == '┘')
        .expect("modal bottom row");
    assert_eq!(buffer.row(bottom)[right].ch, '┘');
    (top, left, right, bottom)
}

fn assert_modal_has_outer_padding(buffer: &nc_helm::PlayfieldBuffer, title_needle: &str) {
    let (top, left, right, bottom) = modal_bounds(buffer, title_needle);
    let body_style = buffer.row(0)[0].style;
    let interior_style = buffer.row(top + 1)[left + 1].style;

    assert_ne!(interior_style, body_style);
    assert_eq!(buffer.row(top - 1)[left + 1].ch, ' ');
    assert_eq!(buffer.row(top - 1)[left + 1].style, interior_style);
    assert_eq!(buffer.row(bottom + 1)[left + 1].ch, ' ');
    assert_eq!(buffer.row(bottom + 1)[left + 1].style, interior_style);
    assert_eq!(buffer.row(top + 1)[left - 1].ch, ' ');
    assert_eq!(buffer.row(top + 1)[left - 1].style, interior_style);
    assert_eq!(buffer.row(top + 1)[right + 1].ch, ' ');
    assert_eq!(buffer.row(top + 1)[right + 1].style, interior_style);
}

#[test]
fn first_run_view_uses_unicode_centered_box_chrome() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::BootLoaded(Ok(BootSnapshot {
        has_keychain: false,
        relay_url: Some("ws://127.0.0.1:8080".to_string()),
        lock_timeout_minutes: 10,
    })));

    let buffer = app.view();
    assert_eq!(buffer.row(7)[16].ch, '┌');
    assert_eq!(buffer.row(7)[83].ch, '┐');
    assert!(buffer.plain_line(7).contains("┐CREATE IDENTITY┌"));
    assert_modal_has_outer_padding(&buffer, "┐CREATE IDENTITY┌");
}

#[test]
fn lobby_view_uses_unicode_shell_and_panel_titles() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));

    let buffer = app.view();
    let header = buffer.plain_line(0);
    let tab_strip = "[My Games] [Open Games] [Comms] [Settings]";
    assert_eq!(buffer.row(0)[0].ch, ' ');
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
    let my_games_row = find_line(&buffer, "┐MY GAMES┌");
    let my_games_border = buffer
        .plain_line(my_games_row)
        .find('┌')
        .expect("my games border");
    assert!(my_games_border > 1);
    assert!(my_games_row > 4);
    assert!(buffer.plain_line(33).contains("┐COMMANDS┌"));
    assert!(buffer.plain_line(34).contains("Alt+ Q>uit"));
    assert!(buffer.plain_line(34).contains("R>efresh"));
    assert!(buffer.plain_line(34).contains("D>elete"));
    assert!(buffer.plain_line(34).contains("<?>Keys H>ints"));
    assert!(!buffer.plain_line(26).contains("STATUS"));
}

#[test]
fn comms_panel_still_uses_full_width_shell() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('c'))));

    let buffer = app.view();
    assert_eq!(buffer.row(4)[1].ch, '┌');
    assert!(buffer.plain_line(4).contains("┐COMMS┌"));
}

#[test]
fn open_games_panel_centers_when_the_list_is_short() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('o'))));

    let buffer = app.view();
    let open_games_row = find_line(&buffer, "┐OPEN GAMES AVAILABLE TO JOIN┌");

    assert!(open_games_row > 4);
}

#[test]
fn settings_panel_shrinkwraps_and_wraps_inside_the_box() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Resize(ScreenGeometry::new(68, 36)));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('s'))));

    let buffer = app.view();
    let settings_row = find_line(&buffer, "┐SETTINGS┌");
    let right_border = buffer
        .row(settings_row)
        .iter()
        .rposition(|cell| cell.ch == '┐')
        .expect("settings panel right border");
    let edit_row = find_line(&buffer, "Edit relay URL");
    let cancel_row = find_line(&buffer, "Cancel edit");

    assert!(settings_row > 4);
    assert_eq!(buffer.row(edit_row)[right_border].ch, '│');
    assert_eq!(buffer.row(cancel_row)[right_border].ch, '│');
}

#[test]
fn command_panel_highlights_hotkeys() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));

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
    assert!(buffer.plain_line(11).contains("┐HELP┌"));
    assert!(buffer.plain_line(11).contains("┐[X]┌"));
    assert_modal_has_outer_padding(&buffer, "┐HELP┌");
}

#[test]
fn help_popup_shrinkwraps_to_last_help_line() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Char('?'))));

    let buffer = app.view();
    let (_, left, right, bottom) = modal_bounds(&buffer, "┐HELP┌");
    let last_help_row = find_line(&buffer, "Background sync still runs");

    assert_eq!(last_help_row, bottom - 1);
    assert!(right.saturating_sub(left).saturating_add(1) < 60);
}

#[test]
fn locked_view_uses_outer_padding_around_unlock_gate() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::BootLoaded(Ok(BootSnapshot {
        has_keychain: true,
        relay_url: Some("ws://127.0.0.1:8080".to_string()),
        lock_timeout_minutes: 10,
    })));

    let buffer = app.view();
    assert_modal_has_outer_padding(&buffer, "┐UNLOCK KEYCHAIN┌");
}

#[test]
fn locked_view_keeps_password_and_quit_copy_inside_the_taller_gate() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::BootLoaded(Ok(BootSnapshot {
        has_keychain: true,
        relay_url: Some("ws://127.0.0.1:8080".to_string()),
        lock_timeout_minutes: 10,
    })));

    let buffer = app.view();
    let (top, left, right, bottom) = modal_bounds(&buffer, "┐UNLOCK KEYCHAIN┌");
    let password_row = find_line(buffer, "Password:");
    let quit_row = find_line(buffer, "Press Esc to quit.");

    assert_eq!(password_row, top + 11);
    assert_eq!(quit_row, password_row + 1);
    assert_eq!(buffer.row(password_row)[left].ch, '│');
    assert_eq!(buffer.row(password_row)[right].ch, '│');
    assert_eq!(buffer.row(quit_row)[left].ch, '│');
    assert_eq!(buffer.row(quit_row)[right].ch, '│');
    assert!(quit_row < bottom);
}

#[test]
fn lobby_quit_popup_uses_outer_padding() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(key(nc_helm::KeyCode::Esc)));

    let buffer = app.view();
    assert_modal_has_outer_padding(&buffer, "┐QUIT┌");
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
    let (_, _, _, bottom) = modal_bounds(&buffer, "DELETE SANDBOX");
    assert_eq!(find_line(&buffer, "Any other key cancels."), bottom - 1);
}

#[test]
fn matrix_locked_route_renders_rain_without_lock_panel_copy() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(alt_key(nc_helm::KeyCode::Char('l'))));

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
fn matrix_locked_view_bypasses_the_simple_route_cache() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(alt_key(nc_helm::KeyCode::Char('l'))));

    let (first_hit, _) = app.view_with_cache_hit();
    assert!(!first_hit);

    let _ = app.dispatch(Msg::MatrixFrame);
    let (second_hit, _) = app.view_with_cache_hit();
    assert!(!second_hit);
}

#[test]
fn matrix_locked_view_changes_after_matrix_frames() {
    let (mut app, _) = App::new(None);
    let _ = app.dispatch(Msg::Unlocked(Ok(dummy_session("captain"))));
    let _ = app.dispatch(Msg::Key(alt_key(nc_helm::KeyCode::Char('l'))));

    let before = {
        let buffer = app.view();
        (0..buffer.height())
            .map(|row| buffer.plain_line(row))
            .collect::<Vec<_>>()
    };

    for _ in 0..8 {
        let _ = app.dispatch(Msg::MatrixFrame);
    }

    let after = {
        let buffer = app.view();
        (0..buffer.height())
            .map(|row| buffer.plain_line(row))
            .collect::<Vec<_>>()
    };

    assert_ne!(before, after);
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
