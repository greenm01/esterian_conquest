//! Right panel: unread counts, scrollable report feed.

use nc_ui::PlayfieldBuffer;
use crate::app::state::DashApp;
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, _app: &DashApp) {
    let col = buf.width().saturating_sub(crate::layout::geometry::SIDE_PANEL_WIDTH);
    let start_row = 14;
    buf.write_text(start_row, col + 1, "REPORTS (0R,0M)", theme::section_title_style());
    buf.write_text(start_row + 1, col + 1, " (none)", theme::dim_style());
}
