mod font;

use std::collections::HashMap;
use std::num::NonZeroU32;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Modifier;
use winit::event::{ElementState, KeyEvent as WinitKeyEvent};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::buffer::Cell;
#[cfg(test)]
use crate::buffer::PlayfieldBuffer;
use crate::rendered::RenderedUi;
use crate::theme::classic;

use self::font::{FontRenderer, color_to_rgb, pack_rgb};

pub const DEFAULT_CELL_WIDTH: usize = 10;
pub const DEFAULT_CELL_HEIGHT: usize = 18;

pub struct CellGridWindowRenderer {
    surface: softbuffer::Surface<&'static winit::window::Window, &'static winit::window::Window>,
    font: FontRenderer,
    previous_frame: RenderSnapshot,
    has_previous_frame: bool,
    cached_pixels: Vec<u32>,
    cached_frame_width: usize,
    cached_frame_height: usize,
    cached_grid_pixel_width: usize,
    cached_grid_pixel_height: usize,
    cached_background: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ChangedSpan {
    start: usize,
    end: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct RenderSnapshot {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
    row_fingerprints: Vec<u64>,
    cursor: Option<(usize, usize)>,
}

impl RenderSnapshot {
    #[cfg(test)]
    fn capture_from_playfield(&mut self, buffer: &PlayfieldBuffer) {
        self.width = buffer.width();
        self.height = buffer.height();
        self.cursor = buffer
            .cursor()
            .map(|(col, row)| (usize::from(col), usize::from(row)));
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

    fn capture_from_rendered(&mut self, rendered: &RenderedUi) {
        self.width = rendered.buffer.area.width as usize;
        self.height = rendered.buffer.area.height as usize;
        self.cursor = rendered
            .cursor
            .map(|(col, row)| (usize::from(col), usize::from(row)));
        self.cells.clear();
        self.row_fingerprints.clear();
        self.cells.reserve(self.width * self.height);
        self.row_fingerprints.reserve(self.height);
        let fallback = crate::theme::body_style();
        for row_idx in 0..self.height {
            let mut row_cells = Vec::with_capacity(self.width);
            for col_idx in 0..self.width {
                let cell = rendered
                    .buffer
                    .cell((col_idx as u16, row_idx as u16))
                    .expect("rendered ui cell should be in bounds");
                row_cells.push(Cell::new(
                    cell.symbol().chars().next().unwrap_or(' '),
                    crate::buffer::CellStyle::new(
                        crate::theme::from_tui_color(cell.fg, fallback.fg),
                        crate::theme::from_tui_color(cell.bg, fallback.bg),
                        cell.modifier.contains(Modifier::BOLD),
                    ),
                ));
            }
            self.row_fingerprints.push(fingerprint_row(&row_cells));
            self.cells.extend_from_slice(&row_cells);
        }
    }

    fn row(&self, row: usize) -> &[Cell] {
        let start = row * self.width;
        &self.cells[start..start + self.width]
    }
}

impl CellGridWindowRenderer {
    pub fn new(window: &'static winit::window::Window) -> Result<Self, Box<dyn std::error::Error>> {
        let context = softbuffer::Context::new(window)?;
        let surface = softbuffer::Surface::new(&context, window)?;
        Ok(Self {
            surface,
            font: FontRenderer::new()?,
            previous_frame: RenderSnapshot::default(),
            has_previous_frame: false,
            cached_pixels: Vec::new(),
            cached_frame_width: 0,
            cached_frame_height: 0,
            cached_grid_pixel_width: 0,
            cached_grid_pixel_height: 0,
            cached_background: 0,
        })
    }

    pub fn render(
        &mut self,
        rendered: &RenderedUi,
        window_pixel_width: u32,
        window_pixel_height: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if window_pixel_width == 0 || window_pixel_height == 0 {
            return Ok(());
        }

        let grid_pixel_width = rendered.buffer.area.width as usize * DEFAULT_CELL_WIDTH;
        let grid_pixel_height = rendered.buffer.area.height as usize * DEFAULT_CELL_HEIGHT;
        self.surface.resize(
            NonZeroU32::new(window_pixel_width).ok_or("pixel width must be non-zero")?,
            NonZeroU32::new(window_pixel_height).ok_or("pixel height must be non-zero")?,
        )?;
        let mut current_frame = RenderSnapshot::default();
        current_frame.capture_from_rendered(rendered);
        let background = pack_rgb(frame_background_snapshot(&current_frame));
        let frame_width = window_pixel_width as usize;
        let frame_height = window_pixel_height as usize;
        let full_repaint = !self.has_previous_frame
            || self.cached_frame_width != frame_width
            || self.cached_frame_height != frame_height
            || self.cached_grid_pixel_width != grid_pixel_width
            || self.cached_grid_pixel_height != grid_pixel_height
            || self.cached_background != background
            || self.previous_frame.width != current_frame.width
            || self.previous_frame.height != current_frame.height;

        if full_repaint {
            self.cached_pixels
                .resize(frame_width * frame_height, background);
            self.cached_pixels.fill(background);
            draw_snapshot(
                &current_frame,
                &mut self.cached_pixels,
                frame_width,
                frame_height,
                grid_pixel_width,
                grid_pixel_height,
                &mut self.font,
            );
            self.cached_frame_width = frame_width;
            self.cached_frame_height = frame_height;
            self.cached_grid_pixel_width = grid_pixel_width;
            self.cached_grid_pixel_height = grid_pixel_height;
            self.cached_background = background;
        } else {
            redraw_snapshot_diff(
                &self.previous_frame,
                &current_frame,
                &mut self.cached_pixels,
                frame_width,
                frame_height,
                grid_pixel_width,
                grid_pixel_height,
                &mut self.font,
            );
        }

        let mut frame = self.surface.buffer_mut()?;
        frame.copy_from_slice(&self.cached_pixels);
        frame.present()?;
        self.previous_frame = current_frame;
        self.has_previous_frame = true;
        Ok(())
    }
}

pub fn terminal_grid_for_pixels(pixel_width: u32, pixel_height: u32) -> (u16, u16) {
    let cols = (pixel_width.max(1) as usize / DEFAULT_CELL_WIDTH).max(1);
    let rows = (pixel_height.max(1) as usize / DEFAULT_CELL_HEIGHT).max(1);
    (
        cols.min(u16::MAX as usize) as u16,
        rows.min(u16::MAX as usize) as u16,
    )
}

pub fn cell_position_at_pixel(
    grid_cols: usize,
    grid_rows: usize,
    window_pixel_width: u32,
    window_pixel_height: u32,
    position: winit::dpi::PhysicalPosition<f64>,
) -> Option<(u16, u16)> {
    if !position.x.is_finite() || !position.y.is_finite() || position.x < 0.0 || position.y < 0.0 {
        return None;
    }

    let x = position.x.floor() as usize;
    let y = position.y.floor() as usize;
    let grid_pixel_width = grid_cols.checked_mul(DEFAULT_CELL_WIDTH)?;
    let grid_pixel_height = grid_rows.checked_mul(DEFAULT_CELL_HEIGHT)?;
    let x_offset = (window_pixel_width as usize).saturating_sub(grid_pixel_width) / 2;
    let y_offset = (window_pixel_height as usize).saturating_sub(grid_pixel_height) / 2;

    if x < x_offset || y < y_offset {
        return None;
    }
    let local_x = x - x_offset;
    let local_y = y - y_offset;
    if local_x >= grid_pixel_width || local_y >= grid_pixel_height {
        return None;
    }

    let col = local_x / DEFAULT_CELL_WIDTH;
    let row = local_y / DEFAULT_CELL_HEIGHT;
    Some((
        col.min(u16::MAX as usize) as u16,
        row.min(u16::MAX as usize) as u16,
    ))
}

pub fn is_key_press(event: &WinitKeyEvent) -> bool {
    event.state == ElementState::Pressed
}

pub fn crossterm_key_event_from_winit(
    event: &WinitKeyEvent,
    modifiers: ModifiersState,
) -> Option<KeyEvent> {
    if !is_key_press(event) {
        return None;
    }
    let key_modifiers = modifiers_to_crossterm(modifiers);
    let code = match &event.logical_key {
        Key::Named(NamedKey::ArrowUp) => KeyCode::Up,
        Key::Named(NamedKey::ArrowDown) => KeyCode::Down,
        Key::Named(NamedKey::ArrowLeft) => KeyCode::Left,
        Key::Named(NamedKey::ArrowRight) => KeyCode::Right,
        Key::Named(NamedKey::PageUp) => KeyCode::PageUp,
        Key::Named(NamedKey::PageDown) => KeyCode::PageDown,
        Key::Named(NamedKey::Home) => KeyCode::Home,
        Key::Named(NamedKey::End) => KeyCode::End,
        Key::Named(NamedKey::Enter) => KeyCode::Enter,
        Key::Named(NamedKey::Escape) => KeyCode::Esc,
        Key::Named(NamedKey::Backspace) => KeyCode::Backspace,
        Key::Named(NamedKey::Delete) => KeyCode::Delete,
        Key::Named(NamedKey::Insert) => KeyCode::Insert,
        Key::Named(NamedKey::Tab) if modifiers.shift_key() => KeyCode::BackTab,
        Key::Named(NamedKey::Tab) => KeyCode::Tab,
        Key::Named(NamedKey::F1) => KeyCode::F(1),
        Key::Named(NamedKey::F2) => KeyCode::F(2),
        Key::Named(NamedKey::F3) => KeyCode::F(3),
        Key::Named(NamedKey::F4) => KeyCode::F(4),
        Key::Named(NamedKey::F5) => KeyCode::F(5),
        Key::Named(NamedKey::F6) => KeyCode::F(6),
        Key::Named(NamedKey::F7) => KeyCode::F(7),
        Key::Named(NamedKey::F8) => KeyCode::F(8),
        Key::Named(NamedKey::F9) => KeyCode::F(9),
        Key::Named(NamedKey::F10) => KeyCode::F(10),
        Key::Named(NamedKey::F11) => KeyCode::F(11),
        Key::Named(NamedKey::F12) => KeyCode::F(12),
        _ => {
            let ch = event
                .text
                .as_ref()
                .and_then(|text| text.chars().next())
                .filter(|ch| !ch.is_control())
                .or_else(|| match &event.logical_key {
                    Key::Character(text) => text.chars().next(),
                    _ => None,
                })?;
            let ch = if key_modifiers.contains(KeyModifiers::CONTROL) {
                ch.to_ascii_lowercase()
            } else {
                ch
            };
            KeyCode::Char(ch)
        }
    };
    Some(KeyEvent::new(code, key_modifiers))
}

fn draw_snapshot(
    snapshot: &RenderSnapshot,
    frame: &mut [u32],
    frame_width: usize,
    frame_height: usize,
    grid_pixel_width: usize,
    grid_pixel_height: usize,
    font: &mut FontRenderer,
) {
    let x_offset = frame_width.saturating_sub(grid_pixel_width) / 2;
    let y_offset = frame_height.saturating_sub(grid_pixel_height) / 2;
    for row in 0..snapshot.height {
        redraw_span(
            snapshot,
            row,
            0,
            snapshot.width,
            frame,
            frame_width,
            frame_height,
            x_offset,
            y_offset,
            font,
        );
    }
}

fn redraw_snapshot_diff(
    previous: &RenderSnapshot,
    current: &RenderSnapshot,
    frame: &mut [u32],
    frame_width: usize,
    frame_height: usize,
    grid_pixel_width: usize,
    grid_pixel_height: usize,
    font: &mut FontRenderer,
) {
    let x_offset = frame_width.saturating_sub(grid_pixel_width) / 2;
    let y_offset = frame_height.saturating_sub(grid_pixel_height) / 2;

    for row in 0..current.height {
        if previous.row_fingerprints[row] == current.row_fingerprints[row] {
            continue;
        }
        for span in changed_spans(previous.row(row), current.row(row)) {
            redraw_span(
                current,
                row,
                span.start,
                span.end,
                frame,
                frame_width,
                frame_height,
                x_offset,
                y_offset,
                font,
            );
        }
    }

    if previous.cursor != current.cursor {
        if let Some((col, row)) = previous.cursor {
            redraw_span(
                current,
                row,
                col,
                col.saturating_add(1),
                frame,
                frame_width,
                frame_height,
                x_offset,
                y_offset,
                font,
            );
        }
        if let Some((col, row)) = current.cursor {
            redraw_span(
                current,
                row,
                col,
                col.saturating_add(1),
                frame,
                frame_width,
                frame_height,
                x_offset,
                y_offset,
                font,
            );
        }
    }
}

fn redraw_span(
    snapshot: &RenderSnapshot,
    row: usize,
    start_col: usize,
    end_col: usize,
    frame: &mut [u32],
    frame_width: usize,
    frame_height: usize,
    x_offset: usize,
    y_offset: usize,
    font: &mut FontRenderer,
) {
    let row_cells = snapshot.row(row);
    for col in start_col..end_col.min(snapshot.width) {
        let cell = row_cells[col];
        let invert = snapshot.cursor == Some((col, row));
        let x = x_offset + col * DEFAULT_CELL_WIDTH;
        let y = y_offset + row * DEFAULT_CELL_HEIGHT;
        if x + DEFAULT_CELL_WIDTH > frame_width || y + DEFAULT_CELL_HEIGHT > frame_height {
            continue;
        }
        font.draw_cell(frame, frame_width, x, y, cell.ch, cell.style, false, invert);
    }
}

#[cfg(test)]
fn frame_background(buffer: &PlayfieldBuffer) -> (u8, u8, u8) {
    let mut snapshot = RenderSnapshot::default();
    snapshot.capture_from_playfield(buffer);
    frame_background_snapshot(&snapshot)
}

fn frame_background_snapshot(snapshot: &RenderSnapshot) -> (u8, u8, u8) {
    let fallback = color_to_rgb(classic::body_style().bg);
    if snapshot.width == 0 || snapshot.height == 0 {
        return fallback;
    }
    let mut counts: HashMap<u32, (usize, (u8, u8, u8))> = HashMap::new();
    for cell in &snapshot.cells {
        let code = color_code(cell.style.bg);
        let rgb = color_to_rgb(cell.style.bg);
        counts
            .entry(code)
            .and_modify(|entry| entry.0 += 1)
            .or_insert((1, rgb));
    }
    counts
        .into_values()
        .max_by_key(|(count, _)| *count)
        .map(|(_, rgb)| rgb)
        .unwrap_or(fallback)
}

fn modifiers_to_crossterm(modifiers: ModifiersState) -> KeyModifiers {
    let mut mapped = KeyModifiers::empty();
    if modifiers.shift_key() {
        mapped.insert(KeyModifiers::SHIFT);
    }
    if modifiers.control_key() {
        mapped.insert(KeyModifiers::CONTROL);
    }
    if modifiers.alt_key() {
        mapped.insert(KeyModifiers::ALT);
    }
    mapped
}

fn changed_spans(previous_row: &[Cell], current_row: &[Cell]) -> Vec<ChangedSpan> {
    assert_eq!(
        previous_row.len(),
        current_row.len(),
        "diff rows must have matching widths"
    );
    let mut spans = Vec::new();
    let mut current_span_start = None;

    for idx in 0..current_row.len() {
        if previous_row[idx] != current_row[idx] {
            current_span_start.get_or_insert(idx);
            continue;
        }
        if let Some(start) = current_span_start.take() {
            spans.push(ChangedSpan { start, end: idx });
        }
    }

    if let Some(start) = current_span_start {
        spans.push(ChangedSpan {
            start,
            end: current_row.len(),
        });
    }

    spans
}

fn fingerprint_row(row: &[Cell]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for cell in row {
        hash = mix_u32(hash, cell.ch as u32);
        hash = mix_u32(hash, color_code(cell.style.fg));
        hash = mix_u32(hash, color_code(cell.style.bg));
        hash = mix_u32(hash, u32::from(cell.style.bold));
    }
    hash
}

fn mix_u32(mut hash: u64, value: u32) -> u64 {
    for byte in value.to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn color_code(color: crate::buffer::GameColor) -> u32 {
    match color {
        crate::buffer::GameColor::Black => 0,
        crate::buffer::GameColor::Red => 1,
        crate::buffer::GameColor::Green => 2,
        crate::buffer::GameColor::Yellow => 3,
        crate::buffer::GameColor::Blue => 4,
        crate::buffer::GameColor::Magenta => 5,
        crate::buffer::GameColor::Cyan => 6,
        crate::buffer::GameColor::White => 7,
        crate::buffer::GameColor::BrightBlack => 8,
        crate::buffer::GameColor::BrightRed => 9,
        crate::buffer::GameColor::BrightGreen => 10,
        crate::buffer::GameColor::BrightYellow => 11,
        crate::buffer::GameColor::BrightBlue => 12,
        crate::buffer::GameColor::BrightMagenta => 13,
        crate::buffer::GameColor::BrightCyan => 14,
        crate::buffer::GameColor::BrightWhite => 15,
        crate::buffer::GameColor::Indexed(idx) => 0x0100_0000 | u32::from(idx),
        crate::buffer::GameColor::Rgb(r, g, b) => {
            0x0200_0000 | (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::font::color_to_rgb;
    use super::{
        RenderSnapshot, changed_spans, fingerprint_row, frame_background, redraw_snapshot_diff,
    };
    use crate::buffer::{CellStyle, GameColor, PlayfieldBuffer};

    fn style() -> CellStyle {
        CellStyle::new(GameColor::White, GameColor::Black, false)
    }

    fn snapshot_for(buffer: &PlayfieldBuffer) -> RenderSnapshot {
        let mut snapshot = RenderSnapshot::default();
        snapshot.capture_from_playfield(buffer);
        snapshot
    }

    #[test]
    fn changed_spans_collapse_contiguous_cell_changes() {
        let previous = [
            crate::buffer::Cell::new('A', style()),
            crate::buffer::Cell::new('B', style()),
            crate::buffer::Cell::new('C', style()),
        ];
        let current = [
            crate::buffer::Cell::new('A', style()),
            crate::buffer::Cell::new('X', style()),
            crate::buffer::Cell::new('Y', style()),
        ];

        assert_eq!(
            changed_spans(&previous, &current),
            vec![super::ChangedSpan { start: 1, end: 3 }]
        );
    }

    #[test]
    fn fingerprint_changes_when_row_style_changes() {
        let previous = [crate::buffer::Cell::new('A', style())];
        let current = [crate::buffer::Cell::new(
            'A',
            CellStyle::new(GameColor::Black, GameColor::White, true),
        )];

        assert_ne!(fingerprint_row(&previous), fingerprint_row(&current));
    }

    #[test]
    fn cursor_change_redraws_both_old_and_new_cells() {
        let mut previous = PlayfieldBuffer::new(2, 1, style());
        previous.write_text(0, 0, "AB", style());
        previous.set_cursor(0, 0);
        let previous = snapshot_for(&previous);

        let mut current = PlayfieldBuffer::new(2, 1, style());
        current.write_text(0, 0, "AB", style());
        current.set_cursor(1, 0);
        let current = snapshot_for(&current);

        let width = 2 * super::DEFAULT_CELL_WIDTH;
        let height = super::DEFAULT_CELL_HEIGHT;
        let mut pixels = vec![0u32; width * height];
        let mut font = super::font::FontRenderer::new().expect("font renderer");

        redraw_snapshot_diff(
            &previous,
            &current,
            &mut pixels,
            width,
            height,
            width,
            height,
            &mut font,
        );

        assert!(pixels.iter().any(|pixel| *pixel != 0));
    }

    #[test]
    fn frame_background_prefers_dominant_background_over_top_left_cell() {
        let mut buffer = PlayfieldBuffer::new(
            3,
            2,
            CellStyle::new(GameColor::White, GameColor::Blue, false),
        );
        buffer.set_cell(
            0,
            0,
            'X',
            CellStyle::new(GameColor::White, GameColor::Red, false),
        );

        assert_eq!(frame_background(&buffer), color_to_rgb(GameColor::Blue));
    }
}
