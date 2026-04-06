//! P overlay: fullscreen planet management table.
//!
//! Shows all owned planets with coordinates, production, armies, batteries,
//! stardock status, and build queue summary. Command line at bottom.

use nc_ui::PlayfieldBuffer;

use crate::app::state::DashApp;
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let w = buf.width();
    let h = buf.height();
    let col = 2;

    // Overlay box.
    buf.fill_row(0, theme::header_style());
    buf.write_text(0, col, "PLANET LIST", theme::title_style());
    buf.fill_row(h.saturating_sub(1), theme::footer_style());
    buf.write_text(
        h.saturating_sub(1),
        col,
        "COMMAND <- ? J K ^U ^D B A C L U X S I T <Q> ->",
        theme::footer_style(),
    );

    // Column header.
    let header_row = 1;
    buf.fill_row(header_row, theme::section_title_style());
    let hdr = format!(
        " {:<12} {:>6} {:>6} {:>6} {:>4} {:>4} {:>4}  {}",
        "Planet", "Coords", "Prod", "Pot", "AR", "RB", "Flt", "Build"
    );
    buf.write_text(header_row, col - 1, &hdr, theme::section_title_style());

    // Separator.
    let sep_row = 2;
    for c in 0..w { buf.set_cell(sep_row, c, '─', theme::border_style()); }

    let owner_slot = app.player_record_index_1_based as u8;
    let mut row = sep_row + 1;
    let max_rows = h.saturating_sub(sep_row + 3);

    // Starbase coords for ★ indicator.
    let starbase_coords: std::collections::HashSet<[u8; 2]> = app
        .game_data
        .bases
        .records
        .iter()
        .filter(|b| b.owner_empire_raw() == owner_slot && b.active_flag_raw() != 0)
        .map(|b| b.coords_raw())
        .collect();

    let mut shown = 0;
    for planet in &app.game_data.planets.records {
        if planet.owner_empire_slot_raw() != owner_slot {
            continue;
        }
        if shown < app.planets_scroll {
            shown += 1;
            continue;
        }
        if shown >= max_rows + app.planets_scroll {
            break;
        }

        let name = planet.planet_name();
        let has_sb = starbase_coords.contains(&planet.coords_raw());
        let sb = if has_sb { '★' } else { ' ' };
        let coords = planet.coords_raw();
        let present = planet.present_production_points().unwrap_or(0);
        let potential = planet.potential_production_points();
        let armies = planet.army_count_raw();
        let batteries = planet.ground_batteries_raw();

        // Build queue summary.
        let build_slots: usize = (0..10)
            .filter(|&s| planet.build_count_raw(s) > 0)
            .count();
        let build_str = if build_slots > 0 {
            format!("{} queued", build_slots)
        } else {
            String::from("—")
        };

        let line = format!(
            " {}{:<11} {:02},{:02}  {:>5} {:>5}  {:>3}  {:>3}  {:>3}  {}",
            sb, &name[..name.len().min(11)], coords[0], coords[1],
            present, potential, armies, batteries, "—", build_str
        );

        let style = if present == 0 {
            theme::enemy_style()
        } else {
            theme::value_style()
        };
        buf.write_text(row, 0, &line, style);
        row += 1;
        shown += 1;
    }

    if shown == 0 {
        buf.write_text(sep_row + 1, col, "(no planets)", theme::dim_style());
    }
}
