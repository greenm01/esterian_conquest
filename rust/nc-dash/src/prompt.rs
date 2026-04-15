#![allow(dead_code)]

use crate::buffer::{PlayfieldBuffer, StyledSpan};
use crate::theme::classic;

const COMMAND_LABEL: &str = "COMMAND";
const COMMAND_ARROW_PREFIX: &str = " <- ";
const COMMAND_ARROW_SUFFIX: &str = " -> ";
const DEFAULT_OPEN: &str = "[";
const DEFAULT_CLOSE_WITH_SPACE: &str = "] ";
const DEFAULT_CLOSE_WITH_SUFFIX: &str = "] -> ";
const SLAP_A_KEY_PROMPT: &str = "(slap a key)";

pub fn plain_prompt_width(prompt: &str) -> usize {
    ensure_cursor_gap(prompt).chars().count() + slap_cursor_padding(prompt)
}

pub fn dismiss_prompt_width() -> usize {
    plain_prompt_width(SLAP_A_KEY_PROMPT)
}

pub fn command_line_text_width(label: &str, text: &str) -> usize {
    label.chars().count() + COMMAND_ARROW_PREFIX.chars().count() + text.chars().count()
}

pub fn command_line_prompt_text_width(label: &str, prompt: &str) -> usize {
    label.chars().count() + COMMAND_ARROW_PREFIX.chars().count() + plain_prompt_width(prompt)
}

pub fn command_line_prompt_input_width(label: &str, prompt: &str, input: &str) -> usize {
    command_line_prompt_text_width(label, prompt) + input.chars().count()
}

pub fn command_line_default_input_width(
    label: &str,
    prompt: &str,
    default: &str,
    input: &str,
) -> usize {
    command_line_default_input_width_with_cancel(label, prompt, default, input, "<ESC> -> ")
}

pub fn command_line_default_input_scaffold_width(
    label: &str,
    prompt: &str,
    default: &str,
) -> usize {
    command_line_default_input_scaffold_width_with_cancel(label, prompt, default, "<ESC> -> ")
}

pub fn command_line_default_input_width_with_cancel(
    label: &str,
    prompt: &str,
    default: &str,
    input: &str,
    cancel_markup: &str,
) -> usize {
    command_line_default_input_scaffold_width_with_cancel(label, prompt, default, cancel_markup)
        + input.chars().count()
}

pub fn command_line_default_input_scaffold_width_with_cancel(
    label: &str,
    prompt: &str,
    default: &str,
    cancel_markup: &str,
) -> usize {
    let mut width =
        label.chars().count() + COMMAND_ARROW_PREFIX.chars().count() + plain_prompt_width(prompt);
    if !default.is_empty() {
        width += DEFAULT_OPEN.chars().count()
            + default.chars().count()
            + DEFAULT_CLOSE_WITH_SPACE.chars().count();
    }
    width + plain_prompt_width(cancel_markup)
}

pub fn table_command_bar_width(hotkeys_markup: &str, default: Option<&str>, input: &str) -> usize {
    table_command_bar_width_for_label(COMMAND_LABEL, hotkeys_markup, default, input)
}

pub fn table_command_bar_scaffold_width(hotkeys_markup: &str, default: Option<&str>) -> usize {
    table_command_bar_scaffold_width_for_label(COMMAND_LABEL, hotkeys_markup, default)
}

pub fn table_command_bar_width_for_label(
    label: &str,
    hotkeys_markup: &str,
    default: Option<&str>,
    input: &str,
) -> usize {
    table_command_bar_scaffold_width_for_label(label, hotkeys_markup, default)
        + input.chars().count()
}

pub fn table_command_bar_scaffold_width_for_label(
    label: &str,
    hotkeys_markup: &str,
    default: Option<&str>,
) -> usize {
    let mut width = COMMAND_LABEL.chars().count() + COMMAND_ARROW_PREFIX.chars().count();
    width += command_rail_width(hotkeys_markup);
    width += label
        .chars()
        .count()
        .saturating_sub(COMMAND_LABEL.chars().count());
    if let Some(default) = default {
        width += " ".chars().count()
            + DEFAULT_OPEN.chars().count()
            + default.chars().count()
            + DEFAULT_CLOSE_WITH_SUFFIX.chars().count();
    } else {
        width += COMMAND_ARROW_SUFFIX.chars().count();
    }
    width
}

pub fn table_command_prompt_width(prompt: &str) -> usize {
    COMMAND_LABEL.chars().count()
        + COMMAND_ARROW_PREFIX.chars().count()
        + plain_prompt_width(prompt)
}

pub fn draw_command_prompt_at(buffer: &mut PlayfieldBuffer, row: usize, label: &str, keys: &str) {
    draw_command_prompt_at_col(buffer, row, 0, label, keys);
}

pub fn draw_command_prompt_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    label: &str,
    keys: &str,
) {
    buffer.fill_row(row, classic::prompt_style());
    let prefix_col = col
        + buffer.write_spans_clipped(
            row,
            col,
            &[
                StyledSpan::new(label, classic::title_style()),
                StyledSpan::new(" <- ", classic::prompt_style()),
            ],
        );
    let mut cursor_col = write_command_rail_tokens(buffer, row, prefix_col, keys);
    cursor_col += buffer.write_text_clipped(row, cursor_col, " -> ", classic::prompt_style());
    if buffer.width() > 0 {
        buffer.set_cursor(cursor_col.min(buffer.width() - 1) as u16, row as u16);
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
    buffer.write_spans_clipped(
        row,
        col,
        &[
            StyledSpan::new(label, classic::title_style()),
            StyledSpan::new(" <- ", classic::prompt_style()),
            StyledSpan::new(text, classic::prompt_style()),
        ],
    );
}

pub fn draw_command_line_text_in_span(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    width: usize,
    label: &str,
    text: &str,
) {
    fill_prompt_span(buffer, row, col, width);
    let mut cursor_col = col;
    cursor_col += write_spans_clipped_to_span(
        buffer,
        row,
        cursor_col,
        col + width,
        &[
            StyledSpan::new(label, classic::title_style()),
            StyledSpan::new(" <- ", classic::prompt_style()),
            StyledSpan::new(text, classic::prompt_style()),
        ],
    );
    if buffer.width() > 0 && width > 0 {
        let clamped = cursor_col.min(col + width - 1).min(buffer.width() - 1);
        buffer.set_cursor(clamped as u16, row as u16);
    }
}

pub fn draw_command_line_prompt_text_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label: &str,
    prompt: &str,
) {
    draw_command_line_prompt_text_at_col(buffer, row, 0, label, prompt);
}

pub fn draw_command_line_prompt_text_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    label: &str,
    prompt: &str,
) {
    buffer.fill_row(row, classic::prompt_style());
    let prefix = col
        + buffer.write_spans_clipped(
            row,
            col,
            &[
                StyledSpan::new(label, classic::title_style()),
                StyledSpan::new(" <- ", classic::prompt_style()),
            ],
        );
    let prompt = ensure_cursor_gap(prompt);
    let cursor_col = write_prompt_markup(buffer, row, prefix, &prompt);
    if contains_slap_a_key_phrase(&prompt) && buffer.width() > 0 {
        set_slap_cursor(buffer, row, cursor_col, buffer.width() - 1);
    } else if buffer.width() > 0 {
        buffer.set_cursor(cursor_col.min(buffer.width() - 1) as u16, row as u16);
    }
}

pub fn draw_command_line_prompt_text_in_span(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    width: usize,
    label: &str,
    prompt: &str,
) {
    fill_prompt_span(buffer, row, col, width);
    let prefix = col
        + write_spans_clipped_to_span(
            buffer,
            row,
            col,
            col + width,
            &[
                StyledSpan::new(label, classic::title_style()),
                StyledSpan::new(" <- ", classic::prompt_style()),
            ],
        );
    let prompt = ensure_cursor_gap(prompt);
    let cursor_col = write_prompt_markup_in_span(buffer, row, prefix, col + width, &prompt);
    if contains_slap_a_key_phrase(&prompt) && buffer.width() > 0 && width > 0 {
        let clamped = (col + width - 1).min(buffer.width() - 1);
        set_slap_cursor(buffer, row, cursor_col, clamped);
    } else if buffer.width() > 0 && width > 0 {
        let clamped = cursor_col.min(col + width - 1).min(buffer.width() - 1);
        buffer.set_cursor(clamped as u16, row as u16);
    }
}

pub fn draw_command_line_prompt_input_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    label: &str,
    prompt: &str,
    input: &str,
) {
    buffer.fill_row(row, classic::prompt_style());
    let prefix = col
        + buffer.write_spans_clipped(
            row,
            col,
            &[
                StyledSpan::new(label, classic::title_style()),
                StyledSpan::new(" <- ", classic::prompt_style()),
            ],
        );
    let prompt = ensure_cursor_gap(prompt);
    let cursor_col = write_prompt_markup(buffer, row, prefix, &prompt);
    let _ = write_live_input_clipped(
        buffer,
        row,
        cursor_col,
        input,
        classic::prompt_hotkey_style(),
    );
}

pub fn draw_command_line_prompt_input_in_span(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    width: usize,
    label: &str,
    prompt: &str,
    input: &str,
) {
    fill_prompt_span(buffer, row, col, width);
    let span_end = col + width;
    let prefix = col
        + write_spans_clipped_to_span(
            buffer,
            row,
            col,
            span_end,
            &[
                StyledSpan::new(label, classic::title_style()),
                StyledSpan::new(" <- ", classic::prompt_style()),
            ],
        );
    let prompt = ensure_cursor_gap(prompt);
    let cursor_col = write_prompt_markup_in_span(buffer, row, prefix, span_end, &prompt);
    let _ = write_live_input_clipped_to_span(
        buffer,
        row,
        cursor_col,
        span_end,
        input,
        classic::prompt_hotkey_style(),
    );
}

pub fn draw_command_line_default_input_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label: &str,
    prompt: &str,
    default: &str,
    input: &str,
) {
    draw_command_line_default_input_at_col(buffer, row, 0, label, prompt, default, input)
}

pub fn draw_command_line_default_input_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    label: &str,
    prompt: &str,
    default: &str,
    input: &str,
) {
    draw_command_line_default_input_with_cancel_at_col(
        buffer,
        row,
        col,
        label,
        prompt,
        default,
        input,
        "<ESC> -> ",
    );
}

pub fn draw_command_line_default_input_with_cancel_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label: &str,
    prompt: &str,
    default: &str,
    input: &str,
    cancel_markup: &str,
) {
    draw_command_line_default_input_with_cancel_at_col(
        buffer,
        row,
        0,
        label,
        prompt,
        default,
        input,
        cancel_markup,
    );
}

pub fn draw_command_line_default_input_with_cancel_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    label: &str,
    prompt: &str,
    default: &str,
    input: &str,
    cancel_markup: &str,
) {
    buffer.fill_row(row, classic::prompt_style());
    let mut cursor_col = col
        + buffer.write_spans_clipped(
            row,
            col,
            &[
                StyledSpan::new(label, classic::title_style()),
                StyledSpan::new(" <- ", classic::prompt_style()),
            ],
        );
    cursor_col = write_prompt_markup(buffer, row, cursor_col, prompt);
    if !default.is_empty() {
        cursor_col += buffer.write_spans_clipped(
            row,
            cursor_col,
            &[
                StyledSpan::new("[", classic::prompt_square_delimiter_style()),
                StyledSpan::new(default, classic::prompt_hotkey_style()),
                StyledSpan::new("]", classic::prompt_square_delimiter_style()),
                StyledSpan::new(" ", classic::prompt_style()),
            ],
        );
    }
    cursor_col = write_prompt_markup(buffer, row, cursor_col, cancel_markup);
    let _ = write_live_input_clipped(
        buffer,
        row,
        cursor_col,
        input,
        classic::prompt_hotkey_style(),
    );
}

pub fn draw_command_line_default_input_with_cancel_in_span(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    width: usize,
    label: &str,
    prompt: &str,
    default: &str,
    input: &str,
    cancel_markup: &str,
) {
    fill_prompt_span(buffer, row, col, width);
    let span_end = col + width;
    let mut cursor_col = col
        + write_spans_clipped_to_span(
            buffer,
            row,
            col,
            span_end,
            &[
                StyledSpan::new(label, classic::title_style()),
                StyledSpan::new(" <- ", classic::prompt_style()),
            ],
        );
    cursor_col = write_prompt_markup_in_span(buffer, row, cursor_col, span_end, prompt);
    if !default.is_empty() {
        cursor_col += write_spans_clipped_to_span(
            buffer,
            row,
            cursor_col,
            span_end,
            &[
                StyledSpan::new("[", classic::prompt_square_delimiter_style()),
                StyledSpan::new(default, classic::prompt_hotkey_style()),
                StyledSpan::new("]", classic::prompt_square_delimiter_style()),
                StyledSpan::new(" ", classic::prompt_style()),
            ],
        );
    }
    cursor_col = write_prompt_markup_in_span(buffer, row, cursor_col, span_end, cancel_markup);
    let _ = write_live_input_clipped_to_span(
        buffer,
        row,
        cursor_col,
        span_end,
        input,
        classic::prompt_hotkey_style(),
    );
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
    draw_labeled_table_command_bar_at_col(
        buffer,
        row,
        col,
        COMMAND_LABEL,
        hotkeys_markup,
        default,
        input,
    )
}

pub fn draw_labeled_table_command_bar_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    label: &str,
    hotkeys_markup: &str,
    default: Option<&str>,
    input: &str,
) -> usize {
    buffer.fill_row(row, classic::prompt_style());
    let mut cursor_col = col
        + buffer.write_spans_clipped(
            row,
            col,
            &[
                StyledSpan::new(label, classic::title_style()),
                StyledSpan::new(COMMAND_ARROW_PREFIX, classic::prompt_style()),
            ],
        );
    cursor_col = write_command_rail_tokens(buffer, row, cursor_col, hotkeys_markup);
    if let Some(default) = default {
        cursor_col += buffer.write_spans_clipped(
            row,
            cursor_col,
            &[
                StyledSpan::new(" ", classic::prompt_style()),
                StyledSpan::new(DEFAULT_OPEN, classic::prompt_square_delimiter_style()),
                StyledSpan::new(default, classic::prompt_hotkey_style()),
                StyledSpan::new("]", classic::prompt_square_delimiter_style()),
                StyledSpan::new(COMMAND_ARROW_SUFFIX, classic::prompt_style()),
            ],
        );
        write_live_input_clipped(
            buffer,
            row,
            cursor_col,
            input,
            classic::prompt_hotkey_style(),
        )
    } else {
        let written = buffer.write_text_clipped(
            row,
            cursor_col,
            COMMAND_ARROW_SUFFIX,
            classic::prompt_style(),
        );
        let final_cursor_col = if COMMAND_ARROW_SUFFIX.chars().count() > written {
            buffer.width().saturating_sub(1)
        } else {
            cursor_col + written
        };
        if final_cursor_col < buffer.width() {
            buffer.set_cursor(final_cursor_col as u16, row as u16);
        }
        final_cursor_col
    }
}

pub fn draw_table_command_bar_in_span(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    width: usize,
    hotkeys_markup: &str,
    default: Option<&str>,
    input: &str,
) -> usize {
    draw_labeled_table_command_bar_in_span(
        buffer,
        row,
        col,
        width,
        COMMAND_LABEL,
        hotkeys_markup,
        default,
        input,
    )
}

pub fn draw_labeled_table_command_bar_in_span(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    width: usize,
    label: &str,
    hotkeys_markup: &str,
    default: Option<&str>,
    input: &str,
) -> usize {
    fill_prompt_span(buffer, row, col, width);
    let span_end = col + width;
    let mut cursor_col = col
        + write_spans_clipped_to_span(
            buffer,
            row,
            col,
            span_end,
            &[
                StyledSpan::new(label, classic::title_style()),
                StyledSpan::new(COMMAND_ARROW_PREFIX, classic::prompt_style()),
            ],
        );
    cursor_col =
        write_command_rail_tokens_in_span(buffer, row, cursor_col, span_end, hotkeys_markup);
    if let Some(default) = default {
        cursor_col += write_spans_clipped_to_span(
            buffer,
            row,
            cursor_col,
            span_end,
            &[
                StyledSpan::new(" ", classic::prompt_style()),
                StyledSpan::new(DEFAULT_OPEN, classic::prompt_square_delimiter_style()),
                StyledSpan::new(default, classic::prompt_hotkey_style()),
                StyledSpan::new("]", classic::prompt_square_delimiter_style()),
                StyledSpan::new(COMMAND_ARROW_SUFFIX, classic::prompt_style()),
            ],
        );
        write_live_input_clipped_to_span(
            buffer,
            row,
            cursor_col,
            span_end,
            input,
            classic::prompt_hotkey_style(),
        )
    } else {
        let written = write_text_clipped_to_span(
            buffer,
            row,
            cursor_col,
            span_end,
            COMMAND_ARROW_SUFFIX,
            classic::prompt_style(),
        );
        let final_cursor_col = if COMMAND_ARROW_SUFFIX.chars().count() > written {
            span_end.saturating_sub(1)
        } else {
            cursor_col + written
        };
        if buffer.width() > 0 && width > 0 {
            let clamped = final_cursor_col.min(span_end - 1).min(buffer.width() - 1);
            buffer.set_cursor(clamped as u16, row as u16);
            clamped
        } else {
            final_cursor_col
        }
    }
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
        + buffer.write_spans_clipped(
            row,
            col,
            &[
                StyledSpan::new(COMMAND_LABEL, classic::title_style()),
                StyledSpan::new(COMMAND_ARROW_PREFIX, classic::prompt_style()),
            ],
        );
    let prompt = ensure_cursor_gap(prompt);
    let cursor_col = write_prompt_markup(buffer, row, prefix, &prompt);
    if contains_slap_a_key_phrase(&prompt) && buffer.width() > 0 {
        set_slap_cursor(buffer, row, cursor_col, buffer.width() - 1);
    } else if buffer.width() > 0 {
        buffer.set_cursor(cursor_col.min(buffer.width() - 1) as u16, row as u16);
    }
    cursor_col
}

pub fn draw_table_command_prompt_in_span(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    width: usize,
    prompt: &str,
) -> usize {
    fill_prompt_span(buffer, row, col, width);
    let prefix = col
        + write_spans_clipped_to_span(
            buffer,
            row,
            col,
            col + width,
            &[
                StyledSpan::new(COMMAND_LABEL, classic::title_style()),
                StyledSpan::new(COMMAND_ARROW_PREFIX, classic::prompt_style()),
            ],
        );
    let prompt = ensure_cursor_gap(prompt);
    let cursor_col = write_prompt_markup_in_span(buffer, row, prefix, col + width, &prompt);
    if contains_slap_a_key_phrase(&prompt) && buffer.width() > 0 && width > 0 {
        let clamped = (col + width - 1).min(buffer.width() - 1);
        set_slap_cursor(buffer, row, cursor_col, clamped)
    } else if buffer.width() > 0 && width > 0 {
        let clamped = cursor_col.min(col + width - 1).min(buffer.width() - 1);
        buffer.set_cursor(clamped as u16, row as u16);
        clamped
    } else {
        cursor_col
    }
}

pub fn draw_plain_prompt(buffer: &mut PlayfieldBuffer, row: usize, prompt: &str) -> usize {
    draw_plain_prompt_at_col(buffer, row, 0, prompt)
}

pub fn draw_plain_prompt_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    prompt: &str,
) -> usize {
    buffer.fill_row(row, classic::prompt_style());
    let prompt = ensure_cursor_gap(prompt);
    let cursor_col = write_prompt_markup(buffer, row, col, &prompt);
    if contains_slap_a_key_phrase(&prompt) && buffer.width() > 0 {
        set_slap_cursor(buffer, row, cursor_col, buffer.width() - 1);
    } else if buffer.width() > 0 {
        buffer.set_cursor(cursor_col.min(buffer.width() - 1) as u16, row as u16);
    }
    cursor_col
}

pub fn draw_plain_prompt_in_span(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    width: usize,
    prompt: &str,
) -> usize {
    fill_prompt_span(buffer, row, col, width);
    let prompt = ensure_cursor_gap(prompt);
    let cursor_col = write_prompt_markup_in_span(buffer, row, col, col + width, &prompt);
    if contains_slap_a_key_phrase(&prompt) && buffer.width() > 0 && width > 0 {
        let clamped = (col + width - 1).min(buffer.width() - 1);
        set_slap_cursor(buffer, row, cursor_col, clamped);
        clamped
    } else if buffer.width() > 0 && width > 0 {
        let clamped = cursor_col.min(col + width - 1).min(buffer.width() - 1);
        buffer.set_cursor(clamped as u16, row as u16);
        clamped
    } else {
        cursor_col
    }
}

pub fn draw_right_aligned_footer_text(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    occupied_until_col: usize,
    text: &str,
    style: crate::buffer::CellStyle,
) -> Option<usize> {
    if text.is_empty() || row >= buffer.height() {
        return None;
    }
    let text_width = text.chars().count();
    if text_width > buffer.width() {
        return None;
    }
    let start_col = buffer.width().saturating_sub(text_width);
    if start_col <= occupied_until_col {
        return None;
    }
    buffer.write_text(row, start_col, text, style);
    Some(start_col)
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

fn slap_cursor_padding(prompt: &str) -> usize {
    if contains_slap_a_key_phrase(prompt) {
        2
    } else {
        0
    }
}

fn fill_prompt_span(buffer: &mut PlayfieldBuffer, row: usize, col: usize, width: usize) {
    if width == 0 || row >= buffer.height() || col >= buffer.width() {
        return;
    }
    buffer.fill_rect(
        row,
        col,
        width.min(buffer.width().saturating_sub(col)),
        1,
        classic::prompt_style(),
    );
}

fn write_text_clipped_to_span(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    span_end: usize,
    text: &str,
    style: crate::buffer::CellStyle,
) -> usize {
    if row >= buffer.height() || col >= span_end || col >= buffer.width() || text.is_empty() {
        return 0;
    }
    let clip_width = span_end
        .saturating_sub(col)
        .min(buffer.width().saturating_sub(col));
    let clipped: String = text.chars().take(clip_width).collect();
    buffer.write_text(row, col, &clipped, style)
}

fn write_spans_clipped_to_span(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    mut col: usize,
    span_end: usize,
    spans: &[StyledSpan<'_>],
) -> usize {
    let start = col;
    for span in spans {
        if col >= span_end || col >= buffer.width() {
            break;
        }
        col += write_text_clipped_to_span(buffer, row, col, span_end, span.text, span.style);
    }
    col.saturating_sub(start)
}

fn command_rail_width(tokens: &str) -> usize {
    tokens
        .split_whitespace()
        .map(|token| token.chars().count())
        .sum::<usize>()
        + tokens.split_whitespace().count().saturating_sub(1)
}

fn write_live_input_clipped_to_span(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    span_end: usize,
    input: &str,
    style: crate::buffer::CellStyle,
) -> usize {
    if buffer.width() == 0 || span_end == 0 {
        return col;
    }
    let max_col = span_end.saturating_sub(1).min(buffer.width() - 1);
    if col >= span_end || col >= buffer.width() {
        buffer.set_cursor(max_col as u16, row as u16);
        return max_col;
    }

    let written = write_text_clipped_to_span(buffer, row, col, span_end, input, style);
    let input_width = input.chars().count();
    let cursor_col = if input_width > written {
        max_col
    } else {
        col + written
    };
    buffer.set_cursor(cursor_col.min(max_col) as u16, row as u16);
    cursor_col.min(max_col)
}

fn write_live_input_clipped(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    input: &str,
    style: crate::buffer::CellStyle,
) -> usize {
    if buffer.width() == 0 {
        return col;
    }
    if col >= buffer.width() {
        buffer.set_cursor((buffer.width() - 1) as u16, row as u16);
        return buffer.width() - 1;
    }

    let written = buffer.write_text_clipped(row, col, input, style);
    let input_width = input.chars().count();
    let cursor_col = if input_width > written {
        buffer.width() - 1
    } else {
        col + written
    };
    if cursor_col < buffer.width() {
        buffer.set_cursor(cursor_col as u16, row as u16);
    }
    cursor_col
}

fn set_slap_cursor(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    prompt_end_col: usize,
    max_col: usize,
) -> usize {
    let cursor_col = prompt_end_col.saturating_add(1).min(max_col);
    buffer.set_cursor(cursor_col as u16, row as u16);
    cursor_col
}

fn write_command_rail_tokens(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    start_col: usize,
    tokens: &str,
) -> usize {
    let mut col = start_col;
    let mut first = true;
    for token in tokens.split_whitespace() {
        if !first {
            col += buffer.write_text_clipped(row, col, " ", classic::prompt_style());
        }
        col += write_command_rail_token(buffer, row, col, token);
        first = false;
    }
    col
}

fn write_command_rail_tokens_in_span(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    start_col: usize,
    span_end: usize,
    tokens: &str,
) -> usize {
    let mut col = start_col;
    let mut first = true;
    for token in tokens.split_whitespace() {
        if !first {
            col += write_text_clipped_to_span(
                buffer,
                row,
                col,
                span_end,
                " ",
                classic::prompt_style(),
            );
        }
        col += write_command_rail_token_in_span(buffer, row, col, span_end, token);
        first = false;
    }
    col
}

fn write_command_rail_token(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    token: &str,
) -> usize {
    if let Some(inner) = token
        .strip_prefix('<')
        .and_then(|value| value.strip_suffix('>'))
    {
        buffer.write_spans_clipped(
            row,
            col,
            &[
                StyledSpan::new("<", classic::prompt_angle_delimiter_style()),
                StyledSpan::new(inner, classic::prompt_hotkey_style()),
                StyledSpan::new(">", classic::prompt_angle_delimiter_style()),
            ],
        )
    } else if let Some(inner) = token
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    {
        buffer.write_spans_clipped(
            row,
            col,
            &[
                StyledSpan::new("[", classic::prompt_square_delimiter_style()),
                StyledSpan::new(inner, classic::prompt_hotkey_style()),
                StyledSpan::new("]", classic::prompt_square_delimiter_style()),
            ],
        )
    } else {
        buffer.write_text_clipped(row, col, token, classic::prompt_hotkey_style())
    }
}

fn write_command_rail_token_in_span(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    span_end: usize,
    token: &str,
) -> usize {
    if let Some(inner) = token
        .strip_prefix('<')
        .and_then(|value| value.strip_suffix('>'))
    {
        write_spans_clipped_to_span(
            buffer,
            row,
            col,
            span_end,
            &[
                StyledSpan::new("<", classic::prompt_angle_delimiter_style()),
                StyledSpan::new(inner, classic::prompt_hotkey_style()),
                StyledSpan::new(">", classic::prompt_angle_delimiter_style()),
            ],
        )
    } else if let Some(inner) = token
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    {
        write_spans_clipped_to_span(
            buffer,
            row,
            col,
            span_end,
            &[
                StyledSpan::new("[", classic::prompt_square_delimiter_style()),
                StyledSpan::new(inner, classic::prompt_hotkey_style()),
                StyledSpan::new("]", classic::prompt_square_delimiter_style()),
            ],
        )
    } else {
        write_text_clipped_to_span(
            buffer,
            row,
            col,
            span_end,
            token,
            classic::prompt_hotkey_style(),
        )
    }
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
                col += buffer.write_text_clipped(row, col, &plain, classic::prompt_style());
                plain.clear();
            }
            if key_start > idx {
                let prefix = chars[idx..key_start].iter().collect::<String>();
                col += buffer.write_text_clipped(
                    row,
                    col,
                    &prefix,
                    classic::prompt_notice_action_style(),
                );
            }
            let key = chars[key_start..key_end].iter().collect::<String>();
            col += buffer.write_text_clipped(row, col, &key, classic::prompt_hotkey_style());
            idx = phrase_end;
            continue;
        }

        if chars[idx] == '<'
            && let Some(close_idx) = chars[idx + 1..].iter().position(|&ch| ch == '>')
        {
            let close_idx = idx + 1 + close_idx;
            if is_prompt_angle_hotkey(&chars[idx + 1..close_idx]) {
                if !plain.is_empty() {
                    col += buffer.write_text_clipped(row, col, &plain, classic::prompt_style());
                    plain.clear();
                }
                col += buffer.write_text_clipped(
                    row,
                    col,
                    "<",
                    classic::prompt_angle_delimiter_style(),
                );
                if close_idx > idx + 1 {
                    let segment = chars[idx + 1..close_idx].iter().collect::<String>();
                    col += buffer.write_text_clipped(
                        row,
                        col,
                        &segment,
                        classic::prompt_hotkey_style(),
                    );
                }
                col += buffer.write_text_clipped(
                    row,
                    col,
                    ">",
                    classic::prompt_angle_delimiter_style(),
                );
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
                    col += buffer.write_text_clipped(row, col, &plain, classic::prompt_style());
                    plain.clear();
                }
                col += buffer.write_text_clipped(
                    row,
                    col,
                    "[",
                    classic::prompt_square_delimiter_style(),
                );
                if close_idx > idx + 1 {
                    let segment = chars[idx + 1..close_idx].iter().collect::<String>();
                    col += buffer.write_text_clipped(
                        row,
                        col,
                        &segment,
                        classic::prompt_hotkey_style(),
                    );
                }
                col += buffer.write_text_clipped(
                    row,
                    col,
                    "]",
                    classic::prompt_square_delimiter_style(),
                );
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
                    col += buffer.write_text_clipped(row, col, &plain, classic::prompt_style());
                    plain.clear();
                }
                col += buffer.write_text_clipped(row, col, &token, classic::prompt_hotkey_style());
            } else {
                plain.push_str(&token);
            }
            continue;
        }

        plain.push(chars[idx]);
        idx += 1;
    }

    if !plain.is_empty() {
        col += buffer.write_text_clipped(row, col, &plain, classic::prompt_style());
    }

    col
}

fn write_prompt_markup_in_span(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    start_col: usize,
    span_end: usize,
    text: &str,
) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut col = start_col;
    let mut plain = String::new();
    let mut idx = 0usize;

    while idx < chars.len() {
        if let Some((phrase_end, key_start, key_end)) = slap_a_key_phrase(&chars, idx) {
            if !plain.is_empty() {
                col += write_text_clipped_to_span(
                    buffer,
                    row,
                    col,
                    span_end,
                    &plain,
                    classic::prompt_style(),
                );
                plain.clear();
            }
            if key_start > idx {
                let prefix = chars[idx..key_start].iter().collect::<String>();
                col += write_text_clipped_to_span(
                    buffer,
                    row,
                    col,
                    span_end,
                    &prefix,
                    classic::prompt_notice_action_style(),
                );
            }
            let key = chars[key_start..key_end].iter().collect::<String>();
            col += write_text_clipped_to_span(
                buffer,
                row,
                col,
                span_end,
                &key,
                classic::prompt_hotkey_style(),
            );
            idx = phrase_end;
            continue;
        }

        if chars[idx] == '<'
            && let Some(close_idx) = chars[idx + 1..].iter().position(|&ch| ch == '>')
        {
            let close_idx = idx + 1 + close_idx;
            if is_prompt_angle_hotkey(&chars[idx + 1..close_idx]) {
                if !plain.is_empty() {
                    col += write_text_clipped_to_span(
                        buffer,
                        row,
                        col,
                        span_end,
                        &plain,
                        classic::prompt_style(),
                    );
                    plain.clear();
                }
                col += write_text_clipped_to_span(
                    buffer,
                    row,
                    col,
                    span_end,
                    "<",
                    classic::prompt_angle_delimiter_style(),
                );
                if close_idx > idx + 1 {
                    let segment = chars[idx + 1..close_idx].iter().collect::<String>();
                    col += write_text_clipped_to_span(
                        buffer,
                        row,
                        col,
                        span_end,
                        &segment,
                        classic::prompt_hotkey_style(),
                    );
                }
                col += write_text_clipped_to_span(
                    buffer,
                    row,
                    col,
                    span_end,
                    ">",
                    classic::prompt_angle_delimiter_style(),
                );
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
                    col += write_text_clipped_to_span(
                        buffer,
                        row,
                        col,
                        span_end,
                        &plain,
                        classic::prompt_style(),
                    );
                    plain.clear();
                }
                col += write_text_clipped_to_span(
                    buffer,
                    row,
                    col,
                    span_end,
                    "[",
                    classic::prompt_square_delimiter_style(),
                );
                if close_idx > idx + 1 {
                    let segment = chars[idx + 1..close_idx].iter().collect::<String>();
                    col += write_text_clipped_to_span(
                        buffer,
                        row,
                        col,
                        span_end,
                        &segment,
                        classic::prompt_hotkey_style(),
                    );
                }
                col += write_text_clipped_to_span(
                    buffer,
                    row,
                    col,
                    span_end,
                    "]",
                    classic::prompt_square_delimiter_style(),
                );
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
                    col += write_text_clipped_to_span(
                        buffer,
                        row,
                        col,
                        span_end,
                        &plain,
                        classic::prompt_style(),
                    );
                    plain.clear();
                }
                col += write_text_clipped_to_span(
                    buffer,
                    row,
                    col,
                    span_end,
                    &token,
                    classic::prompt_hotkey_style(),
                );
            } else {
                plain.push_str(&token);
            }
            continue;
        }

        plain.push(chars[idx]);
        idx += 1;
    }

    if !plain.is_empty() {
        col +=
            write_text_clipped_to_span(buffer, row, col, span_end, &plain, classic::prompt_style());
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

fn contains_slap_a_key_phrase(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    (0..chars.len()).any(|idx| slap_a_key_phrase(&chars, idx).is_some())
}

#[cfg(test)]
mod tests {
    use super::{
        command_line_default_input_scaffold_width,
        command_line_default_input_scaffold_width_with_cancel, command_line_default_input_width,
        command_line_default_input_width_with_cancel, command_line_prompt_input_width,
        draw_command_line_default_input_at, draw_command_line_default_input_with_cancel_at,
        draw_command_line_default_input_with_cancel_in_span,
        draw_command_line_prompt_input_in_span, draw_command_line_prompt_text_at,
        draw_plain_prompt, draw_plain_prompt_in_span, draw_right_aligned_footer_text,
        draw_table_command_bar_at, draw_table_command_bar_in_span,
        table_command_bar_scaffold_width, table_command_bar_width, table_command_prompt_width,
    };
    use crate::buffer::PlayfieldBuffer;
    use crate::theme::classic;

    fn buffer() -> PlayfieldBuffer {
        PlayfieldBuffer::new(80, 25, classic::body_style())
    }

    fn find_in_row(buffer: &PlayfieldBuffer, row: usize, needle: &str) -> usize {
        buffer
            .plain_line(row)
            .find(needle)
            .expect("needle should be present")
    }

    #[test]
    fn plain_prompt_highlights_angle_and_square_tokens() {
        let mut buffer = buffer();
        draw_plain_prompt(&mut buffer, 24, "Delete report [Y]/N <ESC> ->");
        let row = buffer.row(24);
        let bracket = find_in_row(&buffer, 24, "[Y]");
        assert_eq!(row[bracket].style, classic::prompt_square_delimiter_style());
        assert_eq!(row[bracket + 1].style, classic::prompt_hotkey_style());
        let quit = find_in_row(&buffer, 24, "<ESC>");
        assert_eq!(row[quit].style, classic::prompt_angle_delimiter_style());
        assert_eq!(row[quit + 1].style, classic::prompt_hotkey_style());
    }

    #[test]
    fn table_command_bar_renders_commands_label() {
        let mut buffer = buffer();
        draw_table_command_bar_at(&mut buffer, 24, "J K ^U ^D <N> <ESC>", None, "");
        assert!(buffer.plain_line(24).starts_with("COMMAND <- "));
    }

    #[test]
    fn command_bar_in_span_does_not_write_outside_assigned_width() {
        let mut buffer = buffer();
        draw_table_command_bar_in_span(
            &mut buffer,
            12,
            10,
            24,
            "? J K ^U ^D <ESC>",
            Some("12,03"),
            "1",
        );

        let line = buffer.plain_line(12);
        assert_eq!(&line[..10], "          ");
        assert!(line[10..].contains("COMMAND <- "));
        assert_eq!(buffer.row(12)[9].ch, ' ');
    }

    #[test]
    fn default_input_in_span_keeps_cursor_inside_span() {
        let mut buffer = buffer();
        draw_command_line_default_input_with_cancel_in_span(
            &mut buffer,
            13,
            8,
            18,
            "COMMAND",
            "Fleet # ",
            "03",
            "123456789",
            "<ESC> -> ",
        );

        let cursor = buffer.cursor().expect("cursor should be set");
        assert!(usize::from(cursor.0) < 26);
        assert_eq!(usize::from(cursor.1), 13);
    }

    #[test]
    fn command_line_prompt_keeps_one_space_after_arrow_for_cursor() {
        let mut buffer = buffer();
        draw_command_line_prompt_text_at(&mut buffer, 24, "COMMAND", "Are you sure? Y/[N] ->");
        let (col, row) = buffer.cursor().expect("cursor");
        let line = buffer.plain_line(row as usize);
        let arrow = line.find("->").expect("arrow");
        assert_eq!(col as usize, arrow + "->".chars().count() + 1);
    }

    #[test]
    fn table_command_bar_width_matches_rendered_footer_width() {
        let mut buffer = buffer();
        let end_col = draw_table_command_bar_at(&mut buffer, 24, "J K ^U ^D <ESC>", Some("02"), "");
        assert_eq!(
            end_col,
            table_command_bar_width("J K ^U ^D <ESC>", Some("02"), "")
        );
    }

    #[test]
    fn prompt_width_helpers_include_default_and_input_span() {
        let width = command_line_default_input_width("COMMAND", "Qty ", "12", "345");
        assert!(width > table_command_prompt_width("Qty -> "));
        assert_eq!(width, "COMMAND <- Qty [12] <ESC> -> 345".chars().count());
    }

    #[test]
    fn prompt_width_helpers_support_custom_cancel_markup() {
        let width = command_line_default_input_width_with_cancel(
            "COMMAND",
            "Qty ",
            "12",
            "345",
            "<ESC> -> ",
        );
        assert_eq!(width, "COMMAND <- Qty [12] <ESC> -> 345".chars().count());
    }

    #[test]
    fn prompt_input_width_matches_rendered_confirmation_footer() {
        let width = command_line_prompt_input_width("COMMAND", "Confirm [Y]/N <ESC> -> ", "Y");
        assert_eq!(width, "COMMAND <- Confirm [Y]/N <ESC> -> Y".chars().count());
    }

    #[test]
    fn prompt_input_in_span_renders_yes_no_without_extra_default_block() {
        let mut buffer = buffer();
        draw_command_line_prompt_input_in_span(
            &mut buffer,
            10,
            0,
            80,
            "COMMAND",
            "Confirm [Y]/N <ESC> -> ",
            "",
        );

        let line = buffer.plain_line(10);
        assert!(line.contains("COMMAND <- Confirm [Y]/N <ESC> ->"));
        assert!(!line.contains("[Y] <ESC> ->"));
    }

    #[test]
    fn scaffold_width_helpers_ignore_live_input_text() {
        assert_eq!(
            table_command_bar_scaffold_width("J K ^U ^D <ESC>", Some("Matrix")),
            table_command_bar_width("J K ^U ^D <ESC>", Some("Matrix"), "")
        );
        assert_eq!(
            table_command_bar_scaffold_width("J K ^U ^D <ESC>", Some("Matrix")),
            table_command_bar_width("J K ^U ^D <ESC>", Some("Matrix"), "tokyo")
                - "tokyo".chars().count()
        );
        assert_eq!(
            command_line_default_input_scaffold_width("COMMAND", "Qty ", "12"),
            command_line_default_input_width("COMMAND", "Qty ", "12", "345")
                - "345".chars().count()
        );
        assert_eq!(
            command_line_default_input_scaffold_width_with_cancel(
                "COMMAND",
                "Qty ",
                "12",
                "<ESC> -> "
            ),
            command_line_default_input_width_with_cancel(
                "COMMAND",
                "Qty ",
                "12",
                "345",
                "<ESC> -> "
            ) - "345".chars().count()
        );
    }

    #[test]
    fn table_command_bar_clips_long_live_input_without_panicking() {
        let mut buffer = PlayfieldBuffer::new(40, 25, classic::body_style());
        let end_col = draw_table_command_bar_at(
            &mut buffer,
            24,
            "J K ^U ^D <ESC>",
            Some("One Dark"),
            "sdfsdflsdfasdfasldfdd",
        );
        let (cursor_col, cursor_row) = buffer.cursor().expect("cursor");
        assert_eq!(cursor_row, 24);
        assert_eq!(cursor_col as usize, 39);
        assert_eq!(end_col, 39);
        assert!(buffer.plain_line(24).starts_with("COMMAND <- "));
    }

    #[test]
    fn command_input_clips_long_live_input_without_panicking() {
        let mut buffer = PlayfieldBuffer::new(32, 25, classic::body_style());
        draw_command_line_default_input_at(
            &mut buffer,
            24,
            "COMMAND",
            "Qty ",
            "12",
            "12345678901234567890",
        );
        let (cursor_col, cursor_row) = buffer.cursor().expect("cursor");
        assert_eq!(cursor_row, 24);
        assert_eq!(cursor_col as usize, 31);
        assert!(buffer.plain_line(24).starts_with("COMMAND <- Qty "));
    }

    #[test]
    fn command_input_clips_long_prompt_scaffold_without_panicking() {
        let mut buffer = PlayfieldBuffer::new(40, 25, classic::body_style());
        draw_command_line_default_input_at(
            &mut buffer,
            24,
            "COMMAND",
            "Target coordinates for extraordinarily long fleet mission ",
            "1234567890",
            "",
        );
        let (cursor_col, cursor_row) = buffer.cursor().expect("cursor");
        assert_eq!(cursor_row, 24);
        assert_eq!(cursor_col as usize, 39);
        assert!(buffer.plain_line(24).starts_with("COMMAND <- "));
    }

    #[test]
    fn command_input_can_render_custom_cancel_markup() {
        let mut buffer = buffer();
        draw_command_line_default_input_with_cancel_at(
            &mut buffer,
            24,
            "COMMAND",
            "Qty ",
            "12",
            "",
            "<ESC> -> ",
        );
        assert!(
            buffer
                .plain_line(24)
                .contains("COMMAND <- Qty [12] <ESC> ->")
        );
    }

    #[test]
    fn plain_slap_a_key_prompt_sets_cursor_after_gap() {
        let mut buffer = buffer();
        draw_plain_prompt(&mut buffer, 24, "(slap a key)");
        assert!(!buffer.plain_line(24).contains("COMMAND"));
        let (cursor_col, cursor_row) = buffer.cursor().expect("cursor");
        let phrase_end = buffer
            .plain_line(24)
            .find("(slap a key)")
            .expect("slap a key")
            + "(slap a key)".chars().count();
        assert_eq!(cursor_row, 24);
        assert_eq!(cursor_col as usize, phrase_end + 1);
    }

    #[test]
    fn plain_prompt_clips_oversized_text_without_panicking() {
        let mut buffer = PlayfieldBuffer::new(32, 25, classic::body_style());
        draw_plain_prompt(
            &mut buffer,
            24,
            "This is a very long prompt with <ESC> and [Y] choices that should clip safely ->",
        );
        let (cursor_col, cursor_row) = buffer.cursor().expect("cursor");
        assert_eq!(cursor_row, 24);
        assert_eq!(cursor_col as usize, 31);
        assert!(buffer.plain_line(24).starts_with("This is a very long"));
    }

    #[test]
    fn plain_prompt_in_span_keeps_border_columns_intact() {
        let mut buffer = PlayfieldBuffer::new(40, 25, classic::body_style());
        buffer.set_cell(24, 9, '│', classic::table_chrome_style());
        buffer.set_cell(24, 24, '│', classic::table_chrome_style());

        draw_plain_prompt_in_span(&mut buffer, 24, 10, 14, "(slap a key)");

        assert_eq!(buffer.row(24)[9].ch, '│');
        assert_eq!(buffer.row(24)[24].ch, '│');
        assert!(buffer.plain_line(24).contains("(slap a key)"));
        assert!(!buffer.plain_line(24).contains("COMMAND"));
        assert_eq!(buffer.cursor().expect("cursor"), (23, 24));
    }

    #[test]
    fn slap_a_key_for_more_prompt_sets_cursor_after_gap() {
        let mut buffer = buffer();
        draw_command_line_prompt_text_at(&mut buffer, 24, "COMMAND", "(Slap a key for more)");
        let (cursor_col, cursor_row) = buffer.cursor().expect("cursor");
        let phrase_end = buffer
            .plain_line(24)
            .find("(Slap a key for more)")
            .expect("slap a key for more")
            + "(Slap a key for more)".chars().count();
        assert_eq!(cursor_row, 24);
        assert_eq!(cursor_col as usize, phrase_end + 1);
    }

    #[test]
    fn right_aligned_footer_text_draws_with_gap_after_existing_footer() {
        let mut buffer = buffer();
        let end_col = draw_table_command_bar_at(&mut buffer, 24, "J K ^U ^D <ESC>", None, "");

        let drawn = draw_right_aligned_footer_text(
            &mut buffer,
            24,
            end_col,
            "NC 1.0.0",
            classic::prompt_hotkey_style(),
        );

        assert!(drawn.is_some());
        assert!(buffer.plain_line(24).contains("NC 1.0.0"));
    }

    #[test]
    fn right_aligned_footer_text_skips_when_it_would_touch_existing_footer() {
        let mut buffer = PlayfieldBuffer::new(24, 25, classic::body_style());
        let end_col = draw_table_command_bar_at(&mut buffer, 24, "J K ^U ^D <ESC>", None, "");

        let drawn = draw_right_aligned_footer_text(
            &mut buffer,
            24,
            end_col,
            "NC 1.0.0",
            classic::prompt_hotkey_style(),
        );

        assert!(drawn.is_none());
        assert!(!buffer.plain_line(24).contains("NC 1.0.0"));
    }
}
