use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;

use crate::buffer::{CellStyle, PlayfieldBuffer};
use crate::theme;

pub fn render_tui_area_into_playfield(
    playfield: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    width: usize,
    height: usize,
    render: impl FnOnce(&mut Buffer, Rect),
) {
    if width == 0 || height == 0 {
        return;
    }
    let area = Rect::new(0, 0, width as u16, height as u16);
    let mut buffer = Buffer::empty(area);
    render(&mut buffer, area);
    for y in 0..height {
        for x in 0..width {
            let Some(cell) = buffer.cell((x as u16, y as u16)) else {
                continue;
            };
            let symbol = cell.symbol().chars().next().unwrap_or(' ');
            let fg = theme::from_tui_color(cell.fg, theme::body_style().fg);
            let bg = theme::from_tui_color(cell.bg, theme::body_style().bg);
            let bold = cell.modifier.contains(Modifier::BOLD);
            playfield.set_cell(row + y, col + x, symbol, CellStyle::new(fg, bg, bold));
        }
    }
}
