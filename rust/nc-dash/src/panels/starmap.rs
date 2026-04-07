//! Center panel: sector grid, crosshair, and axis labels.

use std::collections::BTreeMap;

use nc_data::{
    CoreGameData, DiplomaticRelation, PlanetIntelSnapshot, PlayerStarmapProjection,
    PlayerStarmapWorld, build_player_starmap_projection_from_snapshots,
};
use nc_ui::{CellStyle, PlayfieldBuffer};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PlanetJumpDirection {
    Backward,
    Forward,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProjectedMapGeometry {
    axis_row: usize,
    row_label_col: usize,
    row_label_width: usize,
    x_min: u8,
    x_max: u8,
    y_min: u8,
    y_max: u8,
    visible_x: u8,
    visible_y: u8,
    tile_width: usize,
    tile_height: usize,
    col_edges: Vec<usize>,
    row_edges: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SectorRect {
    col: usize,
    row: usize,
    width: usize,
    height: usize,
}

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: MapWidgetFrame) {
    let map_size = nc_data::map_size_for_player_count(app.game_data.conquest.player_count());

    let player_empire = app.player_record_index_1_based as u8;
    let snapshot_map = snapshot_map_for_app(app);
    let projection = projection_for_snapshot_map(app, &snapshot_map);
    let projected = projected_map_geometry(app, frame, map_size);

    // Column axis numbers.
    for world_x in projected.x_min..=projected.x_max {
        let Some(rect) = projected.sector_rect([world_x, projected.y_max]) else {
            continue;
        };
        draw_column_axis_label(buf, projected.axis_row, rect, world_x);
    }

    // Grid rows — row_y descends: y_max at top, y_min at bottom.
    for row_y in (projected.y_min..=projected.y_max).rev() {
        let Some(row_rect) = projected.sector_rect([projected.x_min, row_y]) else {
            continue;
        };
        draw_row_axis_label(
            buf,
            projected.row_label_col,
            projected.row_label_width,
            row_rect,
            row_y,
        );

        for col_x in projected.x_min..=projected.x_max {
            let Some(rect) = projected.sector_rect([col_x, row_y]) else {
                continue;
            };
            let is_h_crosshair = row_y == app.crosshair_y;
            let is_v_crosshair = col_x == app.crosshair_x;
            let planet = projection_world_at(&projection, [col_x, row_y]);
            let (sym, base_style) = if let Some(snapshot) = planet {
                marker_for_world(app, player_empire, snapshot)
            } else if is_h_crosshair && is_v_crosshair {
                ('+', theme::map_center_style())
            } else if is_h_crosshair {
                ('-', theme::map_crosshair_style())
            } else if is_v_crosshair {
                ('|', theme::map_crosshair_style())
            } else {
                ('·', theme::dim_style())
            };
            let fill_style = if is_h_crosshair && is_v_crosshair {
                theme::map_center_style()
            } else if is_h_crosshair || is_v_crosshair {
                theme::map_crosshair_style()
            } else {
                theme::body_style()
            };

            fill_sector_rect(buf, rect, ' ', fill_style);
            if is_h_crosshair || is_v_crosshair {
                draw_crosshair_lines(buf, rect, is_h_crosshair, is_v_crosshair, fill_style);
            }
            draw_sector_marker(
                buf,
                rect,
                sym,
                if is_h_crosshair || is_v_crosshair {
                    fill_style
                } else {
                    base_style
                },
            );
        }
    }
}

fn projected_map_geometry(
    app: &DashApp,
    frame: MapWidgetFrame,
    map_size: u8,
) -> ProjectedMapGeometry {
    let cell_area_col = frame.grid.col + frame.row_label_cols;
    let cell_area_width = frame.grid.width.saturating_sub(frame.row_label_cols);
    let projection = projected_display_bounds(app, frame, map_size, cell_area_width);

    ProjectedMapGeometry {
        axis_row: frame.axis_row,
        row_label_col: frame.grid.col,
        row_label_width: frame.row_label_cols,
        x_min: projection.x_min,
        x_max: projection.x_max,
        y_min: projection.y_min,
        y_max: projection.y_max,
        visible_x: projection.visible_x,
        visible_y: projection.visible_y,
        tile_width: projection.tile_width,
        tile_height: projection.tile_height,
        col_edges: partition_edges(
            cell_area_col,
            cell_area_width,
            projection.visible_x as usize,
        ),
        row_edges: partition_edges(
            frame.grid.row,
            frame.grid.height,
            projection.visible_y as usize,
        ),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ProjectionBounds {
    x_min: u8,
    x_max: u8,
    y_min: u8,
    y_max: u8,
    visible_x: u8,
    visible_y: u8,
    tile_width: usize,
    tile_height: usize,
}

fn projected_display_bounds(
    app: &DashApp,
    frame: MapWidgetFrame,
    map_size: u8,
    cell_area_width: usize,
) -> ProjectionBounds {
    let visible = visible_sector_count(map_size, app.map_zoom_level);
    let x_min = viewport_start(app.crosshair_x, visible, map_size);
    let y_min = viewport_start(app.crosshair_y, visible, map_size);
    let x_max = x_min + visible.saturating_sub(1);
    let y_max = y_min + visible.saturating_sub(1);
    ProjectionBounds {
        x_min,
        x_max,
        y_min,
        y_max,
        visible_x: visible,
        visible_y: visible,
        tile_width: exact_fill_tile_hint(cell_area_width, visible),
        tile_height: exact_fill_tile_hint(frame.grid.height, visible),
    }
}

fn visible_sector_count(map_size: u8, zoom_level: u8) -> u8 {
    let divisor = 1u16 << zoom_level.min(5);
    let visible = u16::from(map_size).div_ceil(divisor).max(1);
    visible.min(u16::from(map_size)) as u8
}

fn exact_fill_tile_hint(extent: usize, visible: u8) -> usize {
    extent.div_ceil(usize::from(visible)).max(1)
}

fn viewport_start(center: u8, visible: u8, map_size: u8) -> u8 {
    let half = visible / 2;
    let max_start = map_size.saturating_sub(visible).saturating_add(1);
    center.saturating_sub(half).clamp(1, max_start)
}

fn partition_edges(start: usize, extent: usize, count: usize) -> Vec<usize> {
    (0..=count)
        .map(|idx| start + (idx * extent) / count.max(1))
        .collect()
}

impl ProjectedMapGeometry {
    fn sector_rect(&self, coords: [u8; 2]) -> Option<SectorRect> {
        if coords[0] < self.x_min
            || coords[0] > self.x_max
            || coords[1] < self.y_min
            || coords[1] > self.y_max
        {
            return None;
        }
        let x_idx = usize::from(coords[0] - self.x_min);
        let y_idx = usize::from(self.y_max - coords[1]);
        let col = self.col_edges.get(x_idx).copied()?;
        let next_col = self.col_edges.get(x_idx + 1).copied()?;
        let row = self.row_edges.get(y_idx).copied()?;
        let next_row = self.row_edges.get(y_idx + 1).copied()?;
        Some(SectorRect {
            col,
            row,
            width: next_col.saturating_sub(col),
            height: next_row.saturating_sub(row),
        })
    }
}

fn draw_column_axis_label(
    buf: &mut PlayfieldBuffer,
    axis_row: usize,
    rect: SectorRect,
    world_x: u8,
) {
    if rect.width == 0 {
        return;
    }
    let label = format!("{world_x:02}");
    if rect.width >= 2 {
        let write_col = rect.col + rect.width.saturating_sub(2) / 2;
        layout::write_clipped(buf, axis_row, write_col, 2, &label, theme::dim_style());
    } else if let Some(ch) = label.chars().last() {
        buf.set_cell(axis_row, rect.col, ch, theme::dim_style());
    }
}

fn draw_row_axis_label(
    buf: &mut PlayfieldBuffer,
    col: usize,
    width: usize,
    rect: SectorRect,
    world_y: u8,
) {
    if rect.height == 0 {
        return;
    }
    let row = rect.row + rect.height.saturating_sub(1) / 2;
    layout::write_clipped(
        buf,
        row,
        col,
        width,
        &format!("{world_y:02} "),
        theme::dim_style(),
    );
}

fn fill_sector_rect(buf: &mut PlayfieldBuffer, rect: SectorRect, ch: char, style: CellStyle) {
    for row in rect.row..rect.row + rect.height {
        for col in rect.col..rect.col + rect.width {
            buf.set_cell(row, col, ch, style);
        }
    }
}

fn draw_sector_marker(buf: &mut PlayfieldBuffer, rect: SectorRect, marker: char, style: CellStyle) {
    if rect.width == 0 || rect.height == 0 {
        return;
    }
    let row = rect.row + rect.height / 2;
    let col = rect.col + rect.width / 2;
    buf.set_cell(row, col, marker, style);
}

fn draw_crosshair_lines(
    buf: &mut PlayfieldBuffer,
    rect: SectorRect,
    horizontal: bool,
    vertical: bool,
    style: CellStyle,
) {
    if rect.width == 0 || rect.height == 0 {
        return;
    }
    let mid_row = rect.row + rect.height / 2;
    let mid_col = rect.col + rect.width / 2;

    if horizontal {
        for col in rect.col..rect.col + rect.width {
            buf.set_cell(mid_row, col, '-', style);
        }
    }
    if vertical {
        for row in rect.row..rect.row + rect.height {
            buf.set_cell(row, mid_col, '|', style);
        }
    }
    if horizontal && vertical {
        buf.set_cell(mid_row, mid_col, '+', style);
    }
}

pub(crate) fn jump_planet_target_for_app(
    app: &DashApp,
    current: [u8; 2],
    direction: PlanetJumpDirection,
) -> Option<[u8; 2]> {
    let projection = projection_for_snapshot_map(app, &snapshot_map_for_app(app));
    jump_planet_target_coords(projection.map_width, &projection.worlds, current, direction)
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

fn snapshot_map_for_app(app: &DashApp) -> BTreeMap<usize, PlanetIntelSnapshot> {
    app.planet_intel_snapshots
        .iter()
        .cloned()
        .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
        .collect::<BTreeMap<_, _>>()
}

fn projection_for_snapshot_map(
    app: &DashApp,
    snapshot_map: &BTreeMap<usize, PlanetIntelSnapshot>,
) -> PlayerStarmapProjection {
    build_player_starmap_projection_from_snapshots(
        &app.game_data,
        snapshot_map,
        app.player_record_index_1_based as u8,
    )
}

fn jump_planet_target_coords(
    map_size: u8,
    worlds: &[PlayerStarmapWorld],
    current: [u8; 2],
    direction: PlanetJumpDirection,
) -> Option<[u8; 2]> {
    let mut coords = worlds.iter().map(|world| world.coords).collect::<Vec<_>>();
    if coords.is_empty() {
        return None;
    }
    coords.sort_by_key(|coords| screen_order_index(*coords, map_size));
    coords.dedup();

    let current_index = screen_order_index(current, map_size);
    match direction {
        PlanetJumpDirection::Forward => coords
            .iter()
            .copied()
            .find(|coords| screen_order_index(*coords, map_size) > current_index)
            .or_else(|| coords.first().copied()),
        PlanetJumpDirection::Backward => coords
            .iter()
            .rev()
            .copied()
            .find(|coords| screen_order_index(*coords, map_size) < current_index)
            .or_else(|| coords.last().copied()),
    }
}

fn screen_order_index(coords: [u8; 2], map_size: u8) -> usize {
    let y_rank = usize::from(map_size.saturating_sub(coords[1]));
    let x_rank = usize::from(coords[0].saturating_sub(1));
    y_rank * usize::from(map_size) + x_rank
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
    use crate::app::state::DashApp;
    use crate::layout::dashboard_layout;
    use crate::theme;
    use nc_data::{GameStateBuilder, IntelTier};
    use nc_ui::{PlayfieldBuffer, ScreenGeometry};
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

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

    #[test]
    fn planet_jump_moves_in_wrapped_screen_order() {
        let worlds = vec![make_world([2, 5]), make_world([4, 4]), make_world([1, 1])];

        assert_eq!(
            jump_planet_target_coords(5, &worlds, [1, 5], PlanetJumpDirection::Forward),
            Some([2, 5])
        );
        assert_eq!(
            jump_planet_target_coords(5, &worlds, [2, 5], PlanetJumpDirection::Forward),
            Some([4, 4])
        );
        assert_eq!(
            jump_planet_target_coords(5, &worlds, [4, 4], PlanetJumpDirection::Backward),
            Some([2, 5])
        );
        assert_eq!(
            jump_planet_target_coords(5, &worlds, [5, 1], PlanetJumpDirection::Forward),
            Some([2, 5])
        );
        assert_eq!(
            jump_planet_target_coords(5, &worlds, [1, 5], PlanetJumpDirection::Backward),
            Some([1, 1])
        );
    }

    #[test]
    fn planet_jump_stays_on_single_world() {
        let worlds = vec![make_world([3, 3])];

        assert_eq!(
            jump_planet_target_coords(5, &worlds, [3, 3], PlanetJumpDirection::Forward),
            Some([3, 3])
        );
        assert_eq!(
            jump_planet_target_coords(5, &worlds, [1, 1], PlanetJumpDirection::Backward),
            Some([3, 3])
        );
    }

    #[test]
    fn draw_bottom_map_row_matches_current_padding_mode() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 40),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.crosshair_x = 2;
        app.crosshair_y = 3;
        let widgets = dashboard_layout(&app).widgets;
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            theme::body_style(),
        );

        draw(&mut buffer, &app, widgets.center_map);

        assert!(
            !buffer
                .plain_line(widgets.center_map.bottom_pad_row)
                .is_empty()
        );
    }

    #[test]
    fn readable_mode_uses_full_widget_for_projected_map_block() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.crosshair_x = 2;
        app.crosshair_y = 3;
        let widgets = dashboard_layout(&app).widgets;
        let layout = dashboard_layout(&app);
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            theme::body_style(),
        );

        draw(&mut buffer, &app, widgets.center_map);

        let axis_line = buffer.plain_line(widgets.center_map.axis_row);

        assert!(layout.frame.width() < app.geometry.width());
        assert_eq!(widgets.center_map.map_block, widgets.center_map.outer);
        assert_eq!(
            widgets.center_map.axis_row,
            widgets.center_map.map_block.row
        );
        assert_eq!(
            widgets.center_map.grid.col,
            widgets.center_map.map_block.col
        );
        assert_eq!(
            widgets.center_map.bottom_pad_row,
            widgets.center_map.map_block.last_row()
        );
        assert!(axis_line.contains("01"));
        assert!(axis_line.contains("18"));
    }

    #[test]
    fn projected_geometry_fills_grid_and_follows_zoomed_cursor() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.crosshair_x = 10;
        app.crosshair_y = 11;
        app.map_zoom_level = 1;
        let frame = dashboard_layout(&app).widgets.center_map;
        let projected = projected_map_geometry(&app, frame, 18);

        assert!(projected.x_min <= app.crosshair_x && projected.x_max >= app.crosshair_x);
        assert!(projected.y_min <= app.crosshair_y && projected.y_max >= app.crosshair_y);
        assert_eq!(
            projected.col_edges.first().copied(),
            Some(frame.grid.col + frame.row_label_cols)
        );
        assert!(projected.col_edges.last().copied().unwrap_or(0) <= frame.grid.last_col() + 1);
        assert_eq!(projected.row_edges.first().copied(), Some(frame.grid.row));
        assert!(projected.row_edges.last().copied().unwrap_or(0) <= frame.grid.last_row() + 1);
        assert_eq!(projected.visible_x, 9);
        assert_eq!(projected.visible_y, 9);
        assert!(projected.tile_width >= 4);
        assert!(projected.tile_height >= 2);
        assert_eq!(
            projected.col_edges.last().copied(),
            Some(frame.grid.last_col() + 1)
        );
        assert_eq!(
            projected.row_edges.last().copied(),
            Some(frame.grid.last_row() + 1)
        );
    }

    #[test]
    fn readable_mode_uses_full_map_fit_inside_shrunk_dashboard_frame() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.crosshair_x = 10;
        app.crosshair_y = 11;
        let frame = dashboard_layout(&app).widgets.center_map;
        let projected = projected_map_geometry(&app, frame, 18);

        assert_eq!(projected.x_min, 1);
        assert_eq!(projected.y_min, 1);
        assert_eq!(projected.x_max, 18);
        assert_eq!(projected.y_max, 18);
        assert_eq!(projected.visible_x, 18);
        assert_eq!(projected.visible_y, 18);
        assert!(projected.tile_width >= 4);
        assert!(projected.tile_height >= 1);
        assert_eq!(
            projected.col_edges.first().copied(),
            Some(frame.grid.col + frame.row_label_cols)
        );
        assert_eq!(
            projected.col_edges.last().copied(),
            Some(frame.grid.last_col() + 1)
        );
        assert_eq!(projected.row_edges.first().copied(), Some(frame.grid.row));
        assert_eq!(
            projected.row_edges.last().copied(),
            Some(frame.grid.last_row() + 1)
        );
    }

    #[test]
    fn fill_mode_projects_into_full_center_widget() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.map_view_mode = crate::app::state::MapViewMode::Fill;
        let frame = dashboard_layout(&app).widgets.center_map;

        assert_eq!(frame.map_block, frame.outer);
        assert_eq!(frame.axis_row, frame.outer.row);
        assert_eq!(frame.grid.col, frame.outer.col);
        assert_eq!(frame.bottom_pad_row, frame.outer.last_row());
    }

    #[test]
    fn crosshair_uses_line_glyphs_on_empty_sectors() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.crosshair_x = 4;
        app.crosshair_y = 5;
        let frame = dashboard_layout(&app).widgets.center_map;
        let projected = projected_map_geometry(&app, frame, 18);
        let rect = projected
            .sector_rect([app.crosshair_x, app.crosshair_y])
            .expect("crosshair rect");
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            theme::body_style(),
        );

        draw(&mut buffer, &app, frame);

        let mid_row = rect.row + rect.height / 2;
        let mid_col = rect.col + rect.width / 2;
        assert_eq!(buffer.row(mid_row)[mid_col].ch, '+');
        if rect.width > 1 {
            assert_eq!(buffer.row(mid_row)[rect.col].ch, '-');
        }
        if rect.height > 1 {
            assert_eq!(buffer.row(rect.row)[mid_col].ch, '|');
        }
        assert_eq!(buffer.row(rect.row)[rect.col].ch, ' ');
    }

    fn make_world(coords: [u8; 2]) -> PlayerStarmapWorld {
        PlayerStarmapWorld {
            planet_record_index_1_based: 1,
            coords,
            intel_tier: IntelTier::Unknown,
            known_name: None,
            known_owner_empire_id: None,
            known_owner_empire_name: None,
            known_potential_production: None,
            known_armies: None,
            known_ground_batteries: None,
            known_starbase_count: None,
            known_current_production: None,
            known_stored_points: None,
            known_docked_summary: None,
            known_orbit_summary: None,
        }
    }
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
