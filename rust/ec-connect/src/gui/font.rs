use std::collections::HashMap;

use ec_ui::buffer::{CellStyle, GameColor};
use fontdue::{Font, FontSettings};

use super::{CELL_HEIGHT, CELL_WIDTH};

const PRIMARY_REGULAR_FONT: &[u8] =
    include_bytes!("../../assets/fonts/0xProtoNerdFontMono-Regular.ttf");
const PRIMARY_BOLD_FONT: &[u8] = include_bytes!("../../assets/fonts/0xProtoNerdFontMono-Bold.ttf");
const PRIMARY_ITALIC_FONT: &[u8] =
    include_bytes!("../../assets/fonts/0xProtoNerdFontMono-Italic.ttf");
const FALLBACK_REGULAR_FONT: &[u8] = include_bytes!("../../assets/fonts/NotoSansMono-Regular.ttf");
const FALLBACK_BOLD_FONT: &[u8] = include_bytes!("../../assets/fonts/NotoSansMono-Bold.ttf");
const FONT_SIZE_PX: f32 = 15.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum FontFaceKey {
    PrimaryRegular,
    PrimaryBold,
    PrimaryItalic,
    FallbackRegular,
    FallbackBold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GlyphRoute {
    Font(FontFaceKey, char),
    Primitive(char),
}

#[derive(Clone)]
struct GlyphBitmap {
    width: usize,
    height: usize,
    left: i32,
    top: i32,
    alpha: Vec<u8>,
}

struct LoadedFont {
    font: Font,
    baseline: i32,
    advance_pad: i32,
}

pub struct FontRenderer {
    primary_regular: LoadedFont,
    primary_bold: LoadedFont,
    primary_italic: LoadedFont,
    fallback_regular: LoadedFont,
    fallback_bold: LoadedFont,
    glyphs: HashMap<(FontFaceKey, char), GlyphBitmap>,
}

impl FontRenderer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            primary_regular: load_font(PRIMARY_REGULAR_FONT)?,
            primary_bold: load_font(PRIMARY_BOLD_FONT)?,
            primary_italic: load_font(PRIMARY_ITALIC_FONT)?,
            fallback_regular: load_font(FALLBACK_REGULAR_FONT)?,
            fallback_bold: load_font(FALLBACK_BOLD_FONT)?,
            glyphs: HashMap::new(),
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
        fill_rect(
            frame,
            stride,
            cell_x,
            cell_y,
            CELL_WIDTH,
            CELL_HEIGHT,
            pack_rgb(bg),
        );

        let ch = if ch == '\0' { ' ' } else { ch };
        if ch == ' ' {
            return;
        }

        match self.glyph_route(ch, style, italic) {
            GlyphRoute::Font(face, glyph) => {
                self.draw_font_glyph(frame, stride, cell_x, cell_y, glyph, face, fg, bg);
            }
            GlyphRoute::Primitive(glyph) => {
                draw_terminal_primitive(frame, stride, cell_x, cell_y, glyph, pack_rgb(fg));
            }
        }
    }

    fn glyph_route(&self, ch: char, style: CellStyle, italic: bool) -> GlyphRoute {
        if should_draw_as_primitive(ch) {
            return GlyphRoute::Primitive(ch);
        }

        let primary = primary_face(style, italic);
        if self.face(primary).font.has_glyph(ch) {
            return GlyphRoute::Font(primary, ch);
        }

        let fallback = fallback_face(style);
        if self.face(fallback).font.has_glyph(ch) {
            return GlyphRoute::Font(fallback, ch);
        }

        GlyphRoute::Font(primary, '?')
    }

    fn draw_font_glyph(
        &mut self,
        frame: &mut [u32],
        stride: usize,
        cell_x: usize,
        cell_y: usize,
        ch: char,
        face: FontFaceKey,
        fg: (u8, u8, u8),
        bg: (u8, u8, u8),
    ) {
        let glyph = self.glyph_bitmap(face, ch);
        let metrics = self.face(face);
        let glyph_x = cell_x as i32 + metrics.advance_pad + glyph.left;
        let glyph_y = cell_y as i32 + metrics.baseline - glyph.height as i32 - glyph.top;

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

    fn glyph_bitmap(&mut self, face: FontFaceKey, ch: char) -> GlyphBitmap {
        if let Some(glyph) = self.glyphs.get(&(face, ch)) {
            return glyph.clone();
        }
        let (metrics, alpha) = self.face(face).font.rasterize(ch, FONT_SIZE_PX);
        let glyph = GlyphBitmap {
            width: metrics.width,
            height: metrics.height,
            left: metrics.xmin,
            top: metrics.ymin,
            alpha,
        };
        self.glyphs.insert((face, ch), glyph.clone());
        glyph
    }

    fn face(&self, face: FontFaceKey) -> &LoadedFont {
        match face {
            FontFaceKey::PrimaryRegular => &self.primary_regular,
            FontFaceKey::PrimaryBold => &self.primary_bold,
            FontFaceKey::PrimaryItalic => &self.primary_italic,
            FontFaceKey::FallbackRegular => &self.fallback_regular,
            FontFaceKey::FallbackBold => &self.fallback_bold,
        }
    }
}

fn load_font(bytes: &[u8]) -> Result<LoadedFont, Box<dyn std::error::Error>> {
    let settings = FontSettings {
        scale: 40.0,
        ..FontSettings::default()
    };
    let font = Font::from_bytes(bytes, settings)?;
    let line = font
        .horizontal_line_metrics(FONT_SIZE_PX)
        .ok_or("embedded font is missing horizontal line metrics")?;
    let top_padding = ((CELL_HEIGHT as f32 - line.new_line_size).max(0.0) / 2.0).round() as i32;
    let baseline = top_padding + line.ascent.round() as i32;
    let advance_pad = 0.max(((CELL_WIDTH as f32 - FONT_SIZE_PX * 0.6) / 2.0).round() as i32);
    Ok(LoadedFont {
        font,
        baseline,
        advance_pad,
    })
}

fn primary_face(style: CellStyle, italic: bool) -> FontFaceKey {
    if italic {
        FontFaceKey::PrimaryItalic
    } else if style.bold {
        FontFaceKey::PrimaryBold
    } else {
        FontFaceKey::PrimaryRegular
    }
}

fn fallback_face(style: CellStyle) -> FontFaceKey {
    if style.bold {
        FontFaceKey::FallbackBold
    } else {
        FontFaceKey::FallbackRegular
    }
}

fn should_draw_as_primitive(ch: char) -> bool {
    box_connections(ch).is_some() || block_fill(ch).is_some()
}

fn box_connections(ch: char) -> Option<(bool, bool, bool, bool)> {
    match ch {
        '─' => Some((true, true, false, false)),
        '│' => Some((false, false, true, true)),
        '┌' => Some((false, true, false, true)),
        '┐' => Some((true, false, false, true)),
        '└' => Some((false, true, true, false)),
        '┘' => Some((true, false, true, false)),
        '├' => Some((false, true, true, true)),
        '┤' => Some((true, false, true, true)),
        '┬' => Some((true, true, false, true)),
        '┴' => Some((true, true, true, false)),
        '┼' => Some((true, true, true, true)),
        _ => None,
    }
}

fn draw_terminal_primitive(
    frame: &mut [u32],
    stride: usize,
    cell_x: usize,
    cell_y: usize,
    ch: char,
    color: u32,
) {
    if let Some((left, right, up, down)) = box_connections(ch) {
        draw_box_glyph(frame, stride, cell_x, cell_y, left, right, up, down, color);
        return;
    }
    if let Some(fill) = block_fill(ch) {
        draw_block_glyph(frame, stride, cell_x, cell_y, fill, color);
    }
}

fn draw_box_glyph(
    frame: &mut [u32],
    stride: usize,
    cell_x: usize,
    cell_y: usize,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    color: u32,
) {
    let mid_x = CELL_WIDTH / 2;
    let mid_y = CELL_HEIGHT / 2;
    if left {
        fill_rect(frame, stride, cell_x, cell_y + mid_y, mid_x + 1, 1, color);
    }
    if right {
        fill_rect(
            frame,
            stride,
            cell_x + mid_x,
            cell_y + mid_y,
            CELL_WIDTH - mid_x,
            1,
            color,
        );
    }
    if up {
        fill_rect(frame, stride, cell_x + mid_x, cell_y, 1, mid_y + 1, color);
    }
    if down {
        fill_rect(
            frame,
            stride,
            cell_x + mid_x,
            cell_y + mid_y,
            1,
            CELL_HEIGHT - mid_y,
            color,
        );
    }
    fill_rect(frame, stride, cell_x + mid_x, cell_y + mid_y, 1, 1, color);
}

#[derive(Clone, Copy)]
enum BlockFill {
    Full,
    TopHalf,
    BottomHalf,
    LeftHalf,
    RightHalf,
    LightShade,
    MediumShade,
    DarkShade,
}

fn block_fill(ch: char) -> Option<BlockFill> {
    match ch {
        '█' => Some(BlockFill::Full),
        '▀' => Some(BlockFill::TopHalf),
        '▄' => Some(BlockFill::BottomHalf),
        '▌' => Some(BlockFill::LeftHalf),
        '▐' => Some(BlockFill::RightHalf),
        '░' => Some(BlockFill::LightShade),
        '▒' => Some(BlockFill::MediumShade),
        '▓' => Some(BlockFill::DarkShade),
        _ => None,
    }
}

fn draw_block_glyph(
    frame: &mut [u32],
    stride: usize,
    cell_x: usize,
    cell_y: usize,
    fill: BlockFill,
    color: u32,
) {
    match fill {
        BlockFill::Full => fill_rect(
            frame,
            stride,
            cell_x,
            cell_y,
            CELL_WIDTH,
            CELL_HEIGHT,
            color,
        ),
        BlockFill::TopHalf => fill_rect(
            frame,
            stride,
            cell_x,
            cell_y,
            CELL_WIDTH,
            CELL_HEIGHT / 2,
            color,
        ),
        BlockFill::BottomHalf => fill_rect(
            frame,
            stride,
            cell_x,
            cell_y + CELL_HEIGHT / 2,
            CELL_WIDTH,
            CELL_HEIGHT - (CELL_HEIGHT / 2),
            color,
        ),
        BlockFill::LeftHalf => fill_rect(
            frame,
            stride,
            cell_x,
            cell_y,
            CELL_WIDTH / 2,
            CELL_HEIGHT,
            color,
        ),
        BlockFill::RightHalf => fill_rect(
            frame,
            stride,
            cell_x + CELL_WIDTH / 2,
            cell_y,
            CELL_WIDTH - (CELL_WIDTH / 2),
            CELL_HEIGHT,
            color,
        ),
        BlockFill::LightShade => draw_shade_pattern(frame, stride, cell_x, cell_y, color, 4),
        BlockFill::MediumShade => draw_shade_pattern(frame, stride, cell_x, cell_y, color, 2),
        BlockFill::DarkShade => draw_shade_pattern(frame, stride, cell_x, cell_y, color, 1),
    }
}

fn draw_shade_pattern(
    frame: &mut [u32],
    stride: usize,
    cell_x: usize,
    cell_y: usize,
    color: u32,
    divisor: usize,
) {
    for row in 0..CELL_HEIGHT {
        for col in 0..CELL_WIDTH {
            let hit = match divisor {
                1 => true,
                2 => (row + col) % 2 == 0,
                _ => (row + col) % 4 == 0,
            };
            if hit {
                frame[(cell_y + row) * stride + (cell_x + col)] = color;
            }
        }
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
    let blend =
        |bg: u8, fg: u8| -> u8 { (((u16::from(bg) * inv) + (u16::from(fg) * alpha)) / 255) as u8 };
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

#[cfg(test)]
mod tests {
    use super::*;

    fn style() -> CellStyle {
        CellStyle::new(GameColor::White, GameColor::Black, false)
    }

    #[test]
    fn primary_font_lacks_greek_but_fallback_covers_it() {
        let renderer = FontRenderer::new().expect("font renderer loads");
        assert!(!renderer.primary_regular.font.has_glyph('Α'));
        assert!(renderer.fallback_regular.font.has_glyph('Α'));
        assert!(renderer.primary_regular.font.has_glyph('─'));
    }

    #[test]
    fn glyph_route_prefers_primitives_for_terminal_box_and_block_chars() {
        let renderer = FontRenderer::new().expect("font renderer loads");
        assert_eq!(
            renderer.glyph_route('─', style(), false),
            GlyphRoute::Primitive('─')
        );
        assert_eq!(
            renderer.glyph_route('█', style(), false),
            GlyphRoute::Primitive('█')
        );
    }

    #[test]
    fn glyph_route_falls_back_for_greek() {
        let renderer = FontRenderer::new().expect("font renderer loads");
        assert_eq!(
            renderer.glyph_route('Ω', style(), false),
            GlyphRoute::Font(FontFaceKey::FallbackRegular, 'Ω')
        );
        assert_eq!(
            renderer.glyph_route(
                'Ω',
                CellStyle::new(GameColor::White, GameColor::Black, true),
                false
            ),
            GlyphRoute::Font(FontFaceKey::FallbackBold, 'Ω')
        );
    }

    #[test]
    fn horizontal_box_lines_join_without_gaps() {
        let mut frame = vec![0u32; CELL_WIDTH * CELL_HEIGHT * 2];
        let stride = CELL_WIDTH * 2;
        let color = pack_rgb((0xff, 0xff, 0xff));
        draw_terminal_primitive(&mut frame, stride, 0, 0, '─', color);
        draw_terminal_primitive(&mut frame, stride, CELL_WIDTH, 0, '─', color);

        let y = CELL_HEIGHT / 2;
        assert_eq!(frame[y * stride + (CELL_WIDTH - 1)], color);
        assert_eq!(frame[y * stride + CELL_WIDTH], color);
    }

    #[test]
    fn greek_fallback_rasterizes_visible_pixels() {
        let mut renderer = FontRenderer::new().expect("font renderer loads");
        let mut frame = vec![0u32; CELL_WIDTH * CELL_HEIGHT];
        renderer.draw_cell(&mut frame, CELL_WIDTH, 0, 0, 'Α', style(), false, false);
        assert!(frame.iter().any(|pixel| *pixel != 0));
    }

    #[test]
    fn light_and_dark_shades_have_distinct_density() {
        let stride = CELL_WIDTH * 2;
        let color = pack_rgb((0xff, 0xff, 0xff));
        let mut frame = vec![0u32; stride * CELL_HEIGHT];
        draw_terminal_primitive(&mut frame, stride, 0, 0, '░', color);
        draw_terminal_primitive(&mut frame, stride, CELL_WIDTH, 0, '▓', color);
        let light = frame
            .iter()
            .take(CELL_WIDTH * CELL_HEIGHT)
            .filter(|pixel| **pixel == color)
            .count();
        let mut dark = 0;
        for row in 0..CELL_HEIGHT {
            for col in CELL_WIDTH..(CELL_WIDTH * 2) {
                if frame[row * stride + col] == color {
                    dark += 1;
                }
            }
        }
        assert!(light > 0);
        assert!(dark > light);
    }
}
