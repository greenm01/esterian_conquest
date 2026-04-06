//! Right panel: unread counts (placeholder), scrollable report feed.

use nc_ui::PlayfieldBuffer;

use crate::app::state::DashApp;
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let right_col = buf.width().saturating_sub(crate::layout::RIGHT_WIDTH);
    let col = right_col + 1;
    let panel_width = crate::layout::RIGHT_WIDTH.saturating_sub(2);
    let start_row = buf.height() / 2 + 2;
    let start_row = start_row.max(18).min(buf.height().saturating_sub(5));

    // Unread counts — placeholder until unread tracking is implemented.
    let report_count = app.report_block_rows.len();
    let msg_count = app.queued_mail.len();
    buf.write_text(
        start_row,
        col,
        &format!("REPORTS ({R}R,{M}M)", R = report_count, M = msg_count),
        theme::section_title_style(),
    );

    let max_rows = buf.height().saturating_sub(start_row + 1);

    let viewer = app.player_record_index_1_based as u8;
    let mut row = start_row + 1;
    let mut shown = 0;

    // Show report blocks addressed to this player.
    for block in &app.report_block_rows {
        if block.viewer_empire_id != 0 && block.viewer_empire_id != viewer {
            continue;
        }
        if block.recipient_deleted {
            continue;
        }

        if shown < app.reports_scroll {
            shown += 1;
            continue;
        }
        if shown >= max_rows.max(1) + app.reports_scroll {
            break;
        }

        // Show first line of the report truncated to panel width.
        let first_line = block
            .decoded_text
            .lines()
            .find(|l: &&str| !l.trim().is_empty())
            .unwrap_or("")
            .trim();
        let truncated: String = first_line.chars().take(panel_width.max(1)).collect();
        buf.write_text(row, col, &format!(" {}", truncated), theme::value_style());
        row += 1;
        shown += 1;
    }

    if row == start_row + 1 {
        buf.write_text(start_row + 1, col, " (no reports)", theme::dim_style());
    }
}
