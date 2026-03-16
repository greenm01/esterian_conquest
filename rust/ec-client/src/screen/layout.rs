use crate::screen::{PlayfieldBuffer, StyledSpan};
use crate::theme::classic;

pub const PLAYFIELD_WIDTH: usize = 80;
pub const PLAYFIELD_HEIGHT: usize = 20;
pub const COMMAND_LINE_ROW: usize = PLAYFIELD_HEIGHT - 1;
pub const CMD_COL_1: usize = 2;
pub const CMD_COL_2: usize = 26;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuEntry<'a> {
    pub col: usize,
    pub hotkey: &'a str,
    pub label: &'a str,
}

impl<'a> MenuEntry<'a> {
    pub const fn new(col: usize, hotkey: &'a str, label: &'a str) -> Self {
        Self { col, hotkey, label }
    }
}

pub fn new_playfield() -> PlayfieldBuffer {
    PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style())
}

pub fn draw_title_bar(buffer: &mut PlayfieldBuffer, row: usize, title: &str) {
    buffer.fill_row(row, classic::menu_style());
    buffer.write_text(row, 0, title, classic::title_style());
}

pub fn draw_menu_row(buffer: &mut PlayfieldBuffer, row: usize, entries: &[MenuEntry<'_>]) {
    buffer.fill_row(row, classic::menu_style());
    for entry in entries {
        draw_menu_entry(buffer, row, entry.col, entry.hotkey, entry.label);
    }
}

pub fn draw_command_center(
    buffer: &mut PlayfieldBuffer,
    title: &str,
    top_row_entries: &[MenuEntry<'_>],
    rows: &[&[MenuEntry<'_>]],
    prompt_label: &str,
    prompt_keys: &str,
) {
    draw_title_bar(buffer, 0, title);
    for entry in top_row_entries {
        draw_menu_entry(buffer, 0, entry.col, entry.hotkey, entry.label);
    }
    for (idx, row_entries) in rows.iter().enumerate() {
        draw_menu_row(buffer, idx + 1, row_entries);
    }
    draw_command_prompt(buffer, rows.len() + 1, prompt_label, prompt_keys);
}

pub fn draw_menu_entry(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    hotkey: &str,
    label: &str,
) {
    buffer.write_spans(
        row,
        col,
        &[
            StyledSpan::new(hotkey, classic::menu_hotkey_style()),
            StyledSpan::new(">", classic::menu_style()),
            StyledSpan::new(label, classic::menu_style()),
        ],
    );
}

pub fn draw_status_line(buffer: &mut PlayfieldBuffer, row: usize, label: &str, value: &str) {
    buffer.write_spans(
        row,
        0,
        &[
            StyledSpan::new(label, classic::status_label_style()),
            StyledSpan::new(value, classic::status_value_style()),
        ],
    );
}

pub fn draw_wrapped_status(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    max_rows: usize,
    label: &str,
    value: &str,
) -> usize {
    if max_rows == 0 {
        return 0;
    }
    let label_width = label.chars().count();
    let continuation = " ".repeat(label_width);
    let first_width = PLAYFIELD_WIDTH.saturating_sub(label_width).max(1);
    let continuation_width = PLAYFIELD_WIDTH.saturating_sub(label_width).max(1);
    let lines = wrap_text(value, first_width, continuation_width);
    let rows_to_draw = lines.len().min(max_rows);
    for (idx, line) in lines.into_iter().take(rows_to_draw).enumerate() {
        let current_label = if idx == 0 { label } else { &continuation };
        draw_status_line(buffer, start_row + idx, current_label, &line);
    }
    rows_to_draw
}

pub fn draw_centered_text(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    text: &str,
    style: crate::screen::CellStyle,
) {
    let col = PLAYFIELD_WIDTH.saturating_sub(text.chars().count()) / 2;
    buffer.write_text(row, col, text, style);
}

pub fn draw_command_prompt(buffer: &mut PlayfieldBuffer, _row: usize, label: &str, keys: &str) {
    buffer.fill_row(COMMAND_LINE_ROW, classic::prompt_style());
    buffer.write_spans(
        COMMAND_LINE_ROW,
        0,
        &[
            StyledSpan::new(label, classic::title_style()),
            StyledSpan::new(" <-", classic::prompt_style()),
            StyledSpan::new(keys, classic::prompt_hotkey_style()),
            StyledSpan::new("-> ", classic::prompt_style()),
        ],
    );
}

pub fn draw_command_line_text(buffer: &mut PlayfieldBuffer, label: &str, text: &str) {
    buffer.fill_row(COMMAND_LINE_ROW, classic::prompt_style());
    buffer.write_spans(
        COMMAND_LINE_ROW,
        0,
        &[
            StyledSpan::new(label, classic::title_style()),
            StyledSpan::new(" <- ", classic::prompt_style()),
            StyledSpan::new(text, classic::prompt_style()),
        ],
    );
}

pub fn draw_command_line_default_input(
    buffer: &mut PlayfieldBuffer,
    label: &str,
    prompt: &str,
    default: &str,
    input: &str,
) -> usize {
    buffer.fill_row(COMMAND_LINE_ROW, classic::prompt_style());
    let prefix = buffer.write_spans(
        COMMAND_LINE_ROW,
        0,
        &[
            StyledSpan::new(label, classic::title_style()),
            StyledSpan::new(" <- ", classic::prompt_style()),
            StyledSpan::new(prompt, classic::prompt_style()),
            StyledSpan::new("[", classic::prompt_style()),
            StyledSpan::new(default, classic::prompt_hotkey_style()),
            StyledSpan::new("] -> ", classic::prompt_style()),
        ],
    );
    let written = buffer.write_text(
        COMMAND_LINE_ROW,
        prefix,
        input,
        classic::prompt_hotkey_style(),
    );
    let cursor_col = prefix + written;
    buffer.set_cursor(cursor_col as u16, COMMAND_LINE_ROW as u16);
    cursor_col
}

pub fn draw_plain_prompt(buffer: &mut PlayfieldBuffer, row: usize, prompt: &str) -> usize {
    buffer.fill_row(row, classic::prompt_style());
    let cursor_col = buffer.write_text(row, 0, prompt, classic::prompt_style());
    buffer.set_cursor(cursor_col as u16, row as u16);
    cursor_col
}

pub fn draw_help_panel(
    buffer: &mut PlayfieldBuffer,
    title: &str,
    header: &str,
    lines: &[&str],
    prompt_label: &str,
) {
    draw_title_bar(buffer, 0, title);
    buffer.fill_row(2, classic::help_header_style());
    buffer.write_text(2, 0, header, classic::help_header_style());
    for row in 3..19 {
        buffer.fill_row(row, classic::help_panel_style());
    }
    for (idx, line) in lines.iter().enumerate() {
        if 3 + idx >= 19 {
            break;
        }
        buffer.write_text(3 + idx, 0, line, classic::help_panel_style());
    }
    draw_command_prompt(buffer, 19, prompt_label, "SLAP A KEY");
}

fn wrap_text(value: &str, first_width: usize, continuation_width: usize) -> Vec<String> {
    let normalized = value.split_whitespace().collect::<Vec<_>>();
    if normalized.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut limit = first_width;
    for word in normalized {
        let separator = if current.is_empty() { 0 } else { 1 };
        if current.len() + separator + word.len() > limit && !current.is_empty() {
            lines.push(current);
            current = String::new();
            limit = continuation_width;
        }
        if !current.is_empty() {
            current.push(' ');
        }
        if word.len() > limit && current.is_empty() {
            let mut remaining = word;
            while remaining.len() > limit {
                lines.push(remaining[..limit].to_string());
                remaining = &remaining[limit..];
                limit = continuation_width;
            }
            current.push_str(remaining);
        } else {
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}
