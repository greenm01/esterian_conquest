use crate::screen::PlayfieldBuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableAlign {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableColumn<'a> {
    pub header: &'a str,
    pub width: usize,
    pub align: TableAlign,
}

impl<'a> TableColumn<'a> {
    pub const fn left(header: &'a str, width: usize) -> Self {
        Self {
            header,
            width,
            align: TableAlign::Left,
        }
    }

    pub const fn right(header: &'a str, width: usize) -> Self {
        Self {
            header,
            width,
            align: TableAlign::Right,
        }
    }
}

pub fn write_table_header(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    columns: &[TableColumn<'_>],
    style: crate::screen::CellStyle,
) {
    buffer.write_text(row, 0, &format_table_row(columns, &header_cells(columns)), style);
}

pub fn write_table_row(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    columns: &[TableColumn<'_>],
    cells: &[&str],
    style: crate::screen::CellStyle,
) {
    buffer.write_text(row, 0, &format_table_row(columns, cells), style);
}

pub fn write_table_window<'a>(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    columns: &[TableColumn<'a>],
    rows: &[Vec<String>],
    scroll_offset: usize,
    visible_rows: usize,
    header_style: crate::screen::CellStyle,
    body_style: crate::screen::CellStyle,
) {
    write_table_header(buffer, start_row, columns, header_style);
    buffer.write_text(
        start_row + 1,
        0,
        &table_divider(columns),
        crate::theme::classic::menu_style(),
    );

    for (idx, row_cells) in rows.iter().skip(scroll_offset).take(visible_rows).enumerate() {
        let refs = row_cells.iter().map(String::as_str).collect::<Vec<_>>();
        write_table_row(buffer, start_row + 2 + idx, columns, &refs, body_style);
    }

    write_scroll_indicator(
        buffer,
        start_row + 2,
        visible_rows,
        rows.len(),
        scroll_offset,
        body_style,
    );
}

pub fn table_divider(columns: &[TableColumn<'_>]) -> String {
    let mut out = String::new();
    for (idx, column) in columns.iter().enumerate() {
        if idx != 0 {
            out.push(' ');
        }
        out.push_str(&"-".repeat(column.width));
    }
    out
}

pub fn format_empire_id(empire_id: u8) -> String {
    format!("{empire_id:02}")
}

fn write_scroll_indicator(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    visible_rows: usize,
    total_rows: usize,
    scroll_offset: usize,
    style: crate::screen::CellStyle,
) {
    if total_rows <= visible_rows || visible_rows == 0 || buffer.width() == 0 {
        return;
    }

    let displayed_rows = usize::min(visible_rows, total_rows.saturating_sub(scroll_offset));
    if displayed_rows < 3 {
        return;
    }

    let col = buffer.width() - 1;
    let last_row = start_row + displayed_rows - 1;
    let track_style = crate::theme::classic::menu_style();
    let track_top = start_row + 1;
    let track_bottom = last_row.saturating_sub(1);

    buffer.write_text(start_row, col, "^", style);
    buffer.write_text(last_row, col, "v", style);

    for row in track_top..=track_bottom {
        buffer.write_text(row, col, "|", track_style);
    }

    let max_offset = total_rows.saturating_sub(visible_rows);
    let thumb_top = track_top;
    let thumb_bottom = track_bottom;
    let thumb_span = thumb_bottom.saturating_sub(thumb_top);
    let thumb_row = if max_offset == 0 || thumb_span == 0 {
        thumb_top
    } else {
        thumb_top + (scroll_offset * thumb_span) / max_offset
    };
    buffer.write_text(thumb_row, col, "#", style);
}

fn header_cells<'a>(columns: &'a [TableColumn<'a>]) -> Vec<&'a str> {
    columns.iter().map(|column| column.header).collect()
}

fn format_table_row(columns: &[TableColumn<'_>], cells: &[&str]) -> String {
    let mut out = String::new();

    for (idx, column) in columns.iter().enumerate() {
        if idx != 0 {
            out.push(' ');
        }
        let cell = cells.get(idx).copied().unwrap_or("");
        out.push_str(&format_cell(cell, *column));
    }

    out
}

fn format_cell(cell: &str, column: TableColumn<'_>) -> String {
    let text = truncate_to_width(cell, column.width);
    match column.align {
        TableAlign::Left => format!("{text:<width$}", width = column.width),
        TableAlign::Right => format!("{text:>width$}", width = column.width),
    }
}

fn truncate_to_width(value: &str, width: usize) -> String {
    value.chars().take(width).collect::<String>()
}
