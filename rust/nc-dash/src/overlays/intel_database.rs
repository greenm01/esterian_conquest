//! I overlay: dashboard-sized total planet database.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use nc_data::{PlanetIntelSnapshot, build_player_starmap_projection_from_snapshots};
use nc_ui::PlayfieldBuffer;
use nc_ui::coords::{format_sector_coords_default, format_sector_coords_table};
use nc_ui::modal::Rect;
use nc_ui::table::{
    TableColumn, TableFooter, TableWidthMode, centered_table_start_col, resolve_table_columns,
    table_render_width, write_stacked_table_window_with_theme_at,
};
use nc_ui::table_selection;

use crate::app::state::{
    ActiveOverlay, DashApp, IntelOverlayFilter, IntelOverlayPromptMode, IntelOverlaySort,
    SortDirection,
};
use crate::layout::MapWidgetFrame;
use crate::overlays::frame::{
    OverlaySizePolicy, assert_overlay_body_write_fits,
    draw_overlay_frame_for_body_in_map_with_origin, max_overlay_body_width,
    overlay_popup_rect_for_body_in_map, stacked_table_body_height, write_clipped,
};
use crate::theme;

pub(crate) const HOTKEYS: &str = "? F S <Q>";
pub(crate) const SORT_HOTKEYS: &str = "? L R E M <Q>";
pub(crate) const FILTER_HOTKEYS: &str = "? A R E M <Q>";
const TOP_HEADERS: [&str; 11] = ["Coord", "", "", "", "", "", "", "", "Curr", "", ""];
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IntelOverlayRow {
    pub planet_record_index_1_based: usize,
    pub coords: [u8; 2],
    pub known_owner_empire_id: Option<u8>,
    pub known_max_production: Option<u16>,
    pub cells: Vec<String>,
}

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, map_frame: MapWidgetFrame) {
    let rows = table_rows(app);
    let selected = app.intel_overlay.selected.min(rows.len().saturating_sub(1));
    let selected_default = rows
        .get(selected)
        .map(|row| format_sector_coords_default(row.coords));
    let title = overlay_title(app);
    let sort_footer_label = sort_footer_label(app);
    let footer = match app.intel_overlay.prompt_mode {
        IntelOverlayPromptMode::None => TableFooter::CommandBar {
            hotkeys_markup: HOTKEYS,
            default: selected_default.as_deref(),
            input: &app.intel_overlay.jump_input,
        },
        IntelOverlayPromptMode::SortMenu => TableFooter::LabeledCommandBar {
            label: &sort_footer_label,
            hotkeys_markup: SORT_HOTKEYS,
            default: None,
            input: "",
        },
        IntelOverlayPromptMode::SortRangeInput => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Sort range from ",
            default: &app.intel_overlay.prompt_default,
            input: &app.intel_overlay.prompt_input,
        },
        IntelOverlayPromptMode::FilterMenu => TableFooter::LabeledCommandBar {
            label: "FILTER",
            hotkeys_markup: FILTER_HOTKEYS,
            default: None,
            input: "",
        },
        IntelOverlayPromptMode::FilterRangeCoords => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Range from ",
            default: &app.intel_overlay.prompt_default,
            input: &app.intel_overlay.prompt_input,
        },
        IntelOverlayPromptMode::FilterRangeDistance => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Range radius ",
            default: &app.intel_overlay.prompt_default,
            input: &app.intel_overlay.prompt_input,
        },
        IntelOverlayPromptMode::FilterEmpireInput => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Empire ",
            default: &app.intel_overlay.prompt_default,
            input: &app.intel_overlay.prompt_input,
        },
        IntelOverlayPromptMode::FilterMaxProductionInput => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Max production at least ",
            default: &app.intel_overlay.prompt_default,
            input: &app.intel_overlay.prompt_input,
        },
    };
    let table_cells = rows.iter().map(|row| row.cells.clone()).collect::<Vec<_>>();
    let natural_visible_rows = table_cells.len().max(1);
    let columns = resolve_table_columns(
        &COLUMNS,
        &table_cells,
        max_overlay_body_width(map_frame),
        false,
        TableWidthMode::Compact,
    );
    let body_width =
        table_render_width(&columns).max("No planet intel is available yet.".chars().count() + 4);
    let frame = draw_overlay_frame_for_body_in_map_with_origin(
        buf,
        map_frame,
        &title,
        body_width,
        stacked_table_body_height(natural_visible_rows),
        footer,
        app.overlay_position_for(ActiveOverlay::IntelDatabase),
    );
    let visible_rows = frame.body_height.saturating_sub(5);
    assert_overlay_body_write_fits(
        frame,
        &title,
        table_render_width(&columns),
        stacked_table_body_height(visible_rows),
    );
    let scroll = clamp_scroll(app.intel_overlay.scroll, selected, visible_rows, rows.len());
    let table_col = frame.body_col + centered_table_start_col(frame.body_width, &columns);
    let metrics = write_stacked_table_window_with_theme_at(
        buf,
        frame.body_row,
        table_col,
        &TOP_HEADERS,
        &columns,
        &table_cells,
        scroll,
        visible_rows,
        theme::table_theme(),
        table_cells.get(selected).map(|_| selected),
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

pub(crate) fn popup_rect(app: &DashApp, map_frame: MapWidgetFrame) -> Rect {
    let rows = table_rows(app);
    let selected = app.intel_overlay.selected.min(rows.len().saturating_sub(1));
    let selected_default = rows
        .get(selected)
        .map(|row| format_sector_coords_default(row.coords));
    let title = overlay_title(app);
    let sort_footer_label = sort_footer_label(app);
    let footer = match app.intel_overlay.prompt_mode {
        IntelOverlayPromptMode::None => TableFooter::CommandBar {
            hotkeys_markup: HOTKEYS,
            default: selected_default.as_deref(),
            input: &app.intel_overlay.jump_input,
        },
        IntelOverlayPromptMode::SortMenu => TableFooter::LabeledCommandBar {
            label: &sort_footer_label,
            hotkeys_markup: SORT_HOTKEYS,
            default: None,
            input: "",
        },
        IntelOverlayPromptMode::SortRangeInput => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Sort range from ",
            default: &app.intel_overlay.prompt_default,
            input: &app.intel_overlay.prompt_input,
        },
        IntelOverlayPromptMode::FilterMenu => TableFooter::LabeledCommandBar {
            label: "FILTER",
            hotkeys_markup: FILTER_HOTKEYS,
            default: None,
            input: "",
        },
        IntelOverlayPromptMode::FilterRangeCoords => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Range from ",
            default: &app.intel_overlay.prompt_default,
            input: &app.intel_overlay.prompt_input,
        },
        IntelOverlayPromptMode::FilterRangeDistance => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Range radius ",
            default: &app.intel_overlay.prompt_default,
            input: &app.intel_overlay.prompt_input,
        },
        IntelOverlayPromptMode::FilterEmpireInput => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Empire ",
            default: &app.intel_overlay.prompt_default,
            input: &app.intel_overlay.prompt_input,
        },
        IntelOverlayPromptMode::FilterMaxProductionInput => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Max production at least ",
            default: &app.intel_overlay.prompt_default,
            input: &app.intel_overlay.prompt_input,
        },
    };
    let table_cells = rows.iter().map(|row| row.cells.clone()).collect::<Vec<_>>();
    let natural_visible_rows = table_cells.len().max(1);
    let columns = resolve_table_columns(
        &COLUMNS,
        &table_cells,
        max_overlay_body_width(map_frame),
        false,
        TableWidthMode::Compact,
    );
    let body_width =
        table_render_width(&columns).max("No planet intel is available yet.".chars().count() + 4);
    overlay_popup_rect_for_body_in_map(
        map_frame,
        &title,
        body_width,
        stacked_table_body_height(natural_visible_rows),
        OverlaySizePolicy::default(),
        footer,
        app.overlay_position_for(ActiveOverlay::IntelDatabase),
    )
}

pub(crate) fn selection_rows(app: &DashApp) -> Vec<Vec<String>> {
    table_rows(app)
        .into_iter()
        .map(|row| vec![format_sector_coords_table(row.coords)])
        .collect()
}

pub(crate) fn table_rows(app: &DashApp) -> Vec<IntelOverlayRow> {
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
    let mut rows = projection
        .worlds
        .iter()
        .map(|world| {
            let snapshot = snapshot_map.get(&world.planet_record_index_1_based);
            format_intel_row(app, world, snapshot)
        })
        .collect::<Vec<_>>();

    rows.retain(|row| match app.intel_overlay.filter {
        IntelOverlayFilter::All => true,
        IntelOverlayFilter::Range { anchor, radius } => {
            distance_sq(anchor, row.coords) <= u32::from(radius) * u32::from(radius)
        }
        IntelOverlayFilter::Empire(empire_id) => row.known_owner_empire_id == Some(empire_id),
        IntelOverlayFilter::MaxProduction(min_prod) => row
            .known_max_production
            .is_some_and(|value| value >= min_prod),
    });

    rows.sort_by(|left, right| match app.intel_overlay.sort {
        IntelOverlaySort::Location => apply_sort_direction(
            app.intel_overlay.sort_direction,
            left.coords.cmp(&right.coords),
        ),
        IntelOverlaySort::Range(anchor) => apply_sort_direction(
            app.intel_overlay.sort_direction,
            distance_sq(anchor, left.coords).cmp(&distance_sq(anchor, right.coords)),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        IntelOverlaySort::Empire => apply_sort_direction(
            app.intel_overlay.sort_direction,
            left.known_owner_empire_id.cmp(&right.known_owner_empire_id),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        IntelOverlaySort::MaxProduction => apply_sort_direction(
            app.intel_overlay.sort_direction,
            left.known_max_production.cmp(&right.known_max_production),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
    });

    rows
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
) -> IntelOverlayRow {
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
    IntelOverlayRow {
        planet_record_index_1_based: world.planet_record_index_1_based,
        coords,
        known_owner_empire_id: world.known_owner_empire_id,
        known_max_production: world.known_potential_production,
        cells: vec![
            format_sector_coords_table(coords),
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
        ],
    }
}

pub(crate) fn parse_coords_input(input: &str, default: [u8; 2]) -> Option<[u8; 2]> {
    if input.trim().is_empty() {
        return Some(default);
    }
    let digits = input
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let [x, y] = digits.as_slice() else {
        return None;
    };
    Some([x.parse().ok()?, y.parse().ok()?])
}

pub(crate) fn sync_cursor_to_jump_input(app: &mut DashApp) -> bool {
    let rows = selection_rows(app);
    let Some(matched) = table_selection::find_typed_jump(&rows, 0, &app.intel_overlay.jump_input)
    else {
        return false;
    };
    app.intel_overlay.selected = matched.index;
    matched.is_terminal_exact_match
}

fn distance_sq(a: [u8; 2], b: [u8; 2]) -> u32 {
    let dx = i32::from(a[0]) - i32::from(b[0]);
    let dy = i32::from(a[1]) - i32::from(b[1]);
    (dx * dx + dy * dy) as u32
}

fn overlay_title(app: &DashApp) -> String {
    format!(
        "TOTAL PLANET DATABASE: {} {} {}",
        sort_key_label(app.intel_overlay.sort),
        app.intel_overlay.sort_direction.label(),
        filter_label(app.intel_overlay.filter)
    )
}

fn sort_footer_label(app: &DashApp) -> String {
    format!("SORT {}", app.intel_overlay.sort_direction.label())
}

fn sort_key_label(sort: IntelOverlaySort) -> &'static str {
    match sort {
        IntelOverlaySort::Location => "LOC",
        IntelOverlaySort::Range(_) => "RNG",
        IntelOverlaySort::Empire => "EMP",
        IntelOverlaySort::MaxProduction => "MAX",
    }
}

fn filter_label(filter: crate::app::state::IntelOverlayFilter) -> &'static str {
    match filter {
        crate::app::state::IntelOverlayFilter::All => "ALL",
        crate::app::state::IntelOverlayFilter::Range { .. } => "RNG",
        crate::app::state::IntelOverlayFilter::Empire(_) => "EMP",
        crate::app::state::IntelOverlayFilter::MaxProduction(_) => "MAX",
    }
}

fn apply_sort_direction(direction: SortDirection, ordering: Ordering) -> Ordering {
    match direction {
        SortDirection::Asc => ordering,
        SortDirection::Desc => ordering.reverse(),
    }
}

#[cfg(test)]
mod tests {
    use super::{HOTKEYS, overlay_title, parse_coords_input, sort_footer_label};
    use crate::app::state::{DashApp, SortDirection};
    use nc_data::GameStateBuilder;
    use nc_ui::ScreenGeometry;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    #[test]
    fn browse_hotkeys_match_supported_intel_commands() {
        assert_eq!(HOTKEYS, "? F S <Q>");
    }

    #[test]
    fn parse_coords_input_accepts_rendered_coord_cell() {
        assert_eq!(parse_coords_input("(02,03)", [1, 1]), Some([2, 3]));
    }

    #[test]
    fn titles_and_sort_footer_show_direction() {
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
        app.intel_overlay.sort_direction = SortDirection::Desc;

        assert_eq!(overlay_title(&app), "TOTAL PLANET DATABASE: LOC DESC ALL");
        assert_eq!(sort_footer_label(&app), "SORT DESC");
    }
}
