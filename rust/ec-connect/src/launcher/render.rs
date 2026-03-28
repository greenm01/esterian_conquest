use ec_ui::buffer::PlayfieldBuffer;
use ec_ui::theme::classic;

use crate::password::WALLET_WARNING_LINES;
use crate::picker::layout::{Rect, centered_rect, draw_box};
use crate::shell::{INNER_HEIGHT, INNER_WIDTH, terminal_fits_outer, wrap_inner_buffer};

use super::PasswordGateState;

pub fn render_buffer(state: &PasswordGateState, width: u16, height: u16) -> PlayfieldBuffer {
    let width = usize::from(width.max(1));
    let height = usize::from(height.max(1));

    if !terminal_fits_outer(width, height) {
        let mut buffer = PlayfieldBuffer::new(width, height, classic::body_style());
        render_tiny(&mut buffer, state);
        return buffer;
    }

    let mut buffer = PlayfieldBuffer::new(INNER_WIDTH, INNER_HEIGHT, classic::body_style());
    let outer = Rect::new(0, 2, INNER_WIDTH as u16, 21);
    let content_rows = usize::from(state.error_msg.is_some())
        + 2
        + WALLET_WARNING_LINES.len() * usize::from(state.show_warning());
    let popup_height = (content_rows + 2) as u16;
    let popup = centered_rect(
        68,
        popup_height,
        Rect::new(
            outer.x + 1,
            outer.y + 1,
            outer.width.saturating_sub(2),
            outer.height.saturating_sub(2),
        ),
    );
    draw_box(
        &mut buffer,
        popup,
        state.title(),
        classic::table_chrome_style(),
        classic::table_header_style(),
    );

    let left = popup.x as usize + 2;
    let mut row = popup.y as usize + 1;
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

    wrap_inner_buffer(&buffer)
}

fn render_tiny(buffer: &mut PlayfieldBuffer, state: &PasswordGateState) {
    let lines = [
        "ec-connect requires an 82x27 terminal.",
        state.title(),
        state.lead_line(),
        "Press Q to quit or resize the window.",
    ];
    let start_row = buffer.height().saturating_sub(lines.len()) / 2;
    for (idx, line) in lines.iter().enumerate() {
        let row = start_row + idx;
        let col = buffer.width().saturating_sub(line.chars().count()) / 2;
        let style = if idx == 0 {
            classic::table_header_style()
        } else {
            classic::table_body_style()
        };
        buffer.write_text_clipped(row, col, line, style);
    }
}
