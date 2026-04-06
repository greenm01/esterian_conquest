//! Right panel: world counts by category (My, Neutral, Enemy, ICD, Uncharted).

use nc_ui::PlayfieldBuffer;
use crate::app::state::DashApp;
use crate::layout::{LEFT_WIDTH, center_width};
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, _app: &DashApp) {
    let col = buf.width().saturating_sub(crate::layout::geometry::SIDE_PANEL_WIDTH);
    let start_row = 2;
    buf.write_text(start_row, col + 1, "KNOWN GALAXY", theme::section_title_style());
    buf.write_text(start_row + 1, col + 1, " My      ■  0", theme::friendly_style());
    buf.write_text(start_row + 2, col + 1, " Neutral ○  0", theme::dim_style());
    buf.write_text(start_row + 3, col + 1, " Enemy   ●  0", theme::enemy_style());
    buf.write_text(start_row + 4, col + 1, " ICD     ◊  0", theme::icd_style());
    buf.write_text(start_row + 5, col + 1, " Unch    ·  0", theme::dim_style());
}
