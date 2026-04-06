//! Panel dimensions from terminal size, max width cap.

use nc_ui::ScreenGeometry;

/// Side panel width in characters.
pub const SIDE_PANEL_WIDTH: usize = 20;

/// Maximum rendering width (20 left + 111 grid + 20 right + borders).
pub const MAX_RENDER_WIDTH: usize = 155;

/// Minimum terminal dimensions for the dashboard.
pub const MIN_COLS: u16 = 160;
pub const MIN_ROWS: u16 = 40;

/// Create a fullscreen geometry from the current terminal size.
pub fn fullscreen_geometry() -> Result<ScreenGeometry, Box<dyn std::error::Error>> {
    let (cols, rows) = crossterm::terminal::size()?;
    Ok(ScreenGeometry::new(cols as usize, rows as usize))
}

/// Cap the rendering area to MAX_RENDER_WIDTH centered in the terminal.
pub fn capped_geometry(term_cols: usize, term_rows: usize) -> ScreenGeometry {
    let width = term_cols.min(MAX_RENDER_WIDTH);
    ScreenGeometry::new(width, term_rows)
}
