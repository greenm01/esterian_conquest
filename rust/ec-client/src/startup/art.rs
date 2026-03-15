use std::fs;
use std::path::Path;

use crate::screen::{Cell, CellStyle, PlayfieldBuffer, RgbColor, PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};
use crate::screen::new_playfield;

const ANSI_SCREEN_HEIGHT: usize = 25;

#[derive(Debug, Clone)]
pub struct StartupArt {
    cells: Vec<Cell>,
}

impl StartupArt {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let text = fs::read_to_string(path)?;
        if text.contains('\u{1b}') {
            Ok(parse_ansi_art(&text))
        } else {
            Ok(parse_plain_art(&text))
        }
    }

    pub fn render(&self) -> PlayfieldBuffer {
        let mut buffer = new_playfield();
        let start_row = best_projection_start(&self.cells);
        for row in 0..PLAYFIELD_HEIGHT.saturating_sub(1) {
            for col in 0..PLAYFIELD_WIDTH {
                let index = (start_row + row) * PLAYFIELD_WIDTH + col;
                if let Some(cell) = self.cells.get(index).copied() {
                    buffer.write_text(row, col, &cell.ch.to_string(), cell.style);
                }
            }
        }
        buffer
    }
}

fn parse_plain_art(text: &str) -> StartupArt {
    let mut cells = vec![Cell::new(' ', default_style()); PLAYFIELD_WIDTH * ANSI_SCREEN_HEIGHT];
    for (row, line) in text.lines().take(ANSI_SCREEN_HEIGHT).enumerate() {
        for (col, ch) in line.chars().take(PLAYFIELD_WIDTH).enumerate() {
            cells[row * PLAYFIELD_WIDTH + col] = Cell::new(ch, default_style());
        }
    }
    sanitize_cells(&mut cells);
    StartupArt { cells }
}

fn parse_ansi_art(text: &str) -> StartupArt {
    let mut state = AnsiState::new();
    let mut selected_snapshot: Option<Vec<Cell>> = None;
    let chars = text.chars().collect::<Vec<_>>();
    let mut idx = 0;
    while idx < chars.len() {
        match chars[idx] {
            '\u{1b}' => idx = parse_escape(&chars, idx + 1, &mut state),
            '\r' => {
                state.col = 0;
                idx += 1;
            }
            '\n' => {
                state.row = (state.row + 1).min(ANSI_SCREEN_HEIGHT.saturating_sub(1));
                idx += 1;
            }
            ch => {
                state.write_char(ch);
                idx += 1;
            }
        }
        maybe_capture_snapshot(&state, &mut selected_snapshot);
    }
    if let Some(mut snapshot) = selected_snapshot {
        sanitize_cells(&mut snapshot);
        return StartupArt { cells: snapshot };
    }
    sanitize_ec_frame(&mut state);
    StartupArt { cells: state.cells }
}

fn parse_escape(chars: &[char], mut idx: usize, state: &mut AnsiState) -> usize {
    if chars.get(idx) != Some(&'[') {
        return idx.saturating_add(1);
    }
    idx += 1;
    let start = idx;
    while let Some(ch) = chars.get(idx) {
        if ch.is_ascii_alphabetic() {
            let params = chars[start..idx].iter().collect::<String>();
            apply_csi(state, &params, *ch);
            return idx + 1;
        }
        idx += 1;
    }
    idx
}

fn apply_csi(state: &mut AnsiState, params: &str, final_char: char) {
    let values = params
        .split(';')
        .filter(|part| !part.is_empty())
        .map(|part| part.parse::<usize>().unwrap_or(0))
        .collect::<Vec<_>>();

    match final_char {
        'A' => state.row = state.row.saturating_sub(first_or(&values, 1)),
        'B' => {
            state.row =
                (state.row + first_or(&values, 1)).min(ANSI_SCREEN_HEIGHT.saturating_sub(1))
        }
        'C' => {
            state.col = (state.col + first_or(&values, 1)).min(PLAYFIELD_WIDTH.saturating_sub(1))
        }
        'D' => state.col = state.col.saturating_sub(first_or(&values, 1)),
        'H' | 'f' => {
            state.row = values.first().copied().unwrap_or(1).saturating_sub(1);
            state.col = values.get(1).copied().unwrap_or(1).saturating_sub(1);
            state.row = state.row.min(ANSI_SCREEN_HEIGHT.saturating_sub(1));
            state.col = state.col.min(PLAYFIELD_WIDTH.saturating_sub(1));
        }
        'J' => {
            if values.first().copied().unwrap_or(0) == 2 {
                state.clear_screen();
            }
        }
        'K' => state.clear_to_end_of_line(),
        'm' => apply_sgr(state, &values),
        _ => {}
    }
}

fn apply_sgr(state: &mut AnsiState, values: &[usize]) {
    if values.is_empty() {
        state.style = default_style();
        return;
    }

    for value in values {
        match *value {
            0 => state.style = default_style(),
            1 => state.style.bold = true,
            30 => state.style.fg = palette_color(0, state.style.bold),
            31 => state.style.fg = palette_color(1, state.style.bold),
            32 => state.style.fg = palette_color(2, state.style.bold),
            33 => state.style.fg = palette_color(3, state.style.bold),
            34 => state.style.fg = palette_color(4, state.style.bold),
            35 => state.style.fg = palette_color(5, state.style.bold),
            36 => state.style.fg = palette_color(6, state.style.bold),
            37 => state.style.fg = palette_color(7, state.style.bold),
            40 => state.style.bg = palette_color(0, false),
            41 => state.style.bg = palette_color(1, false),
            42 => state.style.bg = palette_color(2, false),
            43 => state.style.bg = palette_color(3, false),
            44 => state.style.bg = palette_color(4, false),
            45 => state.style.bg = palette_color(5, false),
            46 => state.style.bg = palette_color(6, false),
            47 => state.style.bg = palette_color(7, false),
            _ => {}
        }
    }
}

fn first_or(values: &[usize], default: usize) -> usize {
    values.first().copied().unwrap_or(default)
}

fn sanitize_ec_frame(state: &mut AnsiState) {
    sanitize_cells(&mut state.cells);
}

fn sanitize_cells(cells: &mut [Cell]) {
    replace_plain_text(cells, "Version 1.51", "Version 1.60");
    clear_row_containing(cells, "Compliments of your Sysop");
    clear_row_containing(cells, "Registration #");
}

fn replace_plain_text(cells: &mut [Cell], from: &str, to: &str) {
    for row in 0..ANSI_SCREEN_HEIGHT {
        let line = row_plain_text(cells, row);
        if let Some(col) = line.find(from) {
            let style = cells[row * PLAYFIELD_WIDTH + col].style;
            for offset in 0..from.chars().count() {
                let index = row * PLAYFIELD_WIDTH + col + offset;
                cells[index] = Cell::new(' ', style);
            }
            for (offset, ch) in to.chars().enumerate() {
                let index = row * PLAYFIELD_WIDTH + col + offset;
                if index < cells.len() {
                    cells[index] = Cell::new(ch, style);
                }
            }
            return;
        }
    }
}

fn clear_row_containing(cells: &mut [Cell], needle: &str) {
    for row in 0..ANSI_SCREEN_HEIGHT {
        if row_plain_text(cells, row).contains(needle) {
            let start = row * PLAYFIELD_WIDTH;
            let end = start + PLAYFIELD_WIDTH;
            for cell in &mut cells[start..end] {
                *cell = Cell::new(' ', default_style());
            }
        }
    }
}

fn row_plain_text(cells: &[Cell], row: usize) -> String {
    let start = row * PLAYFIELD_WIDTH;
    let end = start + PLAYFIELD_WIDTH;
    cells[start..end].iter().map(|cell| cell.ch).collect()
}

fn maybe_capture_snapshot(
    state: &AnsiState,
    selected_snapshot: &mut Option<Vec<Cell>>,
) {
    if selected_snapshot.is_some() {
        return;
    }
    let visible_text = state.visible_text();
    if !visible_text.contains("Version 1.51") && !visible_text.contains("Version 1.60") {
        return;
    }
    let start_row = best_projection_start(&state.cells);
    let projected_text = projected_text(&state.cells, start_row);
    if projected_text.contains("Compliments of your Sysop")
        || projected_text.contains("Registration #")
    {
        return;
    }
    if !contains_banner_body(&projected_text) || banner_row_count(&projected_text) < 4 {
        return;
    }
    *selected_snapshot = Some(state.cells.clone());
}

fn best_projection_start(cells: &[Cell]) -> usize {
    let mut best_start = 0usize;
    let mut best_score = isize::MIN;
    for start_row in 0..=ANSI_SCREEN_HEIGHT.saturating_sub(PLAYFIELD_HEIGHT - 1) {
        let score = projection_score_for_window(cells, start_row);
        if score > best_score {
            best_score = score;
            best_start = start_row;
        }
    }
    best_start
}

fn projection_score_for_window(cells: &[Cell], start_row: usize) -> isize {
    let mut score = 0isize;
    let end_row = start_row + (PLAYFIELD_HEIGHT - 1);
    let full_text = projected_text(cells, start_row);
    for row in start_row..end_row {
        let line = row_plain_text(cells, row);
        score += line.chars().filter(|ch| *ch != ' ').count() as isize;
    }
    if contains_banner_body(&full_text) {
        score += 400;
    }
    if full_text.contains("Version 1.51") || full_text.contains("Version 1.60") {
        score += 1000;
    }
    if full_text.contains("Compliments of your Sysop") {
        score -= 500;
    }
    if full_text.contains("Registration #") {
        score -= 500;
    }
    if full_text.contains("────▐") || full_text.contains("▀▀▒█") || full_text.contains("▄▄▄▄▄▄") {
        score -= 120;
    }
    score
}

fn projected_text(cells: &[Cell], start_row: usize) -> String {
    let end_row = start_row + (PLAYFIELD_HEIGHT - 1);
    (start_row..end_row)
        .map(|row| row_plain_text(cells, row))
        .collect::<Vec<_>>()
        .join("\n")
}

fn contains_banner_body(text: &str) -> bool {
    text.contains("▒██████") || text.contains("██████") || text.contains("ESTERIAN")
}

fn banner_row_count(text: &str) -> usize {
    text.lines()
        .filter(|line| {
            let block_count = line.chars().filter(|ch| matches!(ch, '█' | '▒' | '▓')).count();
            block_count >= 8
        })
        .count()
}

struct AnsiState {
    row: usize,
    col: usize,
    style: CellStyle,
    cells: Vec<Cell>,
}

impl AnsiState {
    fn new() -> Self {
        Self {
            row: 0,
            col: 0,
            style: default_style(),
            cells: vec![Cell::new(' ', default_style()); PLAYFIELD_WIDTH * ANSI_SCREEN_HEIGHT],
        }
    }

    fn clear_screen(&mut self) {
        self.cells.fill(Cell::new(' ', default_style()));
        self.row = 0;
        self.col = 0;
    }

    fn clear_to_end_of_line(&mut self) {
        let start = self.row * PLAYFIELD_WIDTH + self.col.min(PLAYFIELD_WIDTH.saturating_sub(1));
        let end = (self.row + 1) * PLAYFIELD_WIDTH;
        for cell in &mut self.cells[start..end] {
            *cell = Cell::new(' ', self.style);
        }
    }

    fn write_char(&mut self, ch: char) {
        if self.row >= ANSI_SCREEN_HEIGHT || self.col >= PLAYFIELD_WIDTH {
            return;
        }
        let index = self.row * PLAYFIELD_WIDTH + self.col;
        self.cells[index] = Cell::new(ch, self.style);
        self.col += 1;
    }

    fn visible_text(&self) -> String {
        (0..ANSI_SCREEN_HEIGHT)
            .map(|row| row_plain_text(&self.cells, row))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn default_style() -> CellStyle {
    CellStyle::new(palette_color(7, true), palette_color(0, false), false)
}

fn palette_color(index: usize, bright: bool) -> RgbColor {
    match (index, bright) {
        (0, _) => RgbColor::new(0, 0, 0),
        (1, false) => RgbColor::new(170, 0, 0),
        (1, true) => RgbColor::new(255, 85, 85),
        (2, false) => RgbColor::new(0, 170, 0),
        (2, true) => RgbColor::new(85, 255, 85),
        (3, false) => RgbColor::new(170, 85, 0),
        (3, true) => RgbColor::new(255, 255, 85),
        (4, false) => RgbColor::new(0, 0, 170),
        (4, true) => RgbColor::new(85, 85, 255),
        (5, false) => RgbColor::new(170, 0, 170),
        (5, true) => RgbColor::new(255, 85, 255),
        (6, false) => RgbColor::new(0, 170, 170),
        (6, true) => RgbColor::new(85, 255, 255),
        (7, false) => RgbColor::new(170, 170, 170),
        (7, true) => RgbColor::new(255, 255, 255),
        _ => RgbColor::new(255, 255, 255),
    }
}
