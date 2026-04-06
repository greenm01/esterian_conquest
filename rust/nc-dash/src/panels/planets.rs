//! Left panel: owned planet summary.

use crate::app::state::DashApp;
use crate::layout::{self, PanelWidgetFrame};
use nc_ui::{CellStyle, PlayfieldBuffer};
use nc_ui::theme::classic::status_value_style;

pub(crate) const TITLE: &str = "MY PLANETS";

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: PanelWidgetFrame) {
    layout::write_panel_title(buf, frame, TITLE, crate::theme::section_title_style());

    for (i, (row, style)) in body_rows(app).into_iter().enumerate() {
        if i >= frame.body.height {
            break;
        }
        layout::write_panel_body_line(buf, frame, i, &row, style);
    }
}

pub(crate) fn body_rows(app: &DashApp) -> Vec<(String, CellStyle)> {
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

    let mut summary_rows = vec![
        (
            layout::format_left_column_value("Tot Worlds", &owned_count.to_string()),
            status_value_style(),
        ),
        (
            layout::format_left_column_value("Act Docks", &stardocks_active.to_string()),
            status_value_style(),
        ),
        (
            layout::format_left_column_value("Starbases", &starbases_built.to_string()),
            status_value_style(),
        ),
        (
            layout::format_left_column_value("Tot Armies", &armies_mustered.to_string()),
            status_value_style(),
        ),
        (
            layout::format_left_column_value("GBs", &ground_batteries.to_string()),
            status_value_style(),
        ),
    ];

    if vulnerable_worlds > 0 {
        summary_rows.push((
            layout::format_left_column_value("Vulnerable", &vulnerable_worlds.to_string()),
            crate::theme::enemy_style(),
        ));
    }

    let stardock = app.game_data.empire_stardock_summary(app.player_record_index_1_based);
    if stardock.battleships > 0 {
        summary_rows.push((layout::format_left_column_value("Bld BBs", &stardock.battleships.to_string()), crate::theme::dim_style()));
    }
    if stardock.cruisers > 0 {
        summary_rows.push((layout::format_left_column_value("Bld CAs", &stardock.cruisers.to_string()), crate::theme::dim_style()));
    }
    if stardock.destroyers > 0 {
        summary_rows.push((layout::format_left_column_value("Bld DDs", &stardock.destroyers.to_string()), crate::theme::dim_style()));
    }
    if stardock.scouts > 0 {
        summary_rows.push((layout::format_left_column_value("Bld SCs", &stardock.scouts.to_string()), crate::theme::dim_style()));
    }
    if stardock.transports > 0 {
        summary_rows.push((layout::format_left_column_value("Bld TRs", &stardock.transports.to_string()), crate::theme::dim_style()));
    }
    if stardock.etacs > 0 {
        summary_rows.push((layout::format_left_column_value("Bld ETs", &stardock.etacs.to_string()), crate::theme::dim_style()));
    }
    if stardock.starbases > 0 {
        summary_rows.push((layout::format_left_column_value("Bld SBs", &stardock.starbases.to_string()), crate::theme::dim_style()));
    }

    summary_rows
}
