//! Left panel: owned planet summary.

use crate::app::state::DashApp;
use crate::layout::{self, PanelWidgetFrame};
use nc_ui::PlayfieldBuffer;
use nc_ui::theme::classic::status_value_style;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: PanelWidgetFrame) {
    layout::write_panel_title(
        buf,
        frame,
        "MY PLANETS",
        crate::theme::section_title_style(),
    );

    let owner_slot = app.player_record_index_1_based as u8;

    let mut owned_count = 0;
    let mut stardocks_active = 0;
    let mut starbases_built = 0;
    let mut armies_mustered = 0;
    let mut ground_batteries = 0;
    let mut vulnerable_worlds = 0;

    let starbase_coords: std::collections::HashSet<[u8; 2]> = app
        .game_data
        .bases
        .records
        .iter()
        .filter(|b| b.owner_empire_raw() == owner_slot && b.active_flag_raw() != 0)
        .map(|b| b.coords_raw())
        .collect();

    for planet in &app.game_data.planets.records {
        if planet.owner_empire_slot_raw() != owner_slot {
            continue;
        }
        owned_count += 1;

        let coords = planet.coords_raw();
        if starbase_coords.contains(&coords) {
            starbases_built += 1;
        }

        let armies = planet.army_count_raw();
        armies_mustered += armies as u32;

        let batteries = planet.ground_batteries_raw();
        ground_batteries += batteries as u32;

        if armies == 0 && batteries == 0 {
            vulnerable_worlds += 1;
        }

        let has_stardock_units = planet.stardock_count_raw(0) > 0
            || planet.stardock_count_raw(1) > 0
            || planet.stardock_count_raw(2) > 0
            || planet.stardock_count_raw(3) > 0
            || planet.stardock_count_raw(4) > 0
            || planet.stardock_count_raw(5) > 0
            || planet.stardock_count_raw(6) > 0;

        if has_stardock_units {
            stardocks_active += 1;
        }
    }

    let summary_rows = vec![
        format!(" Total Worlds:   {:>4}", owned_count),
        format!(" Active Docks:   {:>4}", stardocks_active),
        format!(" Starbases:      {:>4}", starbases_built),
        format!(" Total Armies:   {:>4}", armies_mustered),
        format!(" Grnd Batteries: {:>4}", ground_batteries),
    ];

    for (i, row) in summary_rows.iter().enumerate() {
        if i >= frame.body.height {
            break;
        }
        layout::write_panel_body_line(buf, frame, i, row, status_value_style());
    }

    if summary_rows.len() < frame.body.height && vulnerable_worlds > 0 {
        let warning = format!(" Vulnerable:     {:>4}", vulnerable_worlds);
        layout::write_panel_body_line(
            buf,
            frame,
            summary_rows.len(),
            &warning,
            crate::theme::enemy_style(),
        );
    }
}
