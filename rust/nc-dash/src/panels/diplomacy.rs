//! Right panel: empire list, color-coded diplomatic status.

use nc_data::DiplomaticRelation;
use nc_ui::PlayfieldBuffer;
use crate::app::state::DashApp;
use crate::layout::{self, PanelWidgetFrame};
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: PanelWidgetFrame) {
    layout::write_panel_title(buf, frame, "DIPLOMACY", theme::section_title_style());

    let player_idx = app.player_record_index_1_based.saturating_sub(1);
    let Some(player) = app.game_data.player.records.get(player_idx) else { return };
    let max_rows = frame.body.height.min(12);

    let mut row_offset = 0usize;
    let mut shown = 0;
    for (idx, other) in app.game_data.player.records.iter().enumerate() {
        if idx == player_idx { continue; }
        if other.player_mode_raw() == 0x00 { continue; }
        if shown < app.diplomacy_scroll { shown += 1; continue; }
        if row_offset >= max_rows { break; }

        let empire_slot = (idx + 1) as u8;
        let rel = player.diplomatic_relation_toward(empire_slot);
        let name: String = String::from_utf8_lossy(other.empire_name_bytes())
            .trim_end_matches('\0').chars().take(8).collect();
        let (status_str, style) = match rel {
            Some(DiplomaticRelation::Enemy) => ("Enemy", theme::enemy_style()),
            _ => ("Neut ", theme::dim_style()),
        };
        layout::write_panel_body_line(
            buf,
            frame,
            row_offset,
            &format!(" {:<8} {}", name, status_str),
            style,
        );
        row_offset += 1; shown += 1;
    }
    if row_offset == 0 {
        layout::write_panel_body_line(buf, frame, 0, " (none)", theme::dim_style());
    }
}
