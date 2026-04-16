use std::num::NonZeroU32;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::style::Style;
use ratatui_wgpu::{Builder, Dimensions, Font, WgpuBackend};
use winit::event::{ElementState, KeyEvent as WinitKeyEvent};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::rendered::{RenderedUi, blit_rendered_ui};
use crate::theme;

pub const DEFAULT_CELL_WIDTH: usize = 10;
pub const DEFAULT_CELL_HEIGHT: usize = 18;

const PRIMARY_REGULAR_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../nc-connect/assets/fonts/JetBrainsMono-Regular.ttf"
));
const PRIMARY_BOLD_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../nc-connect/assets/fonts/JetBrainsMono-Bold.ttf"
));
const PRIMARY_ITALIC_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../nc-connect/assets/fonts/JetBrainsMono-Italic.ttf"
));
const FALLBACK_REGULAR_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../nc-connect/assets/fonts/NotoSansMono-Regular.ttf"
));
const FALLBACK_BOLD_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../nc-connect/assets/fonts/NotoSansMono-Bold.ttf"
));

type NativeTerminal = Terminal<WgpuBackend<'static, 'static>>;

pub struct CellGridWindowRenderer {
    terminal: NativeTerminal,
}

impl CellGridWindowRenderer {
    pub fn new(window: Arc<winit::window::Window>) -> Result<Self, Box<dyn std::error::Error>> {
        let size = window.inner_size();
        let primary_regular =
            Font::new(PRIMARY_REGULAR_FONT).ok_or("unable to load primary regular font")?;
        let primary_bold =
            Font::new(PRIMARY_BOLD_FONT).ok_or("unable to load primary bold font")?;
        let primary_italic =
            Font::new(PRIMARY_ITALIC_FONT).ok_or("unable to load primary italic font")?;
        let fallback_regular =
            Font::new(FALLBACK_REGULAR_FONT).ok_or("unable to load fallback regular font")?;
        let fallback_bold =
            Font::new(FALLBACK_BOLD_FONT).ok_or("unable to load fallback bold font")?;
        let backend = pollster::block_on(
            Builder::from_font(primary_regular)
                .with_font_size_px(DEFAULT_CELL_HEIGHT as u32)
                .with_bold_fonts([primary_bold, fallback_bold])
                .with_italic_fonts([primary_italic])
                .with_regular_fonts([fallback_regular])
                .with_width_and_height(Dimensions {
                    width: NonZeroU32::new(size.width.max(1))
                        .ok_or("window width must be non-zero")?,
                    height: NonZeroU32::new(size.height.max(1))
                        .ok_or("window height must be non-zero")?,
                })
                .build_with_target(window),
        )?;
        Ok(Self {
            terminal: Terminal::new(backend)?,
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
        self.terminal
            .backend_mut()
            .resize(window_pixel_width, window_pixel_height);
        let body_style = Style::default()
            .fg(theme::to_tui_color(theme::body_style().fg))
            .bg(theme::to_tui_color(theme::body_style().bg));
        self.terminal.draw(|frame| {
            let area = frame.area();
            blit_rendered_ui(frame.buffer_mut(), area, rendered, body_style);
        })?;
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

#[cfg(test)]
mod tests {
    use super::{cell_position_at_pixel, terminal_grid_for_pixels};

    #[test]
    fn terminal_grid_uses_native_cell_dimensions() {
        assert_eq!(terminal_grid_for_pixels(100, 54), (10, 3));
    }

    #[test]
    fn pixel_position_maps_back_to_grid_cell() {
        assert_eq!(
            cell_position_at_pixel(
                10,
                4,
                100,
                72,
                winit::dpi::PhysicalPosition::new(24.0, 19.0)
            ),
            Some((2, 1))
        );
    }
}
