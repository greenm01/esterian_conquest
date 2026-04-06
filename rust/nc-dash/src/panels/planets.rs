//! Left panel: owned planet list (3-char abbrev, ★ starbase indicator, coords, production).

use nc_ui::PlayfieldBuffer;

use crate::app::state::DashApp;
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let start_row = 8;
    let col = 2;

    buf.write_text(start_row, col, "MY PLANETS", theme::section_title_style());

    let owner_slot = app.player_record_index_1_based as u8;
    let max_rows = buf.height().saturating_sub(start_row + 1 + 8); // leave room for fleets

    // Build starbase coordinate set for ★ indicator.
    let starbase_coords: std::collections::HashSet<[u8; 2]> = app
        .game_data
        .bases
        .records
        .iter()
        .filter(|b| b.owner_empire_raw() == owner_slot && b.active_flag_raw() != 0)
        .map(|b| b.coords_raw())
        .collect();

    let mut row = start_row + 1;
    let mut shown = 0;
    for planet in &app.game_data.planets.records {
        if planet.owner_empire_slot_raw() != owner_slot {
            continue;
        }
        if shown >= max_rows.max(1) {
            break;
        }
        let name = planet.planet_name();
        let abbrev: String = name.chars().take(3).collect();
        let has_sb = starbase_coords.contains(&planet.coords_raw());
        let sb_char = if has_sb { '★' } else { ' ' };
        let coords = planet.coords_raw();
        let present = planet.present_production_points().unwrap_or(0);

        // Skip scrolled entries.
        if shown < app.planets_scroll {
            shown += 1;
            continue;
        }

        let line = format!(
            " {}{} ({:02},{:02}) {:3}",
            abbrev, sb_char, coords[0], coords[1], present
        );
        let style = if present == 0 {
            theme::enemy_style()
        } else if present < planet.potential_production_points() / 2 {
            theme::alert_style()
        } else {
            theme::value_style()
        };
        buf.write_text(row, col, &line, style);
        row += 1;
        shown += 1;
    }

    if row == start_row + 1 {
        buf.write_text(start_row + 1, col, " (none)", theme::dim_style());
    }
}
