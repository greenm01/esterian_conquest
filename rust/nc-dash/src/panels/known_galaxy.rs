//! Right panel: world counts by category (My, Neutral, Enemy, ICD, Uncharted).

use nc_data::DiplomaticRelation;
use nc_ui::PlayfieldBuffer;

use crate::app::state::DashApp;
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let right_col = buf.width().saturating_sub(crate::layout::RIGHT_WIDTH);
    let col = right_col + 1;
    let start_row = 2;

    buf.write_text(start_row, col, "KNOWN GALAXY", theme::section_title_style());

    let owner_slot = app.player_record_index_1_based as u8;
    let player_idx = app.player_record_index_1_based.saturating_sub(1);
    let player = app.game_data.player.records.get(player_idx);

    let mut my = 0u32;
    let mut neutral = 0u32;
    let mut enemy = 0u32;
    let mut icd = 0u32;

    for planet in &app.game_data.planets.records {
        let p_owner = planet.owner_empire_slot_raw();
        if p_owner == 0 {
            continue; // uncharted / unowned
        }
        if p_owner == owner_slot {
            my += 1;
        } else {
            // Check diplomatic relation.
            let rel = player.and_then(|p| p.diplomatic_relation_toward(p_owner));
            // Check if civil disorder.
            let owner_player = app.game_data.player.records.get(p_owner.saturating_sub(1) as usize);
            let is_icd = owner_player.map(|p| p.is_civil_disorder_player()).unwrap_or(false);
            if is_icd {
                icd += 1;
            } else if rel == Some(DiplomaticRelation::Enemy) {
                enemy += 1;
            } else {
                neutral += 1;
            }
        }
    }

    let total_known = my + neutral + enemy + icd;
    let map_dim = nc_data::map_size_for_player_count(app.game_data.conquest.player_count()) as u32;
    let map_size = map_dim * map_dim;
    let uncharted = map_size.saturating_sub(total_known);

    buf.write_text(start_row + 1, col, &format!(" My      ■{:4}", my), theme::friendly_style());
    buf.write_text(start_row + 2, col, &format!(" Neutral ○{:4}", neutral), theme::dim_style());
    buf.write_text(start_row + 3, col, &format!(" Enemy   ●{:4}", enemy), theme::enemy_style());
    buf.write_text(start_row + 4, col, &format!(" ICD     ◊{:4}", icd), theme::icd_style());
    buf.write_text(start_row + 5, col, &format!(" Unch    ·{:4}", uncharted), theme::dim_style());
}
