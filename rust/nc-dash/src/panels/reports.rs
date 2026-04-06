//! Right panel: report feed summary.

use nc_ui::PlayfieldBuffer;
use crate::app::state::DashApp;
use crate::layout;
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let (ox, oy) = layout::frame_offset(app);
    let col = layout::right_content_col(app, ox);
    let right_div = layout::right_divider_col(app, ox);
    let panel_width = buf.width().saturating_sub(right_div + 3);
    let footer_row = layout::section_footer_row(app, oy);

    // Place reports directly after diplomacy.
    let start_row = layout::right_reports_title_row(app, oy);

    let report_count = app.report_block_rows.len();
    let msg_count = app.queued_mail.len();
    layout::write_width_clipped(buf, start_row, col, panel_width,
        &format!("REPORTS ({R}R,{M}M)", R = report_count, M = msg_count),
        theme::section_title_style());

    let max_rows = footer_row.saturating_sub(start_row + 1);
    let viewer = app.player_record_index_1_based as u8;
    let mut row = start_row + 1;
    let mut shown = 0;

    for block in &app.report_block_rows {
        if !block.is_visible_to_viewer(viewer) || block.recipient_deleted { continue; }
        if shown < app.reports_scroll { shown += 1; continue; }
        if row >= start_row + 1 + max_rows { break; }

        let first_line = block.decoded_text.lines()
            .find(|l: &&str| !l.trim().is_empty())
            .unwrap_or("").trim();
        layout::write_width_clipped(buf, row, col, panel_width, &format!(" {}", first_line), theme::value_style());
        row += 1; shown += 1;
    }
    if row == start_row + 1 {
        layout::write_width_clipped(buf, start_row + 1, col, panel_width, " (no reports)", theme::dim_style());
    }
}
