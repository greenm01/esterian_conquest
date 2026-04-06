//! I overlay: dashboard-sized total planet database.

use nc_data::PlanetIntelSnapshot;
use nc_ui::PlayfieldBuffer;
use nc_ui::table::{
    TableColumn, TableWidthMode, centered_table_start_col, resolve_table_columns,
    write_stacked_table_window_with_theme_at,
};

use crate::app::state::DashApp;
use crate::overlays::frame::{draw_overlay_frame, write_clipped};
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
    let preferred_width = buf.width().saturating_sub(12).clamp(94, 134);
    let preferred_height = buf.height().saturating_sub(6).clamp(18, 28);
    let frame = draw_overlay_frame(buf, "TOTAL PLANET DATABASE", preferred_width, preferred_height, FOOTER);

    let rows = app
        .planet_intel_snapshots
        .iter()
        .filter(|snapshot| snapshot.intel_tier != nc_data::IntelTier::Unknown)
        .map(|snapshot| format_intel_row(app, snapshot))
        .collect::<Vec<_>>();

    let visible_rows = frame.body_height.saturating_sub(5);
    let selected = app.intel_overlay.selected.min(rows.len().saturating_sub(1));
    let scroll = clamp_scroll(app.intel_overlay.scroll, selected, visible_rows, rows.len());
    let columns = resolve_table_columns(
        &COLUMNS,
        &rows,
        frame.body_width.saturating_sub(1),
        false,
        TableWidthMode::Compact,
    );
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
            table_col + 2,
            frame.body_width.saturating_sub(4),
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

fn format_intel_row(app: &DashApp, snapshot: &PlanetIntelSnapshot) -> Vec<String> {
    let planet = app
        .game_data
        .planets
        .records
        .get(snapshot.planet_record_index_1_based.saturating_sub(1));
    let coords = planet.map(|planet| planet.coords_raw()).unwrap_or([0, 0]);
    let owner_label = match snapshot.known_owner_empire_id {
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
        truncate(snapshot.known_name.as_deref().unwrap_or("?"), 11),
        owner_label,
        snapshot
            .known_potential_production
            .map(|value| value.to_string())
            .unwrap_or_else(|| String::from("?")),
        snapshot
            .seen_year
            .map(|value| value.to_string())
            .unwrap_or_else(|| String::from("?")),
        snapshot
            .known_armies
            .map(|value| value.to_string())
            .unwrap_or_else(|| String::from("?")),
        snapshot
            .known_ground_batteries
            .map(|value| value.to_string())
            .unwrap_or_else(|| String::from("?")),
        snapshot
            .known_starbase_count
            .map(|value| value.to_string())
            .unwrap_or_else(|| String::from("?")),
        snapshot
            .known_current_production
            .map(|value| value.to_string())
            .unwrap_or_else(|| String::from("?")),
        snapshot
            .known_stored_points
            .map(|value| value.to_string())
            .unwrap_or_else(|| String::from("?")),
        snapshot
            .scout_year
            .map(|value| value.to_string())
            .unwrap_or_else(|| String::from("?")),
    ]
}
