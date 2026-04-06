//! Right panel: empire list, color-coded diplomatic status.

use crate::app::state::DashApp;
use crate::diplomacy_view::{display_name, panel_status_label_and_style};
use crate::layout::{self, PanelWidgetFrame};
use crate::theme;
use nc_ui::{CellStyle, PlayfieldBuffer};

pub(crate) const TITLE: &str = "DIPLOMACY";

#[derive(Debug, Clone)]
pub(crate) struct DiplomacyPanelRow {
    pub empire_slot: u8,
    pub name: String,
    pub status: &'static str,
    pub status_style: CellStyle,
}

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: PanelWidgetFrame) {
    layout::write_panel_title(buf, frame, TITLE, theme::section_title_style());

    let max_rows = frame.body.height;
    let rows = body_rows(app);
    let name_width = diplomacy_name_width(app);

    let mut row_offset = 0usize;
    for row_data in rows.iter().skip(app.diplomacy_scroll) {
        if row_offset >= max_rows {
            break;
        }
        let row = frame.body.row + row_offset;
        layout::write_clipped(buf, row, frame.body.col, 1, " ", theme::value_style());
        layout::write_clipped(
            buf,
            row,
            frame.body.col + 1,
            name_width,
            &format!("{:<name_width$}", row_data.name),
            theme::empire_slot_style(row_data.empire_slot),
        );
        layout::write_clipped(
            buf,
            row,
            frame.body.col + 1 + name_width + 1,
            frame.body.width.saturating_sub(1 + name_width + 1),
            row_data.status,
            row_data.status_style,
        );
        row_offset += 1;
    }
    if row_offset == 0 {
        layout::write_panel_body_line(buf, frame, 0, " (none)", theme::dim_style());
    }
}

pub(crate) fn body_rows(app: &DashApp) -> Vec<DiplomacyPanelRow> {
    let player_idx = app.player_record_index_1_based.saturating_sub(1);
    let Some(player) = app.game_data.player.records.get(player_idx) else {
        return Vec::new();
    };
    let viewer_slot = app.player_record_index_1_based as u8;

    let mut rows = Vec::new();
    for (idx, other) in app.game_data.player.records.iter().enumerate() {
        if idx == player_idx {
            continue;
        }
        let empire_slot = (idx + 1) as u8;
        let (state_text, state_style) =
            panel_status_label_and_style(other, Some(player), viewer_slot, empire_slot);
        rows.push(DiplomacyPanelRow {
            empire_slot,
            name: display_name(other, empire_slot),
            status: state_text,
            status_style: state_style,
        });
    }
    rows
}

pub(crate) fn diplomacy_name_width(app: &DashApp) -> usize {
    body_rows(app)
        .iter()
        .map(|row| row.name.chars().count())
        .max()
        .unwrap_or(0)
}
