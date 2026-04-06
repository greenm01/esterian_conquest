//! P overlay: dashboard-sized planet management table.

use nc_data::{
    PlanetRecord, ProductionItemKind, STARDOCK_SLOT_COUNT, yearly_growth_delta,
    yearly_tax_revenue,
};
use nc_ui::PlayfieldBuffer;
use nc_ui::table::{
    TableColumn, TableWidthMode, centered_table_start_col, resolve_table_columns,
    write_stacked_table_window_with_theme_at,
};

use crate::app::state::DashApp;
use crate::overlays::frame::{draw_overlay_frame, write_clipped};
use crate::theme;

const FOOTER: &str = "COMMAND <- ? J K ^U ^D B A C L U X S I T <Q> ->";
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

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let preferred_width = buf.width().saturating_sub(12).clamp(96, 136);
    let preferred_height = buf.height().saturating_sub(6).clamp(18, 28);
    let frame = draw_overlay_frame(buf, "PLANET LIST", preferred_width, preferred_height, FOOTER);

    let owner_slot = app.player_record_index_1_based as u8;
    let player_tax_rate = app
        .game_data
        .player
        .records
        .get(app.player_record_index_1_based.saturating_sub(1))
        .map(|player| player.tax_rate())
        .unwrap_or(0);
    let planets = app
        .game_data
        .planets
        .records
        .iter()
        .filter(|planet| planet.owner_empire_slot_raw() == owner_slot)
        .collect::<Vec<_>>();
    let selected = app
        .planet_overlay
        .selected
        .min(planets.len().saturating_sub(1));
    let visible_rows = frame.body_height.saturating_sub(5);
    let scroll = clamp_scroll(app.planet_overlay.scroll, selected, visible_rows, planets.len());

    let starbase_coords = app
        .game_data
        .bases
        .records
        .iter()
        .filter(|base| base.owner_empire_raw() == owner_slot && base.active_flag_raw() != 0)
        .map(|base| base.coords_raw())
        .collect::<std::collections::BTreeSet<_>>();

    let rows = planets
        .iter()
        .map(|planet| {
            format_planet_row_cells(
                planet,
                starbase_coords.contains(&planet.coords_raw()),
                player_tax_rate,
            )
        })
        .collect::<Vec<_>>();
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

    if planets.is_empty() {
        write_clipped(
            buf,
            metrics.bottom_row.saturating_sub(1),
            table_col + 2,
            frame.body_width.saturating_sub(4),
            "You do not currently control any planets.",
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

fn format_planet_row_cells(planet: &PlanetRecord, has_starbase: bool, tax_rate: u8) -> Vec<String> {
    let coords = planet.coords_raw();
    let present = planet.present_production_points().unwrap_or(0);
    let potential = planet.potential_production_points();
    let stored = planet.stored_production_points();
    let revenue = yearly_tax_revenue(present, tax_rate);
    let growth = yearly_growth_delta(present, potential, tax_rate, has_starbase) as i16;
    let queue = build_queue_total(planet);
    let docked = docked_total(planet);
    let name = planet.planet_name();

    vec![
        format!("({:02},{:02})", coords[0], coords[1]),
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
    ]
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
