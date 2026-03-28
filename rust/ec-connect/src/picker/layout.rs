use ec_ui::buffer::{CellStyle, PlayfieldBuffer};
use ec_ui::theme::classic;

pub const PLAYFIELD_WIDTH: usize = crate::shell::INNER_WIDTH;
pub const PLAYFIELD_HEIGHT: usize = crate::shell::INNER_HEIGHT;
pub const TITLE_ROW: usize = 0;
pub const TABLE_TOP_ROW: usize = 0;
pub const HEADER_ROW: usize = 1;
pub const DIVIDER_ROW: usize = 2;
pub const BODY_START_ROW: usize = 3;
pub const BODY_END_ROW: usize = 22;
pub const INNER_COMMAND_ROW: usize = 24;
pub const MAX_BODY_ROWS: usize = 20;
const MIN_BODY_ROWS: usize = 1;
const HELP_COMMAND_WIDTH: usize = 6;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableMetrics {
    pub table_width: usize,
    pub right_border_col: usize,
    pub scroll_col: usize,
    pub displayed_rows: usize,
    pub bottom_row: usize,
    pub command_row: usize,
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
        let bordered = format!(" {title} ");
        buffer.write_text_clipped(top, left + 2, &bordered, title_style);
    }
}

pub fn draw_table_frame(
    buffer: &mut PlayfieldBuffer,
    columns: &[Column<'_>],
    displayed_rows: usize,
) -> TableMetrics {
    let displayed_rows = displayed_rows.clamp(MIN_BODY_ROWS, MAX_BODY_ROWS);
    draw_horizontal_line(buffer, TABLE_TOP_ROW, '┌', '┬', '┐', columns);
    draw_header_row(buffer, columns);
    draw_horizontal_line(buffer, DIVIDER_ROW, '├', '┼', '┤', columns);
    let bottom_row = DIVIDER_ROW + displayed_rows + 1;
    for row in BODY_START_ROW..bottom_row {
        buffer.set_cell(row, 0, '│', classic::table_chrome_style());
        let mut col = 1;
        for column in columns {
            col += column.width;
            buffer.set_cell(row, col, '│', classic::table_chrome_style());
            col += 1;
        }
    }
    draw_horizontal_line(buffer, bottom_row, '└', '┴', '┘', columns);
    let table_width = table_render_width(columns);
    let right_border_col = table_width.saturating_sub(1);
    let scroll_col = table_width.min(buffer.width().saturating_sub(1));
    TableMetrics {
        table_width,
        right_border_col,
        scroll_col,
        displayed_rows,
        bottom_row,
        command_row: table_command_row(bottom_row),
    }
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
    metrics: TableMetrics,
    start: usize,
    total: usize,
) {
    if total <= metrics.displayed_rows || metrics.displayed_rows < 3 {
        return;
    }

    let chrome = classic::table_chrome_style();
    let track = classic::body_style();
    let thumb = classic::scrollbar_thumb_style();
    let top_row = BODY_START_ROW;
    let last_row = BODY_START_ROW + metrics.displayed_rows - 1;
    buffer.write_text_clipped(top_row, metrics.scroll_col, "^", chrome);
    buffer.write_text_clipped(last_row, metrics.scroll_col, "v", chrome);
    for row in top_row + 1..last_row {
        buffer.write_text_clipped(row, metrics.scroll_col, "|", track);
    }

    let max_offset = total.saturating_sub(metrics.displayed_rows);
    let thumb_top = top_row + 1;
    let thumb_bottom = last_row.saturating_sub(1);
    let thumb_span = thumb_bottom.saturating_sub(thumb_top);
    let thumb_row = if max_offset == 0 || thumb_span == 0 {
        thumb_top
    } else {
        thumb_top + (start * thumb_span) / max_offset
    };
    buffer.write_text_clipped(thumb_row, metrics.scroll_col, "#", thumb);
}

pub fn displayed_body_rows(total_rows: usize, scroll_offset: usize) -> usize {
    total_rows
        .saturating_sub(scroll_offset)
        .clamp(MIN_BODY_ROWS, MAX_BODY_ROWS)
}

pub fn table_render_width(columns: &[Column<'_>]) -> usize {
    columns.iter().map(|column| column.width).sum::<usize>() + columns.len() + 1
}

pub fn table_command_row(bottom_row: usize) -> usize {
    (bottom_row + 1).min(INNER_COMMAND_ROW)
}

pub fn table_message_col(columns: &[Column<'_>], message: &str) -> usize {
    table_render_width(columns).saturating_sub(message.chars().count()) / 2
}

pub fn format_help_row(command: &str, description: &str) -> String {
    format!("{command:<HELP_COMMAND_WIDTH$} {description}")
}

pub fn table_cell_start(columns: &[Column<'_>], index: usize) -> Option<usize> {
    if index >= columns.len() {
        return None;
    }
    let mut col = 1usize;
    for column in &columns[..index] {
        col += column.width + 1;
    }
    Some(col)
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
