use crate::grid::GameColor;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PrimitiveGlyph {
    Square {
        left: bool,
        right: bool,
        up: bool,
        down: bool,
    },
    Rounded(RoundedCorner),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RoundedCorner {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
}

pub(super) fn should_draw_as_primitive(ch: char) -> bool {
    primitive_glyph(ch).is_some()
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
    let Some(shape) = primitive_glyph(ch) else {
        return;
    };
    match shape {
        PrimitiveGlyph::Square {
            left,
            right,
            up,
            down,
        } => draw_square_box_glyph(
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
        ),
        PrimitiveGlyph::Rounded(corner) => draw_rounded_corner(
            frame,
            stride_px,
            cell_x,
            cell_y,
            cell_width,
            cell_height,
            corner,
            color,
        ),
    }
}

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
            if pixel + 4 <= frame.len() {
                frame[pixel..pixel + 4].copy_from_slice(&color);
            }
        }
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

fn primitive_glyph(ch: char) -> Option<PrimitiveGlyph> {
    match ch {
        '─' => Some(PrimitiveGlyph::Square {
            left: true,
            right: true,
            up: false,
            down: false,
        }),
        '│' => Some(PrimitiveGlyph::Square {
            left: false,
            right: false,
            up: true,
            down: true,
        }),
        '┌' => Some(PrimitiveGlyph::Square {
            left: false,
            right: true,
            up: false,
            down: true,
        }),
        '┐' => Some(PrimitiveGlyph::Square {
            left: true,
            right: false,
            up: false,
            down: true,
        }),
        '└' => Some(PrimitiveGlyph::Square {
            left: false,
            right: true,
            up: true,
            down: false,
        }),
        '┘' => Some(PrimitiveGlyph::Square {
            left: true,
            right: false,
            up: true,
            down: false,
        }),
        '├' => Some(PrimitiveGlyph::Square {
            left: false,
            right: true,
            up: true,
            down: true,
        }),
        '┤' => Some(PrimitiveGlyph::Square {
            left: true,
            right: false,
            up: true,
            down: true,
        }),
        '┬' => Some(PrimitiveGlyph::Square {
            left: true,
            right: true,
            up: false,
            down: true,
        }),
        '┴' => Some(PrimitiveGlyph::Square {
            left: true,
            right: true,
            up: true,
            down: false,
        }),
        '┼' => Some(PrimitiveGlyph::Square {
            left: true,
            right: true,
            up: true,
            down: true,
        }),
        '╭' => Some(PrimitiveGlyph::Rounded(RoundedCorner::TopLeft)),
        '╮' => Some(PrimitiveGlyph::Rounded(RoundedCorner::TopRight)),
        '╯' => Some(PrimitiveGlyph::Rounded(RoundedCorner::BottomRight)),
        '╰' => Some(PrimitiveGlyph::Rounded(RoundedCorner::BottomLeft)),
        _ => None,
    }
}

fn draw_square_box_glyph(
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

fn draw_rounded_corner(
    frame: &mut [u8],
    stride_px: usize,
    cell_x: usize,
    cell_y: usize,
    cell_width: usize,
    cell_height: usize,
    corner: RoundedCorner,
    color: [u8; 4],
) {
    let mid_x = cell_x as isize + (cell_width / 2) as isize;
    let mid_y = cell_y as isize + (cell_height / 2) as isize;
    let left_x = cell_x as isize;
    let right_x = cell_x as isize + cell_width.saturating_sub(1) as isize;
    let top_y = cell_y as isize;
    let bottom_y = cell_y as isize + cell_height.saturating_sub(1) as isize;
    let left_rx = (mid_x - left_x).max(1) as f32;
    let right_rx = (right_x - mid_x).max(1) as f32;
    let top_ry = (mid_y - top_y).max(1) as f32;
    let bottom_ry = (bottom_y - mid_y).max(1) as f32;
    let steps = (cell_width.max(cell_height) * 2).max(12);

    let point_at = |t: f32| -> (isize, isize) {
        let theta = t * std::f32::consts::FRAC_PI_2;
        let sin_t = theta.sin();
        let cos_t = theta.cos();
        match corner {
            RoundedCorner::TopLeft => (
                (right_x as f32 - right_rx * sin_t).round() as isize,
                (mid_y as f32 + bottom_ry * (1.0 - cos_t)).round() as isize,
            ),
            RoundedCorner::TopRight => (
                (left_x as f32 + left_rx * sin_t).round() as isize,
                (mid_y as f32 + bottom_ry * (1.0 - cos_t)).round() as isize,
            ),
            RoundedCorner::BottomRight => (
                (left_x as f32 + left_rx * sin_t).round() as isize,
                (mid_y as f32 - top_ry * (1.0 - cos_t)).round() as isize,
            ),
            RoundedCorner::BottomLeft => (
                (right_x as f32 - right_rx * sin_t).round() as isize,
                (mid_y as f32 - top_ry * (1.0 - cos_t)).round() as isize,
            ),
        }
    };

    let mut previous = point_at(0.0);
    set_pixel_rgba(frame, stride_px, previous.0, previous.1, color);
    for step in 1..=steps {
        let point = point_at(step as f32 / steps as f32);
        draw_line_rgba(
            frame, stride_px, previous.0, previous.1, point.0, point.1, color,
        );
        previous = point;
    }
    set_pixel_rgba(frame, stride_px, previous.0, previous.1, color);
}

fn set_pixel_rgba(frame: &mut [u8], stride_px: usize, x: isize, y: isize, color: [u8; 4]) {
    if x < 0 || y < 0 {
        return;
    }
    let pixel = ((y as usize) * stride_px + x as usize) * 4;
    if pixel + 4 <= frame.len() {
        frame[pixel..pixel + 4].copy_from_slice(&color);
    }
}

fn draw_line_rgba(
    frame: &mut [u8],
    stride_px: usize,
    mut x0: isize,
    mut y0: isize,
    x1: isize,
    y1: isize,
    color: [u8; 4],
) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        set_pixel_rgba(frame, stride_px, x0, y0, color);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let twice_err = err * 2;
        if twice_err >= dy {
            err += dy;
            x0 += sx;
        }
        if twice_err <= dx {
            err += dx;
            y0 += sy;
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn background() -> [u8; 4] {
        color_to_rgba(GameColor::Black)
    }

    #[test]
    fn box_glyphs_route_to_primitives() {
        for ch in [
            '─', '│', '┌', '┐', '└', '┘', '├', '┤', '┬', '┴', '┼', '╭', '╮', '╯', '╰',
        ] {
            assert!(should_draw_as_primitive(ch), "{ch} should use primitives");
        }
        for ch in ['A', '0', '?'] {
            assert!(
                !should_draw_as_primitive(ch),
                "{ch} should stay on text rendering"
            );
        }
    }

    #[test]
    fn square_box_primitive_stays_inside_cell_bounds() {
        let mut frame = vec![0; 20 * 18 * 4];
        fill_rect_rgba(&mut frame, 20, 0, 0, 20, 18, background());
        draw_cell_primitive(
            &mut frame,
            20,
            0,
            0,
            10,
            18,
            '┼',
            color_to_rgba(GameColor::White),
        );

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

    #[test]
    fn rounded_corner_joins_vertical_stem_on_same_x() {
        let width = 12usize;
        let height = 24usize;
        let stride = width;
        let mut frame = vec![0; stride * (height * 2) * 4];
        fill_rect_rgba(&mut frame, stride, 0, 0, width, height * 2, background());
        let color = color_to_rgba(GameColor::White);
        draw_cell_primitive(&mut frame, stride, 0, 0, width, height, '╮', color);
        draw_cell_primitive(&mut frame, stride, 0, height, width, height, '│', color);

        let seam_x = width / 2;
        let top_pixel = ((height - 1) * stride + seam_x) * 4;
        let bottom_pixel = (height * stride + seam_x) * 4;
        assert_eq!(&frame[top_pixel..top_pixel + 4], &color);
        assert_eq!(&frame[bottom_pixel..bottom_pixel + 4], &color);
    }
}
