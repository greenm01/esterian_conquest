//! P overlay: dashboard-sized planet management table.

use nc_data::{
    PlanetRecord, ProductionItemKind, STARDOCK_SLOT_COUNT, yearly_growth_delta, yearly_tax_revenue,
};
use nc_ui::PlayfieldBuffer;
use nc_ui::coords::{format_sector_coords_default, format_sector_coords_table};
use nc_ui::table::{
    TableColumn, TableFooter, TableWidthMode, centered_table_start_col, resolve_table_columns,
    table_render_width, write_stacked_table_window_with_theme_at,
};
use nc_ui::table_selection;

use crate::app::state::{DashApp, PlanetOverlayFilter, PlanetOverlayPromptMode, PlanetOverlaySort};
use crate::overlays::frame::{draw_overlay_frame_for_body, write_clipped};
use crate::theme;

pub(crate) const HOTKEYS: &str = "? F S B A C L U X I T <Q>";
pub(crate) const SORT_HOTKEYS: &str = "? C L M <Q>";
pub(crate) const FILTER_HOTKEYS: &str = "? A R S T <Q>";
const TOP_HEADERS: [&str; 12] = [
    "Coord", "", "Max", "Curr", "Stored", "", "", "Build", "Star", "", "", "",
];
const COLUMNS: [TableColumn<'static>; 12] = [
    TableColumn::left("(XX,YY)", 7),
    TableColumn::left("Planet Name", 13),
    TableColumn::right("Prod", 4),
    TableColumn::right("Prod", 4),
    TableColumn::right("Points", 6),
    TableColumn::right("Rev", 3),
    TableColumn::right("Grow", 4),
    TableColumn::right("Queue", 5),
    TableColumn::right("Dock", 4),
    TableColumn::right("SBs", 3),
    TableColumn::right("ARs", 3),
    TableColumn::right("GBs", 3),
];

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

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let rows = table_rows(app);
    let selected = app
        .planet_overlay
        .selected
        .min(rows.len().saturating_sub(1));
    let selected_default = rows
        .get(selected)
        .map(|row| format_sector_coords_default(row.coords));
    let footer = match app.planet_overlay.prompt_mode {
        PlanetOverlayPromptMode::None => TableFooter::CommandBar {
            hotkeys_markup: HOTKEYS,
            default: selected_default.as_deref(),
            input: &app.planet_overlay.jump_input,
        },
        PlanetOverlayPromptMode::SortMenu => TableFooter::LabeledCommandBar {
            label: "SORT",
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
    };
    let table_cells = rows.iter().map(|row| row.cells.clone()).collect::<Vec<_>>();
    let desired_visible_rows = table_cells.len().clamp(1, buf.height().saturating_sub(11));
    let columns = resolve_table_columns(
        &COLUMNS,
        &table_cells,
        buf.width().saturating_sub(12),
        false,
        TableWidthMode::Compact,
    );
    let body_width = table_render_width(&columns)
        .max("You do not currently control any planets.".chars().count() + 4);
    let frame = draw_overlay_frame_for_body(
        buf,
        "PLANET LIST",
        body_width,
        desired_visible_rows + 5,
        footer,
    );
    let visible_rows = frame.body_height.saturating_sub(5);
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
        PlanetOverlaySort::CurrentProduction => right
            .current_prod
            .cmp(&left.current_prod)
            .then_with(|| left.coords.cmp(&right.coords)),
        PlanetOverlaySort::Location => left.coords.cmp(&right.coords),
        PlanetOverlaySort::MaxProduction => right
            .max_prod
            .cmp(&left.max_prod)
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
