//! Center panel: sector grid, crosshair, axis labels, status line.

use nc_data::{DiplomaticRelation, IntelTier, PlanetIntelSnapshot};
use nc_ui::{CellStyle, GameColor, PlayfieldBuffer};

use crate::app::state::DashApp;
use crate::layout;
use crate::layout::geometry::{CELL_WIDTH, ROW_LABEL_COLS};
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let (ox, oy) = layout::frame_offset(app);
    let map_start_col = layout::center_start_col(ox);
    let right_div = layout::right_divider_col(app, ox);

    let axis_row = oy + 2;
    let grid_start = oy + 3;

    let map_size = nc_data::map_size_for_player_count(
        app.game_data.conquest.player_count(),
    ) as usize;

    let player_empire = app.player_record_index_1_based as u8;

    // Column axis numbers.
    for col_idx in 0..map_size {
        let screen_col = map_start_col + ROW_LABEL_COLS + col_idx * CELL_WIDTH;
        if screen_col + 2 >= right_div { break; }
        buf.write_text(axis_row, screen_col, &format!("{:02}", col_idx + 1), theme::dim_style());
    }

    // Grid rows — row_y descends: map_size at top, 1 at bottom.
    for row_idx in 0..map_size {
        let row_y = (map_size - row_idx) as u8;
        let screen_row = grid_start + row_idx;
        let is_h_crosshair = row_y == app.crosshair_y;

        buf.write_text(screen_row, map_start_col, &format!("{:02} ", row_y), theme::dim_style());

        for col_idx in 0..map_size {
            let col_x = (col_idx + 1) as u8;
            let screen_col = map_start_col + ROW_LABEL_COLS + col_idx * CELL_WIDTH;
            if screen_col + CELL_WIDTH > right_div { break; }
            let is_v_crosshair = col_x == app.crosshair_x;

            let planet = planet_snapshot_at(app, [col_x, row_y]);

            let (sym, base_style) = if let Some(snapshot) = planet {
                marker_for_snapshot(app, player_empire, snapshot)
            } else {
                ('·', theme::dim_style())
            };

            let (left, mid, right, cell_style) = if is_h_crosshair && is_v_crosshair {
                (' ', sym, ' ', CellStyle::new(GameColor::BrightWhite, GameColor::BrightBlack, true))
            } else if is_h_crosshair {
                ('─', sym, '─', CellStyle::new(GameColor::BrightRed, GameColor::Black, false))
            } else if is_v_crosshair {
                (' ', sym, ' ', CellStyle::new(GameColor::BrightRed, GameColor::Black, false))
            } else {
                (' ', sym, ' ', base_style)
            };

            buf.set_cell(screen_row, screen_col, left, cell_style);
            let mid_style = if is_h_crosshair || is_v_crosshair { cell_style } else { base_style };
            buf.set_cell(screen_row, screen_col + 1, mid, mid_style);
            buf.set_cell(screen_row, screen_col + 2, right, cell_style);
        }
    }

    // Status line below grid.
    let status_row = grid_start + map_size;
    let cx = app.crosshair_x;
    let cy = app.crosshair_y;
    let status = if let Some(snapshot) = planet_snapshot_at(app, [cx, cy]) {
        format_snapshot_status(app, [cx, cy], snapshot)
    } else {
        format!("Sector ({:02},{:02}) — uncharted", cx, cy)
    };
    let max_w = layout::center_width(app).saturating_sub(2);
    let truncated: String = status.chars().take(max_w).collect();
    buf.write_text_clipped(status_row, map_start_col + 1, &truncated, theme::value_style());
}

fn planet_snapshot_at(app: &DashApp, coords: [u8; 2]) -> Option<&PlanetIntelSnapshot> {
    app.planet_intel_snapshots.iter().find(|snapshot| {
        snapshot.intel_tier != IntelTier::Unknown
            && app
                .game_data
                .planets
                .records
                .get(snapshot.planet_record_index_1_based.saturating_sub(1))
                .map(|planet| planet.coords_raw() == coords)
                .unwrap_or(false)
    })
}

fn marker_for_snapshot(
    app: &DashApp,
    viewer_empire_id: u8,
    snapshot: &PlanetIntelSnapshot,
) -> (char, CellStyle) {
    match snapshot.known_owner_empire_id {
        Some(owner) if owner == viewer_empire_id => ('■', theme::friendly_style()),
        Some(0) => ('○', theme::dim_style()),
        Some(owner) => {
            let is_icd = app
                .game_data
                .player
                .records
                .get(owner.saturating_sub(1) as usize)
                .map(|player| player.is_civil_disorder_player())
                .unwrap_or(false);
            if is_icd {
                ('◊', theme::icd_style())
            } else {
                let viewer = app
                    .game_data
                    .player
                    .records
                    .get(viewer_empire_id.saturating_sub(1) as usize);
                let is_enemy = viewer
                    .and_then(|viewer| viewer.diplomatic_relation_toward(owner))
                    == Some(DiplomaticRelation::Enemy);
                if is_enemy {
                    ('●', theme::enemy_style())
                } else {
                    ('○', theme::label_style())
                }
            }
        }
        None => ('·', theme::dim_style()),
    }
}

fn format_snapshot_status(app: &DashApp, coords: [u8; 2], snapshot: &PlanetIntelSnapshot) -> String {
    let owner = match snapshot.known_owner_empire_id {
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
    format!(
        "Sector ({:02},{:02}) {} O:{} Pot:{} Seen:{} AR:{} GB:{} SB:{} Curr:{} Pts:{} Scout:{}",
        coords[0],
        coords[1],
        snapshot.known_name.as_deref().unwrap_or("?"),
        owner,
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
