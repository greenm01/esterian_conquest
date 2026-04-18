use super::index::Point;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameColor {
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
    Indexed(u8),
    Rgb(u8, u8, u8),
}

pub type AnsiColor = GameColor;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackgroundMode {
    Cell,
    TextBand,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellStyle {
    pub fg: GameColor,
    pub bg: GameColor,
    pub bold: bool,
    pub bg_mode: BackgroundMode,
}

impl CellStyle {
    pub const fn new(fg: GameColor, bg: GameColor, bold: bool) -> Self {
        Self {
            fg,
            bg,
            bold,
            bg_mode: BackgroundMode::Cell,
        }
    }

    pub const fn with_background_mode(mut self, bg_mode: BackgroundMode) -> Self {
        self.bg_mode = bg_mode;
        self
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum OverlayTextFamily {
    Stormfaze,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OverlayText {
    pub text: String,
    pub family: OverlayTextFamily,
    pub style: CellStyle,
    pub left_col: usize,
    pub top_row: usize,
    pub width_cols: usize,
    pub height_rows: usize,
}

pub struct PlayfieldBuffer {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
    cursor: Option<Point>,
    overlay_texts: Vec<OverlayText>,
}

impl PlayfieldBuffer {
    pub fn new(width: usize, height: usize, base_style: CellStyle) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::new(' ', base_style); width * height],
            cursor: None,
            overlay_texts: Vec::new(),
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn cursor(&self) -> Option<Point> {
        self.cursor
    }

    pub(crate) fn overlay_texts(&self) -> &[OverlayText] {
        &self.overlay_texts
    }

    pub fn has_overlay_text(&self, text: &str) -> bool {
        self.overlay_texts
            .iter()
            .any(|overlay| overlay.text == text)
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

    pub fn fill_rect(
        &mut self,
        row: usize,
        col: usize,
        width: usize,
        height: usize,
        style: CellStyle,
    ) {
        let max_row = row.saturating_add(height).min(self.height);
        let max_col = col.saturating_add(width).min(self.width);
        for y in row..max_row {
            for x in col..max_col {
                self.set_cell(y, x, ' ', style);
            }
        }
    }

    pub fn set_cell(&mut self, row: usize, col: usize, ch: char, style: CellStyle) {
        if row >= self.height || col >= self.width {
            return;
        }
        let index = row * self.width + col;
        self.cells[index] = Cell::new(ch, style);
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

    pub fn write_text_clipped(
        &mut self,
        row: usize,
        col: usize,
        text: &str,
        style: CellStyle,
    ) -> usize {
        if row >= self.height || col >= self.width || text.is_empty() {
            return 0;
        }
        let remaining = self.width.saturating_sub(col);
        let clipped: String = text.chars().take(remaining).collect();
        self.write_text(row, col, &clipped, style)
    }

    pub fn write_spans(&mut self, row: usize, mut col: usize, spans: &[StyledSpan<'_>]) -> usize {
        let start_col = col;
        for span in spans {
            col += self.write_text(row, col, span.text, span.style);
        }
        col.saturating_sub(start_col)
    }

    pub fn write_spans_clipped(
        &mut self,
        row: usize,
        mut col: usize,
        spans: &[StyledSpan<'_>],
    ) -> usize {
        let start_col = col;
        for span in spans {
            if col >= self.width {
                break;
            }
            col += self.write_text_clipped(row, col, span.text, span.style);
        }
        col.saturating_sub(start_col)
    }

    pub fn set_cursor(&mut self, point: Point) {
        assert!(
            point.column.as_usize() < self.width,
            "cursor column {} is outside buffer width {}",
            point.column.as_usize(),
            self.width
        );
        assert!(
            point.row.as_usize() < self.height,
            "cursor row {} is outside buffer height {}",
            point.row.as_usize(),
            self.height
        );
        self.cursor = Some(point);
    }

    pub fn clear_cursor(&mut self) {
        self.cursor = None;
    }

    pub(crate) fn push_overlay_text(
        &mut self,
        text: impl Into<String>,
        family: OverlayTextFamily,
        style: CellStyle,
        left_col: usize,
        top_row: usize,
        width_cols: usize,
        height_rows: usize,
    ) {
        if width_cols == 0 || height_rows == 0 {
            return;
        }
        self.overlay_texts.push(OverlayText {
            text: text.into(),
            family,
            style,
            left_col,
            top_row,
            width_cols,
            height_rows,
        });
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
