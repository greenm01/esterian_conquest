use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ec_game::screen::{FirstTimeMenuScreen, PlayfieldBuffer, ScreenGeometry};
use ec_game::terminal::door::{
    decode_fragmented_input_for_test, decode_input_bytes_for_test, decode_input_stream_for_test,
    serialize_playfield_frame,
};
use ec_game::terminal::{ColorMode, OutputEncoding};

fn apply_mag16_theme() {
    let theme_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config/themes/mag16.kdl");
    ec_game::theme::load_theme_from_path(&theme_path).expect("load mag16 theme");
}

#[test]
fn door_serializer_renders_first_time_menu_rows_and_prompt_cursor() {
    apply_mag16_theme();
    let mut screen = FirstTimeMenuScreen::new();
    let buffer = screen.render(None).expect("first-time menu renders");
    let frame = serialize_playfield_frame(
        &buffer,
        ScreenGeometry::local_default(),
        OutputEncoding::Cp437,
        ColorMode::Ansi16,
    );
    let frame_text = String::from_utf8_lossy(&frame);

    assert!(frame_text.contains("\x1b[1;1H\x1b[0;94;40;1mFIRST TIME MENU:"));
    assert!(frame_text.contains("\x1b[2;1H"));
    assert!(frame_text.contains("elp with commands"));
    assert!(frame_text.contains("\x1b[3;1H"));
    assert!(frame_text.contains("uit back to BBS"));
    assert!(frame_text.contains("\x1b[5;1H"));
    assert!(frame_text.contains("FIRST TIME COMMAND"));

    let (cursor_col, cursor_row) = buffer.cursor().expect("cursor set");
    let final_cursor = format!("\x1b[{};{}H", cursor_row + 1, cursor_col + 1);
    assert!(frame_text.ends_with(&final_cursor));
}

#[test]
fn door_serializer_emits_classic_ansi16_colors_for_mag16_theme() {
    apply_mag16_theme();
    let mut screen = FirstTimeMenuScreen::new();
    let buffer = screen.render(None).expect("first-time menu renders");
    let frame = serialize_playfield_frame(
        &buffer,
        ScreenGeometry::local_default(),
        OutputEncoding::Cp437,
        ColorMode::Ansi16,
    );
    let frame_text = String::from_utf8_lossy(&frame);

    assert!(frame_text.contains("\x1b[0;94;40;1m"));
    assert!(frame_text.contains("\x1b[0;33;40;1m"));
}

#[test]
fn door_serializer_avoids_alt_screen_and_hides_no_cursor() {
    apply_mag16_theme();
    let mut screen = FirstTimeMenuScreen::new();
    let buffer = screen.render(None).expect("first-time menu renders");
    let frame = serialize_playfield_frame(
        &buffer,
        ScreenGeometry::local_default(),
        OutputEncoding::Cp437,
        ColorMode::Ansi16,
    );
    let frame_text = String::from_utf8_lossy(&frame);

    assert!(!frame_text.contains("\x1b[?1049"));
    assert!(!frame_text.contains("\x1b[?25l"));
    assert!(frame_text.contains("\x1b[?25h"));
}

#[test]
fn door_serializer_trims_blank_rows_and_trailing_fill_spaces() {
    apply_mag16_theme();
    let mut buffer = PlayfieldBuffer::new(80, 25, ec_game::theme::classic::body_style());
    buffer.write_text(0, 0, "ABC", ec_game::theme::classic::title_style());
    buffer.set_cursor(2, 0);

    let frame = serialize_playfield_frame(
        &buffer,
        ScreenGeometry::local_default(),
        OutputEncoding::Cp437,
        ColorMode::Ansi16,
    );
    let frame_text = String::from_utf8_lossy(&frame);

    assert!(frame_text.contains("ABC"));
    assert!(!frame_text.contains("ABC "));
    assert_eq!(frame.iter().filter(|byte| **byte == b' ').count(), 0);
    assert!(!frame_text.contains("\x1b[2;1H\x1b[0;37;40m"));
}

fn assert_decode(bytes: &[u8], expected: KeyEvent) {
    let got = decode_input_bytes_for_test(bytes).expect("decode input");
    assert_eq!(got, expected, "bytes: {bytes:?}");
}

#[test]
fn door_input_decoder_maps_ascii_and_control_keys() {
    assert_decode(b"j", KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    assert_decode(b"\r", KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_decode(b"\t", KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_decode(&[0x08], KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
    assert_decode(&[0x7f], KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
    assert_decode(&[0x03], KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert_decode(&[0x05], KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL));
    assert_decode(&[0x18], KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL));
    assert_decode(&[0x1b], KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
}

#[test]
fn door_input_decoder_maps_ansi_navigation_sequences() {
    assert_decode(b"\x1b[A", KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_decode(b"\x1b[B", KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_decode(b"\x1b[C", KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    assert_decode(b"\x1b[D", KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
    assert_decode(b"\x1bOA", KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_decode(b"\x1bOB", KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_decode(b"\x1bOC", KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    assert_decode(b"\x1bOD", KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
    assert_decode(
        b"\x1b[1;2A",
        KeyEvent::new(KeyCode::Up, KeyModifiers::NONE),
    );
    assert_decode(b"\x1b[H", KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
    assert_decode(b"\x1b[F", KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
    assert_decode(b"\x1bOH", KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
    assert_decode(b"\x1bOF", KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
    assert_decode(b"\x1b[3~", KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
    assert_decode(b"\x1b[5~", KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE));
    assert_decode(
        b"\x1b[6~",
        KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE),
    );
    assert_decode(&[0xe0, b'H'], KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_decode(&[0xe0, b'P'], KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_decode(&[0xe0, b'M'], KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    assert_decode(&[0xe0, b'K'], KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
}

#[test]
fn door_input_decoder_keeps_fragmented_arrow_sequences_as_arrows() {
    let got = decode_fragmented_input_for_test(b"\x1b", b"[A").expect("decode fragmented");
    assert_eq!(got, KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));

    let got = decode_fragmented_input_for_test(b"\x1b[", b"A").expect("decode fragmented");
    assert_eq!(got, KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));

    let got = decode_fragmented_input_for_test(b"\x1b[1;", b"2D").expect("decode fragmented");
    assert_eq!(got, KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
}

#[test]
fn door_input_decoder_handles_back_to_back_csi_arrows_without_literal_tail_text() {
    let got = decode_input_stream_for_test(b"\x1b[B\x1b[D").expect("decode stream");
    assert_eq!(
        got,
        vec![
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
        ]
    );
}

#[test]
fn incomplete_known_escape_prefix_does_not_fall_back_to_escape() {
    assert_decode(b"\x1b[", KeyEvent::new(KeyCode::Null, KeyModifiers::NONE));
    assert_decode(b"\x1bO", KeyEvent::new(KeyCode::Null, KeyModifiers::NONE));
}

#[test]
fn door_serializer_uses_bbs_safe_selected_row_cyan_background() {
    apply_mag16_theme();
    let mut buffer = PlayfieldBuffer::new(80, 25, ec_game::theme::classic::body_style());
    buffer.write_text(5, 1, "01", ec_game::theme::classic::selected_row_style());

    let frame = serialize_playfield_frame(
        &buffer,
        ScreenGeometry::local_default(),
        OutputEncoding::Cp437,
        ColorMode::Ansi16,
    );
    let frame_text = String::from_utf8_lossy(&frame);

    assert!(frame_text.contains("\x1b[0;97;46m01"));
    assert!(!frame_text.contains("\x1b[0;97;106m"));
}
