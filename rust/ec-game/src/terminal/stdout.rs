mod diff;

use std::io::{self, IsTerminal, Write};

use crate::screen::{Cell, CellStyle, GameColor, PlayfieldBuffer};
use crate::screen::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};
use crate::terminal::ColorMode;
use crate::terminal::OutputEncoding;
use crate::terminal::Terminal;
use crate::terminal::cp437;
use crate::theme::classic;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyEvent},
    execute, queue,
    style::{Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use diff::changed_spans;

pub struct StdoutTerminal {
    encoding: OutputEncoding,
    color_mode: ColorMode,
    previous_frame: Option<RenderSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RenderSnapshot {
    width: usize,
    height: usize,
    term_width: u16,
    term_height: u16,
    origin: (u16, u16),
    cells: Vec<Cell>,
    cursor: Option<(u16, u16)>,
}

impl RenderSnapshot {
    fn from_playfield(
        playfield: &PlayfieldBuffer,
        term_width: u16,
        term_height: u16,
        origin: (u16, u16),
    ) -> Self {
        let mut cells = Vec::with_capacity(playfield.width() * playfield.height());
        for row in 0..playfield.height() {
            cells.extend_from_slice(playfield.row(row));
        }
        Self {
            width: playfield.width(),
            height: playfield.height(),
            term_width,
            term_height,
            origin,
            cells,
            cursor: playfield.cursor(),
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
            previous_frame: None,
        }
    }

    pub fn with_encoding(encoding: OutputEncoding) -> Self {
        Self {
            encoding,
            color_mode: ColorMode::TrueColor,
            previous_frame: None,
        }
    }

    pub fn with_encoding_and_color_mode(encoding: OutputEncoding, color_mode: ColorMode) -> Self {
        Self {
            encoding,
            color_mode,
            previous_frame: None,
        }
    }
}

impl Terminal for StdoutTerminal {
    fn render(&mut self, playfield: &PlayfieldBuffer) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = io::stdout();
        let bg = resolve_color(classic::app_background(), self.color_mode);
        let fg = resolve_color(classic::terminal_foreground(), self.color_mode);
        if stdout.is_terminal() {
            let (term_width, term_height) = terminal::size()?;
            let (offset_x, offset_y) = render_origin(term_width, term_height, self.encoding);
            let current_frame = RenderSnapshot::from_playfield(
                playfield,
                term_width,
                term_height,
                (offset_x, offset_y),
            );
            execute!(stdout, Hide, SetBackgroundColor(bg), SetForegroundColor(fg))?;
            if frame_reset_required(self.previous_frame.as_ref(), &current_frame) {
                full_repaint(&mut stdout, &current_frame, self.encoding, self.color_mode)?;
            } else if let Some(previous_frame) = self.previous_frame.as_ref() {
                diff_repaint(
                    &mut stdout,
                    previous_frame,
                    &current_frame,
                    self.encoding,
                    self.color_mode,
                )?;
            }
            render_cursor(&mut stdout, &current_frame)?;
            self.previous_frame = Some(current_frame);
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
            match event::read()? {
                Event::Key(key) => return Ok(key),
                _ => continue,
            }
        }
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
                Show
            )?;
            stdout.write_all(b"\x1b[0m")?;
            self.previous_frame = None;
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
                Show
            )?;
            stdout.write_all(b"\x1b[0m")?;
            stdout.flush()?;
            self.previous_frame = None;
        }
        Ok(())
    }
}

fn frame_reset_required(
    previous_frame: Option<&RenderSnapshot>,
    current_frame: &RenderSnapshot,
) -> bool {
    let Some(previous_frame) = previous_frame else {
        return true;
    };
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
) -> Result<(), Box<dyn std::error::Error>> {
    queue!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
    for row in 0..frame.height {
        queue!(stdout, MoveTo(frame.origin.0, frame.origin.1 + row as u16))?;
        write_styled_cells(stdout, frame.row(row), encoding, color_mode)?;
    }
    Ok(())
}

fn diff_repaint(
    stdout: &mut io::Stdout,
    previous_frame: &RenderSnapshot,
    current_frame: &RenderSnapshot,
    encoding: OutputEncoding,
    color_mode: ColorMode,
) -> Result<(), Box<dyn std::error::Error>> {
    for row in 0..current_frame.height {
        let previous_row = previous_frame.row(row);
        let current_row = current_frame.row(row);
        if previous_row == current_row {
            continue;
        }
        for span in changed_spans(previous_row, current_row) {
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
    Ok(())
}

fn render_cursor(
    stdout: &mut io::Stdout,
    frame: &RenderSnapshot,
) -> Result<(), Box<dyn std::error::Error>> {
    match frame.cursor {
        Some((column, row)) => {
            queue!(
                stdout,
                Show,
                MoveTo(frame.origin.0 + column, frame.origin.1 + row)
            )?;
        }
        None => {
            queue!(stdout, Hide)?;
        }
    }
    Ok(())
}

fn render_origin(term_width: u16, term_height: u16, encoding: OutputEncoding) -> (u16, u16) {
    match encoding {
        // BBS doors often report large terminal dimensions (for example SyncTERM
        // full-window sessions), but the EC client is still a fixed 80x25
        // playfield. In door mode, pin to the classic top-left origin instead of
        // centering inside the remote window.
        OutputEncoding::Cp437 => (0, 0),
        OutputEncoding::Utf8 => (
            term_width.saturating_sub(PLAYFIELD_WIDTH as u16) / 2,
            term_height.saturating_sub(PLAYFIELD_HEIGHT as u16) / 2,
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

#[cfg(test)]
mod tests {
    use super::{RenderSnapshot, frame_reset_required};
    use crate::screen::{Cell, CellStyle, GameColor};

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
            cursor: None,
        }
    }

    #[test]
    fn frame_reset_is_required_without_a_previous_frame() {
        let current = snapshot(80, 25, 120, 40, (20, 7));
        assert!(frame_reset_required(None, &current));
    }

    #[test]
    fn frame_reset_is_not_required_when_geometry_is_stable() {
        let previous = snapshot(80, 25, 120, 40, (20, 7));
        let current = snapshot(80, 25, 120, 40, (20, 7));
        assert!(!frame_reset_required(Some(&previous), &current));
    }

    #[test]
    fn frame_reset_is_required_when_render_origin_changes() {
        let previous = snapshot(80, 25, 120, 40, (20, 7));
        let current = snapshot(80, 25, 121, 40, (20, 7));
        assert!(frame_reset_required(Some(&previous), &current));
    }
}

/// Write a text run to stdout, encoding it according to the output mode.
///
/// In UTF-8 mode, the run is emitted via crossterm `Print` (standard UTF-8).
/// In CP437 mode, each character is translated to its single-byte CP437
/// equivalent and written as raw bytes, bypassing crossterm's UTF-8 `Print`.
/// The surrounding SGR/CSI escape sequences are pure ASCII and identical in
/// both encodings, so only the content bytes need translation.
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
            // In CP437 mode we emit raw text bytes directly. Flush any queued ANSI
            // style/cursor escapes first so the control sequences land before the
            // text they are meant to affect.
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
///
/// Named 16-color variants are always emitted as-is, regardless of mode.
/// `Indexed` and `Rgb` values are downgraded when the terminal cannot support them:
/// - In `Ansi16` mode, they are mapped to the nearest named 16-color equivalent.
/// - In `Color256` mode, `Rgb` values are mapped to the nearest xterm-256 index;
///   `Indexed` values pass through as-is.
/// - In `TrueColor` mode, all values are emitted at full fidelity.
pub(crate) fn resolve_color(color: GameColor, mode: ColorMode) -> Color {
    match color {
        // Named 16-color variants — always pass through directly.
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

        // 256-color indexed palette.
        GameColor::Indexed(idx) => match mode {
            ColorMode::TrueColor | ColorMode::Color256 => Color::AnsiValue(idx),
            ColorMode::Ansi16 => ansi256_to_named16(idx),
        },

        // 24-bit RGB truecolor.
        GameColor::Rgb(r, g, b) => match mode {
            ColorMode::TrueColor => Color::Rgb { r, g, b },
            ColorMode::Color256 => Color::AnsiValue(rgb_to_ansi256(r, g, b)),
            ColorMode::Ansi16 => rgb_to_named16(r, g, b),
        },
    }
}

// ---------------------------------------------------------------------------
// Color downgrade helpers
// ---------------------------------------------------------------------------

/// Perceptually-weighted squared color distance (redmean approximation).
///
/// Weights green highest (~4×), with red and blue weighted adaptively
/// based on the average red level of the two colors.  Significantly more
/// accurate than plain Euclidean distance in RGB space for human-perceived
/// color similarity.
///
/// Returns the squared distance — no `sqrt` is needed because we only
/// compare relative magnitudes.
pub fn redmean_dist(r1: u8, g1: u8, b1: u8, r2: u8, g2: u8, b2: u8) -> f32 {
    let (r1, g1, b1) = (r1 as f32, g1 as f32, b1 as f32);
    let (r2, g2, b2) = (r2 as f32, g2 as f32, b2 as f32);
    let dr = r1 - r2;
    let dg = g1 - g2;
    let db = b1 - b2;
    let rbar = (r1 + r2) * 0.5;
    (2.0 + rbar / 256.0) * dr * dr + 4.0 * dg * dg + (2.0 + (255.0 - rbar) / 256.0) * db * db
}

/// Map an xterm-256 palette index to the nearest crossterm named 16-color.
///
/// Indices 0–15 are the standard ANSI colors and map directly.
/// Indices 16–231 are the 6×6×6 color cube; they are converted to RGB and
/// matched to the nearest named 16-color.
/// Indices 232–255 are the grayscale ramp; they map to Black, DarkGrey,
/// Grey, or White depending on brightness.
pub fn ansi256_to_named16(idx: u8) -> Color {
    if idx < 16 {
        // Direct ANSI 0–15 mapping.
        ANSI16_COLORS[idx as usize]
    } else if idx >= 232 {
        // Grayscale ramp: 232=very dark, 255=very bright.
        let level = idx - 232; // 0–23
        match level {
            0..=5 => Color::Black,
            6..=11 => Color::DarkGrey,
            12..=17 => Color::Grey,
            _ => Color::White,
        }
    } else {
        // 6×6×6 color cube: indices 16–231.
        // index = 16 + 36*r + 6*g + b  where r,g,b in 0..=5
        let i = idx - 16;
        let r6 = i / 36;
        let g6 = (i % 36) / 6;
        let b6 = i % 6;
        // Each cube step maps to: 0, 95, 135, 175, 215, 255.
        let expand = |v: u8| if v == 0 { 0u8 } else { 55 + v * 40 };
        rgb_to_named16(expand(r6), expand(g6), expand(b6))
    }
}

/// Map an RGB value to the nearest crossterm named 16-color using
/// redmean perceptually-weighted distance.
pub fn rgb_to_named16(r: u8, g: u8, b: u8) -> Color {
    // Representative RGB values for the 16 ANSI colors (standard VGA palette).
    const PALETTE: [(u8, u8, u8); 16] = [
        (0, 0, 0),       // Black
        (170, 0, 0),     // DarkRed
        (0, 170, 0),     // DarkGreen
        (170, 170, 0),   // DarkYellow
        (0, 0, 170),     // DarkBlue
        (170, 0, 170),   // DarkMagenta
        (0, 170, 170),   // DarkCyan
        (170, 170, 170), // Grey
        (85, 85, 85),    // DarkGrey
        (255, 85, 85),   // Red
        (85, 255, 85),   // Green
        (255, 255, 85),  // Yellow
        (85, 85, 255),   // Blue
        (255, 85, 255),  // Magenta
        (85, 255, 255),  // Cyan
        (255, 255, 255), // White
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

/// Map an RGB value to the nearest xterm-256 palette index.
///
/// Computes both the nearest 6×6×6 color-cube entry (indices 16–231) and
/// the nearest grayscale-ramp entry (indices 232–255), then picks whichever
/// has the lower redmean perceptual distance to the input color.
pub fn rgb_to_ansi256(r: u8, g: u8, b: u8) -> u8 {
    // --- nearest color-cube entry ---
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

    // --- nearest grayscale-ramp entry ---
    let avg = (r as u16 + g as u16 + b as u16) / 3;
    let gray_index = if avg < 8 {
        16u8 // black end of cube
    } else if avg > 247 {
        231u8 // white end of cube
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

/// The 16 crossterm named colors in standard ANSI index order (0–15).
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
