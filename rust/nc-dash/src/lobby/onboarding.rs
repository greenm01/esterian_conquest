use nc_ui::PlayfieldBuffer;
use nc_ui::branding::NOSTRIAN_CONQUEST_LOGO;
use nc_ui::modal::{Rect, centered_rect, draw_box, wrap_modal_text_lines};

use crate::theme;

use super::state::{FirstRunField, LobbyRoute, LobbyState};

pub fn initial_route(keychain_exists: bool) -> LobbyRoute {
    if keychain_exists {
        LobbyRoute::Locked
    } else {
        LobbyRoute::FirstRun
    }
}

const GATE_MIN_POPUP_WIDTH: u16 = 54;
const GATE_MIN_POPUP_HEIGHT: u16 = 18;
const GATE_SIDE_PADDING: usize = 3;
const GATE_FIELD_LABEL_WIDTH: usize = 14;

pub fn render_first_run(buffer: &mut PlayfieldBuffer, state: &LobbyState) {
    let copy_lines = vec![
        "Create your local hosted identity.".to_string(),
        "Choose a handle, set a password, and confirm it.".to_string(),
    ];
    let fields = vec![
        GateField {
            label: "Handle",
            value: display_or_cursor(&state.first_run_handle_input),
            active: state.first_run_field == FirstRunField::Handle,
            cursor_offset: state.first_run_handle_input.chars().count(),
        },
        GateField {
            label: "Set Password",
            value: masked_or_cursor(&state.first_run_password_input),
            active: state.first_run_field == FirstRunField::Password,
            cursor_offset: state.first_run_password_input.chars().count(),
        },
        GateField {
            label: "Confirm",
            value: masked_or_cursor(&state.first_run_confirm_input),
            active: state.first_run_field == FirstRunField::Confirm,
            cursor_offset: state.first_run_confirm_input.chars().count(),
        },
    ];
    render_gate(
        buffer,
        "FIRST RUN",
        state.status_message.as_deref(),
        &copy_lines,
        &fields,
    );
}

pub fn render_locked(buffer: &mut PlayfieldBuffer, state: &LobbyState) {
    let copy_lines = vec!["Enter your keychain password.".to_string()];
    let fields = vec![GateField {
        label: "Password",
        value: masked_or_cursor(&state.unlock_password_input),
        active: true,
        cursor_offset: state.unlock_password_input.chars().count(),
    }];
    render_gate(
        buffer,
        "UNLOCK KEYCHAIN",
        state.status_message.as_deref(),
        &copy_lines,
        &fields,
    );
}

struct GateField<'a> {
    label: &'a str,
    value: String,
    active: bool,
    cursor_offset: usize,
}

fn render_gate(
    buffer: &mut PlayfieldBuffer,
    title: &str,
    status_message: Option<&str>,
    copy_lines: &[String],
    fields: &[GateField<'_>],
) {
    let width = buffer.width() as u16;
    let height = buffer.height() as u16;
    if width < 60 || height < 24 {
        render_tiny(buffer, title);
        return;
    }

    let logo_width = NOSTRIAN_CONQUEST_LOGO
        .iter()
        .map(|line| line.len())
        .max()
        .unwrap_or(0);
    let field_width = fields.iter().map(field_row_width).max().unwrap_or(0);
    let copy_width = copy_lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let status_width = status_message
        .map(|message| message.chars().count())
        .unwrap_or(0);
    let natural_content_width = logo_width
        .max(field_width)
        .max(copy_width)
        .max(status_width);
    let popup_width = (natural_content_width + GATE_SIDE_PADDING * 2 + 2)
        .max(usize::from(GATE_MIN_POPUP_WIDTH))
        .min(width.saturating_sub(2) as usize) as u16;
    let content_width = popup_content_width(popup_width);
    let wrapped_status = status_message
        .map(|message| wrap_modal_text_lines(&[message.to_string()], content_width))
        .unwrap_or_default();
    let wrapped_copy = wrap_modal_text_lines(copy_lines, content_width);
    let fixed_rows = NOSTRIAN_CONQUEST_LOGO.len() + 1 + wrapped_copy.len() + 1 + fields.len();
    let popup_height = (fixed_rows + wrapped_status.len() + 2)
        .min(height.saturating_sub(2) as usize)
        .max(usize::from(GATE_MIN_POPUP_HEIGHT)) as u16;
    let popup = centered_rect(
        popup_width,
        popup_height,
        Rect::new(1, 1, width.saturating_sub(2), height.saturating_sub(2)),
    );

    draw_box(
        buffer,
        popup,
        title,
        theme::table_chrome_style(),
        theme::table_header_style(),
    );
    buffer.fill_rect(
        popup.y as usize + 1,
        popup.x as usize + 1,
        popup.width.saturating_sub(2) as usize,
        popup.height.saturating_sub(2) as usize,
        theme::table_body_style(),
    );

    let left = popup.x as usize + GATE_SIDE_PADDING;
    let content_width = popup_content_width(popup.width);
    let content_bottom = popup.y as usize + popup.height.saturating_sub(2) as usize;
    let mut row = popup.y as usize + 1;
    row += write_wrapped_rows(
        buffer,
        row,
        left,
        content_width,
        content_bottom.saturating_add(1).saturating_sub(row),
        &wrapped_status,
        theme::error_style(),
    );
    if !wrapped_status.is_empty() {
        row += 1;
    }

    row += draw_logo(buffer, row, popup);
    row += 1;

    row += write_wrapped_rows(
        buffer,
        row,
        left,
        content_width,
        content_bottom.saturating_add(1).saturating_sub(row),
        &wrapped_copy,
        theme::prompt_notice_action_style(),
    );
    row += 1;

    for field in fields {
        if row > content_bottom {
            break;
        }
        write_field(buffer, row, left, content_width, field);
        row += 1;
    }
}

fn write_field(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    left: usize,
    content_width: usize,
    field: &GateField<'_>,
) {
    let marker = if field.active { ">" } else { " " };
    let marker_style = if field.active {
        theme::prompt_hotkey_style()
    } else {
        theme::status_label_style()
    };
    let value_style = if field.active {
        theme::prompt_style()
    } else {
        theme::table_body_style()
    };
    let label = format!(
        "{marker} {:<width$} :",
        field.label,
        width = GATE_FIELD_LABEL_WIDTH
    );
    let value_col = left + label.chars().count() + 1;
    buffer.write_text_clipped(row, left, &label, marker_style);
    let value_width = content_width.saturating_sub(value_col.saturating_sub(left));
    let value = clip_to_width(&field.value, value_width);
    buffer.write_text_clipped(row, value_col, &value, value_style);
    if field.active {
        let cursor_col = value_col + field.cursor_offset.min(value.chars().count());
        if cursor_col < buffer.width() {
            buffer.set_cursor(cursor_col as u16, row as u16);
        }
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
                theme::classic::star_decoration_style(row_offset + col_offset)
            } else {
                theme::logo_style()
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

fn render_tiny(buffer: &mut PlayfieldBuffer, title: &str) {
    let lines = ["Nostrian Conquest", title, "Resize the window to continue."];
    let start_row = buffer.height().saturating_sub(lines.len()) / 2;
    for (idx, line) in lines.iter().enumerate() {
        let row = start_row + idx;
        let col = buffer.width().saturating_sub(line.chars().count()) / 2;
        let style = if idx == 0 {
            theme::logo_style()
        } else {
            theme::table_body_style()
        };
        buffer.write_text_clipped(row, col, line, style);
    }
}

fn field_row_width(field: &GateField<'_>) -> usize {
    2 + GATE_FIELD_LABEL_WIDTH + 2 + field.value.chars().count()
}

fn is_star_decoration(ch: char) -> bool {
    matches!(ch, '.' | '*' | 'o')
}

fn masked_or_cursor(value: &str) -> String {
    if value.is_empty() {
        "_".to_string()
    } else {
        "*".repeat(value.chars().count())
    }
}

fn display_or_cursor(value: &str) -> String {
    if value.is_empty() {
        "_".to_string()
    } else {
        value.to_string()
    }
}

fn popup_content_width(popup_width: u16) -> usize {
    popup_width.saturating_sub((GATE_SIDE_PADDING * 2 + 2) as u16) as usize
}

fn write_wrapped_rows(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    left: usize,
    content_width: usize,
    max_rows: usize,
    lines: &[String],
    style: nc_ui::buffer::CellStyle,
) -> usize {
    if content_width == 0 || max_rows == 0 {
        return 0;
    }

    let visible_rows = lines.len().min(max_rows);
    for idx in 0..visible_rows {
        let is_last_visible = idx + 1 == max_rows;
        let overflow_hidden = lines.len() > max_rows;
        let line = if is_last_visible && overflow_hidden {
            truncate_with_continuation(&lines[idx], content_width)
        } else {
            clip_to_width(&lines[idx], content_width)
        };
        buffer.write_text_clipped(start_row + idx, left, &line, style);
    }
    visible_rows
}

fn clip_to_width(text: &str, max_width: usize) -> String {
    text.chars().take(max_width).collect()
}

fn truncate_with_continuation(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if max_width <= 3 {
        return ".".repeat(max_width);
    }
    let clipped = clip_to_width(text, max_width.saturating_sub(3));
    format!("{clipped}...")
}
