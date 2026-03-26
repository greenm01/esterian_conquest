/// A terminal color value used in the game's rendering layer.
///
/// Variants cover three tiers:
/// - The 16 named ANSI colors (safe for all terminals, including BBS door clients).
/// - A 256-color indexed palette value (`Indexed`), supported by most modern terminals.
/// - A 24-bit RGB truecolor value (`Rgb`), supported by most local and SSH terminals.
///
/// When rendering, the output backend downgrades to the active [`crate::terminal::ColorMode`].
/// Extended colors are lossily mapped to lower tiers when needed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameColor {
    // --- Classic 16 ANSI named colors ---
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    // --- Extended color tiers ---
    /// 256-color xterm palette index (0–255).
    Indexed(u8),
    /// 24-bit RGB truecolor.
    Rgb(u8, u8, u8),
}

/// Backwards-compatible type alias so that existing code using `AnsiColor` continues to compile.
pub type AnsiColor = GameColor;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellStyle {
    pub fg: GameColor,
    pub bg: GameColor,
    pub bold: bool,
}

impl CellStyle {
    pub const fn new(fg: GameColor, bg: GameColor, bold: bool) -> Self {
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
        assert!(
            row < self.height,
            "playfield row {row} is outside buffer height {}",
            self.height
        );
        let start = row * self.width;
        &self.cells[start..start + self.width]
    }

    pub fn fill_row(&mut self, row: usize, style: CellStyle) {
        assert!(
            row < self.height,
            "fill_row target row {row} is outside buffer height {}",
            self.height
        );
        let start = row * self.width;
        for cell in &mut self.cells[start..start + self.width] {
            *cell = Cell::new(' ', style);
        }
    }

    pub fn write_text(&mut self, row: usize, col: usize, text: &str, style: CellStyle) -> usize {
        assert!(
            row < self.height,
            "write_text target row {row} is outside buffer height {}",
            self.height
        );
        if text.is_empty() {
            return 0;
        }
        assert!(
            col < self.width,
            "write_text start col {col} is outside buffer width {}",
            self.width
        );
        let text_width = text.chars().count();
        assert!(
            text_width <= self.width.saturating_sub(col),
            "write_text overflow at row {row}, col {col}: text width {text_width} exceeds remaining width {}",
            self.width.saturating_sub(col)
        );

        let mut written = 0;
        for (offset, ch) in text.chars().enumerate() {
            let x = col + offset;
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
        assert!(
            usize::from(column) < self.width,
            "cursor column {} is outside buffer width {}",
            column,
            self.width
        );
        assert!(
            usize::from(row) < self.height,
            "cursor row {} is outside buffer height {}",
            row,
            self.height
        );
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
