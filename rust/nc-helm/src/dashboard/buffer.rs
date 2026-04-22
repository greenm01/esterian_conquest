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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OverlayCrosshair {
    pub fg: GameColor,
    pub center_col: usize,
    pub center_row: usize,
    pub left_col: usize,
    pub right_col: usize,
    pub top_row: usize,
    pub bottom_row: usize,
}

#[derive(Debug, Clone)]
pub struct PlayfieldBuffer {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
    cursor: Option<(u16, u16)>,
    overlay_crosshair: Option<OverlayCrosshair>,
}

impl PlayfieldBuffer {
    pub fn new(width: usize, height: usize, base_style: CellStyle) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::new(' ', base_style); width * height],
            cursor: None,
            overlay_crosshair: None,
        }
    }

    pub fn reset(&mut self, width: usize, height: usize, base_style: CellStyle) {
        self.width = width;
        self.height = height;
        self.cells
            .resize(width * height, Cell::new(' ', base_style));
        self.cells.fill(Cell::new(' ', base_style));
        self.cursor = None;
        self.overlay_crosshair = None;
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

    pub(crate) fn overlay_crosshair(&self) -> Option<OverlayCrosshair> {
        self.overlay_crosshair
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

    pub(crate) fn clear_overlay_crosshair(&mut self) {
        self.overlay_crosshair = None;
    }

    pub(crate) fn set_overlay_crosshair(&mut self, overlay: OverlayCrosshair) {
        self.overlay_crosshair = Some(overlay);
    }

    pub fn plain_line(&self, row: usize) -> String {
        self.row(row)
            .iter()
            .map(|cell| cell.ch)
            .collect::<String>()
            .trim_end_matches(' ')
            .to_string()
    }

    pub fn copy_region(&self, row: usize, col: usize, width: usize, height: usize) -> Vec<Cell> {
        let row_end = (row + height).min(self.height);
        let col_end = (col + width).min(self.width);
        let w = col_end.saturating_sub(col);
        let h = row_end.saturating_sub(row);
        let mut cells = Vec::with_capacity(w * h);
        for r in row..row + h {
            let start = r * self.width + col;
            cells.extend_from_slice(&self.cells[start..start + w]);
        }
        cells
    }

    pub fn blit_region(
        &mut self,
        row: usize,
        col: usize,
        width: usize,
        height: usize,
        cells: &[Cell],
    ) {
        let row_end = (row + height).min(self.height);
        let col_end = (col + width).min(self.width);
        let w = col_end.saturating_sub(col);
        let h = row_end.saturating_sub(row);
        for r in row..row + h {
            let buf_start = r * self.width + col;
            let src_start = (r - row) * w;
            self.cells[buf_start..buf_start + w].copy_from_slice(&cells[src_start..src_start + w]);
        }
    }
}
