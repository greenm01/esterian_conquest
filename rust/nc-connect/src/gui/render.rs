use std::num::NonZeroU32;

use nc_ui::buffer::{GameColor, PlayfieldBuffer};
use nc_ui::theme::classic;

use super::font::FontRenderer;
use super::{CELL_HEIGHT, CELL_WIDTH};

pub struct WindowRenderer {
    surface: softbuffer::Surface<&'static winit::window::Window, &'static winit::window::Window>,
    font: FontRenderer,
}

impl WindowRenderer {
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
        let pixel_width = buffer.width() * CELL_WIDTH;
        let pixel_height = buffer.height() * CELL_HEIGHT;
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
            pixel_width,
            pixel_height,
            &mut self.font,
        );
        frame.present()?;
        Ok(())
    }
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
    let background = pack_rgb(game_color_to_rgb(classic::body_style().bg));
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
            let x = x_offset + col * CELL_WIDTH;
            let y = y_offset + row * CELL_HEIGHT;
            if x + CELL_WIDTH > frame_width || y + CELL_HEIGHT > frame_height {
                continue;
            }
            font.draw_cell(frame, frame_width, x, y, cell.ch, cell.style, false, invert);
        }
    }
}

fn pack_rgb((r, g, b): (u8, u8, u8)) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

fn game_color_to_rgb(color: GameColor) -> (u8, u8, u8) {
    match color {
        GameColor::Black => (0x00, 0x00, 0x00),
        GameColor::Red => (0x80, 0x00, 0x00),
        GameColor::Green => (0x00, 0x80, 0x00),
        GameColor::Yellow => (0x80, 0x80, 0x00),
        GameColor::Blue => (0x00, 0x00, 0x80),
        GameColor::Magenta => (0x80, 0x00, 0x80),
        GameColor::Cyan => (0x00, 0x80, 0x80),
        GameColor::White => (0xc0, 0xc0, 0xc0),
        GameColor::BrightBlack => (0x80, 0x80, 0x80),
        GameColor::BrightRed => (0xff, 0x00, 0x00),
        GameColor::BrightGreen => (0x00, 0xff, 0x00),
        GameColor::BrightYellow => (0xff, 0xff, 0x00),
        GameColor::BrightBlue => (0x00, 0x00, 0xff),
        GameColor::BrightMagenta => (0xff, 0x00, 0xff),
        GameColor::BrightCyan => (0x00, 0xff, 0xff),
        GameColor::BrightWhite => (0xff, 0xff, 0xff),
        GameColor::Indexed(index) => ansi_indexed_rgb(index),
        GameColor::Rgb(r, g, b) => (r, g, b),
    }
}

fn ansi_indexed_rgb(index: u8) -> (u8, u8, u8) {
    match index {
        0..=15 => game_color_to_rgb(match index {
            0 => GameColor::Black,
            1 => GameColor::Red,
            2 => GameColor::Green,
            3 => GameColor::Yellow,
            4 => GameColor::Blue,
            5 => GameColor::Magenta,
            6 => GameColor::Cyan,
            7 => GameColor::White,
            8 => GameColor::BrightBlack,
            9 => GameColor::BrightRed,
            10 => GameColor::BrightGreen,
            11 => GameColor::BrightYellow,
            12 => GameColor::BrightBlue,
            13 => GameColor::BrightMagenta,
            14 => GameColor::BrightCyan,
            _ => GameColor::BrightWhite,
        }),
        16..=231 => {
            let idx = index - 16;
            let b = idx % 6;
            let g = (idx / 6) % 6;
            let r = idx / 36;
            let expand = |value: u8| if value == 0 { 0 } else { 55 + value * 40 };
            (expand(r), expand(g), expand(b))
        }
        232..=255 => {
            let value = 8 + (index - 232) * 10;
            (value, value, value)
        }
    }
}
