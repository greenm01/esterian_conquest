use winit::dpi::PhysicalPosition;

use crate::grid::{Column, Point, Row, ScreenGeometry};

use super::{CellMetrics, TextMetrics};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalRect {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextOrigin {
    pub left: f32,
    pub top: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridMapper {
    pub origin_x: usize,
    pub origin_y: usize,
    pub geometry: ScreenGeometry,
    pub cell: CellMetrics,
}

impl GridMapper {
    pub fn centered(
        frame_width: usize,
        frame_height: usize,
        geometry: ScreenGeometry,
        cell: CellMetrics,
    ) -> Self {
        let grid_width = geometry.width().saturating_mul(cell.width_px);
        let grid_height = geometry.height().saturating_mul(cell.height_px);
        Self {
            origin_x: frame_width.saturating_sub(grid_width) / 2,
            origin_y: frame_height.saturating_sub(grid_height) / 2,
            geometry,
            cell,
        }
    }

    pub fn cell_rect(self, point: Point) -> PhysicalRect {
        PhysicalRect {
            x: self.origin_x + point.column.as_usize() * self.cell.width_px,
            y: self.origin_y + point.row.as_usize() * self.cell.height_px,
            width: self.cell.width_px,
            height: self.cell.height_px,
        }
    }

    pub fn text_origin(self, point: Point, _text: TextMetrics) -> TextOrigin {
        let rect = self.cell_rect(point);
        TextOrigin {
            left: rect.x as f32,
            top: rect.y as f32,
        }
    }

    pub fn pixel_to_cell(self, position: PhysicalPosition<f64>) -> Option<Point> {
        if !position.x.is_finite()
            || !position.y.is_finite()
            || position.x < 0.0
            || position.y < 0.0
        {
            return None;
        }

        let x = position.x.floor() as usize;
        let y = position.y.floor() as usize;
        if x < self.origin_x || y < self.origin_y {
            return None;
        }

        let local_x = x - self.origin_x;
        let local_y = y - self.origin_y;
        let grid_width = self.geometry.width().saturating_mul(self.cell.width_px);
        let grid_height = self.geometry.height().saturating_mul(self.cell.height_px);
        if local_x >= grid_width || local_y >= grid_height {
            return None;
        }

        Some(Point::new(
            Column(local_x / self.cell.width_px),
            Row(local_y / self.cell.height_px),
        ))
    }
}

