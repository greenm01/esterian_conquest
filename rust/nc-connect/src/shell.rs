use nc_ui::buffer::PlayfieldBuffer;
use nc_ui::theme::classic;

pub const INNER_WIDTH: usize = 80;
pub const INNER_HEIGHT: usize = 25;
pub const OUTER_WIDTH: usize = INNER_WIDTH + 2;
pub const OUTER_HEIGHT: usize = INNER_HEIGHT + 2;
pub const INNER_ORIGIN_COL: usize = 1;
pub const INNER_ORIGIN_ROW: usize = 1;

pub fn outer_title() -> String {
    format!("NC CONNECT v{}", env!("CARGO_PKG_VERSION"))
}

pub fn terminal_fits_outer(width: usize, height: usize) -> bool {
    width >= OUTER_WIDTH && height >= OUTER_HEIGHT
}

pub fn wrap_inner_buffer_in_terminal(
    inner: &PlayfieldBuffer,
    right_label: Option<&str>,
    term_width: usize,
    term_height: usize,
    outside_hint: Option<&str>,
) -> PlayfieldBuffer {
    let width = term_width.max(OUTER_WIDTH);
    let height = term_height.max(OUTER_HEIGHT);
    let mut outer = PlayfieldBuffer::new(width, height, classic::body_style());
    let origin_col = width.saturating_sub(OUTER_WIDTH) / 2;
    let origin_row = height.saturating_sub(OUTER_HEIGHT) / 2;
    draw_outer_frame_at(&mut outer, origin_col, origin_row, right_label);

    for row in 0..inner.height() {
        for (col, cell) in inner.row(row).iter().enumerate() {
            outer.set_cell(
                origin_row + INNER_ORIGIN_ROW + row,
                origin_col + INNER_ORIGIN_COL + col,
                cell.ch,
                cell.style,
            );
        }
    }

    if let Some(hint) = outside_hint.filter(|hint| !hint.is_empty()) {
        let hint_row = origin_row + OUTER_HEIGHT + 1;
        if hint_row < height {
            let col = width.saturating_sub(hint.chars().count()) / 2;
            outer.write_text_clipped(hint_row, col, hint, classic::shell_label_style());
        }
    }

    if let Some((col, row)) = inner.cursor() {
        outer.set_cursor(
            col + (origin_col + INNER_ORIGIN_COL) as u16,
            row + (origin_row + INNER_ORIGIN_ROW) as u16,
        );
    }

    outer
}

fn draw_outer_frame_at(
    buffer: &mut PlayfieldBuffer,
    origin_col: usize,
    origin_row: usize,
    right_label: Option<&str>,
) {
    let chrome = classic::logo_style();
    let title_style = classic::shell_title_style();
    let right = origin_col + OUTER_WIDTH.saturating_sub(1);
    let bottom = origin_row + OUTER_HEIGHT.saturating_sub(1);

    for x in origin_col + 1..right {
        buffer.set_cell(origin_row, x, '─', chrome);
        buffer.set_cell(bottom, x, '─', chrome);
    }
    for y in origin_row + 1..bottom {
        buffer.set_cell(y, origin_col, '│', chrome);
        buffer.set_cell(y, right, '│', chrome);
    }
    buffer.set_cell(origin_row, origin_col, '┌', chrome);
    buffer.set_cell(origin_row, right, '┐', chrome);
    buffer.set_cell(bottom, origin_col, '└', chrome);
    buffer.set_cell(bottom, right, '┘', chrome);

    let title = format!(" {} ", outer_title());
    buffer.write_text_clipped(origin_row, origin_col + 2, &title, title_style);
    if let Some(label) = right_label.filter(|label| !label.is_empty()) {
        let left_end = origin_col + 2 + title.chars().count();
        let max_segment = right.saturating_sub(left_end + 2);
        if max_segment >= 3 {
            let visible = truncate(label, max_segment.saturating_sub(2));
            let segment = format!(" {} ", visible);
            let col = right.saturating_sub(segment.chars().count());
            if col > left_end {
                buffer.write_text_clipped(origin_row, col, &segment, classic::shell_label_style());
            }
        }
    }
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    if max <= 1 {
        return "…".to_string();
    }
    let mut out = value
        .chars()
        .take(max.saturating_sub(1))
        .collect::<String>();
    out.push('…');
    out
}
