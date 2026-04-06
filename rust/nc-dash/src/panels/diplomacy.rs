//! Right panel: empire list, color-coded diplomatic status.

use crate::app::state::DashApp;
use crate::diplomacy_view::{display_name, panel_status_label_and_style};
use crate::layout::{self, PanelWidgetFrame};
use crate::theme;
use nc_ui::PlayfieldBuffer;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: PanelWidgetFrame) {
    layout::write_panel_title(buf, frame, "DIPLOMACY", theme::section_title_style());

    let player_idx = app.player_record_index_1_based.saturating_sub(1);
    let Some(player) = app.game_data.player.records.get(player_idx) else {
        return;
    };
    let viewer_slot = app.player_record_index_1_based as u8;
    let max_rows = frame.body.height;

    let mut row_offset = 0usize;
    let mut shown = 0;
    for (idx, other) in app.game_data.player.records.iter().enumerate() {
        if idx == player_idx {
            continue;
        }
        if shown < app.diplomacy_scroll {
            shown += 1;
            continue;
        }
        if row_offset >= max_rows {
            break;
        }

        let empire_slot = (idx + 1) as u8;
        let name: String = display_name(other, empire_slot).chars().take(8).collect();
        let (state_text, state_style) =
            panel_status_label_and_style(other, Some(player), viewer_slot, empire_slot);
        let row = frame.body.row + row_offset;
        layout::write_clipped(buf, row, frame.body.col, 1, " ", theme::value_style());
        layout::write_clipped(
            buf,
            row,
            frame.body.col + 1,
            8,
            &format!("{name:<8}"),
            theme::empire_slot_style(empire_slot),
        );
        layout::write_clipped(
            buf,
            row,
            frame.body.col + 10,
            frame.body.width.saturating_sub(10),
            state_text,
            state_style,
        );
        row_offset += 1;
        shown += 1;
    }
    if row_offset == 0 {
        layout::write_panel_body_line(buf, frame, 0, " (none)", theme::dim_style());
    }
}
