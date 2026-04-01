mod diff;

use std::io::{self, Write};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    queue,
    style::{Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};

use crate::buffer::{Cell, CellStyle, GameColor, PlayfieldBuffer};
use crate::theme::classic;
use diff::{changed_spans, fingerprint_row};

pub struct StdoutRenderer {
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RenderStats {
    full_repaint: bool,
    content_redrawn: bool,
}

impl RenderSnapshot {
    fn capture_from_buffer(
        &mut self,
        buffer: &PlayfieldBuffer,
        term_width: u16,
        term_height: u16,
        origin: (u16, u16),
    ) {
        self.width = buffer.width();
        self.height = buffer.height();
        self.term_width = term_width;
        self.term_height = term_height;
        self.origin = origin;
        self.cursor = buffer.cursor();
        self.cells.clear();
        self.row_fingerprints.clear();
        self.cells.reserve(self.width * self.height);
        self.row_fingerprints.reserve(self.height);
        for row_idx in 0..self.height {
            let row = buffer.row(row_idx);
            self.row_fingerprints.push(fingerprint_row(row));
            self.cells.extend_from_slice(row);
        }
    }

    fn row(&self, row: usize) -> &[Cell] {
        let start = row * self.width;
        &self.cells[start..start + self.width]
    }
}

impl StdoutRenderer {
    pub fn new() -> Self {
        Self {
            previous_frame: RenderSnapshot::default(),
            current_frame: RenderSnapshot::default(),
            has_previous_frame: false,
        }
    }

    pub fn reset(&mut self) {
        self.has_previous_frame = false;
    }

    pub fn render(&mut self, buffer: &PlayfieldBuffer) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = io::stdout();
        let (term_width, term_height) =
            terminal::size().unwrap_or((buffer.width() as u16, buffer.height() as u16));
        self.render_with_writer(&mut stdout, buffer, term_width, term_height)?;
        stdout.flush()?;
        Ok(())
    }

    fn render_with_writer<W: Write>(
        &mut self,
        writer: &mut W,
        buffer: &PlayfieldBuffer,
        term_width: u16,
        term_height: u16,
    ) -> Result<RenderStats, Box<dyn std::error::Error>> {
        let origin = render_origin(term_width, term_height, buffer.width(), buffer.height());
        self.current_frame
            .capture_from_buffer(buffer, term_width, term_height, origin);

        let (full_repaint, content_redrawn) = if !self.has_previous_frame
            || frame_reset_required(&self.previous_frame, &self.current_frame)
        {
            full_repaint(writer, &self.current_frame)?;
            (true, true)
        } else {
            (
                false,
                diff_repaint(writer, &self.previous_frame, &self.current_frame)?,
            )
        };

        if cursor_update_required(
            self.has_previous_frame
                .then_some(self.previous_frame.cursor)
                .flatten(),
            self.current_frame.cursor,
            content_redrawn,
        ) {
            render_cursor(writer, self.current_frame.cursor, self.current_frame.origin)?;
        }

        std::mem::swap(&mut self.previous_frame, &mut self.current_frame);
        self.has_previous_frame = true;

        Ok(RenderStats {
            full_repaint,
            content_redrawn,
        })
    }
}

impl Default for StdoutRenderer {
    fn default() -> Self {
        Self::new()
    }
}

pub fn render_to_stdout(buffer: &PlayfieldBuffer) -> Result<(), Box<dyn std::error::Error>> {
    StdoutRenderer::new().render(buffer)
}

fn write_styled_cells<W: Write>(
    stdout: &mut W,
    row: &[Cell],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut current_style = None;
    let mut run = String::new();
    for cell in row {
        if current_style != Some(cell.style) {
            if !run.is_empty() {
                queue!(stdout, Print(&run))?;
                run.clear();
            }
            apply_style(stdout, cell.style)?;
            current_style = Some(cell.style);
        }
        run.push(cell.ch);
    }
    if !run.is_empty() {
        queue!(stdout, Print(&run))?;
    }
    queue!(
        stdout,
        SetAttribute(Attribute::Reset),
        SetForegroundColor(resolve_color(classic::terminal_foreground())),
        SetBackgroundColor(resolve_color(classic::app_background()))
    )?;
    Ok(())
}

fn frame_reset_required(previous_frame: &RenderSnapshot, current_frame: &RenderSnapshot) -> bool {
    previous_frame.width != current_frame.width
        || previous_frame.height != current_frame.height
        || previous_frame.term_width != current_frame.term_width
        || previous_frame.term_height != current_frame.term_height
        || previous_frame.origin != current_frame.origin
}

fn full_repaint<W: Write>(
    stdout: &mut W,
    frame: &RenderSnapshot,
) -> Result<(), Box<dyn std::error::Error>> {
    queue!(
        stdout,
        Hide,
        SetBackgroundColor(resolve_color(classic::app_background())),
        SetForegroundColor(resolve_color(classic::terminal_foreground())),
        Clear(ClearType::All),
        MoveTo(0, 0)
    )?;
    for row in 0..frame.height {
        queue!(stdout, MoveTo(frame.origin.0, frame.origin.1 + row as u16))?;
        write_styled_cells(stdout, frame.row(row))?;
    }
    Ok(())
}

fn diff_repaint<W: Write>(
    stdout: &mut W,
    previous_frame: &RenderSnapshot,
    current_frame: &RenderSnapshot,
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
            write_styled_cells(stdout, &current_row[span.start..span.end])?;
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
            queue!(stdout, Show, MoveTo(origin.0 + column, origin.1 + row))?;
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

fn render_origin(
    term_width: u16,
    term_height: u16,
    buffer_width: usize,
    buffer_height: usize,
) -> (u16, u16) {
    (
        term_width.saturating_sub(buffer_width as u16) / 2,
        term_height.saturating_sub(buffer_height as u16) / 2,
    )
}

fn apply_style<W: Write>(
    stdout: &mut W,
    style: CellStyle,
) -> Result<(), Box<dyn std::error::Error>> {
    queue!(
        stdout,
        SetForegroundColor(resolve_color(style.fg)),
        SetBackgroundColor(resolve_color(style.bg)),
        SetAttribute(if style.bold {
            Attribute::Bold
        } else {
            Attribute::NormalIntensity
        })
    )?;
    Ok(())
}

fn resolve_color(color: GameColor) -> Color {
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
        GameColor::Indexed(idx) => Color::AnsiValue(idx),
        GameColor::Rgb(r, g, b) => Color::Rgb { r, g, b },
    }
}

#[cfg(test)]
mod tests {
    use super::{RenderSnapshot, StdoutRenderer, cursor_update_required, frame_reset_required};
    use crate::buffer::{Cell, CellStyle, GameColor, PlayfieldBuffer};

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

    fn buffer_with_text(text: &str) -> PlayfieldBuffer {
        let style = CellStyle::new(GameColor::White, GameColor::Black, false);
        let mut buffer = PlayfieldBuffer::new(10, 4, style);
        buffer.write_text(1, 1, text, style);
        buffer
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
    fn first_render_forces_full_repaint() {
        let mut renderer = StdoutRenderer::new();
        let buffer = buffer_with_text("hello");
        let mut out = Vec::new();
        let stats = renderer
            .render_with_writer(&mut out, &buffer, 100, 30)
            .expect("render should succeed");
        assert!(stats.full_repaint);
        assert!(stats.content_redrawn);
    }

    #[test]
    fn identical_second_render_skips_content_redraw() {
        let mut renderer = StdoutRenderer::new();
        let buffer = buffer_with_text("hello");
        let mut out = Vec::new();
        renderer
            .render_with_writer(&mut out, &buffer, 100, 30)
            .expect("initial render should succeed");
        out.clear();
        let stats = renderer
            .render_with_writer(&mut out, &buffer, 100, 30)
            .expect("second render should succeed");
        assert!(!stats.full_repaint);
        assert!(!stats.content_redrawn);
    }

    #[test]
    fn changed_content_uses_diff_repaint() {
        let mut renderer = StdoutRenderer::new();
        let initial = buffer_with_text("hello");
        let updated = buffer_with_text("hullo");
        let mut out = Vec::new();
        renderer
            .render_with_writer(&mut out, &initial, 100, 30)
            .expect("initial render should succeed");
        out.clear();
        let stats = renderer
            .render_with_writer(&mut out, &updated, 100, 30)
            .expect("diff render should succeed");
        assert!(!stats.full_repaint);
        assert!(stats.content_redrawn);
    }

    #[test]
    fn reset_forces_next_render_to_repaint() {
        let mut renderer = StdoutRenderer::new();
        let buffer = buffer_with_text("hello");
        let mut out = Vec::new();
        renderer
            .render_with_writer(&mut out, &buffer, 100, 30)
            .expect("initial render should succeed");
        renderer.reset();
        out.clear();
        let stats = renderer
            .render_with_writer(&mut out, &buffer, 100, 30)
            .expect("reset render should succeed");
        assert!(stats.full_repaint);
        assert!(stats.content_redrawn);
    }
}
