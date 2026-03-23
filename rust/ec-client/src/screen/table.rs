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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableRowState {
    Normal,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitTableRow {
    pub left_cells: Vec<String>,
    pub right_cells: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TableArea {
    row: usize,
    col: usize,
    width: usize,
}

impl TableArea {
    const fn new(row: usize, col: usize, width: usize) -> Self {
        Self { row, col, width }
    }
}

pub fn write_table_header(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    columns: &[TableColumn<'_>],
    style: crate::screen::CellStyle,
) {
    write_table_header_at(buffer, row, 0, columns, style);
}

pub fn write_table_row(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    columns: &[TableColumn<'_>],
    cells: &[&str],
    style: crate::screen::CellStyle,
) {
    write_table_row_at(buffer, row, 0, columns, cells, style);
}

#[allow(dead_code)]
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
    write_table_window_with_states(
        buffer,
        start_row,
        columns,
        rows,
        scroll_offset,
        visible_rows,
        header_style,
        body_style,
        None,
        None,
    );
}

pub fn write_table_window_with_cursor<'a>(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    columns: &[TableColumn<'a>],
    rows: &[Vec<String>],
    scroll_offset: usize,
    visible_rows: usize,
    header_style: crate::screen::CellStyle,
    body_style: crate::screen::CellStyle,
    // Absolute row index (0-based into `rows`) to highlight as selected.
    selected: Option<usize>,
) {
    write_table_window_with_states(
        buffer,
        start_row,
        columns,
        rows,
        scroll_offset,
        visible_rows,
        header_style,
        body_style,
        selected,
        None,
    );
}

pub fn write_table_window_with_states<'a>(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    columns: &[TableColumn<'a>],
    rows: &[Vec<String>],
    scroll_offset: usize,
    visible_rows: usize,
    header_style: crate::screen::CellStyle,
    body_style: crate::screen::CellStyle,
    selected: Option<usize>,
    row_states: Option<&[TableRowState]>,
) {
    let area = TableArea::new(start_row, 0, buffer.width());
    write_table_header_at(buffer, area.row, area.col, columns, header_style);
    buffer.write_text(
        area.row + 1,
        area.col,
        &table_divider(columns),
        crate::theme::classic::menu_style(),
    );

    render_standard_body(
        buffer,
        TableArea::new(area.row + 2, area.col, area.width),
        columns,
        rows,
        scroll_offset,
        visible_rows,
        body_style,
        selected,
        row_states,
    );
}

pub fn write_stacked_table_window_with_states<'a>(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    top_header_line: &str,
    columns: &[TableColumn<'a>],
    rows: &[Vec<String>],
    scroll_offset: usize,
    visible_rows: usize,
    header_style: crate::screen::CellStyle,
    body_style: crate::screen::CellStyle,
    selected: Option<usize>,
    row_states: Option<&[TableRowState]>,
) {
    let area = TableArea::new(start_row, 0, buffer.width());
    buffer.write_text(area.row, area.col, top_header_line, header_style);
    write_table_header_at(buffer, area.row + 1, area.col, columns, header_style);
    buffer.write_text(
        area.row + 2,
        area.col,
        &table_divider(columns),
        crate::theme::classic::menu_style(),
    );

    render_standard_body(
        buffer,
        TableArea::new(area.row + 3, area.col, area.width),
        columns,
        rows,
        scroll_offset,
        visible_rows,
        body_style,
        selected,
        row_states,
    );
}

pub fn write_split_table(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    left_columns: &[TableColumn<'_>],
    right_columns: &[TableColumn<'_>],
    rows: &[SplitTableRow],
    style: crate::screen::CellStyle,
) {
    let left_width = table_width(left_columns);
    let gap = 10;
    let right_col = left_width + gap;

    write_table_header_at(buffer, start_row, 0, left_columns, style);
    write_table_header_at(buffer, start_row, right_col, right_columns, style);
    buffer.write_text(
        start_row + 1,
        0,
        &format!(
            "{}{}{}",
            table_divider(left_columns),
            " ".repeat(gap),
            table_divider(right_columns)
        ),
        crate::theme::classic::menu_style(),
    );

    for (idx, row) in rows.iter().enumerate() {
        let left_refs = row
            .left_cells
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let right_refs = row
            .right_cells
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        write_table_row_at(
            buffer,
            start_row + 2 + idx,
            0,
            left_columns,
            &left_refs,
            style,
        );
        write_table_row_at(
            buffer,
            start_row + 2 + idx,
            right_col,
            right_columns,
            &right_refs,
            style,
        );
    }

    buffer.write_text(
        start_row + 2 + rows.len(),
        0,
        &format!(
            "{}{}{}",
            table_divider(left_columns),
            " ".repeat(gap),
            table_divider(right_columns)
        ),
        crate::theme::classic::menu_style(),
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

pub fn fleet_number_width(max_fleet_number: u16) -> usize {
    max_fleet_number.max(1).to_string().chars().count()
}

pub fn fleet_id_column_width(max_fleet_number: u16) -> usize {
    "ID".len().max(fleet_number_width(max_fleet_number))
}

pub fn format_fleet_number(fleet_number: u16, max_fleet_number: u16) -> String {
    let width = fleet_number_width(max_fleet_number);
    format!("{fleet_number:0width$}")
}

fn write_scroll_indicator(
    buffer: &mut PlayfieldBuffer,
    area: TableArea,
    visible_rows: usize,
    total_rows: usize,
    scroll_offset: usize,
    style: crate::screen::CellStyle,
) {
    if total_rows <= visible_rows || visible_rows == 0 || area.width == 0 {
        return;
    }

    let displayed_rows = usize::min(visible_rows, total_rows.saturating_sub(scroll_offset));
    if displayed_rows < 3 {
        return;
    }

    let col = area.col + area.width - 1;
    let last_row = area.row + displayed_rows - 1;
    let track_style = crate::theme::classic::menu_style();
    let track_top = area.row + 1;
    let track_bottom = last_row.saturating_sub(1);

    buffer.write_text(area.row, col, "^", style);
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

fn write_table_header_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    columns: &[TableColumn<'_>],
    style: crate::screen::CellStyle,
) {
    buffer.write_text(
        row,
        col,
        &format_table_row(columns, &header_cells(columns)),
        style,
    );
}

fn write_table_row_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    columns: &[TableColumn<'_>],
    cells: &[&str],
    style: crate::screen::CellStyle,
) {
    buffer.write_text(row, col, &format_table_row(columns, cells), style);
}

fn render_standard_body(
    buffer: &mut PlayfieldBuffer,
    area: TableArea,
    columns: &[TableColumn<'_>],
    rows: &[Vec<String>],
    scroll_offset: usize,
    visible_rows: usize,
    body_style: crate::screen::CellStyle,
    selected: Option<usize>,
    row_states: Option<&[TableRowState]>,
) {
    for (idx, row_cells) in rows
        .iter()
        .skip(scroll_offset)
        .take(visible_rows)
        .enumerate()
    {
        let abs_idx = scroll_offset + idx;
        let style = match row_states
            .and_then(|states| states.get(abs_idx))
            .copied()
            .unwrap_or(TableRowState::Normal)
        {
            TableRowState::Disabled if selected == Some(abs_idx) => {
                crate::theme::classic::selected_row_style()
            }
            TableRowState::Disabled => crate::theme::classic::disabled_row_style(),
            TableRowState::Normal if selected == Some(abs_idx) => {
                crate::theme::classic::selected_row_style()
            }
            TableRowState::Normal => body_style,
        };
        let refs = row_cells.iter().map(String::as_str).collect::<Vec<_>>();
        write_table_row_at(buffer, area.row + idx, area.col, columns, &refs, style);
    }

    write_scroll_indicator(
        buffer,
        area,
        visible_rows,
        rows.len(),
        scroll_offset,
        body_style,
    );
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

fn table_width(columns: &[TableColumn<'_>]) -> usize {
    columns.iter().map(|column| column.width).sum::<usize>() + columns.len().saturating_sub(1)
}
