//! Center panel: sector grid, crosshair, axis labels, status line.

use std::collections::BTreeMap;

use nc_data::{
    DiplomaticRelation, PlanetIntelSnapshot, PlayerStarmapProjection, PlayerStarmapWorld,
    build_player_starmap_projection_from_snapshots,
};
use nc_ui::{CellStyle, GameColor, PlayfieldBuffer};

use crate::app::state::DashApp;
use crate::layout::{self, MapWidgetFrame};
use crate::theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StarmapMarkerKind {
    Owned,
    Unowned,
    Icd,
    Enemy,
    Neutral,
    Partial,
    Unknown,
}

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: MapWidgetFrame) {
    let map_size = nc_data::map_size_for_player_count(
        app.game_data.conquest.player_count(),
    ) as usize;

    let player_empire = app.player_record_index_1_based as u8;
    let snapshot_map = app
        .planet_intel_snapshots
        .iter()
        .cloned()
        .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
        .collect::<BTreeMap<_, _>>();
    let projection = build_player_starmap_projection_from_snapshots(
        &app.game_data,
        &snapshot_map,
        player_empire,
    );

    // Column axis numbers.
    for col_idx in 0..map_size {
        let screen_col = frame.grid.col + frame.row_label_cols + col_idx * frame.cell_width;
        if screen_col + 1 > frame.grid.last_col() { break; }
        layout::write_strict_span(
            buf,
            frame.axis_row,
            screen_col,
            2,
            &format!("{:02}", col_idx + 1),
            theme::dim_style(),
            "starmap axis label",
        );
    }

    // Grid rows — row_y descends: map_size at top, 1 at bottom.
    for row_idx in 0..map_size {
        let row_y = (map_size - row_idx) as u8;
        let screen_row = frame.grid.row + row_idx;
        let is_h_crosshair = row_y == app.crosshair_y;

        layout::write_strict_span(
            buf,
            screen_row,
            frame.grid.col,
            frame.row_label_cols,
            &format!("{:02} ", row_y),
            theme::dim_style(),
            "starmap row label",
        );

        for col_idx in 0..map_size {
            let col_x = (col_idx + 1) as u8;
            let screen_col = frame.grid.col + frame.row_label_cols + col_idx * frame.cell_width;
            if screen_col + frame.cell_width - 1 > frame.grid.last_col() { break; }
            let is_v_crosshair = col_x == app.crosshair_x;

            let planet = projection_world_at(&projection, [col_x, row_y]);

            let (sym, base_style) = if let Some(snapshot) = planet {
                marker_for_world(app, player_empire, snapshot)
            } else {
                ('·', theme::dim_style())
            };

            let (left, mid, right, cell_style) = if is_h_crosshair && is_v_crosshair {
                (' ', sym, ' ', CellStyle::new(GameColor::BrightWhite, GameColor::BrightBlack, true))
            } else if is_h_crosshair {
                (' ', sym, ' ', CellStyle::new(GameColor::BrightRed, GameColor::Black, false))
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
    let cx = app.crosshair_x;
    let cy = app.crosshair_y;
    let status = if let Some(world) = projection_world_at(&projection, [cx, cy]) {
        format_world_status(app, [cx, cy], world, snapshot_map.get(&world.planet_record_index_1_based))
    } else {
        format!("Sector ({:02},{:02}) — uncharted", cx, cy)
    };
    let max_w = frame.outer.width.saturating_sub(2);
    layout::write_clipped(
        buf,
        frame.status_row,
        frame.outer.col + 1,
        max_w,
        &status,
        theme::value_style(),
    );
}

fn projection_world_at(
    projection: &PlayerStarmapProjection,
    coords: [u8; 2],
) -> Option<&PlayerStarmapWorld> {
    projection.worlds.iter().find(|world| world.coords == coords)
}

pub(crate) fn marker_kind_for_world(
    app: &DashApp,
    viewer_empire_id: u8,
    world: &PlayerStarmapWorld,
) -> StarmapMarkerKind {
    match world.known_owner_empire_id {
        Some(owner) if owner == viewer_empire_id => StarmapMarkerKind::Owned,
        Some(0) => StarmapMarkerKind::Unowned,
        Some(owner) => {
            let is_icd = app
                .game_data
                .player
                .records
                .get(owner.saturating_sub(1) as usize)
                .map(|player| player.is_civil_disorder_player())
                .unwrap_or(false);
            if is_icd {
                StarmapMarkerKind::Icd
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
                    StarmapMarkerKind::Enemy
                } else {
                    StarmapMarkerKind::Neutral
                }
            }
        }
        None if world.known_name.is_some()
            || world.known_potential_production.is_some()
            || world.known_armies.is_some()
            || world.known_ground_batteries.is_some() =>
        {
            StarmapMarkerKind::Partial
        }
        None => StarmapMarkerKind::Unknown,
    }
}

fn marker_for_world(
    app: &DashApp,
    viewer_empire_id: u8,
    world: &PlayerStarmapWorld,
) -> (char, CellStyle) {
    match marker_kind_for_world(app, viewer_empire_id, world) {
        StarmapMarkerKind::Owned => ('O', theme::friendly_style()),
        StarmapMarkerKind::Unowned => ('#', theme::dim_style()),
        StarmapMarkerKind::Icd => ('◊', theme::icd_style()),
        StarmapMarkerKind::Enemy => ('#', theme::enemy_style()),
        StarmapMarkerKind::Neutral => ('#', theme::label_style()),
        StarmapMarkerKind::Partial => ('*', theme::value_style()),
        StarmapMarkerKind::Unknown => ('?', theme::dim_style()),
    }
}

fn format_world_status(
    app: &DashApp,
    coords: [u8; 2],
    world: &PlayerStarmapWorld,
    snapshot: Option<&PlanetIntelSnapshot>,
) -> String {
    let owner = match world.known_owner_empire_id {
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
        world.known_name.as_deref().unwrap_or("?"),
        owner,
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
    )
}
