use super::layout::Rect;
use super::style::{Color, Modifier, Style};

#[derive(Clone, Debug)]
pub struct Cell {
    symbol: String,
    pub fg: Color,
    pub bg: Color,
    pub modifier: Modifier,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            symbol: " ".to_string(),
            fg: Color::Reset,
            bg: Color::Reset,
            modifier: Modifier::empty(),
        }
    }
}

impl Cell {
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    fn set_char(&mut self, ch: char, style: Style) {
        self.symbol.clear();
        self.symbol.push(ch);
        self.fg = style.fg.unwrap_or(Color::Reset);
        self.bg = style.bg.unwrap_or(Color::Reset);
        self.modifier = style.modifier;
    }
}

#[derive(Clone, Debug)]
pub struct Buffer {
    pub area: Rect,
    cells: Vec<Cell>,
}

impl Buffer {
    pub fn empty(area: Rect) -> Self {
        let len = usize::from(area.width) * usize::from(area.height);
        Self {
            area,
            cells: vec![Cell::default(); len],
        }
    }

    pub fn cell(&self, position: (u16, u16)) -> Option<&Cell> {
        let (x, y) = position;
        if x < self.area.x || y < self.area.y || x >= self.area.right() || y >= self.area.bottom()
        {
            return None;
        }
        let local_x = usize::from(x - self.area.x);
        let local_y = usize::from(y - self.area.y);
        self.cells
            .get(local_y * usize::from(self.area.width) + local_x)
    }

    pub fn set_stringn(
        &mut self,
        x: u16,
        y: u16,
        text: impl AsRef<str>,
        max_width: usize,
        style: Style,
    ) {
        if y < self.area.y || y >= self.area.bottom() || max_width == 0 {
            return;
        }
        let start = x.max(self.area.x);
        let end = self.area.right();
        let mut col = start;
        for ch in text.as_ref().chars().take(max_width) {
            if col >= end {
                break;
            }
            self.set_cell(col, y, ch, style);
            col += 1;
        }
    }

    pub fn fill_rect(&mut self, area: Rect, style: Style) {
        for row in area.y..area.bottom() {
            for col in area.x..area.right() {
                self.set_cell(col, row, ' ', style);
            }
        }
    }

    fn set_cell(&mut self, x: u16, y: u16, ch: char, style: Style) {
        if x < self.area.x || y < self.area.y || x >= self.area.right() || y >= self.area.bottom()
        {
            return;
        }
        let local_x = usize::from(x - self.area.x);
        let local_y = usize::from(y - self.area.y);
        if let Some(cell) = self
            .cells
            .get_mut(local_y * usize::from(self.area.width) + local_x)
        {
            cell.set_char(ch, style);
        }
    }
}
