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

    pub fn text_band_rect(self, point: Point, text: TextMetrics) -> PhysicalRect {
        let rect = self.cell_rect(point);
        let top = rect.y + text.band_top_px.min(rect.height.saturating_sub(1));
        let height = text.band_height_px.clamp(1, rect.height);
        let clamped_top = top.min(rect.y + rect.height.saturating_sub(height));
        PhysicalRect {
            x: rect.x,
            y: clamped_top,
            width: rect.width,
            height,
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

#[cfg(test)]
mod tests {
    use super::GridMapper;
    use crate::geometry::{CellMetrics, TextMetrics};
    use crate::grid::{Point, ScreenGeometry};

    fn mapper() -> (GridMapper, TextMetrics) {
        let text = TextMetrics {
            font_size_px: 18.0,
            line_height_px: 24.0,
            baseline_px: 18,
            band_top_px: 5,
            band_height_px: 14,
        };
        (
            GridMapper::centered(
                1200,
                900,
                ScreenGeometry::new(100, 36),
                CellMetrics {
                    width_px: 12,
                    height_px: 24,
                },
            ),
            text,
        )
    }

    #[test]
    fn text_band_rect_stays_within_the_cell_row() {
        let (mapper, text) = mapper();
        let cell = mapper.cell_rect(Point::from_usize(31, 16));
        let band = mapper.text_band_rect(Point::from_usize(31, 16), text);
        assert_eq!(band.x, cell.x);
        assert_eq!(band.width, cell.width);
        assert_eq!(band.y, cell.y + text.band_top_px);
        assert_eq!(band.height, text.band_height_px);
    }

    #[test]
    fn pixel_to_cell_floors_at_cell_boundaries() {
        let (mapper, _) = mapper();
        let cell = mapper.cell_rect(Point::from_usize(7, 4));
        assert_eq!(
            mapper.pixel_to_cell(winit::dpi::PhysicalPosition::new(
                (cell.x + cell.width - 1) as f64,
                (cell.y + cell.height - 1) as f64,
            )),
            Some(Point::from_usize(7, 4))
        );
    }
}
