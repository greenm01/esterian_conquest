//! Right panel: unread reports and messages.

use crate::app::state::DashApp;
use crate::inbox::{project_inbox_items, DashInboxItemType, ReportSummaryBucket};
use crate::layout::{self, PanelWidgetFrame};
use crate::theme;
use nc_ui::{CellStyle, PlayfieldBuffer};

pub(crate) const TITLE: &str = "INBOX";

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: PanelWidgetFrame) {
    layout::write_panel_title(buf, frame, TITLE, theme::section_title_style());

    for (row_idx, (text, style)) in body_rows(app).into_iter().enumerate() {
        if row_idx >= frame.body.height {
            break;
        }
        layout::write_panel_body_line(buf, frame, row_idx, &text, style);
    }
}

pub(crate) fn body_rows(app: &DashApp) -> Vec<(String, CellStyle)> {
    let viewer_empire_id = app.player_record_index_1_based as u8;
    let items = project_inbox_items(
        &app.game_data,
        viewer_empire_id,
        &app.report_block_rows,
        &app.queued_mail,
    );

    let current_year = app.game_data.conquest.game_year();

    let mut unread_combat = 0;
    let mut unread_intel = 0;
    let mut unread_msgs = 0;

    for item in items {
        if item.item_type == DashInboxItemType::Message {
            unread_msgs += 1;
        } else if item.year == current_year {
            match item.report_bucket {
                Some(ReportSummaryBucket::Combat) => unread_combat += 1,
                Some(ReportSummaryBucket::Intel) => unread_intel += 1,
                Some(ReportSummaryBucket::Ops) => {} // Ops handled by normal screens usually
                None => {}
            }
        }
    }

    vec![
        (
            layout::format_left_column_value("Combat Rep", &unread_combat.to_string()),
            if unread_combat > 0 { theme::enemy_style() } else { theme::dim_style() },
        ),
        (
            layout::format_left_column_value("Intel Rep", &unread_intel.to_string()),
            if unread_intel > 0 { theme::friendly_style() } else { theme::dim_style() },
        ),
        (
            layout::format_left_column_value("Messages", &unread_msgs.to_string()),
            if unread_msgs > 0 { theme::alert_style() } else { theme::dim_style() },
        ),
    ]
}
