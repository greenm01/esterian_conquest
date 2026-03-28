use ec_ui::buffer::{CellStyle, PlayfieldBuffer};
use ec_ui::theme::classic;

pub const PLAYFIELD_WIDTH: usize = 80;
pub const PLAYFIELD_HEIGHT: usize = 25;
pub const TABLE_RIGHT: usize = 78;
pub const TABLE_SCROLL_COL: usize = 79;
pub const TITLE_ROW: usize = 0;
pub const TABLE_TOP_ROW: usize = 1;
pub const HEADER_ROW: usize = 2;
pub const DIVIDER_ROW: usize = 3;
pub const BODY_START_ROW: usize = 4;
pub const BODY_END_ROW: usize = 22;
pub const BOTTOM_ROW: usize = 23;
pub const COMMAND_ROW: usize = 24;
pub const BODY_ROWS: usize = BODY_END_ROW - BODY_START_ROW + 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Column<'a> {
    pub title: &'a str,
    pub width: usize,
}

pub fn draw_title(buffer: &mut PlayfieldBuffer, title: &str, right_label: Option<&str>) {
    buffer.fill_row(TITLE_ROW, classic::title_style());
    let title_col = buffer.width().saturating_sub(title.chars().count()) / 2;
    buffer.write_text_clipped(TITLE_ROW, title_col, title, classic::title_style());
    if let Some(label) = right_label.filter(|label| !label.is_empty()) {
        let truncated = truncate(label, 18);
        let col = buffer.width().saturating_sub(truncated.chars().count() + 1);
        buffer.write_text_clipped(TITLE_ROW, col, &truncated, classic::prompt_hotkey_style());
    }
}

pub fn draw_box(
    buffer: &mut PlayfieldBuffer,
    rect: Rect,
    title: &str,
    chrome_style: CellStyle,
    title_style: CellStyle,
) {
    if rect.width < 2 || rect.height < 2 {
        return;
    }
    let left = rect.x as usize;
    let top = rect.y as usize;
    let right = left + rect.width as usize - 1;
    let bottom = top + rect.height as usize - 1;
    for x in left + 1..right {
        buffer.set_cell(top, x, '─', chrome_style);
        buffer.set_cell(bottom, x, '─', chrome_style);
    }
    for y in top + 1..bottom {
        buffer.set_cell(y, left, '│', chrome_style);
        buffer.set_cell(y, right, '│', chrome_style);
    }
    buffer.set_cell(top, left, '┌', chrome_style);
    buffer.set_cell(top, right, '┐', chrome_style);
    buffer.set_cell(bottom, left, '└', chrome_style);
    buffer.set_cell(bottom, right, '┘', chrome_style);
    if !title.is_empty() && rect.width > 4 {
        buffer.write_text_clipped(top, left + 2, title, title_style);
    }
}

pub fn draw_table_frame(buffer: &mut PlayfieldBuffer, columns: &[Column<'_>]) {
    draw_horizontal_line(buffer, TABLE_TOP_ROW, '┌', '┬', '┐', columns);
    draw_header_row(buffer, columns);
    draw_horizontal_line(buffer, DIVIDER_ROW, '├', '┼', '┤', columns);
    for row in BODY_START_ROW..=BODY_END_ROW {
        buffer.set_cell(row, 0, '│', classic::table_chrome_style());
        let mut col = 1;
        for column in columns {
            col += column.width;
            buffer.set_cell(row, col, '│', classic::table_chrome_style());
            col += 1;
        }
    }
    draw_horizontal_line(buffer, BOTTOM_ROW, '└', '┴', '┘', columns);
}

fn draw_horizontal_line(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    left: char,
    join: char,
    right: char,
    columns: &[Column<'_>],
) {
    let style = classic::table_chrome_style();
    buffer.set_cell(row, 0, left, style);
    let mut col = 1usize;
    for (idx, column) in columns.iter().enumerate() {
        for _ in 0..column.width {
            buffer.set_cell(row, col, '─', style);
            col += 1;
        }
        let glyph = if idx + 1 == columns.len() {
            right
        } else {
            join
        };
        buffer.set_cell(row, col, glyph, style);
        col += 1;
    }
}

fn draw_header_row(buffer: &mut PlayfieldBuffer, columns: &[Column<'_>]) {
    let mut col = 1usize;
    for column in columns {
        buffer.write_text_clipped(
            HEADER_ROW,
            col,
            &pad_right(column.title, column.width),
            classic::table_header_style(),
        );
        col += column.width;
        buffer.set_cell(HEADER_ROW, col, '│', classic::table_chrome_style());
        col += 1;
    }
    buffer.set_cell(HEADER_ROW, 0, '│', classic::table_chrome_style());
}

pub fn draw_scroll_gutter(
    buffer: &mut PlayfieldBuffer,
    start: usize,
    visible_rows: usize,
    total: usize,
) {
    if total <= visible_rows {
        return;
    }
    if start > 0 {
        buffer.write_text_clipped(
            BODY_START_ROW,
            TABLE_SCROLL_COL,
            "↑",
            classic::notice_style(),
        );
    }
    if start + visible_rows < total {
        buffer.write_text_clipped(BODY_END_ROW, TABLE_SCROLL_COL, "↓", classic::notice_style());
    }
}

pub fn scroll_start(selected: usize, visible_rows: usize, total_rows: usize) -> usize {
    if visible_rows == 0 || total_rows <= visible_rows {
        return 0;
    }
    let half = visible_rows / 2;
    selected
        .saturating_sub(half)
        .min(total_rows.saturating_sub(visible_rows))
}

pub fn pad_right(value: &str, width: usize) -> String {
    let mut out = truncate(value, width);
    while out.chars().count() < width {
        out.push(' ');
    }
    out
}

pub fn middle_ellipsis(value: &str, width: usize, left: usize, right: usize) -> String {
    let len = value.chars().count();
    if len <= width {
        return value.to_string();
    }
    if width <= 1 {
        return "…".to_string();
    }
    let left = left.min(width.saturating_sub(1));
    let right = right.min(width.saturating_sub(left + 1));
    let chars: Vec<char> = value.chars().collect();
    format!(
        "{}…{}",
        chars[..left].iter().collect::<String>(),
        chars[chars.len().saturating_sub(right)..]
            .iter()
            .collect::<String>()
    )
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    if max <= 1 {
        return "…".to_string();
    }
    let mut out = s.chars().take(max.saturating_sub(1)).collect::<String>();
    out.push('…');
    out
}

pub fn short_npub(value: &str) -> String {
    middle_ellipsis(value, 25, 16, 8)
}

pub fn draw_centered_text(buffer: &mut PlayfieldBuffer, row: usize, text: &str, style: CellStyle) {
    let col = buffer.width().saturating_sub(text.chars().count()) / 2;
    buffer.write_text_clipped(row, col, text, style);
}

pub fn centered_rect(width: u16, height: u16, parent: Rect) -> Rect {
    let width = width.min(parent.width);
    let height = height.min(parent.height);
    let x = parent.x + parent.width.saturating_sub(width) / 2;
    let y = parent.y + parent.height.saturating_sub(height) / 2;
    Rect::new(x, y, width, height)
}

pub fn relative_time(_timestamp: Option<&str>) -> String {
    String::new()
}
