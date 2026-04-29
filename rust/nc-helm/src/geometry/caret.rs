#![allow(dead_code)]

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
    let band = mapper.text_band_rect(point, text);
    let beam_width = if mapper.cell.width_px <= 2 { 1 } else { 2 };
    let height = band.height.max(1);
    CaretRect {
        x: band.x,
        y: band.y,
        width: beam_width,
        height,
    }
}
