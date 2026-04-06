//! Right panel: empire list, color-coded diplomatic status.

use nc_data::DiplomaticRelation;
use nc_ui::PlayfieldBuffer;
use crate::app::state::DashApp;
use crate::layout;
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let (ox, oy) = layout::frame_offset(app);
    let col = layout::right_content_col(app, ox);
    let start_row = layout::right_diplomacy_title_row(oy);
    let width = layout::right_panel_content_width();

    layout::write_width_clipped(buf, start_row, col, width, "DIPLOMACY", theme::section_title_style());

    let player_idx = app.player_record_index_1_based.saturating_sub(1);
    let Some(player) = app.game_data.player.records.get(player_idx) else { return };
    let footer_row = layout::section_footer_row(app, oy);
    let max_rows = footer_row.saturating_sub(start_row + 1).min(12);

    let mut row = start_row + 1;
    let mut shown = 0;
    for (idx, other) in app.game_data.player.records.iter().enumerate() {
        if idx == player_idx { continue; }
        if other.player_mode_raw() == 0x00 { continue; }
        if shown < app.diplomacy_scroll { shown += 1; continue; }
        if row >= start_row + 1 + max_rows { break; }

        let empire_slot = (idx + 1) as u8;
        let rel = player.diplomatic_relation_toward(empire_slot);
        let name: String = String::from_utf8_lossy(other.empire_name_bytes())
            .trim_end_matches('\0').chars().take(8).collect();
        let (status_str, style) = match rel {
            Some(DiplomaticRelation::Enemy) => ("Enemy", theme::enemy_style()),
            _ => ("Neut ", theme::dim_style()),
        };
        layout::write_width_clipped(
            buf,
            row,
            col,
            width,
            &format!(" {:<8} {}", name, status_str),
            style,
        );
        row += 1; shown += 1;
    }
    if row == start_row + 1 {
        layout::write_width_clipped(buf, start_row + 1, col, width, " (none)", theme::dim_style());
    }
}
