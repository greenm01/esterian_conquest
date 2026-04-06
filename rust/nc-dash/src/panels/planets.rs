//! Left panel: owned planet list (3-char abbrev, ★ starbase indicator, coords, production).

use nc_ui::PlayfieldBuffer;
use crate::app::state::DashApp;
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let start_row = 8;
    buf.write_text(start_row, 2, "MY PLANETS", theme::section_title_style());
    // Stub: populated in Phase 3
    buf.write_text(start_row + 1, 2, " (none)", theme::dim_style());
    let _ = app;
}
