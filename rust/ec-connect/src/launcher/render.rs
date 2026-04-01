use ec_ui::branding::NOSTRIAN_CONQUEST_LOGO;
use ec_ui::buffer::PlayfieldBuffer;
use ec_ui::theme::classic;

use crate::input_field::{draw_labeled_input_row, input_width};
use crate::picker::layout::{Rect, centered_rect, draw_box};
use crate::shell::{INNER_HEIGHT, INNER_WIDTH, terminal_fits_outer, wrap_inner_buffer_in_terminal};
use crate::text_wrap::{wrapped_lines, write_wrapped_lines_clamped};

use super::PasswordGateState;

const POPUP_WIDTH: u16 = 68;

pub fn render_inner_buffer(state: &PasswordGateState) -> PlayfieldBuffer {
    let mut buffer = PlayfieldBuffer::new(INNER_WIDTH, INNER_HEIGHT, classic::body_style());
    let outer = Rect::new(0, 1, INNER_WIDTH as u16, 23);
    let error_lines = state
        .error_msg
        .as_deref()
        .map(|msg| wrapped_lines(msg, 64).len())
        .unwrap_or(0);
    let fixed_rows = NOSTRIAN_CONQUEST_LOGO.len() + 1 + state.copy_lines().len() + 1 + 1;
    let popup_height = (error_lines + usize::from(error_lines > 0) + fixed_rows + 2)
        .min(outer.height.saturating_sub(2) as usize) as u16;
    let popup = centered_rect(
        POPUP_WIDTH,
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
    buffer.fill_rect(
        popup.y as usize + 1,
        popup.x as usize + 1,
        popup.width.saturating_sub(2) as usize,
        popup.height.saturating_sub(2) as usize,
        classic::table_body_style(),
    );

    let left = popup.x as usize + 2;
    let max_error_rows = popup
        .height
        .saturating_sub(2)
        .saturating_sub(fixed_rows as u16) as usize;
    let mut row = popup.y as usize + 1;
    if let Some(msg) = state.error_msg.as_deref() {
        row += write_wrapped_lines_clamped(
            &mut buffer,
            row,
            left,
            64,
            max_error_rows,
            msg,
            classic::error_style(),
        );
        row += 1;
    }

    row += draw_logo(&mut buffer, row, popup);
    row += 1;

    for line in state.copy_lines() {
        buffer.write_text_clipped(row, left, line, classic::table_body_style());
        row += 1;
    }
    row += 1;

    let label = state.field_label();
    let input_col = left + label.chars().count() + 1;
    let inner_right = popup.x as usize + popup.width as usize - 2;
    draw_labeled_input_row(
        &mut buffer,
        row,
        left,
        label,
        &state.masked_input(),
        input_width(inner_right, input_col),
        classic::status_label_style(),
        classic::prompt_hotkey_style(),
    );

    buffer
}

pub fn render_buffer(state: &PasswordGateState, width: u16, height: u16) -> PlayfieldBuffer {
    let width = usize::from(width.max(1));
    let height = usize::from(height.max(1));

    if !terminal_fits_outer(width, height) {
        let mut buffer = PlayfieldBuffer::new(width, height, classic::body_style());
        render_tiny(&mut buffer, state);
        return buffer;
    }

    let buffer = render_inner_buffer(state);
    wrap_inner_buffer_in_terminal(&buffer, None, width, height, None)
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

fn draw_logo(buffer: &mut PlayfieldBuffer, start_row: usize, popup: Rect) -> usize {
    let logo_width = NOSTRIAN_CONQUEST_LOGO
        .iter()
        .map(|line| line.len())
        .max()
        .unwrap_or(0);
    let inner_left = popup.x as usize + 1;
    let inner_width = popup.width.saturating_sub(2) as usize;
    let logo_left = inner_left + inner_width.saturating_sub(logo_width) / 2;

    for (row_offset, line) in NOSTRIAN_CONQUEST_LOGO.iter().enumerate() {
        for (col_offset, ch) in line.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            let style = if is_star_decoration(ch) {
                classic::star_decoration_style(row_offset + col_offset)
            } else {
                classic::logo_style()
            };
            buffer.write_text(
                start_row + row_offset,
                logo_left + col_offset,
                &ch.to_string(),
                style,
            );
        }
    }

    NOSTRIAN_CONQUEST_LOGO.len()
}

fn is_star_decoration(ch: char) -> bool {
    matches!(ch, '.' | '*' | 'o')
}
