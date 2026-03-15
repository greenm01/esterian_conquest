use std::fs;
use std::path::Path;

use crate::screen::{Cell, CellStyle, PlayfieldBuffer, RgbColor, PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};
use crate::screen::new_playfield;

#[derive(Debug, Clone)]
pub struct StartupArt {
    cells: Vec<Cell>,
}

impl StartupArt {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let text = fs::read_to_string(path)?;
        Ok(parse_ansi_art(&text))
    }

    pub fn render(&self) -> PlayfieldBuffer {
        let mut buffer = new_playfield();
        for row in 0..PLAYFIELD_HEIGHT.saturating_sub(1) {
            for col in 0..PLAYFIELD_WIDTH {
                let index = row * PLAYFIELD_WIDTH + col;
                if let Some(cell) = self.cells.get(index).copied() {
                    buffer.write_text(row, col, &cell.ch.to_string(), cell.style);
                }
            }
        }
        buffer
    }
}

fn parse_ansi_art(text: &str) -> StartupArt {
    let mut state = AnsiState::new();
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
                state.row = (state.row + 1).min(PLAYFIELD_HEIGHT.saturating_sub(1));
                idx += 1;
            }
            ch => {
                state.write_char(ch);
                idx += 1;
            }
        }
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
            state.row = (state.row + first_or(&values, 1)).min(PLAYFIELD_HEIGHT.saturating_sub(1))
        }
        'C' => {
            state.col = (state.col + first_or(&values, 1)).min(PLAYFIELD_WIDTH.saturating_sub(1))
        }
        'D' => state.col = state.col.saturating_sub(first_or(&values, 1)),
        'H' | 'f' => {
            state.row = values.first().copied().unwrap_or(1).saturating_sub(1);
            state.col = values.get(1).copied().unwrap_or(1).saturating_sub(1);
            state.row = state.row.min(PLAYFIELD_HEIGHT.saturating_sub(1));
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
    replace_plain_text(state, "Version 1.51", "Version 1.60");
    clear_row_containing(state, "Compliments of your Sysop");
    clear_row_containing(state, "Registration #");
}

fn replace_plain_text(state: &mut AnsiState, from: &str, to: &str) {
    for row in 0..PLAYFIELD_HEIGHT {
        let line = state.row_plain_text(row);
        if let Some(col) = line.find(from) {
            let style = state.cells[row * PLAYFIELD_WIDTH + col].style;
            for offset in 0..from.chars().count() {
                let index = row * PLAYFIELD_WIDTH + col + offset;
                state.cells[index] = Cell::new(' ', style);
            }
            for (offset, ch) in to.chars().enumerate() {
                let index = row * PLAYFIELD_WIDTH + col + offset;
                if index < state.cells.len() {
                    state.cells[index] = Cell::new(ch, style);
                }
            }
            return;
        }
    }
}

fn clear_row_containing(state: &mut AnsiState, needle: &str) {
    for row in 0..PLAYFIELD_HEIGHT {
        if state.row_plain_text(row).contains(needle) {
            let start = row * PLAYFIELD_WIDTH;
            let end = start + PLAYFIELD_WIDTH;
            for cell in &mut state.cells[start..end] {
                *cell = Cell::new(' ', default_style());
            }
        }
    }
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
            cells: vec![Cell::new(' ', default_style()); PLAYFIELD_WIDTH * PLAYFIELD_HEIGHT],
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
        if self.row >= PLAYFIELD_HEIGHT || self.col >= PLAYFIELD_WIDTH {
            return;
        }
        let index = self.row * PLAYFIELD_WIDTH + self.col;
        self.cells[index] = Cell::new(ch, self.style);
        self.col += 1;
    }

    fn row_plain_text(&self, row: usize) -> String {
        let start = row * PLAYFIELD_WIDTH;
        let end = start + PLAYFIELD_WIDTH;
        self.cells[start..end].iter().map(|cell| cell.ch).collect()
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
