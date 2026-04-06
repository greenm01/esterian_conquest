//! Panel dimensions from terminal size, map-aware sizing.

use nc_ui::ScreenGeometry;

/// Interior left map padding before the rendered map block.
pub const MAP_LEFT_PADDING: usize = 0;
/// Interior right map padding after the rendered map block.
pub const MAP_RIGHT_PADDING: usize = 0;
/// Interior vertical map padding above and below the rendered map block.
pub const MAP_VERTICAL_PADDING: usize = 0;

/// Row label width: "18 " = 3 chars.
pub const ROW_LABEL_COLS: usize = 3;

/// Characters per grid sector.
pub const CELL_WIDTH: usize = 3;

pub const fn minimum_projected_map_width(map_size: usize) -> usize {
    ROW_LABEL_COLS + map_size.saturating_mul(2)
}

pub const fn minimum_projected_map_height(map_size: usize) -> usize {
    1 + map_size
}

pub const fn preferred_readable_map_width(map_size: usize) -> usize {
    ROW_LABEL_COLS + map_size.saturating_mul(4)
}

pub const fn dashboard_frame_geometry(
    center_width: usize,
    left_width: usize,
    right_width: usize,
    content_height: usize,
) -> ScreenGeometry {
    let width = 1 + left_width + 1 + center_width + 1 + right_width + 1;
    let height = content_height + 6;
    ScreenGeometry::new(width, height)
}
