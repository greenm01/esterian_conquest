//! Panel dimensions from terminal size, map-aware sizing.

use nc_ui::ScreenGeometry;

/// Side panel width in characters (inside the border).
pub const SIDE_PANEL_WIDTH: usize = 20;

/// Interior left map padding before the rendered map block.
pub const MAP_LEFT_PADDING: usize = 0;
/// Interior right map padding after the rendered map block.
pub const MAP_RIGHT_PADDING: usize = 0;
/// Interior vertical map padding above and below the rendered map block.
pub const MAP_VERTICAL_PADDING: usize = 0;

/// Minimum terminal dimensions for the dashboard.
pub const MIN_COLS: u16 = 160;
pub const MIN_ROWS: u16 = 43;

/// Row label width: "18 " = 3 chars.
pub const ROW_LABEL_COLS: usize = 3;

/// Characters per grid sector.
pub const CELL_WIDTH: usize = 3;

/// Compute the dashboard buffer geometry sized to the actual map.
///
/// Layout height: 1 top border + 1 header bar + 1 header-divider
///   + 1 col-axis + map_size grid
///   + 1 footer-divider + 1 footer bar + 1 bottom border = map_size + 7.
///
/// Layout width: 1 left-border + SIDE_PANEL_WIDTH + 1 left-divider
///   + ROW_LABEL_COLS + (map_size * CELL_WIDTH) + 1 right-divider
///   + SIDE_PANEL_WIDTH + 1 right-border.
pub fn dashboard_geometry(map_size: usize) -> ScreenGeometry {
    let grid_width = MAP_LEFT_PADDING + MAP_RIGHT_PADDING + ROW_LABEL_COLS + map_size * CELL_WIDTH;
    let width = 1 + SIDE_PANEL_WIDTH + 1 + grid_width + 1 + SIDE_PANEL_WIDTH + 1;
    let height = map_size + 7;
    ScreenGeometry::new(width, height)
}
