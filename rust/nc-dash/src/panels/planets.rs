//! Left panel: owned planet list.

use nc_ui::PlayfieldBuffer;
use crate::app::state::DashApp;
use crate::layout::{self, PanelWidgetFrame};
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: PanelWidgetFrame) {
    layout::write_panel_title(buf, frame, "MY PLANETS", theme::section_title_style());

    let owner_slot = app.player_record_index_1_based as u8;
    let max_rows = frame.body.height.min(10);

    let starbase_coords: std::collections::HashSet<[u8; 2]> = app.game_data.bases.records.iter()
        .filter(|b| b.owner_empire_raw() == owner_slot && b.active_flag_raw() != 0)
        .map(|b| b.coords_raw()).collect();

    let mut row_offset = 0usize;
    let mut shown = 0;
    for planet in &app.game_data.planets.records {
        if planet.owner_empire_slot_raw() != owner_slot { continue; }
        if shown < app.planets_scroll { shown += 1; continue; }
        if row_offset >= max_rows { break; }

        let name = planet.planet_name();
        let abbrev: String = name.chars().take(3).collect();
        let sb = if starbase_coords.contains(&planet.coords_raw()) { '★' } else { ' ' };
        let c = planet.coords_raw();
        let present = planet.present_production_points().unwrap_or(0);
        let style = if present == 0 { theme::enemy_style() }
            else if present < planet.potential_production_points() / 2 { theme::alert_style() }
            else { theme::value_style() };
        layout::write_panel_body_line(
            buf,
            frame,
            row_offset,
            &format!("{}{} ({:02},{:02}) {:3}", abbrev, sb, c[0], c[1], present),
            style,
        );
        row_offset += 1;
        shown += 1;
    }
    if row_offset == 0 {
        layout::write_panel_body_line(buf, frame, 0, "(none)", theme::dim_style());
    }
}
