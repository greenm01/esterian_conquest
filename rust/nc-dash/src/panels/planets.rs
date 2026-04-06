//! Left panel: owned planet list.

use nc_ui::PlayfieldBuffer;
use crate::app::state::DashApp;
use crate::layout;
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let (ox, oy) = layout::frame_offset(app);
    let col = ox + 2;
    let start_row = layout::left_planets_title_row(oy);
    let width = layout::left_panel_content_width();

    layout::write_width_clipped(buf, start_row, col, width, "MY PLANETS", theme::section_title_style());

    let owner_slot = app.player_record_index_1_based as u8;
    let footer_row = layout::section_footer_row(app, oy);
    let max_rows = footer_row.saturating_sub(start_row + 1).min(10);

    let starbase_coords: std::collections::HashSet<[u8; 2]> = app.game_data.bases.records.iter()
        .filter(|b| b.owner_empire_raw() == owner_slot && b.active_flag_raw() != 0)
        .map(|b| b.coords_raw()).collect();

    let mut row = start_row + 1;
    let mut shown = 0;
    for planet in &app.game_data.planets.records {
        if planet.owner_empire_slot_raw() != owner_slot { continue; }
        if shown < app.planets_scroll { shown += 1; continue; }
        if row >= start_row + 1 + max_rows { break; }

        let name = planet.planet_name();
        let abbrev: String = name.chars().take(3).collect();
        let sb = if starbase_coords.contains(&planet.coords_raw()) { '★' } else { ' ' };
        let c = planet.coords_raw();
        let present = planet.present_production_points().unwrap_or(0);
        let style = if present == 0 { theme::enemy_style() }
            else if present < planet.potential_production_points() / 2 { theme::alert_style() }
            else { theme::value_style() };
        layout::write_width_clipped(
            buf,
            row,
            col,
            width,
            &format!("{}{} ({:02},{:02}) {:3}", abbrev, sb, c[0], c[1], present),
            style,
        );
        row += 1;
        shown += 1;
    }
    if row == start_row + 1 {
        layout::write_width_clipped(buf, start_row + 1, col, width, "(none)", theme::dim_style());
    }
}
