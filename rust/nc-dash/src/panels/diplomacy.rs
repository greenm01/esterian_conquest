//! Right panel: empire list, color-coded diplomatic status.

use nc_data::DiplomaticRelation;
use nc_ui::PlayfieldBuffer;

use crate::app::state::DashApp;
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let right_col = buf.width().saturating_sub(crate::layout::RIGHT_WIDTH);
    let col = right_col + 1;
    let start_row = 9;

    buf.write_text(start_row, col, "DIPLOMACY", theme::section_title_style());

    let player_idx = app.player_record_index_1_based.saturating_sub(1);
    let Some(player) = app.game_data.player.records.get(player_idx) else {
        return;
    };

    let total = app.game_data.player.records.len();
    let max_rows = buf.height().saturating_sub(start_row + 1 + 5); // leave room for reports

    let mut row = start_row + 1;
    let mut shown = 0;

    for (idx, other) in app.game_data.player.records.iter().enumerate() {
        if idx == player_idx {
            continue; // skip self
        }
        let empire_slot = (idx + 1) as u8;
        if other.player_mode_raw() == 0x00 {
            continue; // unjoined slot
        }

        if shown < app.diplomacy_scroll {
            shown += 1;
            continue;
        }
        if shown >= max_rows.max(1) + app.diplomacy_scroll {
            break;
        }

        let rel = player.diplomatic_relation_toward(empire_slot);
        let name_bytes = other.empire_name_bytes();
        let name: String = String::from_utf8_lossy(name_bytes)
            .trim_end_matches('\0')
            .chars()
            .take(10)
            .collect();

        let (status_str, style) = match rel {
            Some(DiplomaticRelation::Enemy) => ("Enemy  ", theme::enemy_style()),
            _ => ("Neutral", theme::dim_style()),
        };

        buf.write_text(row, col, &format!(" {:<10} {}", name, status_str), style);
        row += 1;
        shown += 1;
    }

    if row == start_row + 1 {
        buf.write_text(start_row + 1, col, " (none)", theme::dim_style());
    }
}
