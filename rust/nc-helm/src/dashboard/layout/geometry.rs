//! Panel dimensions from terminal size, map-aware sizing.

use crate::dashboard::geometry::ScreenGeometry;

/// Interior left map padding before the rendered map block.
pub const MAP_LEFT_PADDING: usize = 0;
/// Interior right map padding after the rendered map block.
///
/// Matches `ROW_LABEL_COLS` so the sector grid sits horizontally centred
/// inside the map block: the left side has row labels (e.g. "01 ") taking
/// 3 cells; the right side has an equivalent 3-cell blank gutter.
pub const MAP_RIGHT_PADDING: usize = 3;
/// Interior vertical map padding above and below the rendered map block.
pub const MAP_VERTICAL_PADDING: usize = 0;

/// Row label width: "18 " = 3 chars.
pub const ROW_LABEL_COLS: usize = 3;

/// Characters per grid sector pitch.
pub const CELL_WIDTH: usize = 4;

/// Rows per grid sector pitch (separator + content).
pub const CELL_HEIGHT: usize = 2;

const MINIMUM_REQUIRED_VIEWPORT_SECTORS: usize = 12;

pub const fn minimum_projected_map_height(map_size: usize) -> usize {
    2 + (map_size * CELL_HEIGHT + 1)
}

pub const fn minimum_required_map_width(map_size: usize) -> usize {
    ROW_LABEL_COLS
        + 1
        + if map_size < MINIMUM_REQUIRED_VIEWPORT_SECTORS {
            map_size * CELL_WIDTH
        } else {
            MINIMUM_REQUIRED_VIEWPORT_SECTORS * CELL_WIDTH
        }
        + MAP_RIGHT_PADDING
}

pub const fn minimum_required_map_height(map_size: usize) -> usize {
    2 + 1
        + if map_size < MINIMUM_REQUIRED_VIEWPORT_SECTORS {
            map_size * CELL_HEIGHT
        } else {
            MINIMUM_REQUIRED_VIEWPORT_SECTORS * CELL_HEIGHT
        }
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
