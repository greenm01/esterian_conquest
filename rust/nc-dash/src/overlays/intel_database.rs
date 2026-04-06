//! I overlay: fullscreen planet database with filters.

use nc_ui::PlayfieldBuffer;
use crate::app::state::DashApp;
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, _app: &DashApp) {
    buf.fill_row(0, theme::header_style());
    buf.write_text(0, 2, "PLANET DATABASE", theme::title_style());
    buf.fill_row(buf.height().saturating_sub(1), theme::footer_style());
    buf.write_text(buf.height().saturating_sub(1), 2,
        "COMMANDS <- ? J K ^U ^D S <Q> ->", theme::footer_style());
    buf.write_text(3, 2, "(intel database — Phase 3)", theme::dim_style());
}
