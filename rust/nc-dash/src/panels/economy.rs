//! Left panel: Treasury, Production/Potential, Revenue, Growth.

use nc_ui::PlayfieldBuffer;
use crate::app::state::DashApp;
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let start_row = 2;
    buf.write_text(start_row, 2, "ECONOMY", theme::section_title_style());
    buf.write_text(start_row + 1, 2, " Treasury: ---", theme::label_style());
    buf.write_text(start_row + 2, 2, " Prod: ---/---", theme::label_style());
    buf.write_text(start_row + 3, 2, " Revenue: ---", theme::label_style());
    buf.write_text(start_row + 4, 2, " Growth: ---", theme::label_style());
}
