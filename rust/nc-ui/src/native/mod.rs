mod font;

use std::num::NonZeroU32;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use winit::event::{ElementState, KeyEvent as WinitKeyEvent};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::buffer::PlayfieldBuffer;
use crate::theme::classic;

use self::font::{FontRenderer, color_to_rgb, pack_rgb};

pub const DEFAULT_CELL_WIDTH: usize = 10;
pub const DEFAULT_CELL_HEIGHT: usize = 18;

pub struct CellGridWindowRenderer {
    surface: softbuffer::Surface<&'static winit::window::Window, &'static winit::window::Window>,
    font: FontRenderer,
}

impl CellGridWindowRenderer {
    pub fn new(window: &'static winit::window::Window) -> Result<Self, Box<dyn std::error::Error>> {
        let context = softbuffer::Context::new(window)?;
        let surface = softbuffer::Surface::new(&context, window)?;
        Ok(Self {
            surface,
            font: FontRenderer::new()?,
        })
    }

    pub fn render(
        &mut self,
        buffer: &PlayfieldBuffer,
        window_pixel_width: u32,
        window_pixel_height: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if window_pixel_width == 0 || window_pixel_height == 0 {
            return Ok(());
        }

        let grid_pixel_width = buffer.width() * DEFAULT_CELL_WIDTH;
        let grid_pixel_height = buffer.height() * DEFAULT_CELL_HEIGHT;
        self.surface.resize(
            NonZeroU32::new(window_pixel_width).ok_or("pixel width must be non-zero")?,
            NonZeroU32::new(window_pixel_height).ok_or("pixel height must be non-zero")?,
        )?;
        let mut frame = self.surface.buffer_mut()?;
        draw_buffer(
            buffer,
            &mut frame,
            window_pixel_width as usize,
            window_pixel_height as usize,
            grid_pixel_width,
            grid_pixel_height,
            &mut self.font,
        );
        frame.present()?;
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

fn draw_buffer(
    buffer: &PlayfieldBuffer,
    frame: &mut [u32],
    frame_width: usize,
    frame_height: usize,
    grid_pixel_width: usize,
    grid_pixel_height: usize,
    font: &mut FontRenderer,
) {
    let background = pack_rgb(frame_background(buffer));
    frame.fill(background);
    let x_offset = frame_width.saturating_sub(grid_pixel_width) / 2;
    let y_offset = frame_height.saturating_sub(grid_pixel_height) / 2;
    let cursor = buffer
        .cursor()
        .map(|(col, row)| (usize::from(col), usize::from(row)));

    for row in 0..buffer.height() {
        for col in 0..buffer.width() {
            let cell = buffer.row(row)[col];
            let invert = cursor == Some((col, row));
            let x = x_offset + col * DEFAULT_CELL_WIDTH;
            let y = y_offset + row * DEFAULT_CELL_HEIGHT;
            if x + DEFAULT_CELL_WIDTH > frame_width || y + DEFAULT_CELL_HEIGHT > frame_height {
                continue;
            }
            font.draw_cell(frame, frame_width, x, y, cell.ch, cell.style, false, invert);
        }
    }
}

fn frame_background(buffer: &PlayfieldBuffer) -> (u8, u8, u8) {
    let fallback = color_to_rgb(classic::body_style().bg);
    if buffer.width() == 0 || buffer.height() == 0 {
        return fallback;
    }
    color_to_rgb(buffer.row(0)[0].style.bg)
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
