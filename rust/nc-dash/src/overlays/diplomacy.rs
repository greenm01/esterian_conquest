use nc_ui::PlayfieldBuffer;
use crate::app::state::DashApp;
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, _app: &DashApp) {
    buf.fill_row(0, theme::header_style());
    buf.fill_row(buf.height().saturating_sub(1), theme::footer_style());
    buf.write_text(buf.height().saturating_sub(1), 2, "Esc:Back", theme::footer_style());
    buf.write_text(3, 2, "(coming in Phase 3)", theme::dim_style());
}
