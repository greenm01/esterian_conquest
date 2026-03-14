#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RgbColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl RgbColor {
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellStyle {
    pub fg: RgbColor,
    pub bg: RgbColor,
    pub bold: bool,
}

impl CellStyle {
    pub const fn new(fg: RgbColor, bg: RgbColor, bold: bool) -> Self {
        Self { fg, bg, bold }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub style: CellStyle,
}

impl Cell {
    pub const fn new(ch: char, style: CellStyle) -> Self {
        Self { ch, style }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct StyledSpan<'a> {
    pub text: &'a str,
    pub style: CellStyle,
}

impl<'a> StyledSpan<'a> {
    pub const fn new(text: &'a str, style: CellStyle) -> Self {
        Self { text, style }
    }
}

pub struct PlayfieldBuffer {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
    cursor: Option<(u16, u16)>,
}

impl PlayfieldBuffer {
    pub fn new(width: usize, height: usize, base_style: CellStyle) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::new(' ', base_style); width * height],
            cursor: None,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn cursor(&self) -> Option<(u16, u16)> {
        self.cursor
    }

    pub fn row(&self, row: usize) -> &[Cell] {
        let start = row * self.width;
        &self.cells[start..start + self.width]
    }

    pub fn fill_row(&mut self, row: usize, style: CellStyle) {
        if row >= self.height {
            return;
        }
        let start = row * self.width;
        for cell in &mut self.cells[start..start + self.width] {
            *cell = Cell::new(' ', style);
        }
    }

    pub fn write_text(&mut self, row: usize, col: usize, text: &str, style: CellStyle) -> usize {
        if row >= self.height || col >= self.width {
            return 0;
        }

        let mut written = 0;
        for (offset, ch) in text.chars().enumerate() {
            let x = col + offset;
            if x >= self.width {
                break;
            }
            let index = row * self.width + x;
            self.cells[index] = Cell::new(ch, style);
            written += 1;
        }
        written
    }

    pub fn write_spans(&mut self, row: usize, mut col: usize, spans: &[StyledSpan<'_>]) -> usize {
        let start_col = col;
        for span in spans {
            col += self.write_text(row, col, span.text, span.style);
        }
        col.saturating_sub(start_col)
    }

    pub fn set_cursor(&mut self, column: u16, row: u16) {
        self.cursor = Some((column, row));
    }

    pub fn clear_cursor(&mut self) {
        self.cursor = None;
    }

    pub fn plain_line(&self, row: usize) -> String {
        self.row(row)
            .iter()
            .map(|cell| cell.ch)
            .collect::<String>()
            .trim_end_matches(' ')
            .to_string()
    }
}
