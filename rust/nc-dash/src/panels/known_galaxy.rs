//! Right panel: world counts by category.

use nc_data::DiplomaticRelation;
use nc_ui::PlayfieldBuffer;
use crate::app::state::DashApp;
use crate::layout;
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let (ox, oy) = layout::frame_offset(app);
    let col = layout::right_content_col(app, ox);
    let start_row = layout::right_galaxy_title_row(oy);
    let width = layout::right_panel_content_width();

    layout::write_width_clipped(buf, start_row, col, width, "KNOWN GALAXY", theme::section_title_style());

    let owner_slot = app.player_record_index_1_based as u8;
    let player = app.game_data.player.records.get(app.player_record_index_1_based.saturating_sub(1));
    let (mut my, mut neutral, mut enemy, mut icd) = (0u32, 0u32, 0u32, 0u32);

    for planet in &app.game_data.planets.records {
        let p_owner = planet.owner_empire_slot_raw();
        if p_owner == 0 { continue; }
        if p_owner == owner_slot { my += 1; continue; }
        let is_icd = app.game_data.player.records.get(p_owner.saturating_sub(1) as usize)
            .map(|p| p.is_civil_disorder_player()).unwrap_or(false);
        if is_icd { icd += 1; }
        else if player.and_then(|p| p.diplomatic_relation_toward(p_owner)) == Some(DiplomaticRelation::Enemy) { enemy += 1; }
        else { neutral += 1; }
    }

    let map_dim = nc_data::map_size_for_player_count(app.game_data.conquest.player_count()) as u32;
    let uncharted = (map_dim * map_dim).saturating_sub(my + neutral + enemy + icd);

    layout::write_width_clipped(buf, start_row + 1, col, width, &format!(" My      ■{:4}", my), theme::friendly_style());
    layout::write_width_clipped(buf, start_row + 2, col, width, &format!(" Neutral ○{:4}", neutral), theme::dim_style());
    layout::write_width_clipped(buf, start_row + 3, col, width, &format!(" Enemy   ●{:4}", enemy), theme::enemy_style());
    layout::write_width_clipped(buf, start_row + 4, col, width, &format!(" ICD     ◊{:4}", icd), theme::icd_style());
    layout::write_width_clipped(buf, start_row + 5, col, width, &format!(" Unch    ·{:4}", uncharted), theme::dim_style());
}
