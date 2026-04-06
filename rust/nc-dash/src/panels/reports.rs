//! Right panel: report feed summary.

use nc_ui::PlayfieldBuffer;
use crate::app::state::DashApp;
use crate::layout::{self, PanelWidgetFrame};
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: PanelWidgetFrame) {
    let report_count = app.report_block_rows.len();
    let msg_count = app.queued_mail.len();
    layout::write_panel_title(
        buf,
        frame,
        &format!("REPORTS ({R}R,{M}M)", R = report_count, M = msg_count),
        theme::section_title_style(),
    );

    let max_rows = frame.body.height;
    let viewer = app.player_record_index_1_based as u8;
    let mut row_offset = 0usize;
    let mut shown = 0;

    for block in &app.report_block_rows {
        if !block.is_visible_to_viewer(viewer) || block.recipient_deleted { continue; }
        if shown < app.reports_scroll { shown += 1; continue; }
        if row_offset >= max_rows { break; }

        let first_line = block.decoded_text.lines()
            .find(|l: &&str| !l.trim().is_empty())
            .unwrap_or("").trim();
        layout::write_panel_body_line(buf, frame, row_offset, &format!(" {}", first_line), theme::value_style());
        row_offset += 1; shown += 1;
    }
    if row_offset == 0 {
        layout::write_panel_body_line(buf, frame, 0, " (no reports)", theme::dim_style());
    }
}
