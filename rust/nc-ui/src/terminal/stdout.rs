mod diff;

use std::io::{self, IsTerminal, Write};

use crossterm::{
    cursor::{Hide, MoveTo, SetCursorStyle, Show},
    event::{self, Event, KeyEvent},
    execute, queue,
    style::{Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use diff::{changed_spans, fingerprint_row};

use crate::buffer::{Cell, CellStyle, GameColor, PlayfieldBuffer};
use crate::terminal::cp437;
use crate::terminal::{ColorMode, OutputEncoding, Terminal};
use crate::theme::classic;

pub struct StdoutTerminal {
    encoding: OutputEncoding,
    color_mode: ColorMode,
    previous_frame: RenderSnapshot,
    current_frame: RenderSnapshot,
    has_previous_frame: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct RenderSnapshot {
    width: usize,
    height: usize,
    term_width: u16,
    term_height: u16,
    origin: (u16, u16),
    cells: Vec<Cell>,
    row_fingerprints: Vec<u64>,
    cursor: Option<(u16, u16)>,
}

impl RenderSnapshot {
    fn capture_from_playfield(
        &mut self,
        playfield: &PlayfieldBuffer,
        term_width: u16,
        term_height: u16,
        origin: (u16, u16),
    ) {
        self.width = playfield.width();
        self.height = playfield.height();
        self.term_width = term_width;
        self.term_height = term_height;
        self.origin = origin;
        self.cursor = playfield.cursor();
        self.cells.clear();
        self.row_fingerprints.clear();
        self.cells.reserve(self.width * self.height);
        self.row_fingerprints.reserve(self.height);
        for row_idx in 0..self.height {
            let row = playfield.row(row_idx);
            self.row_fingerprints.push(fingerprint_row(row));
            self.cells.extend_from_slice(row);
        }
    }

    fn row(&self, row: usize) -> &[Cell] {
        let start = row * self.width;
        &self.cells[start..start + self.width]
    }
}

impl StdoutTerminal {
    pub fn new() -> Self {
        Self {
            encoding: OutputEncoding::Utf8,
            color_mode: ColorMode::TrueColor,
            previous_frame: RenderSnapshot::default(),
            current_frame: RenderSnapshot::default(),
            has_previous_frame: false,
        }
    }

    pub fn with_encoding(encoding: OutputEncoding) -> Self {
        Self {
            encoding,
            color_mode: ColorMode::TrueColor,
            previous_frame: RenderSnapshot::default(),
            current_frame: RenderSnapshot::default(),
            has_previous_frame: false,
        }
    }

    pub fn with_encoding_and_color_mode(encoding: OutputEncoding, color_mode: ColorMode) -> Self {
        Self {
            encoding,
            color_mode,
            previous_frame: RenderSnapshot::default(),
            current_frame: RenderSnapshot::default(),
            has_previous_frame: false,
        }
    }
}

impl Default for StdoutTerminal {
    fn default() -> Self {
        Self::new()
    }
}

impl Terminal for StdoutTerminal {
    fn render(&mut self, playfield: &PlayfieldBuffer) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = io::stdout();
        if stdout.is_terminal() {
            let (term_width, term_height) = terminal::size()?;
            let (offset_x, offset_y) =
                render_origin(term_width, term_height, self.encoding, playfield);
            self.current_frame.capture_from_playfield(
                playfield,
                term_width,
                term_height,
                (offset_x, offset_y),
            );

            let content_redrawn = if !self.has_previous_frame
                || frame_reset_required(&self.previous_frame, &self.current_frame)
            {
                full_repaint(
                    &mut stdout,
                    &self.current_frame,
                    self.encoding,
                    self.color_mode,
                )?
            } else {
                diff_repaint(
                    &mut stdout,
                    &self.previous_frame,
                    &self.current_frame,
                    self.encoding,
                    self.color_mode,
                )?
            };

            if cursor_update_required(
                self.has_previous_frame
                    .then_some(self.previous_frame.cursor)
                    .flatten(),
                self.current_frame.cursor,
                content_redrawn,
            ) {
                render_cursor(
                    &mut stdout,
                    self.current_frame.cursor,
                    self.current_frame.origin,
                )?;
            }

            std::mem::swap(&mut self.previous_frame, &mut self.current_frame);
            self.has_previous_frame = true;
        } else {
            for row in 0..playfield.height() {
                let line = playfield.plain_line(row);
                stdout.write_all(line.as_bytes())?;
                stdout.write_all(b"\n")?;
            }
        }
        stdout.flush()?;
        Ok(())
    }

    fn read_key(&mut self) -> Result<KeyEvent, Box<dyn std::error::Error>> {
        loop {
            if let Event::Key(key) = self.read_event()? {
                return Ok(key);
            }
        }
    }

    fn read_event(&mut self) -> Result<Event, Box<dyn std::error::Error>> {
        Ok(event::read()?)
    }

    fn dump_text_capture(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = io::stdout();
        let bg = resolve_color(classic::app_background(), self.color_mode);
        let fg = resolve_color(classic::terminal_foreground(), self.color_mode);
        if stdout.is_terminal() {
            execute!(
                stdout,
                SetBackgroundColor(bg),
                SetForegroundColor(fg),
                Clear(ClearType::All),
                MoveTo(0, 0),
                SetCursorStyle::DefaultUserShape,
                Show
            )?;
            stdout.write_all(b"\x1b[0m")?;
            self.has_previous_frame = false;
        }
        stdout.write_all(text.as_bytes())?;
        if !text.ends_with('\n') {
            stdout.write_all(b"\n")?;
        }
        stdout.flush()?;
        Ok(())
    }

    fn clear_and_restore(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = io::stdout();
        let bg = resolve_color(classic::app_background(), self.color_mode);
        let fg = resolve_color(classic::terminal_foreground(), self.color_mode);
        if stdout.is_terminal() {
            execute!(
                stdout,
                SetBackgroundColor(bg),
                SetForegroundColor(fg),
                Clear(ClearType::All),
                MoveTo(0, 0),
                SetCursorStyle::DefaultUserShape,
                Show
            )?;
            stdout.write_all(b"\x1b[0m")?;
            stdout.flush()?;
            self.has_previous_frame = false;
        }
        Ok(())
    }
}

fn frame_reset_required(previous_frame: &RenderSnapshot, current_frame: &RenderSnapshot) -> bool {
    previous_frame.width != current_frame.width
        || previous_frame.height != current_frame.height
        || previous_frame.term_width != current_frame.term_width
        || previous_frame.term_height != current_frame.term_height
        || previous_frame.origin != current_frame.origin
}

fn full_repaint(
    stdout: &mut io::Stdout,
    frame: &RenderSnapshot,
    encoding: OutputEncoding,
    color_mode: ColorMode,
) -> Result<bool, Box<dyn std::error::Error>> {
    queue!(
        stdout,
        Hide,
        SetBackgroundColor(resolve_color(classic::app_background(), color_mode)),
        SetForegroundColor(resolve_color(classic::terminal_foreground(), color_mode)),
        Clear(ClearType::All),
        MoveTo(0, 0)
    )?;
    for row in 0..frame.height {
        queue!(stdout, MoveTo(frame.origin.0, frame.origin.1 + row as u16))?;
        write_styled_cells(stdout, frame.row(row), encoding, color_mode)?;
    }
    Ok(true)
}

fn diff_repaint(
    stdout: &mut io::Stdout,
    previous_frame: &RenderSnapshot,
    current_frame: &RenderSnapshot,
    encoding: OutputEncoding,
    color_mode: ColorMode,
) -> Result<bool, Box<dyn std::error::Error>> {
    let mut content_redrawn = false;
    for row in 0..current_frame.height {
        if previous_frame.row_fingerprints[row] == current_frame.row_fingerprints[row] {
            continue;
        }
        let previous_row = previous_frame.row(row);
        let current_row = current_frame.row(row);
        for span in changed_spans(previous_row, current_row) {
            if !content_redrawn {
                queue!(stdout, Hide)?;
                content_redrawn = true;
            }
            queue!(
                stdout,
                MoveTo(
                    current_frame.origin.0 + span.start as u16,
                    current_frame.origin.1 + row as u16
                )
            )?;
            write_styled_cells(
                stdout,
                &current_row[span.start..span.end],
                encoding,
                color_mode,
            )?;
        }
    }
    Ok(content_redrawn)
}

fn render_cursor<W: Write>(
    stdout: &mut W,
    cursor: Option<(u16, u16)>,
    origin: (u16, u16),
) -> Result<(), Box<dyn std::error::Error>> {
    match cursor {
        Some((column, row)) => {
            queue!(
                stdout,
                SetCursorStyle::BlinkingBlock,
                Show,
                MoveTo(origin.0 + column, origin.1 + row)
            )?;
        }
        None => {
            queue!(stdout, Hide)?;
        }
    }
    Ok(())
}

fn cursor_update_required(
    previous_cursor: Option<(u16, u16)>,
    current_cursor: Option<(u16, u16)>,
    content_redrawn: bool,
) -> bool {
    content_redrawn || previous_cursor != current_cursor
}

/// Compute the top-left origin for centering a buffer in the terminal.
///
/// For CP437 (BBS door) mode, always pin to (0, 0) — door sessions report
/// large terminal dimensions but the playfield must not float.
/// For UTF-8, center the buffer within the terminal window.
fn render_origin(
    term_width: u16,
    term_height: u16,
    encoding: OutputEncoding,
    buffer: &PlayfieldBuffer,
) -> (u16, u16) {
    match encoding {
        OutputEncoding::Cp437 => (0, 0),
        OutputEncoding::Utf8 => (
            term_width.saturating_sub(buffer.width() as u16) / 2,
            term_height.saturating_sub(buffer.height() as u16) / 2,
        ),
    }
}

fn write_styled_cells(
    stdout: &mut io::Stdout,
    cells: &[Cell],
    encoding: OutputEncoding,
    color_mode: ColorMode,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut current_style = None;
    let mut run = String::new();
    for cell in cells {
        if current_style != Some(cell.style) {
            if !run.is_empty() {
                flush_run(stdout, &run, encoding)?;
                run.clear();
            }
            apply_style(stdout, cell.style, color_mode)?;
            current_style = Some(cell.style);
        }
        run.push(cell.ch);
    }
    if !run.is_empty() {
        flush_run(stdout, &run, encoding)?;
    }
    queue!(
        stdout,
        SetAttribute(Attribute::Reset),
        SetForegroundColor(resolve_color(classic::terminal_foreground(), color_mode)),
        SetBackgroundColor(resolve_color(classic::app_background(), color_mode))
    )?;
    Ok(())
}

fn flush_run(
    stdout: &mut io::Stdout,
    run: &str,
    encoding: OutputEncoding,
) -> Result<(), Box<dyn std::error::Error>> {
    match encoding {
        OutputEncoding::Utf8 => {
            queue!(stdout, Print(run))?;
        }
        OutputEncoding::Cp437 => {
            stdout.flush()?;
            stdout.write_all(&cp437::str_to_cp437(run))?;
        }
    }
    Ok(())
}

fn apply_style(
    stdout: &mut io::Stdout,
    style: CellStyle,
    color_mode: ColorMode,
) -> Result<(), Box<dyn std::error::Error>> {
    queue!(
        stdout,
        SetForegroundColor(resolve_color(style.fg, color_mode)),
        SetBackgroundColor(resolve_color(style.bg, color_mode)),
        SetAttribute(if style.bold {
            Attribute::Bold
        } else {
            Attribute::NormalIntensity
        })
    )?;
    Ok(())
}

/// Resolve a [`GameColor`] to a crossterm [`Color`] for the given [`ColorMode`].
pub fn resolve_color(color: GameColor, mode: ColorMode) -> Color {
    match color {
        GameColor::Black => Color::Black,
        GameColor::Red => Color::DarkRed,
        GameColor::Green => Color::DarkGreen,
        GameColor::Yellow => Color::DarkYellow,
        GameColor::Blue => Color::DarkBlue,
        GameColor::Magenta => Color::DarkMagenta,
        GameColor::Cyan => Color::DarkCyan,
        GameColor::White => Color::Grey,
        GameColor::BrightBlack => Color::DarkGrey,
        GameColor::BrightRed => Color::Red,
        GameColor::BrightGreen => Color::Green,
        GameColor::BrightYellow => Color::Yellow,
        GameColor::BrightBlue => Color::Blue,
        GameColor::BrightMagenta => Color::Magenta,
        GameColor::BrightCyan => Color::Cyan,
        GameColor::BrightWhite => Color::White,
        GameColor::Indexed(idx) => match mode {
            ColorMode::TrueColor | ColorMode::Color256 => Color::AnsiValue(idx),
            ColorMode::Ansi16 => ansi256_to_named16(idx),
        },
        GameColor::Rgb(r, g, b) => match mode {
            ColorMode::TrueColor => Color::Rgb { r, g, b },
            ColorMode::Color256 => Color::AnsiValue(rgb_to_ansi256(r, g, b)),
            ColorMode::Ansi16 => rgb_to_named16(r, g, b),
        },
    }
}

pub fn redmean_dist(r1: u8, g1: u8, b1: u8, r2: u8, g2: u8, b2: u8) -> f32 {
    let (r1, g1, b1) = (r1 as f32, g1 as f32, b1 as f32);
    let (r2, g2, b2) = (r2 as f32, g2 as f32, b2 as f32);
    let dr = r1 - r2;
    let dg = g1 - g2;
    let db = b1 - b2;
    let rbar = (r1 + r2) * 0.5;
    (2.0 + rbar / 256.0) * dr * dr + 4.0 * dg * dg + (2.0 + (255.0 - rbar) / 256.0) * db * db
}

pub fn ansi256_to_named16(idx: u8) -> Color {
    if idx < 16 {
        ANSI16_COLORS[idx as usize]
    } else if idx >= 232 {
        let level = idx - 232;
        match level {
            0..=5 => Color::Black,
            6..=11 => Color::DarkGrey,
            12..=17 => Color::Grey,
            _ => Color::White,
        }
    } else {
        let i = idx - 16;
        let r6 = i / 36;
        let g6 = (i % 36) / 6;
        let b6 = i % 6;
        let expand = |v: u8| if v == 0 { 0u8 } else { 55 + v * 40 };
        rgb_to_named16(expand(r6), expand(g6), expand(b6))
    }
}

pub fn rgb_to_named16(r: u8, g: u8, b: u8) -> Color {
    const PALETTE: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (170, 0, 0),
        (0, 170, 0),
        (170, 170, 0),
        (0, 0, 170),
        (170, 0, 170),
        (0, 170, 170),
        (170, 170, 170),
        (85, 85, 85),
        (255, 85, 85),
        (85, 255, 85),
        (255, 255, 85),
        (85, 85, 255),
        (255, 85, 255),
        (85, 255, 255),
        (255, 255, 255),
    ];
    let mut best_idx = 0usize;
    let mut best_dist = f32::MAX;
    for (i, &(pr, pg, pb)) in PALETTE.iter().enumerate() {
        let dist = redmean_dist(r, g, b, pr, pg, pb);
        if dist < best_dist {
            best_dist = dist;
            best_idx = i;
        }
    }
    ANSI16_COLORS[best_idx]
}

pub fn rgb_to_ansi256(r: u8, g: u8, b: u8) -> u8 {
    let cube_idx = |v: u8| -> u8 {
        if v < 48 {
            0
        } else if v < 115 {
            1
        } else {
            (v - 35) / 40
        }
    };
    let ri = cube_idx(r);
    let gi = cube_idx(g);
    let bi = cube_idx(b);
    let cube_index = 16 + 36 * ri + 6 * gi + bi;
    let expand = |v: u8| -> u8 { if v == 0 { 0 } else { 55 + v * 40 } };
    let cube_dist = redmean_dist(r, g, b, expand(ri), expand(gi), expand(bi));

    let avg = (r as u16 + g as u16 + b as u16) / 3;
    let gray_index = if avg < 8 {
        16u8
    } else if avg > 247 {
        231u8
    } else {
        ((avg - 8) / 10 + 232) as u8
    };
    let gray_rgb: u8 = match gray_index {
        16 => 0,
        231 => 255,
        g => 8 + 10 * (g - 232),
    };
    let gray_dist = redmean_dist(r, g, b, gray_rgb, gray_rgb, gray_rgb);

    if gray_dist < cube_dist {
        gray_index
    } else {
        cube_index
    }
}

const ANSI16_COLORS: [Color; 16] = [
    Color::Black,
    Color::DarkRed,
    Color::DarkGreen,
    Color::DarkYellow,
    Color::DarkBlue,
    Color::DarkMagenta,
    Color::DarkCyan,
    Color::Grey,
    Color::DarkGrey,
    Color::Red,
    Color::Green,
    Color::Yellow,
    Color::Blue,
    Color::Magenta,
    Color::Cyan,
    Color::White,
];

#[cfg(test)]
mod tests {
    use super::{RenderSnapshot, cursor_update_required, frame_reset_required, render_cursor};
    use crate::buffer::PlayfieldBuffer;
    use crate::buffer::{Cell, CellStyle, GameColor};
    use crate::terminal::OutputEncoding;

    fn snapshot(
        width: usize,
        height: usize,
        term_width: u16,
        term_height: u16,
        origin: (u16, u16),
    ) -> RenderSnapshot {
        let style = CellStyle::new(GameColor::White, GameColor::Black, false);
        RenderSnapshot {
            width,
            height,
            term_width,
            term_height,
            origin,
            cells: vec![Cell::new(' ', style); width * height],
            row_fingerprints: vec![0; height],
            cursor: None,
        }
    }

    #[test]
    fn frame_reset_is_not_required_when_geometry_is_stable() {
        let previous = snapshot(80, 25, 120, 40, (20, 7));
        let current = snapshot(80, 25, 120, 40, (20, 7));
        assert!(!frame_reset_required(&previous, &current));
    }

    #[test]
    fn frame_reset_is_required_when_render_origin_changes() {
        let previous = snapshot(80, 25, 120, 40, (20, 7));
        let current = snapshot(80, 25, 121, 40, (20, 7));
        assert!(frame_reset_required(&previous, &current));
    }

    #[test]
    fn cursor_update_is_skipped_when_nothing_changed() {
        assert!(!cursor_update_required(Some((4, 5)), Some((4, 5)), false));
    }

    #[test]
    fn cursor_update_happens_after_content_redraw_even_if_position_matches() {
        assert!(cursor_update_required(Some((4, 5)), Some((4, 5)), true));
    }

    #[test]
    fn cursor_update_happens_when_visibility_changes() {
        assert!(cursor_update_required(Some((4, 5)), None, false));
    }

    #[test]
    fn visible_cursor_requests_blinking_block_style() {
        let mut out = Vec::new();
        render_cursor(&mut out, Some((4, 5)), (2, 3)).expect("cursor render should succeed");
        let output = String::from_utf8_lossy(&out);
        assert!(output.contains("\x1b[1 q\x1b[?25h"));
    }

    #[test]
    fn render_origin_centers_utf8_buffer() {
        use crate::terminal::stdout::render_origin;
        let style = crate::buffer::CellStyle::new(GameColor::White, GameColor::Black, false);
        let buf = PlayfieldBuffer::new(80, 25, style);
        let (ox, oy) = render_origin(120, 40, OutputEncoding::Utf8, &buf);
        assert_eq!(ox, 20);
        assert_eq!(oy, 7);
    }

    #[test]
    fn render_origin_pins_cp437_to_zero() {
        use crate::terminal::stdout::render_origin;
        let style = crate::buffer::CellStyle::new(GameColor::White, GameColor::Black, false);
        let buf = PlayfieldBuffer::new(80, 25, style);
        let (ox, oy) = render_origin(120, 40, OutputEncoding::Cp437, &buf);
        assert_eq!(ox, 0);
        assert_eq!(oy, 0);
    }
}
