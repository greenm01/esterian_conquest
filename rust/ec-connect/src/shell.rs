use ec_ui::buffer::PlayfieldBuffer;
use ec_ui::theme::classic;

pub const INNER_WIDTH: usize = 80;
pub const INNER_HEIGHT: usize = 25;
pub const OUTER_WIDTH: usize = INNER_WIDTH + 2;
pub const OUTER_HEIGHT: usize = INNER_HEIGHT + 2;
pub const INNER_ORIGIN_COL: usize = 1;
pub const INNER_ORIGIN_ROW: usize = 1;

pub fn outer_title() -> String {
    format!("EC CONNECT v{}", env!("CARGO_PKG_VERSION"))
}

pub fn terminal_fits_outer(width: usize, height: usize) -> bool {
    width >= OUTER_WIDTH && height >= OUTER_HEIGHT
}

pub fn wrap_inner_buffer(inner: &PlayfieldBuffer) -> PlayfieldBuffer {
    let mut outer = PlayfieldBuffer::new(OUTER_WIDTH, OUTER_HEIGHT, classic::body_style());
    draw_outer_frame(&mut outer);

    for row in 0..inner.height() {
        for (col, cell) in inner.row(row).iter().enumerate() {
            outer.set_cell(
                INNER_ORIGIN_ROW + row,
                INNER_ORIGIN_COL + col,
                cell.ch,
                cell.style,
            );
        }
    }

    if let Some((col, row)) = inner.cursor() {
        outer.set_cursor(col + INNER_ORIGIN_COL as u16, row + INNER_ORIGIN_ROW as u16);
    }

    outer
}

fn draw_outer_frame(buffer: &mut PlayfieldBuffer) {
    let chrome = classic::logo_style();
    let width = buffer.width();
    let height = buffer.height();
    let right = width.saturating_sub(1);
    let bottom = height.saturating_sub(1);

    for x in 1..right {
        buffer.set_cell(0, x, '─', chrome);
        buffer.set_cell(bottom, x, '─', chrome);
    }
    for y in 1..bottom {
        buffer.set_cell(y, 0, '│', chrome);
        buffer.set_cell(y, right, '│', chrome);
    }
    buffer.set_cell(0, 0, '┌', chrome);
    buffer.set_cell(0, right, '┐', chrome);
    buffer.set_cell(bottom, 0, '└', chrome);
    buffer.set_cell(bottom, right, '┘', chrome);

    let title = format!(" {} ", outer_title());
    buffer.write_text_clipped(0, 2, &title, chrome);
}
