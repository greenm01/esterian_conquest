use crate::screen::format_sector_coords_default;
use crate::screen::{PlayfieldBuffer, StyledSpan};
use crate::theme::classic;

pub const PLAYFIELD_WIDTH: usize = 80;
pub const PLAYFIELD_HEIGHT: usize = 25;
pub const COMMAND_LINE_ROW: usize = PLAYFIELD_HEIGHT - 1;
pub const EXPERT_MENU_PROMPT_ROW: usize = 0;
pub const CMD_COL_1: usize = 2;
pub const CMD_COL_2: usize = 26;

pub const fn last_body_row() -> usize {
    COMMAND_LINE_ROW - 1
}

pub const fn menu_prompt_row(last_content_row: usize) -> usize {
    let desired = last_content_row + 2;
    if desired > COMMAND_LINE_ROW {
        COMMAND_LINE_ROW
    } else {
        desired
    }
}

pub const fn dismiss_prompt_row(last_content_row: usize) -> usize {
    let desired = last_content_row + 2;
    if desired > COMMAND_LINE_ROW {
        COMMAND_LINE_ROW
    } else {
        desired
    }
}

pub const fn menu_notice_row(command_row: usize) -> usize {
    let desired = command_row + 4;
    if desired > last_body_row() {
        last_body_row()
    } else {
        desired
    }
}

pub const fn menu_general_message_row(command_row: usize) -> usize {
    let desired = command_row + 2;
    if desired > last_body_row() {
        last_body_row()
    } else {
        desired
    }
}

pub const fn table_prompt_row(table_bottom_row: usize) -> usize {
    let desired = table_bottom_row + 1;
    if desired > COMMAND_LINE_ROW {
        COMMAND_LINE_ROW
    } else {
        desired
    }
}

pub const fn table_dismiss_prompt_row(table_bottom_row: usize) -> usize {
    let desired = table_bottom_row + 1;
    if desired > COMMAND_LINE_ROW {
        COMMAND_LINE_ROW
    } else {
        desired
    }
}

pub const fn centered_row(first_row: usize, last_row: usize, block_height: usize) -> usize {
    let available_rows = last_row.saturating_sub(first_row) + 1;
    first_row + available_rows.saturating_sub(block_height) / 2
}

pub const fn standard_table_visible_rows(start_row: usize) -> usize {
    COMMAND_LINE_ROW.saturating_sub(start_row + 4)
}

pub const fn stacked_table_visible_rows(start_row: usize) -> usize {
    COMMAND_LINE_ROW.saturating_sub(start_row + 5)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandMessage<'a> {
    General { label: &'a str, value: &'a str },
    Notice(&'a str),
    Error(&'a str),
    Warning(&'a str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PromptFeedback {
    Notice(String),
    Error(String),
    Warning(String),
}

impl PromptFeedback {
    pub fn notice(value: impl Into<String>) -> Self {
        Self::Notice(value.into())
    }

    pub fn error(value: impl Into<String>) -> Self {
        Self::Error(value.into())
    }

    pub fn warning(value: impl Into<String>) -> Self {
        Self::Warning(value.into())
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Notice(value) | Self::Error(value) | Self::Warning(value) => value,
        }
    }
}

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
    draw_command_prompt_at(
        buffer,
        menu_prompt_row(rows.len()),
        prompt_label,
        prompt_keys,
    );
}

pub fn draw_expert_menu(
    buffer: &mut PlayfieldBuffer,
    prompt_label: &str,
    prompt_keys: &str,
    notice: Option<&str>,
) {
    draw_command_prompt_at(buffer, EXPERT_MENU_PROMPT_ROW, prompt_label, prompt_keys);
    if let Some(notice) = notice {
        draw_menu_notice(buffer, EXPERT_MENU_PROMPT_ROW, notice);
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

/// Draw a menu entry with an inline `[ON] [OFF]` toggle indicator.
///
/// The active state is highlighted with `indicator_on_style()` and the
/// inactive state is dimmed with `indicator_off_style()`.  Brackets stay
/// in the normal `menu_style()`.
pub fn draw_menu_entry_with_toggle(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    hotkey: &str,
    label: &str,
    is_on: bool,
) {
    let menu = classic::menu_style();
    let on_style = if is_on {
        classic::indicator_on_style()
    } else {
        classic::indicator_off_style()
    };
    let off_style = if is_on {
        classic::indicator_off_style()
    } else {
        classic::indicator_on_style()
    };
    buffer.write_spans(
        row,
        col,
        &[
            StyledSpan::new(hotkey, classic::menu_hotkey_style()),
            StyledSpan::new(">", menu),
            StyledSpan::new(label, menu),
            StyledSpan::new("[", menu),
            StyledSpan::new("ON", on_style),
            StyledSpan::new("] [", menu),
            StyledSpan::new("OFF", off_style),
            StyledSpan::new("]", menu),
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

pub fn draw_notice_line(buffer: &mut PlayfieldBuffer, row: usize, value: &str) {
    buffer.write_spans(
        row,
        0,
        &[
            StyledSpan::new("Notice: ", classic::notice_style()),
            StyledSpan::new(value, classic::status_value_style()),
        ],
    );
}

pub fn draw_alert_line(buffer: &mut PlayfieldBuffer, row: usize, label: &str, value: &str) {
    buffer.write_spans(
        row,
        0,
        &[
            StyledSpan::new(label, classic::error_style()),
            StyledSpan::new(value, classic::status_value_style()),
        ],
    );
}

pub fn draw_message_line(buffer: &mut PlayfieldBuffer, row: usize, label: &str, value: &str) {
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

pub fn draw_wrapped_notice(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    max_rows: usize,
    value: &str,
) -> usize {
    if max_rows == 0 {
        return 0;
    }
    let label = "Notice: ";
    let label_width = label.chars().count();
    let continuation = " ".repeat(label_width);
    let first_width = PLAYFIELD_WIDTH.saturating_sub(label_width).max(1);
    let continuation_width = PLAYFIELD_WIDTH.saturating_sub(label_width).max(1);
    let lines = wrap_text(value, first_width, continuation_width);
    let rows_to_draw = lines.len().min(max_rows);
    for (idx, line) in lines.into_iter().take(rows_to_draw).enumerate() {
        if idx == 0 {
            draw_notice_line(buffer, start_row + idx, &line);
        } else {
            buffer.write_spans(
                start_row + idx,
                0,
                &[
                    StyledSpan::new(&continuation, classic::notice_style()),
                    StyledSpan::new(&line, classic::status_value_style()),
                ],
            );
        }
    }
    rows_to_draw
}

pub fn draw_wrapped_alert(
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
        if idx == 0 {
            draw_alert_line(buffer, start_row + idx, label, &line);
        } else {
            buffer.write_spans(
                start_row + idx,
                0,
                &[
                    StyledSpan::new(&continuation, classic::error_style()),
                    StyledSpan::new(&line, classic::status_value_style()),
                ],
            );
        }
    }
    rows_to_draw
}

pub fn draw_wrapped_message(
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
        if idx == 0 {
            draw_message_line(buffer, start_row + idx, label, &line);
        } else {
            buffer.write_spans(
                start_row + idx,
                0,
                &[
                    StyledSpan::new(&continuation, classic::status_label_style()),
                    StyledSpan::new(&line, classic::status_value_style()),
                ],
            );
        }
    }
    rows_to_draw
}

pub fn draw_menu_notice(buffer: &mut PlayfieldBuffer, command_row: usize, notice: &str) -> usize {
    draw_command_message_stack_at(
        buffer,
        menu_notice_row(command_row),
        &[CommandMessage::Notice(notice)],
    )
}

pub fn draw_menu_notice_after(
    buffer: &mut PlayfieldBuffer,
    previous_end_row: usize,
    notice: &str,
) -> usize {
    draw_command_message_stack_after(buffer, previous_end_row, &[CommandMessage::Notice(notice)])
}

pub fn draw_menu_alert_after(
    buffer: &mut PlayfieldBuffer,
    previous_end_row: usize,
    label: &str,
    value: &str,
) -> usize {
    let message = match label {
        "Error: " => CommandMessage::Error(value),
        _ => CommandMessage::Warning(value),
    };
    draw_command_message_stack_after(buffer, previous_end_row, &[message])
}

pub fn draw_prompt_notice_after(
    buffer: &mut PlayfieldBuffer,
    previous_end_row: usize,
    value: &str,
) -> usize {
    draw_command_message_stack_after(buffer, previous_end_row, &[CommandMessage::Notice(value)])
}

pub fn draw_prompt_warning_after(
    buffer: &mut PlayfieldBuffer,
    previous_end_row: usize,
    value: &str,
) -> usize {
    draw_command_message_stack_after(buffer, previous_end_row, &[CommandMessage::Warning(value)])
}

pub fn draw_prompt_error_after(
    buffer: &mut PlayfieldBuffer,
    previous_end_row: usize,
    value: &str,
) -> usize {
    draw_command_message_stack_after(buffer, previous_end_row, &[CommandMessage::Error(value)])
}

pub fn draw_prompt_feedback_after(
    buffer: &mut PlayfieldBuffer,
    previous_end_row: usize,
    feedback: &PromptFeedback,
) -> usize {
    match feedback {
        PromptFeedback::Notice(value) => draw_prompt_notice_after(buffer, previous_end_row, value),
        PromptFeedback::Error(value) => draw_prompt_error_after(buffer, previous_end_row, value),
        PromptFeedback::Warning(value) => {
            draw_prompt_warning_after(buffer, previous_end_row, value)
        }
    }
}

pub fn draw_menu_general_message(
    buffer: &mut PlayfieldBuffer,
    command_row: usize,
    label: &str,
    value: &str,
) -> usize {
    draw_general_message_after_command(buffer, command_row, label, value)
}

pub fn draw_general_message_after_command(
    buffer: &mut PlayfieldBuffer,
    command_row: usize,
    label: &str,
    value: &str,
) -> usize {
    draw_command_message_stack(
        buffer,
        command_row,
        &[CommandMessage::General { label, value }],
    )
}

pub fn draw_command_message_stack(
    buffer: &mut PlayfieldBuffer,
    command_row: usize,
    messages: &[CommandMessage<'_>],
) -> usize {
    draw_command_message_stack_at(buffer, menu_general_message_row(command_row), messages)
}

pub fn draw_command_message_stack_after(
    buffer: &mut PlayfieldBuffer,
    previous_end_row: usize,
    messages: &[CommandMessage<'_>],
) -> usize {
    draw_command_message_stack_at(
        buffer,
        (previous_end_row + 2).min(last_body_row()),
        messages,
    )
}

fn draw_command_message_stack_at(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    messages: &[CommandMessage<'_>],
) -> usize {
    if messages.is_empty() {
        return start_row.saturating_sub(1);
    }
    let mut end_row = start_row.saturating_sub(1);
    for (idx, message) in messages.iter().enumerate() {
        let row = if idx == 0 {
            start_row
        } else {
            (end_row + 2).min(last_body_row())
        };
        end_row = draw_command_message_block(buffer, row, *message);
    }
    end_row
}

fn draw_command_message_block(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    message: CommandMessage<'_>,
) -> usize {
    match message {
        CommandMessage::General { label, value } => {
            let max_rows = last_body_row().saturating_sub(row) + 1;
            let drawn = draw_wrapped_message(buffer, row, max_rows, label, value);
            row + drawn.saturating_sub(1)
        }
        CommandMessage::Notice(value) => {
            let max_rows = (last_body_row().saturating_sub(row) + 1).min(3);
            let drawn = draw_wrapped_notice(buffer, row, max_rows, value);
            row + drawn.saturating_sub(1)
        }
        CommandMessage::Error(value) => {
            let max_rows = (last_body_row().saturating_sub(row) + 1).min(3);
            let drawn = draw_wrapped_alert(buffer, row, max_rows, "Error: ", value);
            row + drawn.saturating_sub(1)
        }
        CommandMessage::Warning(value) => {
            let max_rows = (last_body_row().saturating_sub(row) + 1).min(3);
            let drawn = draw_wrapped_alert(buffer, row, max_rows, "Warning: ", value);
            row + drawn.saturating_sub(1)
        }
    }
}

pub fn draw_inline_planet_info_prompt(
    buffer: &mut PlayfieldBuffer,
    command_row: usize,
    default_coords: [u8; 2],
    input: &str,
    error: Option<&str>,
    notice: Option<&str>,
) -> usize {
    draw_command_line_default_input_at(
        buffer,
        command_row,
        "COMMAND",
        "Planet coords ",
        &format_sector_coords_default(default_coords),
        input,
    );
    let mut messages = vec![CommandMessage::General {
        label: "",
        value: "Enter coordinates of the planet to view.",
    }];
    if let Some(error) = error {
        messages.push(CommandMessage::Error(error));
    }
    if let Some(notice) = notice {
        messages.push(CommandMessage::Notice(notice));
    }
    draw_command_message_stack(buffer, command_row, &messages)
}

pub fn draw_inline_delete_reviewables_prompt(
    buffer: &mut PlayfieldBuffer,
    command_row: usize,
    notice: Option<&str>,
) -> usize {
    draw_inline_confirm_prompt(buffer, command_row, "COMMAND");
    draw_inline_confirm_block(
        buffer,
        command_row,
        "DELETE ALL MESSAGES / RESULTS:",
        &["This will clear all currently reviewable messages and results."],
        notice,
    )
}

pub fn draw_inline_confirm_prompt(buffer: &mut PlayfieldBuffer, command_row: usize, label: &str) {
    draw_command_line_prompt_text_at(buffer, command_row, label, "Y/[N] -> ");
}

pub fn draw_inline_confirm_block(
    buffer: &mut PlayfieldBuffer,
    command_row: usize,
    title: &str,
    lines: &[&str],
    notice: Option<&str>,
) -> usize {
    let title_row = menu_general_message_row(command_row);
    buffer.write_text(title_row, 0, title, classic::notice_style());
    let mut end_row = title_row;
    for line in lines {
        end_row = (end_row + 1).min(last_body_row());
        buffer.write_text(end_row, 0, line, classic::status_value_style());
    }
    if let Some(notice) = notice {
        draw_command_message_stack_after(buffer, end_row, &[CommandMessage::Notice(notice)])
    } else {
        end_row
    }
}

pub fn draw_inline_tax_prompt(
    buffer: &mut PlayfieldBuffer,
    command_row: usize,
    current_tax: &str,
    input: &str,
    error: Option<&str>,
    notice: Option<&str>,
) -> usize {
    draw_command_line_default_input_at(
        buffer,
        command_row,
        "PLANET COMMAND",
        "Empire tax rate (0 - 100) ",
        current_tax,
        input,
    );
    let mut messages = vec![
        CommandMessage::General {
            label: "PLANET TAX: ",
            value: "Set empire tax rate.",
        },
        CommandMessage::Warning(
            "Taxes in excess of 65% may actually REDUCE your planets' productivity!",
        ),
    ];
    if let Some(error) = error {
        messages.push(CommandMessage::Error(error));
    }
    if let Some(notice) = notice {
        messages.push(CommandMessage::Notice(notice));
    }
    draw_command_message_stack(buffer, command_row, &messages)
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

pub fn draw_command_prompt_at(buffer: &mut PlayfieldBuffer, row: usize, label: &str, keys: &str) {
    buffer.fill_row(row, classic::prompt_style());
    let prefix = buffer.write_spans(
        row,
        0,
        &[
            StyledSpan::new(label, classic::title_style()),
            StyledSpan::new(" <-", classic::prompt_style()),
        ],
    );
    let suffix = "-> ";
    if keys == "SLAP A KEY" {
        let slap_width = "(slap a key)".chars().count();
        let suffix_col = buffer.width().saturating_sub(suffix.chars().count() + 1);
        let slap_col = suffix_col.saturating_sub(slap_width);
        write_slap_a_key(buffer, row, slap_col);
        let written = buffer.write_text(row, suffix_col, suffix, classic::prompt_style());
        let cursor_col = suffix_col + written;
        buffer.set_cursor(cursor_col as u16, row as u16);
    } else {
        let written = buffer.write_spans(
            row,
            prefix,
            &[
                StyledSpan::new(keys, classic::prompt_hotkey_style()),
                StyledSpan::new(suffix, classic::prompt_style()),
            ],
        );
        let cursor_col = prefix + written;
        buffer.set_cursor(cursor_col as u16, row as u16);
    }
}

pub fn draw_command_line_text_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label: &str,
    text: &str,
) {
    draw_command_line_text_at_col(buffer, row, 0, label, text);
}

pub fn draw_command_line_text_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    label: &str,
    text: &str,
) {
    buffer.fill_row(row, classic::prompt_style());
    buffer.write_spans(
        row,
        col,
        &[
            StyledSpan::new(label, classic::title_style()),
            StyledSpan::new(" <- ", classic::prompt_style()),
            StyledSpan::new(text, classic::prompt_style()),
        ],
    );
}

pub fn draw_command_line_prompt_text_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label: &str,
    prompt: &str,
) {
    buffer.fill_row(row, classic::prompt_style());
    let prefix = buffer.write_spans(
        row,
        0,
        &[
            StyledSpan::new(label, classic::title_style()),
            StyledSpan::new(" <- ", classic::prompt_style()),
        ],
    );
    let cursor_col = write_prompt_markup(buffer, row, prefix, prompt);
    buffer.set_cursor(cursor_col as u16, row as u16);
}

pub fn draw_command_line_default_input_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label: &str,
    prompt: &str,
    default: &str,
    input: &str,
) {
    buffer.fill_row(row, classic::prompt_style());
    let mut col = buffer.write_spans(
        row,
        0,
        &[
            StyledSpan::new(label, classic::title_style()),
            StyledSpan::new(" <- ", classic::prompt_style()),
        ],
    );
    col = write_prompt_markup(buffer, row, col, prompt);
    if !default.is_empty() {
        col += buffer.write_spans(
            row,
            col,
            &[
                StyledSpan::new("[", classic::prompt_style()),
                StyledSpan::new(default, classic::prompt_hotkey_style()),
                StyledSpan::new("] ", classic::prompt_style()),
            ],
        );
    }
    col = write_prompt_markup(buffer, row, col, "<Q> -> ");
    let written = buffer.write_text(row, col, input, classic::prompt_hotkey_style());
    let cursor_col = col + written;
    buffer.set_cursor(cursor_col as u16, row as u16);
}

pub fn draw_table_command_bar(
    buffer: &mut PlayfieldBuffer,
    hotkeys_markup: &str,
    default: Option<&str>,
    input: &str,
) -> usize {
    draw_table_command_bar_at(buffer, COMMAND_LINE_ROW, hotkeys_markup, default, input)
}

pub fn draw_table_command_bar_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    hotkeys_markup: &str,
    default: Option<&str>,
    input: &str,
) -> usize {
    draw_table_command_bar_at_col(buffer, row, 0, hotkeys_markup, default, input)
}

pub fn draw_table_command_bar_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    hotkeys_markup: &str,
    default: Option<&str>,
    input: &str,
) -> usize {
    buffer.fill_row(row, classic::prompt_style());
    let mut cursor_col = col
        + buffer.write_spans(
            row,
            col,
            &[
                StyledSpan::new("COMMANDS", classic::title_style()),
                StyledSpan::new(" ", classic::prompt_style()),
            ],
        );
    cursor_col = write_prompt_markup(buffer, row, cursor_col, hotkeys_markup);
    if let Some(default) = default {
        cursor_col += buffer.write_spans(
            row,
            cursor_col,
            &[
                StyledSpan::new(" [", classic::prompt_style()),
                StyledSpan::new(default, classic::prompt_hotkey_style()),
                StyledSpan::new("] -> ", classic::prompt_style()),
            ],
        );
        let written = buffer.write_text(row, cursor_col, input, classic::prompt_hotkey_style());
        let final_cursor_col = cursor_col + written;
        buffer.set_cursor(final_cursor_col as u16, row as u16);
        final_cursor_col
    } else {
        let written = buffer.write_text(row, cursor_col, " -> ", classic::prompt_style());
        let final_cursor_col = cursor_col + written;
        buffer.set_cursor(final_cursor_col as u16, row as u16);
        final_cursor_col
    }
}

pub fn draw_table_command_prompt(buffer: &mut PlayfieldBuffer, prompt: &str) -> usize {
    draw_table_command_prompt_at(buffer, COMMAND_LINE_ROW, prompt)
}

pub fn draw_table_command_prompt_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    prompt: &str,
) -> usize {
    draw_table_command_prompt_at_col(buffer, row, 0, prompt)
}

pub fn draw_table_command_prompt_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    prompt: &str,
) -> usize {
    buffer.fill_row(row, classic::prompt_style());
    let prefix = col
        + buffer.write_spans(
            row,
            col,
            &[
                StyledSpan::new("COMMANDS", classic::title_style()),
                StyledSpan::new(" <- ", classic::prompt_style()),
            ],
        );
    let prompt = ensure_cursor_gap(prompt);
    let cursor_col = write_prompt_markup(buffer, row, prefix, &prompt);
    buffer.set_cursor(cursor_col as u16, row as u16);
    cursor_col
}

pub fn draw_plain_prompt(buffer: &mut PlayfieldBuffer, row: usize, prompt: &str) -> usize {
    buffer.fill_row(row, classic::prompt_style());
    let prompt = ensure_cursor_gap(prompt);
    let cursor_col = write_prompt_markup(buffer, row, 0, &prompt);
    buffer.set_cursor(cursor_col as u16, row as u16);
    cursor_col
}

pub fn draw_dismiss_prompt(buffer: &mut PlayfieldBuffer, row: usize) -> usize {
    draw_plain_prompt(buffer, row, "(slap a key)")
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
    for row in 3..COMMAND_LINE_ROW {
        buffer.fill_row(row, classic::help_panel_style());
    }
    let mut last_content_row = 2;
    for (idx, line) in lines.iter().enumerate() {
        let row = 3 + idx;
        if row >= COMMAND_LINE_ROW - 1 {
            break;
        }
        buffer.write_text(row, 0, line, classic::help_panel_style());
        last_content_row = row;
    }
    let _ = prompt_label;
    draw_dismiss_prompt(buffer, dismiss_prompt_row(last_content_row));
}

pub fn draw_bottom_aligned_transcript_rows<F>(
    buffer: &mut PlayfieldBuffer,
    rows: &[String],
    revealed_rows: usize,
    first_content_row: usize,
    last_content_row: usize,
    mut draw_row: F,
) where
    F: FnMut(&mut PlayfieldBuffer, usize, &str),
{
    if first_content_row > last_content_row {
        return;
    }
    let page_height = last_content_row - first_content_row + 1;
    let revealed_end = revealed_rows.min(rows.len());
    let visible_start = revealed_end.saturating_sub(page_height);
    let visible_rows = &rows[visible_start..revealed_end];
    let first_row = last_content_row
        .saturating_add(1)
        .saturating_sub(visible_rows.len());
    for (idx, line) in visible_rows.iter().enumerate() {
        draw_row(buffer, first_row + idx, line);
    }
}

pub fn wrap_text(value: &str, first_width: usize, continuation_width: usize) -> Vec<String> {
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

fn ensure_cursor_gap(prompt: &str) -> String {
    if prompt.ends_with("-> ") {
        prompt.to_string()
    } else if prompt.ends_with("->") {
        format!("{prompt} ")
    } else {
        prompt.to_string()
    }
}

fn write_slap_a_key(buffer: &mut PlayfieldBuffer, row: usize, col: usize) -> usize {
    let after_open = col + buffer.write_text(row, col, "(", classic::prompt_hotkey_style());
    let after_text = after_open
        + buffer.write_text(
            row,
            after_open,
            "slap a ",
            classic::prompt_notice_action_style(),
        );
    let after_key =
        after_text + buffer.write_text(row, after_text, "key", classic::prompt_hotkey_style());
    after_key + buffer.write_text(row, after_key, ")", classic::prompt_hotkey_style())
}

fn write_prompt_markup(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    start_col: usize,
    text: &str,
) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut col = start_col;
    let mut plain = String::new();
    let mut idx = 0usize;

    while idx < chars.len() {
        if let Some((phrase_end, key_start, key_end)) = slap_a_key_phrase(&chars, idx) {
            if !plain.is_empty() {
                col += buffer.write_text(row, col, &plain, classic::prompt_style());
                plain.clear();
            }
            if key_start > idx {
                let prefix = chars[idx..key_start].iter().collect::<String>();
                col += buffer.write_text(row, col, &prefix, classic::prompt_notice_action_style());
            }
            let key = chars[key_start..key_end].iter().collect::<String>();
            col += buffer.write_text(row, col, &key, classic::prompt_hotkey_style());
            idx = phrase_end;
            continue;
        }

        if chars[idx] == '<'
            && let Some(close_idx) = chars[idx + 1..].iter().position(|&ch| ch == '>')
        {
            let close_idx = idx + 1 + close_idx;
            if is_prompt_angle_hotkey(&chars[idx + 1..close_idx]) {
                if !plain.is_empty() {
                    col += buffer.write_text(row, col, &plain, classic::prompt_style());
                    plain.clear();
                }
                col += buffer.write_text(row, col, "<", classic::prompt_style());
                if close_idx > idx + 1 {
                    let segment = chars[idx + 1..close_idx].iter().collect::<String>();
                    col += buffer.write_text(row, col, &segment, classic::prompt_hotkey_style());
                }
                col += buffer.write_text(row, col, ">", classic::prompt_style());
                idx = close_idx + 1;
                continue;
            }
        }

        if chars[idx] == '['
            && let Some(close_idx) = chars[idx + 1..].iter().position(|&ch| ch == ']')
        {
            let close_idx = idx + 1 + close_idx;
            if is_prompt_bracket_hotkey(&chars[idx + 1..close_idx]) {
                if !plain.is_empty() {
                    col += buffer.write_text(row, col, &plain, classic::prompt_style());
                    plain.clear();
                }
                col += buffer.write_text(row, col, "[", classic::prompt_style());
                if close_idx > idx + 1 {
                    let segment = chars[idx + 1..close_idx].iter().collect::<String>();
                    col += buffer.write_text(row, col, &segment, classic::prompt_hotkey_style());
                }
                col += buffer.write_text(row, col, "]", classic::prompt_style());
                idx = close_idx + 1;
                continue;
            }
        }

        if chars[idx].is_ascii_alphanumeric() {
            let start = idx;
            while idx < chars.len() && chars[idx].is_ascii_alphanumeric() {
                idx += 1;
            }
            let token = chars[start..idx].iter().collect::<String>();
            if is_prompt_slash_hotkey_token(&chars, start, idx) {
                if !plain.is_empty() {
                    col += buffer.write_text(row, col, &plain, classic::prompt_style());
                    plain.clear();
                }
                col += buffer.write_text(row, col, &token, classic::prompt_hotkey_style());
            } else {
                plain.push_str(&token);
            }
            continue;
        }

        plain.push(chars[idx]);
        idx += 1;
    }

    if !plain.is_empty() {
        col += buffer.write_text(row, col, &plain, classic::prompt_style());
    }

    col
}

fn is_prompt_slash_hotkey_token(chars: &[char], start: usize, end: usize) -> bool {
    let token_len = end.saturating_sub(start);
    token_len > 0
        && token_len <= 3
        && (matches!(chars.get(end), Some('/')) || (start > 0 && chars[start - 1] == '/'))
}

fn is_prompt_bracket_hotkey(chars: &[char]) -> bool {
    !chars.is_empty() && chars.len() <= 5 && chars.iter().all(|ch| ch.is_ascii_alphanumeric())
}

fn is_prompt_angle_hotkey(chars: &[char]) -> bool {
    !chars.is_empty()
        && chars
            .iter()
            .all(|ch| ch.is_ascii_alphanumeric() || ch.is_ascii_whitespace())
}

fn slap_a_key_phrase(chars: &[char], start: usize) -> Option<(usize, usize, usize)> {
    const KEYWORD: [&str; 2] = ["slap a key", "Slap a key"];
    for keyword in KEYWORD {
        let kw_chars: Vec<char> = keyword.chars().collect();
        let end = start + kw_chars.len();
        if end > chars.len() || chars[start..end] != kw_chars[..] {
            continue;
        }
        let key_start = start + kw_chars.len() - 3;
        return Some((end, key_start, end));
    }
    None
}
