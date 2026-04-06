//! Panel dimensions from terminal size, map-aware sizing.

use nc_ui::ScreenGeometry;

/// Side panel width in characters (inside the border).
pub const SIDE_PANEL_WIDTH: usize = 20;

/// Minimum terminal dimensions for the dashboard.
pub const MIN_COLS: u16 = 160;
pub const MIN_ROWS: u16 = 40;

/// Row label width: "18 " = 3 chars.
pub const ROW_LABEL_COLS: usize = 3;

/// Characters per grid sector.
pub const CELL_WIDTH: usize = 3;

/// Compute the dashboard buffer geometry sized to the actual map.
///
/// Layout height: 1 header + 1 header-divider + 1 col-axis + map_size grid
///   + 1 status + 1 footer-divider + 1 footer = map_size + 6.
///
/// Layout width: 1 left-border + SIDE_PANEL_WIDTH + 1 left-divider
///   + ROW_LABEL_COLS + (map_size * CELL_WIDTH) + 1 right-divider
///   + SIDE_PANEL_WIDTH + 1 right-border.
pub fn dashboard_geometry(map_size: usize) -> ScreenGeometry {
    let grid_width = ROW_LABEL_COLS + map_size * CELL_WIDTH;
    let width = 1 + SIDE_PANEL_WIDTH + 1 + grid_width + 1 + SIDE_PANEL_WIDTH + 1;
    let height = map_size + 6;
    ScreenGeometry::new(width, height)
}
