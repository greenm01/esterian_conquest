//! Screen geometry — terminal dimensions abstracted from fixed 80×25.
//!
//! `ScreenGeometry` is the single source of truth for screen dimensions
//! used by both nc-game (80×25) and nc-helm (fullscreen). Helper functions
//! that compute row positions from geometry live here so both frontends
//! can use them.

/// Standard 80×25 playfield width (legacy nc-game fixed layout).
pub const CLASSIC_WIDTH: usize = 80;
/// Standard 80×25 playfield height (legacy nc-game fixed layout).
pub const CLASSIC_HEIGHT: usize = 25;
/// BBS door fallback height (24 rows when dropfile reports no height).
pub const DOOR_FALLBACK_HEIGHT: usize = 24;

/// Screen dimensions used by the terminal renderer.
///
/// nc-game always uses 80×25 (or 80×24 for BBS doors).
/// nc-helm uses the actual terminal size detected at startup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenGeometry {
    width: usize,
    height: usize,
}

impl ScreenGeometry {
    /// Create a geometry from explicit dimensions.
    pub const fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    /// Classic 80×25 geometry for local interactive play.
    pub const fn local_default() -> Self {
        Self::new(CLASSIC_WIDTH, CLASSIC_HEIGHT)
    }

    /// BBS door geometry: 80 columns, 24–25 rows depending on the dropfile.
    pub fn for_door(rows: Option<usize>) -> Self {
        let height = rows
            .unwrap_or(DOOR_FALLBACK_HEIGHT)
            .clamp(DOOR_FALLBACK_HEIGHT, CLASSIC_HEIGHT);
        Self::new(CLASSIC_WIDTH, height)
    }

    pub const fn width(self) -> usize {
        self.width
    }

    pub const fn height(self) -> usize {
        self.height
    }
}

/// Row containing the command-line prompt (last row of the screen).
pub const fn command_line_row_for(geometry: ScreenGeometry) -> usize {
    geometry.height - 1
}

/// Last row available for body content (one above the command line).
pub const fn last_body_row_for(geometry: ScreenGeometry) -> usize {
    command_line_row_for(geometry) - 1
}

/// Center a block vertically within a row range.
pub const fn centered_row(first_row: usize, last_row: usize, block_height: usize) -> usize {
    let available = last_row.saturating_sub(first_row);
    if block_height >= available {
        first_row
    } else {
        first_row + (available - block_height) / 2
    }
}

/// Number of visible rows for a standard table within the given geometry.
pub const fn standard_table_visible_rows_for(geometry: ScreenGeometry, start_row: usize) -> usize {
    let command_row = command_line_row_for(geometry);
    if command_row > start_row + 2 {
        command_row - start_row - 2
    } else {
        0
    }
}
