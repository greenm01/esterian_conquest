use ec_connect::launcher::render::render_buffer;
use ec_connect::launcher::{GateSubmit, PasswordGateMode, PasswordGateState};
use ec_ui::theme::classic;

fn first_non_space_column(line: &str) -> Option<usize> {
    line.chars().position(|ch| ch != ' ')
}

#[test]
fn existing_wallet_starts_in_unlock_mode() {
    let state = PasswordGateState::new(true, None);
    assert_eq!(state.mode, PasswordGateMode::UnlockExisting);
    assert!(!state.show_warning());
    assert_eq!(state.title(), "Unlock Wallet");
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
fn render_buffer_shows_left_aligned_warning_lines() {
    let state = PasswordGateState::new(false, None);
    let buffer = render_buffer(&state, 80, 25);

    let warning_rows: Vec<(usize, String)> = (0..buffer.height())
        .map(|row| (row, buffer.plain_line(row)))
        .filter(|(_, line)| line.contains("This password encrypts your wallet."))
        .collect();
    assert_eq!(warning_rows.len(), 1);

    let warning_row = warning_rows[0].0;
    let line1 = buffer.plain_line(warning_row);
    let line2 = buffer.plain_line(warning_row + 1);
    let line3 = buffer.plain_line(warning_row + 2);

    let col1 = first_non_space_column(&line1).unwrap();
    let col2 = first_non_space_column(&line2).unwrap();
    let col3 = first_non_space_column(&line3).unwrap();

    assert_eq!(col1, col2);
    assert_eq!(col2, col3);
    assert!(line1.contains("This password encrypts your wallet."));
    assert!(line2.contains("If you lose it, you will be locked out."));
    assert!(line3.contains("No IT support."));
}

#[test]
fn render_buffer_masks_input_without_showing_plaintext() {
    let mut state = PasswordGateState::new(true, None);
    state.input = "secret".to_string();

    let buffer = render_buffer(&state, 80, 25);
    let line = (0..buffer.height())
        .map(|row| buffer.plain_line(row))
        .find(|line| line.contains("Password:"))
        .expect("password line should exist");

    assert!(line.contains("******"));
    assert!(!line.contains("secret"));
    assert!(buffer.cursor().is_some());
}

#[test]
fn render_buffer_uses_versioned_outer_title_in_logo_style() {
    let state = PasswordGateState::new(true, None);
    let buffer = render_buffer(&state, 80, 25);
    let title = format!("EC CONNECT v{}", env!("CARGO_PKG_VERSION"));
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
    assert_eq!(col, 2);
    assert_eq!(buffer.row(row)[col].style, classic::logo_style());
}
