use std::num::NonZeroU32;

use ec_ui::buffer::PlayfieldBuffer;

use super::font::FontRenderer;
use super::{CELL_HEIGHT, CELL_WIDTH};

pub struct WindowRenderer {
    surface: softbuffer::Surface<&'static winit::window::Window, &'static winit::window::Window>,
    font: FontRenderer,
}

impl WindowRenderer {
    pub fn new(
        window: &'static winit::window::Window,
    ) -> Result<Self, Box<dyn std::error::Error>> {
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
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pixel_width = buffer.width() * CELL_WIDTH;
        let pixel_height = buffer.height() * CELL_HEIGHT;
        self.surface.resize(
            NonZeroU32::new(pixel_width as u32).ok_or("pixel width must be non-zero")?,
            NonZeroU32::new(pixel_height as u32).ok_or("pixel height must be non-zero")?,
        )?;
        let mut frame = self.surface.buffer_mut()?;
        draw_buffer(buffer, &mut frame, pixel_width, &mut self.font);
        frame.present()?;
        Ok(())
    }
}

fn draw_buffer(
    buffer: &PlayfieldBuffer,
    frame: &mut [u32],
    stride: usize,
    font: &mut FontRenderer,
) {
    let cursor = buffer
        .cursor()
        .map(|(col, row)| (usize::from(col), usize::from(row)));
    for row in 0..buffer.height() {
        for col in 0..buffer.width() {
            let cell = buffer.row(row)[col];
            let invert = cursor == Some((col, row));
            let x = col * CELL_WIDTH;
            let y = row * CELL_HEIGHT;
            font.draw_cell(frame, stride, x, y, cell.ch, cell.style, false, invert);
        }
    }
}
