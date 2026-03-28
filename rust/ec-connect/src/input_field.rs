use ec_ui::buffer::{CellStyle, PlayfieldBuffer};

pub fn draw_labeled_input_row(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label_col: usize,
    label: &str,
    input: &str,
    input_width: usize,
    label_style: CellStyle,
    input_style: CellStyle,
) -> usize {
    buffer.write_text_clipped(row, label_col, label, label_style);
    let input_col = label_col + label.chars().count() + 1;
    draw_input_tail(buffer, row, input_col, input_width, input, input_style)
}

pub fn draw_input_tail(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    width: usize,
    input: &str,
    style: CellStyle,
) -> usize {
    let width = width.max(1);
    let visible = visible_tail(input, width);
    for offset in 0..width {
        buffer.set_cell(row, col + offset, ' ', style);
    }
    let cursor_col = col + buffer.write_text_clipped(row, col, &visible, style);
    if cursor_col < buffer.width() {
        buffer.set_cursor(cursor_col as u16, row as u16);
    }
    cursor_col
}

pub fn input_width(inner_right: usize, input_col: usize) -> usize {
    inner_right
        .saturating_sub(input_col)
        .saturating_add(1)
        .max(1)
}

fn visible_tail(value: &str, width: usize) -> String {
    let len = value.chars().count();
    if len <= width {
        return value.to_string();
    }
    value.chars().skip(len - width).collect()
}
