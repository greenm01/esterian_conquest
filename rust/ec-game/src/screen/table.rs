use crate::screen::PlayfieldBuffer;
use crate::theme::classic;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableAlign {
    Left,
    Center,
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

    pub const fn center(header: &'a str, width: usize) -> Self {
        Self {
            header,
            width,
            align: TableAlign::Center,
        }
    }
}

pub fn fit_table_columns<'a>(
    columns: &[TableColumn<'a>],
    rows: &[Vec<String>],
) -> Vec<TableColumn<'a>> {
    columns
        .iter()
        .enumerate()
        .map(|(index, column)| {
            let content_width = rows
                .iter()
                .filter_map(|row| row.get(index))
                .filter(|cell| !cell.trim().is_empty())
                .map(|cell| cell.chars().count())
                .max()
                .unwrap_or(0);
            TableColumn {
                header: column.header,
                width: column.header.chars().count().max(content_width),
                align: column.align,
            }
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableRowState {
    Normal,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableRenderMetrics {
    pub bottom_row: usize,
    pub displayed_rows: usize,
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
    write_table_header_at(buffer, row, 0, columns, style, style);
}

pub fn table_column_start(columns: &[TableColumn<'_>], index: usize) -> Option<usize> {
    if index >= columns.len() {
        return None;
    }
    Some(column_start(columns, index))
}

pub fn write_table_row(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    columns: &[TableColumn<'_>],
    cells: &[&str],
    style: crate::screen::CellStyle,
) {
    write_table_row_at(buffer, row, 0, columns, cells, style, style);
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
) -> TableRenderMetrics {
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
        0,
        None,
    )
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
    selection_col: usize,
) -> TableRenderMetrics {
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
        selection_col,
        None,
    )
}

pub fn write_table_window_with_states<'a>(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    columns: &[TableColumn<'a>],
    rows: &[Vec<String>],
    scroll_offset: usize,
    visible_rows: usize,
    _header_style: crate::screen::CellStyle,
    _body_style: crate::screen::CellStyle,
    selected: Option<usize>,
    selection_col: usize,
    row_states: Option<&[TableRowState]>,
) -> TableRenderMetrics {
    write_table_window_with_states_at(
        buffer,
        start_row,
        0,
        columns,
        rows,
        scroll_offset,
        visible_rows,
        _header_style,
        _body_style,
        selected,
        selection_col,
        row_states,
    )
}

pub fn write_table_window_with_states_at<'a>(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    start_col: usize,
    columns: &[TableColumn<'a>],
    rows: &[Vec<String>],
    scroll_offset: usize,
    visible_rows: usize,
    _header_style: crate::screen::CellStyle,
    _body_style: crate::screen::CellStyle,
    selected: Option<usize>,
    selection_col: usize,
    row_states: Option<&[TableRowState]>,
) -> TableRenderMetrics {
    let area = TableArea::new(
        start_row,
        start_col,
        buffer.width().saturating_sub(start_col),
    );
    let header_style = classic::table_header_style();
    let chrome_style = classic::table_chrome_style();
    let body_style = classic::table_body_style();
    buffer.write_text(area.row, area.col, &table_top_border(columns), chrome_style);
    write_table_header_at(
        buffer,
        area.row + 1,
        area.col,
        columns,
        header_style,
        chrome_style,
    );
    buffer.write_text(
        area.row + 2,
        area.col,
        &table_divider(columns),
        chrome_style,
    );

    let displayed_rows = render_standard_body(
        buffer,
        TableArea::new(area.row + 3, area.col, area.width),
        columns,
        rows,
        scroll_offset,
        visible_rows,
        body_style,
        chrome_style,
        selected,
        selection_col,
        row_states,
    );
    let bottom_row = area.row + 3 + displayed_rows;
    buffer.write_text(
        bottom_row,
        area.col,
        &table_bottom_border(columns),
        chrome_style,
    );
    TableRenderMetrics {
        bottom_row,
        displayed_rows,
    }
}

pub fn table_render_width(columns: &[TableColumn<'_>]) -> usize {
    columns.iter().map(|column| column.width).sum::<usize>() + columns.len() + 1
}

pub fn centered_table_start_col(total_width: usize, columns: &[TableColumn<'_>]) -> usize {
    total_width.saturating_sub(table_render_width(columns)) / 2
}

pub fn write_stacked_table_window_with_states<'a>(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    top_header_cells: &[&str],
    columns: &[TableColumn<'a>],
    rows: &[Vec<String>],
    scroll_offset: usize,
    visible_rows: usize,
    _header_style: crate::screen::CellStyle,
    _body_style: crate::screen::CellStyle,
    selected: Option<usize>,
    selection_col: usize,
    row_states: Option<&[TableRowState]>,
) -> TableRenderMetrics {
    write_stacked_table_window_with_states_at(
        buffer,
        start_row,
        0,
        top_header_cells,
        columns,
        rows,
        scroll_offset,
        visible_rows,
        _header_style,
        _body_style,
        selected,
        selection_col,
        row_states,
    )
}

pub fn write_stacked_table_window_with_states_at<'a>(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    start_col: usize,
    top_header_cells: &[&str],
    columns: &[TableColumn<'a>],
    rows: &[Vec<String>],
    scroll_offset: usize,
    visible_rows: usize,
    _header_style: crate::screen::CellStyle,
    _body_style: crate::screen::CellStyle,
    selected: Option<usize>,
    selection_col: usize,
    row_states: Option<&[TableRowState]>,
) -> TableRenderMetrics {
    let area = TableArea::new(
        start_row,
        start_col,
        buffer.width().saturating_sub(start_col),
    );
    let header_style = classic::table_header_style();
    let chrome_style = classic::table_chrome_style();
    let body_style = classic::table_body_style();
    buffer.write_text(area.row, area.col, &table_top_border(columns), chrome_style);
    write_table_row_at(
        buffer,
        area.row + 1,
        area.col,
        columns,
        top_header_cells,
        header_style,
        chrome_style,
    );
    write_table_header_at(
        buffer,
        area.row + 2,
        area.col,
        columns,
        header_style,
        chrome_style,
    );
    buffer.write_text(
        area.row + 3,
        area.col,
        &table_divider(columns),
        chrome_style,
    );

    let displayed_rows = render_standard_body(
        buffer,
        TableArea::new(area.row + 4, area.col, area.width),
        columns,
        rows,
        scroll_offset,
        visible_rows,
        body_style,
        chrome_style,
        selected,
        selection_col,
        row_states,
    );
    let bottom_row = area.row + 4 + displayed_rows;
    buffer.write_text(
        bottom_row,
        area.col,
        &table_bottom_border(columns),
        chrome_style,
    );
    TableRenderMetrics {
        bottom_row,
        displayed_rows,
    }
}

pub fn write_split_table(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    left_columns: &[TableColumn<'_>],
    right_columns: &[TableColumn<'_>],
    rows: &[SplitTableRow],
    style: crate::screen::CellStyle,
) -> TableRenderMetrics {
    let combined_columns = left_columns
        .iter()
        .chain(right_columns.iter())
        .copied()
        .collect::<Vec<_>>();
    let combined_rows = rows
        .iter()
        .map(|row| {
            row.left_cells
                .iter()
                .chain(row.right_cells.iter())
                .cloned()
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    write_table_window_with_states(
        buffer,
        start_row,
        &combined_columns,
        &combined_rows,
        0,
        combined_rows.len(),
        style,
        style,
        None,
        0,
        None,
    )
}

pub fn table_divider(columns: &[TableColumn<'_>]) -> String {
    table_rule(columns, '├', '┼', '┤')
}

fn table_top_border(columns: &[TableColumn<'_>]) -> String {
    table_rule(columns, '┌', '┬', '┐')
}

fn table_bottom_border(columns: &[TableColumn<'_>]) -> String {
    table_rule(columns, '└', '┴', '┘')
}

fn table_rule(columns: &[TableColumn<'_>], left: char, join: char, right: char) -> String {
    let mut out = String::new();
    out.push(left);
    for (idx, column) in columns.iter().enumerate() {
        if idx != 0 {
            out.push(join);
        }
        out.push_str(&"─".repeat(column.width));
    }
    out.push(right);
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
    table_width: usize,
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

    let max_col = buffer.width().saturating_sub(1);
    let right_border_col = area.col + table_width.saturating_sub(1);
    assert!(
        right_border_col < max_col,
        "scrollable table must leave a gutter to the right of its border"
    );
    let col = area.col.saturating_add(table_width).min(max_col);
    assert!(
        col > right_border_col,
        "scrollbar must render strictly to the right of the table border"
    );
    let last_row = area.row + displayed_rows - 1;
    let track_style = crate::theme::classic::body_style();
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
    buffer.write_text(
        thumb_row,
        col,
        "#",
        crate::theme::classic::scrollbar_thumb_style(),
    );
}

fn header_cells<'a>(columns: &'a [TableColumn<'a>]) -> Vec<&'a str> {
    columns.iter().map(|column| column.header).collect()
}

fn write_table_header_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    columns: &[TableColumn<'_>],
    cell_style: crate::screen::CellStyle,
    chrome_style: crate::screen::CellStyle,
) {
    let cells = header_cells(columns);
    write_table_row_at(buffer, row, col, columns, &cells, cell_style, chrome_style);
}

fn write_table_row_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    columns: &[TableColumn<'_>],
    cells: &[&str],
    cell_style: crate::screen::CellStyle,
    chrome_style: crate::screen::CellStyle,
) {
    buffer.write_text(row, col, &format_table_row(columns, cells), chrome_style);
    for (idx, column) in columns.iter().enumerate() {
        let start = col + column_start(columns, idx);
        let cell = cells.get(idx).copied().unwrap_or("");
        buffer.write_text(row, start, &format_cell(cell, *column), cell_style);
    }
}

fn render_standard_body(
    buffer: &mut PlayfieldBuffer,
    area: TableArea,
    columns: &[TableColumn<'_>],
    rows: &[Vec<String>],
    scroll_offset: usize,
    visible_rows: usize,
    body_style: crate::screen::CellStyle,
    chrome_style: crate::screen::CellStyle,
    selected: Option<usize>,
    selection_col: usize,
    row_states: Option<&[TableRowState]>,
) -> usize {
    let mut displayed_rows = 0usize;
    for (idx, row_cells) in rows
        .iter()
        .skip(scroll_offset)
        .take(visible_rows)
        .enumerate()
    {
        let abs_idx = scroll_offset + idx;
        let base_style = match row_states
            .and_then(|states| states.get(abs_idx))
            .copied()
            .unwrap_or(TableRowState::Normal)
        {
            TableRowState::Disabled => crate::theme::classic::disabled_row_style(),
            TableRowState::Normal => body_style,
        };
        let refs = row_cells.iter().map(String::as_str).collect::<Vec<_>>();
        write_table_row_at(
            buffer,
            area.row + idx,
            area.col,
            columns,
            &refs,
            base_style,
            chrome_style,
        );
        if selected == Some(abs_idx) {
            highlight_selected_row_cell(
                buffer,
                area.row + idx,
                area.col,
                columns,
                &refs,
                selection_col,
            );
        }
        displayed_rows = idx + 1;
    }

    write_scroll_indicator(
        buffer,
        area,
        table_render_width(columns),
        visible_rows,
        rows.len(),
        scroll_offset,
        chrome_style,
    );
    displayed_rows
}

fn highlight_selected_row_cell(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    columns: &[TableColumn<'_>],
    cells: &[&str],
    selection_col: usize,
) {
    let selected_style = crate::theme::classic::selected_row_style();
    let Some(column) = columns.get(selection_col).copied() else {
        return;
    };
    let cell = cells.get(selection_col).copied().unwrap_or("");
    let rendered = format_cell(cell, column);
    buffer.write_text(
        row,
        col + column_start(columns, selection_col),
        &rendered,
        selected_style,
    );
}

fn column_start(columns: &[TableColumn<'_>], index: usize) -> usize {
    let mut start = 1;
    for column in columns.iter().take(index) {
        start += column.width + 1;
    }
    start
}

fn format_table_row(columns: &[TableColumn<'_>], cells: &[&str]) -> String {
    let mut out = String::new();
    out.push('│');

    for (idx, column) in columns.iter().enumerate() {
        if idx != 0 {
            out.push('│');
        }
        let cell = cells.get(idx).copied().unwrap_or("");
        out.push_str(&format_cell(cell, *column));
    }

    out.push('│');
    out
}

fn format_cell(cell: &str, column: TableColumn<'_>) -> String {
    let text = truncate_to_width(cell, column.width);
    match column.align {
        TableAlign::Left => format!("{text:<width$}", width = column.width),
        TableAlign::Center => {
            let pad = column.width.saturating_sub(text.chars().count());
            let left = pad / 2;
            let right = pad.saturating_sub(left);
            format!("{}{}{}", " ".repeat(left), text, " ".repeat(right))
        }
        TableAlign::Right => format!("{text:>width$}", width = column.width),
    }
}

fn truncate_to_width(value: &str, width: usize) -> String {
    value.chars().take(width).collect::<String>()
}
