use std::collections::HashMap;

use ec_ui::buffer::{CellStyle, GameColor};
use fontdue::{Font, FontSettings};

use super::{CELL_HEIGHT, CELL_WIDTH};

const REGULAR_FONT: &[u8] =
    include_bytes!("../../assets/fonts/0xProtoNerdFontMono-Regular.ttf");
const BOLD_FONT: &[u8] = include_bytes!("../../assets/fonts/0xProtoNerdFontMono-Bold.ttf");
const ITALIC_FONT: &[u8] =
    include_bytes!("../../assets/fonts/0xProtoNerdFontMono-Italic.ttf");
const FONT_SIZE_PX: f32 = 15.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum FontVariant {
    Regular,
    Bold,
    Italic,
}

#[derive(Clone)]
struct GlyphBitmap {
    width: usize,
    height: usize,
    left: i32,
    top: i32,
    alpha: Vec<u8>,
}

pub struct FontRenderer {
    regular: Font,
    bold: Font,
    italic: Font,
    glyphs: HashMap<(FontVariant, char), GlyphBitmap>,
    baseline: i32,
    advance_pad: i32,
}

impl FontRenderer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let settings = FontSettings {
            scale: 40.0,
            ..FontSettings::default()
        };
        let regular = Font::from_bytes(REGULAR_FONT, settings)?;
        let bold = Font::from_bytes(BOLD_FONT, settings)?;
        let italic = Font::from_bytes(ITALIC_FONT, settings)?;
        let line = regular
            .horizontal_line_metrics(FONT_SIZE_PX)
            .ok_or("embedded font is missing horizontal line metrics")?;
        let top_padding = ((CELL_HEIGHT as f32 - line.new_line_size).max(0.0) / 2.0).round() as i32;
        let baseline = top_padding + line.ascent.round() as i32;
        let advance_pad = 0.max(((CELL_WIDTH as f32 - FONT_SIZE_PX * 0.6) / 2.0).round() as i32);
        Ok(Self {
            regular,
            bold,
            italic,
            glyphs: HashMap::new(),
            baseline,
            advance_pad,
        })
    }

    pub fn draw_cell(
        &mut self,
        frame: &mut [u32],
        stride: usize,
        cell_x: usize,
        cell_y: usize,
        ch: char,
        style: CellStyle,
        italic: bool,
        invert: bool,
    ) {
        let mut fg = color_to_rgb(style.fg);
        let mut bg = color_to_rgb(style.bg);
        if invert {
            std::mem::swap(&mut fg, &mut bg);
        }
        fill_rect(frame, stride, cell_x, cell_y, CELL_WIDTH, CELL_HEIGHT, pack_rgb(bg));

        let ch = if ch == '\0' { ' ' } else { ch };
        if ch == ' ' {
            return;
        }

        let variant = if italic {
            FontVariant::Italic
        } else if style.bold {
            FontVariant::Bold
        } else {
            FontVariant::Regular
        };
        let glyph = self.glyph_bitmap(variant, ch);
        let glyph_x = cell_x as i32 + self.advance_pad + glyph.left;
        let glyph_y = cell_y as i32 + self.baseline - glyph.height as i32 - glyph.top;

        for row in 0..glyph.height {
            let dest_y = glyph_y + row as i32;
            if dest_y < 0 {
                continue;
            }
            let dest_y = dest_y as usize;
            if dest_y >= frame.len() / stride {
                continue;
            }
            for col in 0..glyph.width {
                let dest_x = glyph_x + col as i32;
                if dest_x < 0 {
                    continue;
                }
                let dest_x = dest_x as usize;
                if dest_x >= stride {
                    continue;
                }
                let alpha = glyph.alpha[row * glyph.width + col];
                if alpha == 0 {
                    continue;
                }
                let pixel = blend(bg, fg, alpha);
                frame[dest_y * stride + dest_x] = pack_rgb(pixel);
            }
        }
    }

    fn glyph_bitmap(&mut self, variant: FontVariant, ch: char) -> GlyphBitmap {
        if let Some(glyph) = self.glyphs.get(&(variant, ch)) {
            return glyph.clone();
        }
        let font = match variant {
            FontVariant::Regular => &self.regular,
            FontVariant::Bold => &self.bold,
            FontVariant::Italic => &self.italic,
        };
        let (metrics, alpha) = font.rasterize(ch, FONT_SIZE_PX);
        let glyph = GlyphBitmap {
            width: metrics.width,
            height: metrics.height,
            left: metrics.xmin,
            top: metrics.ymin,
            alpha,
        };
        self.glyphs.insert((variant, ch), glyph.clone());
        glyph
    }
}

fn fill_rect(
    frame: &mut [u32],
    stride: usize,
    x0: usize,
    y0: usize,
    width: usize,
    height: usize,
    color: u32,
) {
    for row in 0..height {
        let offset = (y0 + row) * stride + x0;
        for col in 0..width {
            frame[offset + col] = color;
        }
    }
}

fn blend(bg: (u8, u8, u8), fg: (u8, u8, u8), alpha: u8) -> (u8, u8, u8) {
    let alpha = u16::from(alpha);
    let inv = 255u16.saturating_sub(alpha);
    let blend = |bg: u8, fg: u8| -> u8 {
        (((u16::from(bg) * inv) + (u16::from(fg) * alpha)) / 255) as u8
    };
    (blend(bg.0, fg.0), blend(bg.1, fg.1), blend(bg.2, fg.2))
}

fn pack_rgb((r, g, b): (u8, u8, u8)) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

fn color_to_rgb(color: GameColor) -> (u8, u8, u8) {
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
        0..=15 => color_to_rgb(match index {
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
