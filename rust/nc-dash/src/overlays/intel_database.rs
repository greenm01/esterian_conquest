//! I overlay: dashboard-sized total planet database.

use nc_data::PlanetIntelSnapshot;
use nc_ui::PlayfieldBuffer;

use crate::app::state::DashApp;
use crate::overlays::frame::{draw_hline, draw_overlay_frame, write_clipped};
use crate::theme;

const FOOTER: &str = "COMMAND <- ? J K ^U ^D S I <Q> ->";

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let preferred_width = buf.width().saturating_sub(12).clamp(94, 134);
    let preferred_height = buf.height().saturating_sub(6).clamp(18, 28);
    let frame = draw_overlay_frame(buf, "TOTAL PLANET DATABASE", preferred_width, preferred_height, FOOTER);

    write_clipped(
        buf,
        frame.body_row,
        frame.body_col,
        frame.body_width,
        "Coord   Planet Name  Owner Prod Seen ARs GBs SBs Curr Points Scout",
        theme::section_title_style(),
    );
    draw_hline(buf, frame.body_row + 1, frame.body_col, frame.body_width, theme::border_style());

    let rows = app
        .planet_intel_snapshots
        .iter()
        .filter(|snapshot| snapshot.intel_tier != nc_data::IntelTier::Unknown)
        .map(|snapshot| format_intel_row(app, snapshot))
        .collect::<Vec<_>>();

    let list_start = frame.body_row + 2;
    let max_rows = frame.body_height.saturating_sub(2);
    let selected = app.intel_overlay.selected.min(rows.len().saturating_sub(1));
    let scroll = clamp_scroll(app.intel_overlay.scroll, selected, max_rows, rows.len());

    for (visible_idx, line) in rows.iter().skip(scroll).take(max_rows).enumerate() {
        let row = list_start + visible_idx;
        let absolute_idx = scroll + visible_idx;
        let style = if absolute_idx == selected {
            theme::alert_style()
        } else {
            theme::value_style()
        };
        if absolute_idx == selected {
            buf.fill_rect(row, frame.body_col, frame.body_width, 1, style);
        }
        write_clipped(buf, row, frame.body_col, frame.body_width, line, style);
    }

    if rows.is_empty() {
        write_clipped(
            buf,
            list_start,
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

fn format_intel_row(app: &DashApp, snapshot: &PlanetIntelSnapshot) -> String {
    let planet = app
        .game_data
        .planets
        .records
        .get(snapshot.planet_record_index_1_based.saturating_sub(1));
    let coords = planet.map(|planet| planet.coords_raw()).unwrap_or([0, 0]);
    let owner_label = match snapshot.known_owner_empire_id {
        Some(0) => String::from("UN"),
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
    format!(
        "({:02},{:02}) {:<11} {:>5} {:>4} {:>4} {:>3} {:>3} {:>3} {:>4} {:>6} {:>5}",
        coords[0],
        coords[1],
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
    )
}
