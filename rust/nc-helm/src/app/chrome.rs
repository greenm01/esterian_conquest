use crate::{CellStyle, PlayfieldBuffer};

const H_LINE: char = '─';
const V_LINE: char = '│';
const TOP_LEFT: char = '╭';
const TOP_RIGHT: char = '╮';
const BOTTOM_LEFT: char = '╰';
const BOTTOM_RIGHT: char = '╯';
const TITLE_LEFT: char = '┐';
const TITLE_RIGHT: char = '┌';
const TITLE_LEFT_DOWN: char = '┘';
const TITLE_RIGHT_DOWN: char = '└';

pub fn fill_rect(
    buffer: &mut PlayfieldBuffer,
    left: usize,
    top: usize,
    width: usize,
    height: usize,
    style: CellStyle,
) {
    for row in top..top.saturating_add(height).min(buffer.height()) {
        for col in left..left.saturating_add(width).min(buffer.width()) {
            buffer.set_cell(row, col, ' ', style);
        }
    }
}

pub fn draw_panel(
    buffer: &mut PlayfieldBuffer,
    left: usize,
    top: usize,
    width: usize,
    height: usize,
    border_style: CellStyle,
    title_style: CellStyle,
    fill_style: Option<CellStyle>,
    top_title: Option<&str>,
    bottom_title: Option<&str>,
) {
    if width < 2 || height < 2 {
        return;
    }

    if let Some(style) = fill_style {
        fill_rect(buffer, left, top, width, height, style);
    }

    for col in left + 1..left + width - 1 {
        buffer.set_cell(top, col, H_LINE, border_style);
        buffer.set_cell(top + height - 1, col, H_LINE, border_style);
    }
    for row in top + 1..top + height - 1 {
        buffer.set_cell(row, left, V_LINE, border_style);
        buffer.set_cell(row, left + width - 1, V_LINE, border_style);
    }
    buffer.set_cell(top, left, TOP_LEFT, border_style);
    buffer.set_cell(top, left + width - 1, TOP_RIGHT, border_style);
    buffer.set_cell(top + height - 1, left, BOTTOM_LEFT, border_style);
    buffer.set_cell(top + height - 1, left + width - 1, BOTTOM_RIGHT, border_style);

    if let Some(title) = top_title {
        draw_top_tag(buffer, top, left + 2, width, title, border_style, title_style);
    }
    if let Some(title) = bottom_title {
        draw_bottom_tag(
            buffer,
            top + height - 1,
            left + 2,
            width,
            title,
            border_style,
            title_style,
        );
    }
}

pub fn draw_top_tag(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    available_width: usize,
    label: &str,
    border_style: CellStyle,
    title_style: CellStyle,
) -> usize {
    draw_tag(
        buffer,
        row,
        col,
        available_width,
        label,
        border_style,
        title_style,
        TITLE_LEFT,
        TITLE_RIGHT,
    )
}

pub fn draw_top_tag_right(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    left: usize,
    panel_width: usize,
    label: &str,
    border_style: CellStyle,
    title_style: CellStyle,
) -> usize {
    let Some(col) = top_tag_right_col(left, panel_width, label) else {
        return 0;
    };
    draw_top_tag(
        buffer,
        row,
        col,
        left.saturating_add(panel_width).saturating_sub(col),
        label,
        border_style,
        title_style,
    )
}

pub fn top_tag_width(label: &str) -> usize {
    label.chars().count() + 4
}

pub fn top_tag_right_col(left: usize, panel_width: usize, label: &str) -> Option<usize> {
    let width = top_tag_width(label);
    panel_width.checked_sub(width + 2).map(|offset| left + offset)
}

fn draw_bottom_tag(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    available_width: usize,
    label: &str,
    border_style: CellStyle,
    title_style: CellStyle,
) -> usize {
    draw_tag(
        buffer,
        row,
        col,
        available_width,
        label,
        border_style,
        title_style,
        TITLE_LEFT_DOWN,
        TITLE_RIGHT_DOWN,
    )
}

fn draw_tag(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    available_width: usize,
    label: &str,
    border_style: CellStyle,
    title_style: CellStyle,
    left_notch: char,
    right_notch: char,
) -> usize {
    if available_width < 7 || row >= buffer.height() || col >= buffer.width() {
        return 0;
    }

    let max_label_width = available_width.saturating_sub(4);
    if max_label_width == 0 {
        return 0;
    }
    let label = truncate_chars(label, max_label_width);
    let width = top_tag_width(&label);
    let label_width = label.chars().count();
    if col + width > buffer.width() {
        return 0;
    }

    buffer.set_cell(row, col, left_notch, border_style);
    buffer.set_cell(row, col + 1, ' ', title_style);
    buffer.write_text(row, col + 2, &label, title_style);
    buffer.set_cell(row, col + 2 + label_width, ' ', title_style);
    buffer.set_cell(row, col + 3 + label_width, right_notch, border_style);
    width
}

fn truncate_chars(value: &str, width: usize) -> String {
    value.chars().take(width).collect()
}
