use crate::buffer::GameColor;

pub(super) fn fill_rect_rgba(
    frame: &mut [u8],
    stride_px: usize,
    x0: usize,
    y0: usize,
    width: usize,
    height: usize,
    color: [u8; 4],
) {
    for row in 0..height {
        let row_start = ((y0 + row) * stride_px + x0) * 4;
        for col in 0..width {
            let pixel = row_start + col * 4;
            frame[pixel..pixel + 4].copy_from_slice(&color);
        }
    }
}

pub(super) fn should_draw_as_primitive(ch: char) -> bool {
    box_connections(ch).is_some() || block_fill(ch).is_some()
}

pub(super) fn draw_cell_primitive(
    frame: &mut [u8],
    stride_px: usize,
    cell_x: usize,
    cell_y: usize,
    cell_width: usize,
    cell_height: usize,
    ch: char,
    color: [u8; 4],
) {
    if let Some((left, right, up, down)) = box_connections(ch) {
        draw_box_glyph(
            frame,
            stride_px,
            cell_x,
            cell_y,
            cell_width,
            cell_height,
            left,
            right,
            up,
            down,
            color,
        );
        return;
    }
    if let Some(fill) = block_fill(ch) {
        draw_block_glyph(
            frame,
            stride_px,
            cell_x,
            cell_y,
            cell_width,
            cell_height,
            fill,
            color,
        );
    }
}

pub(super) fn color_to_rgba(color: GameColor) -> [u8; 4] {
    let (r, g, b) = match color {
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
    };
    [r, g, b, 0xff]
}

fn ansi_indexed_rgb(index: u8) -> (u8, u8, u8) {
    match index {
        0..=15 => match index {
            0 => (0x00, 0x00, 0x00),
            1 => (0x80, 0x00, 0x00),
            2 => (0x00, 0x80, 0x00),
            3 => (0x80, 0x80, 0x00),
            4 => (0x00, 0x00, 0x80),
            5 => (0x80, 0x00, 0x80),
            6 => (0x00, 0x80, 0x80),
            7 => (0xc0, 0xc0, 0xc0),
            8 => (0x80, 0x80, 0x80),
            9 => (0xff, 0x00, 0x00),
            10 => (0x00, 0xff, 0x00),
            11 => (0xff, 0xff, 0x00),
            12 => (0x00, 0x00, 0xff),
            13 => (0xff, 0x00, 0xff),
            14 => (0x00, 0xff, 0xff),
            _ => (0xff, 0xff, 0xff),
        },
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

fn draw_box_glyph(
    frame: &mut [u8],
    stride_px: usize,
    cell_x: usize,
    cell_y: usize,
    cell_width: usize,
    cell_height: usize,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    color: [u8; 4],
) {
    let mid_x = cell_width / 2;
    let mid_y = cell_height / 2;
    if left {
        fill_rect_rgba(
            frame,
            stride_px,
            cell_x,
            cell_y + mid_y,
            mid_x + 1,
            1,
            color,
        );
    }
    if right {
        fill_rect_rgba(
            frame,
            stride_px,
            cell_x + mid_x,
            cell_y + mid_y,
            cell_width - mid_x,
            1,
            color,
        );
    }
    if up {
        fill_rect_rgba(
            frame,
            stride_px,
            cell_x + mid_x,
            cell_y,
            1,
            mid_y + 1,
            color,
        );
    }
    if down {
        fill_rect_rgba(
            frame,
            stride_px,
            cell_x + mid_x,
            cell_y + mid_y,
            1,
            cell_height - mid_y,
            color,
        );
    }
    fill_rect_rgba(
        frame,
        stride_px,
        cell_x + mid_x,
        cell_y + mid_y,
        1,
        1,
        color,
    );
}

fn draw_block_glyph(
    frame: &mut [u8],
    stride_px: usize,
    cell_x: usize,
    cell_y: usize,
    cell_width: usize,
    cell_height: usize,
    fill: BlockFill,
    color: [u8; 4],
) {
    match fill {
        BlockFill::Full => fill_rect_rgba(
            frame,
            stride_px,
            cell_x,
            cell_y,
            cell_width,
            cell_height,
            color,
        ),
        BlockFill::TopHalf => fill_rect_rgba(
            frame,
            stride_px,
            cell_x,
            cell_y,
            cell_width,
            cell_height / 2,
            color,
        ),
        BlockFill::BottomHalf => fill_rect_rgba(
            frame,
            stride_px,
            cell_x,
            cell_y + cell_height / 2,
            cell_width,
            cell_height - (cell_height / 2),
            color,
        ),
        BlockFill::LeftHalf => fill_rect_rgba(
            frame,
            stride_px,
            cell_x,
            cell_y,
            cell_width / 2,
            cell_height,
            color,
        ),
        BlockFill::RightHalf => fill_rect_rgba(
            frame,
            stride_px,
            cell_x + cell_width / 2,
            cell_y,
            cell_width - (cell_width / 2),
            cell_height,
            color,
        ),
        BlockFill::LightShade => draw_shade_pattern(
            frame,
            stride_px,
            cell_x,
            cell_y,
            cell_width,
            cell_height,
            color,
            4,
        ),
        BlockFill::MediumShade => draw_shade_pattern(
            frame,
            stride_px,
            cell_x,
            cell_y,
            cell_width,
            cell_height,
            color,
            2,
        ),
        BlockFill::DarkShade => draw_shade_pattern(
            frame,
            stride_px,
            cell_x,
            cell_y,
            cell_width,
            cell_height,
            color,
            1,
        ),
    }
}

fn draw_shade_pattern(
    frame: &mut [u8],
    stride_px: usize,
    cell_x: usize,
    cell_y: usize,
    cell_width: usize,
    cell_height: usize,
    color: [u8; 4],
    divisor: usize,
) {
    for row in 0..cell_height {
        for col in 0..cell_width {
            let hit = match divisor {
                1 => true,
                2 => (row + col) % 2 == 0,
                _ => (row + col) % 4 == 0,
            };
            if hit {
                let pixel = ((cell_y + row) * stride_px + (cell_x + col)) * 4;
                frame[pixel..pixel + 4].copy_from_slice(&color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::CellStyle;

    fn style() -> CellStyle {
        CellStyle::new(GameColor::White, GameColor::Black, false)
    }

    fn background() -> [u8; 4] {
        color_to_rgba(GameColor::Black)
    }

    #[test]
    fn glyph_catalog_routes_block_shade_chars_to_primitives() {
        for ch in ['─', '│', '┼', '█', '░', '▒', '▓'] {
            assert!(should_draw_as_primitive(ch), "{ch} should use primitives");
        }
        for ch in ['△', '⨁', '◊', '·', 'Ω'] {
            assert!(!should_draw_as_primitive(ch), "{ch} should use text");
        }
    }

    #[test]
    fn box_primitive_stays_inside_cell_bounds() {
        let mut frame = vec![0; 10 * 2 * 18 * 4];
        fill_rect_rgba(&mut frame, 20, 0, 0, 20, 18, background());
        draw_cell_primitive(&mut frame, 20, 0, 0, 10, 18, '┼', color_to_rgba(style().fg));

        for row in 0..18 {
            for col in 10..20 {
                let pixel = (row * 20 + col) * 4;
                assert_eq!(
                    &frame[pixel..pixel + 4],
                    &background(),
                    "primitive spilled into adjacent cell at row {row}, col {col}"
                );
            }
        }
    }
}
