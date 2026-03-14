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
