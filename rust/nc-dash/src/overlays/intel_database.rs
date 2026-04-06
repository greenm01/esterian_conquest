//! I overlay: dashboard-sized total planet database.

use std::collections::BTreeMap;

use nc_data::{PlanetIntelSnapshot, build_player_starmap_projection_from_snapshots};
use nc_ui::PlayfieldBuffer;
use nc_ui::table::{
    TableColumn, TableWidthMode, centered_table_start_col, resolve_table_columns, table_render_width,
    write_stacked_table_window_with_theme_at,
};

use crate::app::state::DashApp;
use crate::overlays::frame::{draw_overlay_frame_for_body, write_clipped};
use crate::theme;

const FOOTER: &str = "COMMAND <- ? J K ^U ^D S I <Q> ->";
const TOP_HEADERS: [&str; 11] = [
    "Coord", "", "", "", "", "", "", "", "Curr", "", "",
];
const COLUMNS: [TableColumn<'static>; 11] = [
    TableColumn::left("(XX,YY)", 7),
    TableColumn::left("Planet Name", 11),
    TableColumn::left("Owner", 7),
    TableColumn::right("Prod", 4),
    TableColumn::right("Seen", 4),
    TableColumn::right("ARs", 3),
    TableColumn::right("GBs", 3),
    TableColumn::right("SBs", 3),
    TableColumn::right("Prod", 4),
    TableColumn::right("Points", 6),
    TableColumn::right("Scout", 5),
];

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let snapshot_map = app
        .planet_intel_snapshots
        .iter()
        .cloned()
        .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
        .collect::<BTreeMap<_, _>>();
    let projection = build_player_starmap_projection_from_snapshots(
        &app.game_data,
        &snapshot_map,
        app.player_record_index_1_based as u8,
    );
    let rows = projection
        .worlds
        .iter()
        .map(|world| {
            let snapshot = snapshot_map.get(&world.planet_record_index_1_based);
            format_intel_row(app, world, snapshot)
        })
        .collect::<Vec<_>>();
    let desired_visible_rows = rows.len().clamp(1, buf.height().saturating_sub(11));
    let columns = resolve_table_columns(
        &COLUMNS,
        &rows,
        buf.width().saturating_sub(12),
        false,
        TableWidthMode::Compact,
    );
    let body_width = table_render_width(&columns).max("No planet intel is available yet.".chars().count() + 4);
    let frame = draw_overlay_frame_for_body(
        buf,
        "TOTAL PLANET DATABASE",
        body_width,
        desired_visible_rows + 5,
        FOOTER,
    );
    let visible_rows = frame.body_height.saturating_sub(5);
    let selected = app.intel_overlay.selected.min(rows.len().saturating_sub(1));
    let scroll = clamp_scroll(app.intel_overlay.scroll, selected, visible_rows, rows.len());
    let table_col = frame.body_col + centered_table_start_col(frame.body_width, &columns);
    let metrics = write_stacked_table_window_with_theme_at(
        buf,
        frame.body_row,
        table_col,
        &TOP_HEADERS,
        &columns,
        &rows,
        scroll,
        visible_rows,
        theme::table_theme(),
        rows.get(selected).map(|_| selected),
        0,
        None,
    );

    if rows.is_empty() {
        write_clipped(
            buf,
            metrics.bottom_row.saturating_sub(1),
            frame.body_col,
            frame.body_width,
            "No planet intel is available yet.",
            theme::dim_style(),
        );
    }
}

fn clamp_scroll(scroll: usize, selected: usize, max_rows: usize, total_rows: usize) -> usize {
    if max_rows == 0 || total_rows <= max_rows {
        return 0;
    }
    if selected < scroll {
        return selected;
    }
    if selected >= scroll + max_rows {
        return selected + 1 - max_rows;
    }
    scroll.min(total_rows.saturating_sub(max_rows))
}

fn truncate(value: &str, width: usize) -> String {
    value.chars().take(width).collect()
}

fn format_intel_row(
    app: &DashApp,
    world: &nc_data::PlayerStarmapWorld,
    snapshot: Option<&PlanetIntelSnapshot>,
) -> Vec<String> {
    let coords = world.coords;
    let owner_label = match world.known_owner_empire_id {
        Some(0) => String::from("Unowned"),
        Some(owner) => app
            .game_data
            .player
            .records
            .get(owner.saturating_sub(1) as usize)
            .map(|player| {
                if player.is_civil_disorder_player() {
                    String::from("ICD")
                } else {
                    format!("#{owner}")
                }
            })
            .unwrap_or_else(|| format!("#{owner}")),
        None => String::from("?"),
    };
    vec![
        format!("({:02},{:02})", coords[0], coords[1]),
        truncate(world.known_name.as_deref().unwrap_or("?"), 11),
        owner_label,
        world
            .known_potential_production
            .map(|value| value.to_string())
            .unwrap_or_else(|| String::from("?")),
        snapshot
            .and_then(|row| row.last_intel_year)
            .map(|value| value.to_string())
            .unwrap_or_else(|| String::from("?")),
        world
            .known_armies
            .map(|value| value.to_string())
            .unwrap_or_else(|| String::from("?")),
        world
            .known_ground_batteries
            .map(|value| value.to_string())
            .unwrap_or_else(|| String::from("?")),
        world
            .known_starbase_count
            .map(|value| value.to_string())
            .unwrap_or_else(|| String::from("?")),
        world
            .known_current_production
            .map(|value| value.to_string())
            .unwrap_or_else(|| String::from("?")),
        world
            .known_stored_points
            .map(|value| value.to_string())
            .unwrap_or_else(|| String::from("?")),
        snapshot
            .and_then(|row| row.scout_year)
            .map(|value| value.to_string())
            .unwrap_or_else(|| String::from("?")),
    ]
}
