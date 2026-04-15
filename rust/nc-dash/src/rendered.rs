use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use crate::buffer::{CellStyle, PlayfieldBuffer};
use crate::theme;

pub struct RenderedUi {
    pub buffer: Buffer,
    pub cursor: Option<(u16, u16)>,
    pub cursor_style: Style,
}

impl RenderedUi {
    pub fn new(buffer: Buffer) -> Self {
        Self {
            buffer,
            cursor: None,
            cursor_style: theme::tui_theme().cursor,
        }
    }

    pub fn with_cursor(mut self, cursor: Option<(u16, u16)>, cursor_style: Style) -> Self {
        self.cursor = cursor;
        self.cursor_style = cursor_style;
        self
    }

    pub fn from_playfield(playfield: &PlayfieldBuffer, cursor_style: Style) -> Self {
        let area = Rect::new(0, 0, playfield.width() as u16, playfield.height() as u16);
        let mut buffer = Buffer::empty(area);
        for row in 0..playfield.height() {
            for col in 0..playfield.width() {
                let source = playfield.row(row)[col];
                if let Some(cell) = buffer.cell_mut((col as u16, row as u16)) {
                    let mut style = Style::default()
                        .fg(theme::to_tui_color(source.style.fg))
                        .bg(theme::to_tui_color(source.style.bg));
                    if source.style.bold {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    cell.set_char(source.ch).set_style(style);
                }
            }
        }
        Self {
            buffer,
            cursor: playfield.cursor(),
            cursor_style,
        }
    }

    pub fn to_playfield(&self, base_style: CellStyle) -> PlayfieldBuffer {
        let mut playfield = PlayfieldBuffer::new(
            self.buffer.area.width as usize,
            self.buffer.area.height as usize,
            base_style,
        );
        for y in 0..self.buffer.area.height {
            for x in 0..self.buffer.area.width {
                let Some(cell) = self
                    .buffer
                    .cell((self.buffer.area.x + x, self.buffer.area.y + y))
                else {
                    continue;
                };
                let symbol = cell.symbol().chars().next().unwrap_or(' ');
                let fg = theme::from_tui_color(cell.fg, base_style.fg);
                let bg = theme::from_tui_color(cell.bg, base_style.bg);
                let bold = cell.modifier.contains(Modifier::BOLD);
                playfield.set_cell(y as usize, x as usize, symbol, CellStyle::new(fg, bg, bold));
            }
        }
        if let Some((col, row)) = self.cursor {
            playfield.set_cursor(col, row);
        }
        playfield
    }
}

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
