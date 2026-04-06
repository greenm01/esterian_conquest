//! Left panel: active fleet + starbase list with 2-letter order codes.

use nc_ui::PlayfieldBuffer;
use crate::app::state::DashApp;
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, _app: &DashApp) {
    let start_row = 14;
    buf.write_text(start_row, 2, "ACTIVE FLEETS", theme::section_title_style());
    buf.write_text(start_row + 1, 2, " (none)", theme::dim_style());
}
