pub const H_LINE: char = '─';
pub const V_LINE: char = '│';
pub const TOP_LEFT: char = '┌';
pub const TOP_RIGHT: char = '┐';
pub const BOTTOM_LEFT: char = '└';
pub const BOTTOM_RIGHT: char = '┘';

pub fn fill_rect<S: Copy>(
    buffer_width: usize,
    buffer_height: usize,
    left: usize,
    top: usize,
    width: usize,
    height: usize,
    style: S,
    mut set_cell: impl FnMut(usize, usize, char, S),
) {
    let max_row = top.saturating_add(height).min(buffer_height);
    let max_col = left.saturating_add(width).min(buffer_width);
    for row in top..max_row {
        for col in left..max_col {
            set_cell(row, col, ' ', style);
        }
    }
}

pub fn draw_box_outline<S: Copy>(
    buffer_width: usize,
    buffer_height: usize,
    left: usize,
    top: usize,
    width: usize,
    height: usize,
    style: S,
    mut set_cell: impl FnMut(usize, usize, char, S),
) {
    if width < 2 || height < 2 || left >= buffer_width || top >= buffer_height {
        return;
    }

    let right = left.saturating_add(width.saturating_sub(1));
    let bottom = top.saturating_add(height.saturating_sub(1));
    if right >= buffer_width || bottom >= buffer_height {
        return;
    }

    for col in left + 1..right {
        set_cell(top, col, H_LINE, style);
        set_cell(bottom, col, H_LINE, style);
    }
    for row in top + 1..bottom {
        set_cell(row, left, V_LINE, style);
        set_cell(row, right, V_LINE, style);
    }
    set_cell(top, left, TOP_LEFT, style);
    set_cell(top, right, TOP_RIGHT, style);
    set_cell(bottom, left, BOTTOM_LEFT, style);
    set_cell(bottom, right, BOTTOM_RIGHT, style);
}
