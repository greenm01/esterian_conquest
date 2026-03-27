use ec_ui::buffer::{PlayfieldBuffer, StyledSpan};
use ec_ui::theme::classic;

use crate::password::WALLET_WARNING_LINES;
use crate::picker::render::{Rect, centered_rect, draw_box, draw_title};

use super::PasswordGateState;

pub fn render_buffer(state: &PasswordGateState, width: u16, height: u16) -> PlayfieldBuffer {
    let width = width.max(1) as usize;
    let height = height.max(1) as usize;
    let mut buffer = PlayfieldBuffer::new(width, height, classic::body_style());

    if width < 48 || height < 10 {
        render_tiny(&mut buffer, state);
        return buffer;
    }

    draw_title(&mut buffer, "ESTERIAN CONQUEST  CONNECT");
    let popup_height = if state.show_warning() { 11 } else { 8 };
    let popup = centered_rect(
        68,
        popup_height,
        Rect::new(0, 0, width as u16, height as u16),
    );
    draw_box(
        &mut buffer,
        popup,
        state.title(),
        classic::table_chrome_style(),
        classic::table_header_style(),
    );

    let left = popup.x as usize + 2;
    let mut row = popup.y as usize + 2;
    if let Some(msg) = state.error_msg.as_deref() {
        buffer.write_text_clipped(row, left, msg, classic::error_style());
        row += 1;
    }

    buffer.write_text_clipped(row, left, state.lead_line(), classic::table_body_style());
    row += 1;

    if state.show_warning() {
        for line in WALLET_WARNING_LINES {
            buffer.write_text_clipped(row, left, line, classic::alert_style());
            row += 1;
        }
    }

    row += 1;
    buffer.write_text_clipped(
        row,
        left,
        state.field_label(),
        classic::status_label_style(),
    );
    let label_width = state.field_label().chars().count() + 1;
    let cursor_col = left
        + label_width
        + buffer.write_text_clipped(
            row,
            left + label_width,
            &state.masked_input(),
            classic::prompt_hotkey_style(),
        );
    if cursor_col < buffer.width() {
        buffer.set_cursor(cursor_col as u16, row as u16);
    }

    draw_footer(&mut buffer, popup, "<Q>");
    buffer
}

fn render_tiny(buffer: &mut PlayfieldBuffer, state: &PasswordGateState) {
    buffer.write_text_clipped(0, 0, "ec-connect", classic::title_style());
    let mut row = 1;
    if row < buffer.height() {
        buffer.write_text_clipped(row, 0, state.title(), classic::table_header_style());
        row += 1;
    }
    if row < buffer.height() {
        buffer.write_text_clipped(row, 0, state.lead_line(), classic::table_body_style());
        row += 1;
    }
    if let Some(msg) = state.error_msg.as_deref().filter(|_| row < buffer.height()) {
        buffer.write_text_clipped(row, 0, msg, classic::error_style());
        row += 1;
    }
    if state.show_warning() {
        for line in WALLET_WARNING_LINES {
            if row >= buffer.height() {
                break;
            }
            buffer.write_text_clipped(row, 0, line, classic::alert_style());
            row += 1;
        }
    }
}

fn draw_footer(buffer: &mut PlayfieldBuffer, rect: Rect, hotkey: &str) {
    let row = rect.y as usize + rect.height as usize - 2;
    let col = rect.x as usize + 2;
    buffer.write_spans(
        row,
        col,
        &[
            StyledSpan::new("COMMANDS", classic::title_style()),
            StyledSpan::new(" <- ", classic::prompt_style()),
            StyledSpan::new("<", classic::prompt_angle_delimiter_style()),
            StyledSpan::new(
                hotkey.trim_matches(['<', '>']),
                classic::prompt_hotkey_style(),
            ),
            StyledSpan::new(">", classic::prompt_angle_delimiter_style()),
            StyledSpan::new(" ->", classic::prompt_style()),
        ],
    );
}
