use crate::screen::{PlayfieldBuffer, StyledSpan};
use crate::theme::classic;

pub const PLAYFIELD_WIDTH: usize = 80;
pub const PLAYFIELD_HEIGHT: usize = 20;

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

pub fn draw_status_line(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label: &str,
    value: &str,
) {
    buffer.write_spans(
        row,
        0,
        &[
            StyledSpan::new(label, classic::status_label_style()),
            StyledSpan::new(value, classic::status_value_style()),
        ],
    );
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

pub fn draw_command_prompt(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label: &str,
    keys: &str,
) {
    let cursor_col = buffer.write_spans(
        row,
        0,
        &[
            StyledSpan::new(label, classic::title_style()),
            StyledSpan::new(" <-", classic::prompt_style()),
            StyledSpan::new(keys, classic::prompt_hotkey_style()),
            StyledSpan::new("-> ", classic::prompt_style()),
        ],
    );
    buffer.set_cursor(cursor_col as u16, row as u16);
}

pub fn draw_plain_prompt(buffer: &mut PlayfieldBuffer, row: usize, prompt: &str) {
    let cursor_col = buffer.write_text(row, 0, prompt, classic::prompt_style());
    buffer.set_cursor(cursor_col as u16, row as u16);
}
