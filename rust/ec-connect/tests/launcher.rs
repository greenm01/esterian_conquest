use ec_connect::launcher::render::{render_buffer, render_inner_buffer};
use ec_connect::launcher::{GateSubmit, PasswordGateMode, PasswordGateState};
use ec_connect::picker::layout::{Rect, centered_rect};
use ec_ui::theme::classic;

fn first_non_space_column(line: &str) -> Option<usize> {
    line.chars().position(|ch| ch != ' ')
}

#[test]
fn existing_wallet_starts_in_unlock_mode() {
    let state = PasswordGateState::new(true, None);
    assert_eq!(state.mode, PasswordGateMode::UnlockExisting);
    assert_eq!(state.title(), "Unlock Wallet");
    assert_eq!(state.copy_lines(), &["Enter your wallet password."]);
}

#[test]
fn create_password_advances_to_confirm_mode() {
    let mut state = PasswordGateState::new(false, None);
    state.push_char('s');
    state.push_char('e');
    state.push_char('c');

    assert_eq!(state.submit(), GateSubmit::Pending);
    assert_eq!(state.mode, PasswordGateMode::ConfirmNew);
    assert_eq!(state.staged_password, "sec");
    assert!(state.input.is_empty());
}

#[test]
fn mismatched_confirmation_resets_to_create_mode() {
    let mut state = PasswordGateState::new(false, None);
    state.input = "hunter2".to_string();
    let _ = state.submit();
    state.input = "hunter3".to_string();

    assert_eq!(state.submit(), GateSubmit::Pending);
    assert_eq!(state.mode, PasswordGateMode::CreateNew);
    assert!(state.staged_password.is_empty());
    assert_eq!(
        state.error_msg.as_deref(),
        Some("Error: passwords do not match. Start over.")
    );
}

#[test]
fn unlock_mode_accepts_non_empty_password() {
    let mut state = PasswordGateState::new(true, None);
    state.input = "griffith".to_string();

    assert_eq!(state.submit(), GateSubmit::Accepted("griffith".to_string()));
}

#[test]
fn render_buffer_shows_left_aligned_create_copy() {
    let state = PasswordGateState::new(false, None);
    let buffer = render_buffer(&state, 82, 27);

    let copy_rows: Vec<(usize, String)> = (0..buffer.height())
        .map(|row| (row, buffer.plain_line(row)))
        .filter(|(_, line)| line.contains("This password encrypts your wallet."))
        .collect();
    assert_eq!(copy_rows.len(), 1);

    let copy_row = copy_rows[0].0;
    let line1 = buffer.plain_line(copy_row);
    let line2 = buffer.plain_line(copy_row + 1);

    let col1 = first_non_space_column(&line1).unwrap();
    let col2 = first_non_space_column(&line2).unwrap();

    assert_eq!(col1, col2);
    assert!(line1.contains("This password encrypts your wallet."));
    assert!(line2.contains("If you lose it, you will lose your game identity."));
    assert!(!(0..buffer.height()).any(|row| buffer.plain_line(row).contains("No IT support.")));
}

#[test]
fn render_buffer_places_logo_above_password_copy() {
    let state = PasswordGateState::new(false, None);
    let buffer = render_buffer(&state, 82, 27);

    let logo_row = (0..buffer.height())
        .find(|&row| {
            buffer
                .plain_line(row)
                .contains("____ ___  _   _  ___  _   _ _____ ____ _____")
        })
        .expect("logo row should exist");
    let copy_row = (0..buffer.height())
        .find(|&row| {
            buffer
                .plain_line(row)
                .contains("This password encrypts your wallet.")
        })
        .expect("copy row should exist");

    assert!(logo_row < copy_row);
}

#[test]
fn render_buffer_unlock_mode_uses_single_reassuring_line() {
    let state = PasswordGateState::new(true, None);
    let buffer = render_buffer(&state, 82, 27);

    assert!((0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains("Enter your wallet password.")
    }));
    assert!(!(0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains("If you lose it, you will lose your game identity.")
    }));
}

#[test]
fn render_buffer_confirm_mode_uses_confirmation_copy() {
    let mut state = PasswordGateState::new(false, None);
    state.mode = PasswordGateMode::ConfirmNew;
    state.staged_password = "hunter2".to_string();
    let buffer = render_buffer(&state, 82, 27);

    assert!((0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains("Enter the password again to confirm it.")
    }));
    assert!((0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains("If you lose it, you will lose your game identity.")
    }));
}

#[test]
fn render_buffer_masks_input_without_showing_plaintext() {
    let mut state = PasswordGateState::new(true, None);
    state.input = "secret".to_string();

    let buffer = render_buffer(&state, 82, 27);
    let line = (0..buffer.height())
        .map(|row| buffer.plain_line(row))
        .find(|line| line.contains("Password:"))
        .expect("password line should exist");

    assert!(line.contains("******"));
    assert!(!line.contains("secret"));
    assert!(buffer.cursor().is_some());
}

#[test]
fn render_buffer_fills_popup_interior_with_body_style() {
    let state = PasswordGateState::new(true, None);
    let buffer = render_buffer(&state, 82, 27);
    let popup = centered_rect(68, 17, Rect::new(1, 2, 78, 21));
    let interior_cell = &buffer.row(popup.y as usize + 2)[popup.x as usize + 2];

    assert_eq!(interior_cell.ch, ' ');
    assert_eq!(interior_cell.style, classic::table_body_style());
}

#[test]
fn render_buffer_uses_versioned_outer_title_in_shell_title_style() {
    let state = PasswordGateState::new(true, None);
    let buffer = render_buffer(&state, 82, 27);
    let title = format!("NC CONNECT v{}", env!("CARGO_PKG_VERSION"));
    let row = (0..buffer.height())
        .find(|&idx| buffer.plain_line(idx).contains(&title))
        .expect("outer title row");
    let line = buffer
        .row(row)
        .iter()
        .map(|cell| cell.ch)
        .collect::<String>();
    let byte_idx = line.find(&title).expect("title column");
    let col = line[..byte_idx].chars().count();
    assert_eq!(col, 3);
    assert_eq!(buffer.row(row)[col].style, classic::shell_title_style());
    assert_eq!(classic::shell_title_style().bg, classic::body_style().bg);
}

#[test]
fn render_inner_buffer_uses_plain_80x25_canvas_without_outer_shell_title() {
    let state = PasswordGateState::new(true, None);
    let buffer = render_inner_buffer(&state);
    let title = format!("NC CONNECT v{}", env!("CARGO_PKG_VERSION"));

    assert_eq!(buffer.width(), 80);
    assert_eq!(buffer.height(), 25);
    assert!(!(0..buffer.height()).any(|row| buffer.plain_line(row).contains(&title)));
    assert_ne!(buffer.row(0)[0].ch, '┌');
}

#[test]
fn render_buffer_omits_password_footer_command_line() {
    let state = PasswordGateState::new(true, None);
    let buffer = render_buffer(&state, 82, 27);

    assert!(!(0..buffer.height()).any(|row| { buffer.plain_line(row).contains("COMMANDS <-") }));
}

#[test]
fn password_cursor_sits_one_space_after_label() {
    let mut state = PasswordGateState::new(true, None);
    state.input = "secret".to_string();
    let buffer = render_buffer(&state, 82, 27);
    assert!(
        (0..buffer.height()).any(|row| { buffer.plain_line(row).contains("Password: ******") })
    );
}

#[test]
fn launcher_wraps_long_error_messages() {
    let state = PasswordGateState::new(
        true,
        Some(
            "Error: this is a deliberately long launcher error message that should wrap across multiple rows."
                .to_string(),
        ),
    );
    let buffer = render_inner_buffer(&state);

    assert!((0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains("Error: this is a deliberately long launcher error")
    }));
    assert!((0..buffer.height()).any(|row| {
        let line = buffer.plain_line(row);
        line.contains("message that should wrap across") || line.contains("multiple rows.")
    }));
}
