use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::Widget;

use crate::theme;

use super::chrome::{panel_block, with_panel_bg};
use super::layout::scroll_offset;

#[derive(Clone, Copy)]
pub(super) enum TableCellAlign {
    Left,
    Right,
}

#[derive(Clone, Copy)]
pub(super) struct TableColumnSpec {
    pub(super) title_top: Option<&'static str>,
    pub(super) title: &'static str,
    pub(super) constraint: Constraint,
    pub(super) align: TableCellAlign,
}

pub(super) fn render_table_panel(
    buffer: &mut Buffer,
    area: Rect,
    title: &str,
    focused: bool,
    columns: &[TableColumnSpec],
    header_rows: u16,
    row_count: usize,
    selected: Option<usize>,
    empty: &str,
    row_cells: impl Fn(usize) -> Vec<String>,
) {
    let styles = theme::tui_theme();
    let block = panel_block(title, focused);
    let inner = block.inner(area);
    block.render(area, buffer);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let [header_area, body_area] =
        Layout::vertical([Constraint::Length(header_rows), Constraint::Min(0)]).areas(inner);
    render_table_header(buffer, header_area, columns);

    if row_count == 0 {
        if body_area.height > 0 {
            buffer.set_stringn(
                body_area.x,
                body_area.y,
                empty,
                body_area.width as usize,
                with_panel_bg(styles.dim),
            );
        }
        return;
    }

    let visible_rows = body_area.height as usize;
    if visible_rows == 0 {
        return;
    }
    let scroll = scroll_offset(row_count, visible_rows, selected.unwrap_or(0));
    for (offset, index) in (scroll..row_count).take(visible_rows).enumerate() {
        let row_area = Rect::new(body_area.x, body_area.y + offset as u16, body_area.width, 1);
        let row_style = if selected == Some(index) {
            styles.selected
        } else {
            with_panel_bg(styles.value)
        };
        render_table_row(buffer, row_area, columns, &row_cells(index), row_style);
    }
}

fn render_table_header(buffer: &mut Buffer, area: Rect, columns: &[TableColumnSpec]) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let styles = theme::tui_theme();
    for row in area.top()..area.bottom() {
        buffer.set_stringn(
            area.x,
            row,
            &" ".repeat(area.width as usize),
            area.width as usize,
            with_panel_bg(styles.label),
        );
    }
    let top_cells = columns
        .iter()
        .map(|column| column.title_top.unwrap_or(""))
        .collect::<Vec<_>>();
    let bottom_cells = columns.iter().map(|column| column.title).collect::<Vec<_>>();
    if area.height > 1 {
        let top_area = Rect::new(area.x, area.y, area.width, 1);
        render_table_cells(buffer, top_area, columns, &top_cells, with_panel_bg(styles.label));
        let bottom_area = Rect::new(area.x, area.y + area.height - 1, area.width, 1);
        render_table_cells(
            buffer,
            bottom_area,
            columns,
            &bottom_cells,
            with_panel_bg(styles.label),
        );
    } else {
        render_table_cells(
            buffer,
            area,
            columns,
            &bottom_cells,
            with_panel_bg(styles.label),
        );
    }
}

fn render_table_row(
    buffer: &mut Buffer,
    area: Rect,
    columns: &[TableColumnSpec],
    cells: &[String],
    style: Style,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    buffer.set_stringn(
        area.x,
        area.y,
        &" ".repeat(area.width as usize),
        area.width as usize,
        style,
    );
    let borrowed = cells.iter().map(String::as_str).collect::<Vec<_>>();
    render_table_cells(buffer, area, columns, &borrowed, style);
}

fn render_table_cells(
    buffer: &mut Buffer,
    area: Rect,
    columns: &[TableColumnSpec],
    cells: &[&str],
    style: Style,
) {
    let cell_areas = Layout::horizontal(
        columns
            .iter()
            .map(|column| column.constraint)
            .collect::<Vec<_>>(),
    )
    .spacing(1)
    .split(area);
    for ((column, cell), cell_area) in columns.iter().zip(cells.iter()).zip(cell_areas.iter()) {
        if cell_area.width == 0 {
            continue;
        }
        let text_width = cell.chars().count().min(cell_area.width as usize) as u16;
        let start = match column.align {
            TableCellAlign::Left => cell_area.x,
            TableCellAlign::Right => cell_area.right().saturating_sub(text_width),
        };
        buffer.set_stringn(start, cell_area.y, cell, cell_area.width as usize, style);
    }
}
