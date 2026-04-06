//! Center panel: sector grid, crosshair, axis labels, status line.

use std::collections::BTreeMap;

use nc_data::{
    CoreGameData, DiplomaticRelation, PlanetIntelSnapshot, PlayerStarmapProjection,
    PlayerStarmapWorld, build_player_starmap_projection_from_snapshots,
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
    let map_size =
        nc_data::map_size_for_player_count(app.game_data.conquest.player_count()) as usize;

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
        if screen_col + 1 > frame.grid.last_col() {
            break;
        }
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
            if screen_col + frame.cell_width - 1 > frame.grid.last_col() {
                break;
            }
            let is_v_crosshair = col_x == app.crosshair_x;

            let planet = projection_world_at(&projection, [col_x, row_y]);

            let (sym, base_style) = if let Some(snapshot) = planet {
                marker_for_world(app, player_empire, snapshot)
            } else {
                ('·', theme::dim_style())
            };

            let (left, mid, right, cell_style) = if is_h_crosshair && is_v_crosshair {
                (
                    ' ',
                    sym,
                    ' ',
                    CellStyle::new(GameColor::BrightWhite, GameColor::BrightBlack, true),
                )
            } else if is_h_crosshair {
                (
                    ' ',
                    sym,
                    ' ',
                    CellStyle::new(GameColor::BrightRed, GameColor::Black, false),
                )
            } else if is_v_crosshair {
                (
                    ' ',
                    sym,
                    ' ',
                    CellStyle::new(GameColor::BrightRed, GameColor::Black, false),
                )
            } else {
                (' ', sym, ' ', base_style)
            };

            buf.set_cell(screen_row, screen_col, left, cell_style);
            let mid_style = if is_h_crosshair || is_v_crosshair {
                cell_style
            } else {
                base_style
            };
            buf.set_cell(screen_row, screen_col + 1, mid, mid_style);
            buf.set_cell(screen_row, screen_col + 2, right, cell_style);
        }
    }

    // Status line below grid.
    let cx = app.crosshair_x;
    let cy = app.crosshair_y;
    let status = if let Some(world) = projection_world_at(&projection, [cx, cy]) {
        format_world_status(
            &app.game_data,
            [cx, cy],
            world,
            snapshot_map.get(&world.planet_record_index_1_based),
        )
    } else {
        format!("({:02},{:02}) uncharted", cx, cy)
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
    projection
        .worlds
        .iter()
        .find(|world| world.coords == coords)
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
                let is_enemy = viewer.and_then(|viewer| viewer.diplomatic_relation_toward(owner))
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
        StarmapMarkerKind::Owned => (
            'O',
            theme::empire_slot_style(world.known_owner_empire_id.unwrap_or(viewer_empire_id)),
        ),
        StarmapMarkerKind::Unowned => ('#', theme::dim_style()),
        StarmapMarkerKind::Icd => (
            '◊',
            theme::empire_slot_style(world.known_owner_empire_id.unwrap_or(viewer_empire_id)),
        ),
        StarmapMarkerKind::Enemy => (
            '#',
            theme::empire_slot_style(world.known_owner_empire_id.unwrap_or(viewer_empire_id)),
        ),
        StarmapMarkerKind::Neutral => (
            '#',
            theme::empire_slot_style(world.known_owner_empire_id.unwrap_or(viewer_empire_id)),
        ),
        StarmapMarkerKind::Partial => ('*', theme::value_style()),
        StarmapMarkerKind::Unknown => ('?', theme::dim_style()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nc_data::{GameStateBuilder, IntelTier};

    #[test]
    fn owner_markers_use_empire_slot_colors() {
        let owner = Some(4);
        let expected = theme::empire_slot_color(4);

        let (_, owned_style) = marker_for_world_kind(owner, StarmapMarkerKind::Owned);
        let (_, enemy_style) = marker_for_world_kind(owner, StarmapMarkerKind::Enemy);
        let (_, neutral_style) = marker_for_world_kind(owner, StarmapMarkerKind::Neutral);
        let (_, icd_style) = marker_for_world_kind(owner, StarmapMarkerKind::Icd);

        assert_eq!(owned_style.fg, expected);
        assert_eq!(enemy_style.fg, expected);
        assert_eq!(neutral_style.fg, expected);
        assert_eq!(icd_style.fg, expected);
    }

    fn marker_for_world_kind(owner: Option<u8>, kind: StarmapMarkerKind) -> (char, CellStyle) {
        match kind {
            StarmapMarkerKind::Owned => ('O', theme::empire_slot_style(owner.unwrap())),
            StarmapMarkerKind::Unowned => ('#', theme::dim_style()),
            StarmapMarkerKind::Icd => ('◊', theme::empire_slot_style(owner.unwrap())),
            StarmapMarkerKind::Enemy => ('#', theme::empire_slot_style(owner.unwrap())),
            StarmapMarkerKind::Neutral => ('#', theme::empire_slot_style(owner.unwrap())),
            StarmapMarkerKind::Partial => ('*', theme::value_style()),
            StarmapMarkerKind::Unknown => ('?', theme::dim_style()),
        }
    }

    #[test]
    fn world_status_uses_compact_grouped_fields() {
        let game_data = GameStateBuilder::new()
            .with_player_count(4)
            .with_year(3006)
            .build_initialized_baseline()
            .expect("baseline game data");
        let world = PlayerStarmapWorld {
            planet_record_index_1_based: 1,
            coords: [9, 9],
            intel_tier: IntelTier::Partial,
            known_name: Some(String::from("98")),
            known_owner_empire_id: Some(4),
            known_owner_empire_name: None,
            known_potential_production: Some(98),
            known_armies: None,
            known_ground_batteries: None,
            known_starbase_count: Some(0),
            known_current_production: Some(45),
            known_stored_points: Some(12),
            known_docked_summary: None,
            known_orbit_summary: None,
        };
        let snapshot = PlanetIntelSnapshot {
            planet_record_index_1_based: 1,
            intel_tier: IntelTier::Partial,
            compat_is_orbit_seed: false,
            last_intel_year: Some(3006),
            seen_year: Some(3006),
            scout_year: Some(3005),
            known_name: Some(String::from("98")),
            known_owner_empire_id: Some(4),
            known_potential_production: Some(98),
            known_armies: None,
            known_ground_batteries: None,
            known_starbase_count: Some(0),
            known_current_production: Some(45),
            known_stored_points: Some(12),
            known_docked_summary: None,
            known_orbit_summary: None,
            compat_word_1e: None,
        };

        let status = format_world_status(&game_data, [9, 9], &world, Some(&snapshot));
        assert_eq!(status, "(09,09) O:#4 E:98/45/12 D:?/?/0 Y:3006");
        assert!(status.chars().count() <= 55);
    }

    #[test]
    fn world_status_handles_unknown_and_special_owners() {
        let mut game_data = GameStateBuilder::new()
            .with_player_count(4)
            .with_year(3006)
            .build_initialized_baseline()
            .expect("baseline game data");
        game_data.player.records[2].set_player_mode_raw(0x00);

        assert_eq!(owner_label(&game_data, Some(0)), "Unowned");
        assert_eq!(owner_label(&game_data, Some(3)), "ICD");
        assert_eq!(owner_label(&game_data, None), "?");
        assert_eq!(known_u16(None), "?");
        assert_eq!(known_u8(None), "?");
    }
}

fn format_world_status(
    game_data: &CoreGameData,
    coords: [u8; 2],
    world: &PlayerStarmapWorld,
    snapshot: Option<&PlanetIntelSnapshot>,
) -> String {
    let owner = owner_label(game_data, world.known_owner_empire_id);
    format!(
        "({:02},{:02}) O:{} E:{}/{}/{} D:{}/{}/{} Y:{}",
        coords[0],
        coords[1],
        owner,
        known_u16(world.known_potential_production),
        known_u8(world.known_current_production),
        known_u16(world.known_stored_points),
        known_u8(world.known_armies),
        known_u8(world.known_ground_batteries),
        known_u8(world.known_starbase_count),
        known_u16(snapshot.and_then(|row| row.last_intel_year)),
    )
}

fn owner_label(game_data: &CoreGameData, known_owner_empire_id: Option<u8>) -> String {
    match known_owner_empire_id {
        Some(0) => String::from("Unowned"),
        Some(owner) => game_data
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
    }
}

fn known_u8(value: Option<u8>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| String::from("?"))
}

fn known_u16(value: Option<u16>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| String::from("?"))
}
