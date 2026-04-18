use crate::grid::Point;

use super::{GridMapper, TextMetrics};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaretRect {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

pub fn caret_rect(point: Point, mapper: GridMapper, text: TextMetrics) -> CaretRect {
    let origin = mapper.text_origin(point, text);
    let beam_width = if mapper.cell.width_px <= 2 { 1 } else { 2 };
    let height = text
        .line_height_px
        .round()
        .max(1.0)
        .min(mapper.cell.height_px as f32) as usize;
    CaretRect {
        x: origin.left.round() as usize,
        y: origin.top.round() as usize,
        width: beam_width,
        height,
    }
}

