//! P overlay: dashboard-sized planet management table.

use std::cmp::Ordering;

use nc_data::{
    build_capacity, yearly_growth_delta, yearly_tax_revenue, PlanetRecord, ProductionItemKind,
    STARDOCK_SLOT_COUNT,
};
use nc_engine::BUILD_UNITS;
use nc_ui::coords::{format_sector_coords_default, format_sector_coords_table};
use nc_ui::modal::Rect;
use nc_ui::table::{
    centered_table_start_col, resolve_table_columns, table_render_width, write_split_table_at,
    write_stacked_table_window_with_theme_at, SplitTableRow, TableColumn, TableFooter,
    TableWidthMode, TABLE_TEXT_INSET,
};
use nc_ui::table_selection;
use nc_ui::PlayfieldBuffer;

use crate::app::state::{
    ActiveOverlay, DashApp, PlanetOverlayFilter, PlanetOverlayPromptMode, PlanetOverlaySort,
    SortDirection,
};
use crate::layout::dashboard;
use crate::layout::MapWidgetFrame;
use crate::overlays::frame::{
    assert_overlay_body_write_fits, dashboard_overlay_parent_rect,
    draw_overlay_frame_for_body_in_parent_with_policy_and_origin, max_overlay_body_width,
    overlay_popup_rect_for_body_in_parent, stacked_table_body_height, standard_table_body_height,
    write_clipped, OverlaySizePolicy,
};
use crate::theme;

pub(crate) const HOTKEYS: &str = "? F S B <Q>";
pub(crate) const SORT_HOTKEYS: &str = "? C L M <Q>";
pub(crate) const FILTER_HOTKEYS: &str = "? A R S T <Q>";
const TOP_HEADERS: [&str; 13] = [
    "Coord", "", "Max", "Curr", "Trsry", "", "", "", "Build", "Star", "", "", "",
];
const COLUMNS: [TableColumn<'static>; 13] = [
    TableColumn::left("(XX,YY)", 7),
    TableColumn::left("Planet Name", 13),
    TableColumn::right("Prod", 4),
    TableColumn::right("Prod", 4),
    TableColumn::right("Points", 6),
    TableColumn::right("Bdgt", 5),
    TableColumn::right("Rev", 3),
    TableColumn::right("Grow", 4),
    TableColumn::right("Queue", 5),
    TableColumn::right("Dock", 4),
    TableColumn::right("SBs", 3),
    TableColumn::right("ARs", 3),
    TableColumn::right("GBs", 3),
];

fn overlay_parent_rect(app: &DashApp) -> Rect {
    dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlanetOverlayRow {
    pub planet_record_index_1_based: usize,
    pub coords: [u8; 2],
    pub current_prod: u16,
    pub max_prod: u16,
    pub has_starbase: bool,
    pub docked: u32,
    pub cells: Vec<String>,
}

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, map_frame: MapWidgetFrame) {
    match app.planet_overlay.prompt_mode {
        PlanetOverlayPromptMode::BuildSpecify => {
            draw_build_specify(buf, app, map_frame);
            return;
        }
        PlanetOverlayPromptMode::BuildQuantity => {
            draw_build_quantity(buf, app, map_frame);
            return;
        }
        _ => {}
    }
    let rows = table_rows(app);
    let selected = app
        .planet_overlay
        .selected
        .min(rows.len().saturating_sub(1));
    let selected_default = rows
        .get(selected)
        .map(|row| format_sector_coords_default(row.coords));
    let title = overlay_title(app);
    let sort_footer_label = sort_footer_label(app);
    let footer = match app.planet_overlay.prompt_mode {
        PlanetOverlayPromptMode::None => TableFooter::CommandBar {
            hotkeys_markup: HOTKEYS,
            default: selected_default.as_deref(),
            input: &app.planet_overlay.jump_input,
        },
        PlanetOverlayPromptMode::SortMenu => TableFooter::LabeledCommandBar {
            label: &sort_footer_label,
            hotkeys_markup: SORT_HOTKEYS,
            default: None,
            input: "",
        },
        PlanetOverlayPromptMode::FilterMenu => TableFooter::LabeledCommandBar {
            label: "FILTER",
            hotkeys_markup: FILTER_HOTKEYS,
            default: None,
            input: "",
        },
        PlanetOverlayPromptMode::FilterRangeCoords => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Range from ",
            default: &app.planet_overlay.prompt_default,
            input: &app.planet_overlay.prompt_input,
        },
        PlanetOverlayPromptMode::FilterRangeDistance => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Range radius ",
            default: &app.planet_overlay.prompt_default,
            input: &app.planet_overlay.prompt_input,
        },
        PlanetOverlayPromptMode::BuildSpecify | PlanetOverlayPromptMode::BuildQuantity => {
            unreachable!("build flows render separately")
        }
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
    let body_width = table_render_width(&columns)
        .max("You do not currently control any planets.".chars().count() + 4);
    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        overlay_parent_rect(app),
        &title,
        body_width,
        stacked_table_body_height(natural_visible_rows),
        OverlaySizePolicy::default(),
        footer,
        app.overlay_position_for(ActiveOverlay::PlanetList),
    );
    let visible_rows = frame.body_height.saturating_sub(5);
    assert_overlay_body_write_fits(
        frame,
        &title,
        table_render_width(&columns),
        stacked_table_body_height(visible_rows),
    );
    let scroll = clamp_scroll(
        app.planet_overlay.scroll,
        selected,
        visible_rows,
        rows.len(),
    );
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
            "You do not currently control any planets.",
            theme::dim_style(),
        );
    }
}

pub(crate) fn popup_rect(app: &DashApp, map_frame: MapWidgetFrame) -> Option<Rect> {
    match app.planet_overlay.prompt_mode {
        PlanetOverlayPromptMode::BuildSpecify => {
            return Some(build_specify_popup_rect(app));
        }
        PlanetOverlayPromptMode::BuildQuantity => {
            return Some(build_quantity_popup_rect(app));
        }
        _ => {}
    }
    let rows = table_rows(app);
    let selected = app
        .planet_overlay
        .selected
        .min(rows.len().saturating_sub(1));
    let selected_default = rows
        .get(selected)
        .map(|row| format_sector_coords_default(row.coords));
    let title = overlay_title(app);
    let sort_footer_label = sort_footer_label(app);
    let footer = match app.planet_overlay.prompt_mode {
        PlanetOverlayPromptMode::None => TableFooter::CommandBar {
            hotkeys_markup: HOTKEYS,
            default: selected_default.as_deref(),
            input: &app.planet_overlay.jump_input,
        },
        PlanetOverlayPromptMode::SortMenu => TableFooter::LabeledCommandBar {
            label: &sort_footer_label,
            hotkeys_markup: SORT_HOTKEYS,
            default: None,
            input: "",
        },
        PlanetOverlayPromptMode::FilterMenu => TableFooter::LabeledCommandBar {
            label: "FILTER",
            hotkeys_markup: FILTER_HOTKEYS,
            default: None,
            input: "",
        },
        PlanetOverlayPromptMode::FilterRangeCoords => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Range from ",
            default: &app.planet_overlay.prompt_default,
            input: &app.planet_overlay.prompt_input,
        },
        PlanetOverlayPromptMode::FilterRangeDistance => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Range radius ",
            default: &app.planet_overlay.prompt_default,
            input: &app.planet_overlay.prompt_input,
        },
        PlanetOverlayPromptMode::BuildSpecify | PlanetOverlayPromptMode::BuildQuantity => {
            unreachable!("build flows are not draggable")
        }
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
    let body_width = table_render_width(&columns)
        .max("You do not currently control any planets.".chars().count() + 4);
    Some(overlay_popup_rect_for_body_in_parent(
        overlay_parent_rect(app),
        &title,
        body_width,
        stacked_table_body_height(natural_visible_rows),
        OverlaySizePolicy::default(),
        footer,
        app.overlay_position_for(ActiveOverlay::PlanetList),
    ))
}

fn draw_build_specify(buf: &mut PlayfieldBuffer, app: &DashApp, _map_frame: MapWidgetFrame) {
    let view = app.planet_build_view();
    let entries = app.planet_build_specify_entries();
    let split_rows = build_specify_split_rows(&entries);
    let table_width = build_specify_table_width();
    let status_rows = usize::from(app.planet_overlay.build_unit_status.is_some());
    let max_unit_num = app.planet_build_max_selectable_unit_number();
    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        overlay_parent_rect(app),
        "SPECIFY BUILD ORDERS",
        table_width,
        1 + standard_table_body_height(split_rows.len()) + status_rows,
        OverlaySizePolicy::default(),
        TableFooter::CommandInput {
            label: "COMMAND",
            prompt: &format!("Unit number or 0 if done (0 - {}) ", max_unit_num),
            default: "0",
            input: &app.planet_overlay.build_unit_input,
        },
        app.overlay_position_for(ActiveOverlay::PlanetList),
    );
    assert_overlay_body_write_fits(
        frame,
        "SPECIFY BUILD ORDERS",
        table_width,
        1 + standard_table_body_height(split_rows.len()) + status_rows,
    );

    let Some(view) = view else {
        write_clipped(
            buf,
            frame.body_row,
            frame.body_col,
            frame.body_width,
            "No owned planets available for building.",
            theme::dim_style(),
        );
        return;
    };

    write_build_points_line(buf, frame, view.points_left, table_width);
    let table_col =
        frame.body_col + centered_table_start_col(frame.body_width, &build_specify_all_columns());
    let _ = write_split_table_at(
        buf,
        frame.body_row + 1,
        table_col,
        &BUILD_HALF_COLUMNS,
        &BUILD_HALF_COLUMNS,
        &split_rows,
        theme::value_style(),
    );
    if let Some(status) = app.planet_overlay.build_unit_status.as_deref() {
        write_clipped(
            buf,
            frame.body_row + 1 + standard_table_body_height(split_rows.len()),
            frame.body_col,
            frame.body_width,
            status,
            theme::error_style(),
        );
    }
}

fn draw_build_quantity(buf: &mut PlayfieldBuffer, app: &DashApp, map_frame: MapWidgetFrame) {
    let view = app.planet_build_view();
    let Some(kind) = app.planet_overlay.build_selected_kind else {
        draw_build_specify(buf, app, map_frame);
        return;
    };
    let Some(unit) = BUILD_UNITS.iter().copied().find(|unit| unit.kind == kind) else {
        draw_build_specify(buf, app, map_frame);
        return;
    };
    let max_qty = app.planet_build_max_quantity_for(kind).unwrap_or(0);
    let entries = app.planet_build_specify_entries();
    let split_rows = build_specify_split_rows(&entries);
    let table_width = build_specify_table_width();
    let status_rows = usize::from(app.planet_overlay.build_quantity_status.is_some());
    let default_qty = max_qty.to_string();
    let prompt = format!(
        "How many new {} to build (0 - {}) ",
        unit.singular_label, max_qty
    );
    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        overlay_parent_rect(app),
        "BUILD QUANTITY",
        table_width,
        1 + standard_table_body_height(split_rows.len()) + status_rows,
        OverlaySizePolicy::default(),
        TableFooter::CommandInput {
            label: "COMMAND",
            prompt: &prompt,
            default: &default_qty,
            input: &app.planet_overlay.build_quantity_input,
        },
        app.overlay_position_for(ActiveOverlay::PlanetList),
    );
    assert_overlay_body_write_fits(
        frame,
        "BUILD QUANTITY",
        table_width,
        1 + standard_table_body_height(split_rows.len()) + status_rows,
    );

    let Some(view) = view else {
        write_clipped(
            buf,
            frame.body_row,
            frame.body_col,
            frame.body_width,
            "No owned planets available for building.",
            theme::dim_style(),
        );
        return;
    };

    write_build_points_line(buf, frame, view.points_left, table_width);
    let table_col =
        frame.body_col + centered_table_start_col(frame.body_width, &build_specify_all_columns());
    let _ = write_split_table_at(
        buf,
        frame.body_row + 1,
        table_col,
        &BUILD_HALF_COLUMNS,
        &BUILD_HALF_COLUMNS,
        &split_rows,
        theme::value_style(),
    );
    if let Some(status) = app.planet_overlay.build_quantity_status.as_deref() {
        write_clipped(
            buf,
            frame.body_row + 1 + standard_table_body_height(split_rows.len()),
            frame.body_col,
            frame.body_width,
            status,
            theme::error_style(),
        );
    }
}

fn build_specify_popup_rect(app: &DashApp) -> Rect {
    let entries = app.planet_build_specify_entries();
    let split_rows = build_specify_split_rows(&entries);
    let table_width = build_specify_table_width();
    let status_rows = usize::from(app.planet_overlay.build_unit_status.is_some());
    let max_unit_num = app.planet_build_max_selectable_unit_number();
    let prompt = format!("Unit number or 0 if done (0 - {}) ", max_unit_num);
    overlay_popup_rect_for_body_in_parent(
        overlay_parent_rect(app),
        "SPECIFY BUILD ORDERS",
        table_width,
        1 + standard_table_body_height(split_rows.len()) + status_rows,
        OverlaySizePolicy::default(),
        TableFooter::CommandInput {
            label: "COMMAND",
            prompt: &prompt,
            default: "0",
            input: &app.planet_overlay.build_unit_input,
        },
        app.overlay_position_for(ActiveOverlay::PlanetList),
    )
}

fn build_quantity_popup_rect(app: &DashApp) -> Rect {
    let Some(kind) = app.planet_overlay.build_selected_kind else {
        return build_specify_popup_rect(app);
    };
    let Some(unit) = BUILD_UNITS.iter().copied().find(|unit| unit.kind == kind) else {
        return build_specify_popup_rect(app);
    };
    let max_qty = app.planet_build_max_quantity_for(kind).unwrap_or(0);
    let entries = app.planet_build_specify_entries();
    let split_rows = build_specify_split_rows(&entries);
    let table_width = build_specify_table_width();
    let status_rows = usize::from(app.planet_overlay.build_quantity_status.is_some());
    let default_qty = max_qty.to_string();
    let prompt = format!(
        "How many new {} to build (0 - {}) ",
        unit.singular_label, max_qty
    );
    overlay_popup_rect_for_body_in_parent(
        overlay_parent_rect(app),
        "BUILD QUANTITY",
        table_width,
        1 + standard_table_body_height(split_rows.len()) + status_rows,
        OverlaySizePolicy::default(),
        TableFooter::CommandInput {
            label: "COMMAND",
            prompt: &prompt,
            default: &default_qty,
            input: &app.planet_overlay.build_quantity_input,
        },
        app.overlay_position_for(ActiveOverlay::PlanetList),
    )
}

const BUILD_HALF_COLUMNS: [TableColumn<'static>; 4] = [
    TableColumn::left("NO.", 4),
    TableColumn::left("UNIT TYPE", 19),
    TableColumn::right("COST", 4),
    TableColumn::right("QTY.", 5),
];

fn build_specify_all_columns() -> [TableColumn<'static>; 8] {
    [
        BUILD_HALF_COLUMNS[0],
        BUILD_HALF_COLUMNS[1],
        BUILD_HALF_COLUMNS[2],
        BUILD_HALF_COLUMNS[3],
        BUILD_HALF_COLUMNS[0],
        BUILD_HALF_COLUMNS[1],
        BUILD_HALF_COLUMNS[2],
        BUILD_HALF_COLUMNS[3],
    ]
}

fn build_specify_table_width() -> usize {
    table_render_width(&build_specify_all_columns())
}

fn write_build_points_line(
    buf: &mut PlayfieldBuffer,
    frame: crate::overlays::frame::OverlayFrame,
    points_left: u32,
    table_width: usize,
) {
    let table_col =
        frame.body_col + centered_table_start_col(frame.body_width, &build_specify_all_columns());
    let points_left_label = format!("BUDGET: {}", points_left);
    let points_left_col = table_col + table_width - TABLE_TEXT_INSET - points_left_label.len();
    write_clipped(
        buf,
        frame.body_row,
        points_left_col,
        frame
            .body_width
            .saturating_sub(points_left_col.saturating_sub(frame.body_col)),
        &points_left_label,
        theme::title_style(),
    );
}

fn build_specify_split_rows(entries: &[nc_engine::PlanetBuildSpecifyEntry]) -> Vec<SplitTableRow> {
    let left_units = [0usize, 1, 2, 3, 4];
    let right_units = [5usize, 6, 7, 8];

    (0..left_units.len())
        .map(|idx| {
            let left = entries[left_units[idx]];
            let right = right_units
                .get(idx)
                .and_then(|entry_idx| entries.get(*entry_idx).copied());
            SplitTableRow {
                left_cells: build_specify_cells(left),
                right_cells: right.map(build_specify_cells).unwrap_or_else(|| {
                    vec![String::new(), String::new(), String::new(), String::new()]
                }),
            }
        })
        .collect()
}

fn build_specify_cells(entry: nc_engine::PlanetBuildSpecifyEntry) -> Vec<String> {
    vec![
        if entry.selectable {
            format!("<{:02}>", entry.number)
        } else {
            String::new()
        },
        entry.label.to_string(),
        format!("{:02}", entry.cost),
        format!("({})", entry.queued_qty),
    ]
}

pub(crate) fn selection_rows(app: &DashApp) -> Vec<Vec<String>> {
    table_rows(app)
        .into_iter()
        .map(|row| vec![format_sector_coords_table(row.coords)])
        .collect()
}

pub(crate) fn table_rows(app: &DashApp) -> Vec<PlanetOverlayRow> {
    let owner_slot = app.player_record_index_1_based as u8;
    let player_tax_rate = app
        .game_data
        .player
        .records
        .get(app.player_record_index_1_based.saturating_sub(1))
        .map(|player| player.tax_rate())
        .unwrap_or(0);
    let starbase_coords = app
        .game_data
        .bases
        .records
        .iter()
        .filter(|base| base.owner_empire_raw() == owner_slot && base.active_flag_raw() != 0)
        .map(|base| base.coords_raw())
        .collect::<std::collections::BTreeSet<_>>();

    let mut rows = app
        .game_data
        .planets
        .records
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner_empire_slot_raw() == owner_slot)
        .map(|(idx, planet)| {
            format_planet_row_cells(
                idx + 1,
                planet,
                starbase_coords.contains(&planet.coords_raw()),
                player_tax_rate,
            )
        })
        .collect::<Vec<_>>();

    rows.retain(|row| match app.planet_overlay.filter {
        PlanetOverlayFilter::All => true,
        PlanetOverlayFilter::Range { anchor, radius } => {
            distance_sq(anchor, row.coords) <= u32::from(radius) * u32::from(radius)
        }
        PlanetOverlayFilter::Starbase => row.has_starbase,
        PlanetOverlayFilter::Stardock => row.docked > 0,
    });

    rows.sort_by(|left, right| match app.planet_overlay.sort {
        PlanetOverlaySort::CurrentProduction => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.current_prod.cmp(&right.current_prod),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        PlanetOverlaySort::Location => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.coords.cmp(&right.coords),
        ),
        PlanetOverlaySort::MaxProduction => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.max_prod.cmp(&right.max_prod),
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

fn format_planet_row_cells(
    planet_record_index_1_based: usize,
    planet: &PlanetRecord,
    has_starbase: bool,
    tax_rate: u8,
) -> PlanetOverlayRow {
    let coords = planet.coords_raw();
    let present = planet.present_production_points().unwrap_or(0);
    let potential = planet.potential_production_points();
    let stored = planet.stored_production_points();
    let revenue = yearly_tax_revenue(present, tax_rate);
    let budget = u32::from(build_capacity(present, has_starbase)).min(stored);
    let growth = yearly_growth_delta(present, potential, tax_rate, has_starbase) as i16;
    let queue = build_queue_total(planet);
    let docked = docked_total(planet);
    let name = planet.planet_name();

    PlanetOverlayRow {
        planet_record_index_1_based,
        coords,
        current_prod: present,
        max_prod: potential,
        has_starbase,
        docked,
        cells: vec![
            format_sector_coords_table(coords),
            truncate(&name, 13),
            potential.to_string(),
            present.to_string(),
            stored.to_string(),
            budget.to_string(),
            revenue.to_string(),
            format!("{growth:+}"),
            queue.to_string(),
            docked.to_string(),
            u8::from(has_starbase).to_string(),
            planet.army_count_raw().to_string(),
            planet.ground_batteries_raw().to_string(),
        ],
    }
}

fn build_queue_total(planet: &PlanetRecord) -> u32 {
    (0..10)
        .map(|slot| {
            let points = u32::from(planet.build_count_raw(slot));
            let kind = ProductionItemKind::from_raw(planet.build_kind_raw(slot));
            let Some(cost) = kind.build_cost() else {
                return 0;
            };
            points / cost
        })
        .sum()
}

fn docked_total(planet: &PlanetRecord) -> u32 {
    (0..STARDOCK_SLOT_COUNT)
        .map(|slot| u32::from(planet.stardock_count_raw(slot)))
        .sum()
}

fn truncate(value: &str, width: usize) -> String {
    value.chars().take(width).collect()
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
    let Some(matched) = table_selection::find_typed_jump(&rows, 0, &app.planet_overlay.jump_input)
    else {
        return false;
    };
    app.planet_overlay.selected = matched.index;
    matched.is_terminal_exact_match
}

fn distance_sq(a: [u8; 2], b: [u8; 2]) -> u32 {
    let dx = i32::from(a[0]) - i32::from(b[0]);
    let dy = i32::from(a[1]) - i32::from(b[1]);
    (dx * dx + dy * dy) as u32
}

fn overlay_title(app: &DashApp) -> String {
    format!(
        "PLANET LIST: {} {} {}",
        sort_key_label(app.planet_overlay.sort),
        app.planet_overlay.sort_direction.label(),
        filter_label(app.planet_overlay.filter)
    )
}

fn sort_footer_label(app: &DashApp) -> String {
    format!("SORT {}", app.planet_overlay.sort_direction.label())
}

fn sort_key_label(sort: PlanetOverlaySort) -> &'static str {
    match sort {
        PlanetOverlaySort::CurrentProduction => "CURR",
        PlanetOverlaySort::Location => "LOC",
        PlanetOverlaySort::MaxProduction => "MAX",
    }
}

fn filter_label(filter: crate::app::state::PlanetOverlayFilter) -> &'static str {
    match filter {
        crate::app::state::PlanetOverlayFilter::All => "ALL",
        crate::app::state::PlanetOverlayFilter::Range { .. } => "RNG",
        crate::app::state::PlanetOverlayFilter::Starbase => "SB",
        crate::app::state::PlanetOverlayFilter::Stardock => "DOCK",
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
    use super::*;
    use nc_data::{GameStateBuilder, ProductionItemKind};
    use nc_ui::ScreenGeometry;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    #[test]
    fn browse_hotkeys_match_supported_planet_list_commands() {
        assert_eq!(HOTKEYS, "? F S B <Q>");
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
        app.planet_overlay.sort_direction = SortDirection::Asc;

        assert_eq!(overlay_title(&app), "PLANET LIST: CURR ASC ALL");
        assert_eq!(sort_footer_label(&app), "SORT ASC");
    }

    #[test]
    fn build_specify_rows_keep_all_choices_and_blank_unselectable_tags() {
        let rows = build_specify_split_rows(&[
            nc_engine::PlanetBuildSpecifyEntry {
                number: 1,
                kind: ProductionItemKind::Destroyer,
                label: "Destroyers",
                cost: 5,
                queued_qty: 0,
                selectable: true,
            },
            nc_engine::PlanetBuildSpecifyEntry {
                number: 2,
                kind: ProductionItemKind::Cruiser,
                label: "Cruisers",
                cost: 15,
                queued_qty: 0,
                selectable: false,
            },
            nc_engine::PlanetBuildSpecifyEntry {
                number: 3,
                kind: ProductionItemKind::Battleship,
                label: "Battleships",
                cost: 45,
                queued_qty: 0,
                selectable: false,
            },
            nc_engine::PlanetBuildSpecifyEntry {
                number: 4,
                kind: ProductionItemKind::Scout,
                label: "Scouts",
                cost: 15,
                queued_qty: 0,
                selectable: false,
            },
            nc_engine::PlanetBuildSpecifyEntry {
                number: 5,
                kind: ProductionItemKind::Transport,
                label: "Troop transports",
                cost: 5,
                queued_qty: 0,
                selectable: true,
            },
            nc_engine::PlanetBuildSpecifyEntry {
                number: 6,
                kind: ProductionItemKind::Etac,
                label: "ETACs",
                cost: 20,
                queued_qty: 0,
                selectable: false,
            },
            nc_engine::PlanetBuildSpecifyEntry {
                number: 7,
                kind: ProductionItemKind::Starbase,
                label: "Starbases",
                cost: 50,
                queued_qty: 0,
                selectable: false,
            },
            nc_engine::PlanetBuildSpecifyEntry {
                number: 9,
                kind: ProductionItemKind::Army,
                label: "Armies",
                cost: 2,
                queued_qty: 3,
                selectable: true,
            },
            nc_engine::PlanetBuildSpecifyEntry {
                number: 10,
                kind: ProductionItemKind::GroundBattery,
                label: "Ground batteries",
                cost: 20,
                queued_qty: 1,
                selectable: false,
            },
        ]);

        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].left_cells[0], "<01>");
        assert_eq!(rows[0].right_cells[0], "");
        assert_eq!(rows[2].right_cells[0], "<09>");
        assert_eq!(rows[2].right_cells[3], "(3)");
        assert_eq!(rows[3].right_cells[0], "");
        assert_eq!(rows[4].left_cells[0], "<05>");
        assert!(rows[4].right_cells.iter().all(|cell| cell.is_empty()));
    }
}
