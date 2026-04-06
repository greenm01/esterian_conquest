//! P overlay: dashboard-sized planet management table.

use nc_data::{PlanetRecord, ProductionItemKind, STARDOCK_SLOT_COUNT, yearly_growth_delta, yearly_tax_revenue};
use nc_ui::PlayfieldBuffer;

use crate::app::state::DashApp;
use crate::overlays::frame::{draw_hline, draw_overlay_frame, write_clipped};
use crate::theme;

const FOOTER: &str = "COMMAND <- ? J K ^U ^D B A C L U X S I T <Q> ->";

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let preferred_width = buf.width().saturating_sub(12).clamp(96, 136);
    let preferred_height = buf.height().saturating_sub(6).clamp(18, 28);
    let frame = draw_overlay_frame(buf, "PLANET LIST", preferred_width, preferred_height, FOOTER);
    let body_right = frame.body_col + frame.body_width;

    write_clipped(
        buf,
        frame.body_row,
        frame.body_col,
        frame.body_width,
        "Coord   Planet          Max Curr Stored Rev Grow Queue Dock SBs ARs GBs",
        theme::section_title_style(),
    );
    write_clipped(
        buf,
        frame.body_row + 1,
        frame.body_col,
        frame.body_width,
        "(XX,YY) Name           Prod Prod Points         Build Star",
        theme::section_title_style(),
    );
    draw_hline(
        buf,
        frame.body_row + 2,
        frame.body_col,
        frame.body_width.min(body_right.saturating_sub(frame.body_col)),
        theme::border_style(),
    );

    let owner_slot = app.player_record_index_1_based as u8;
    let planets = app
        .game_data
        .planets
        .records
        .iter()
        .filter(|planet| planet.owner_empire_slot_raw() == owner_slot)
        .collect::<Vec<_>>();
    let list_start = frame.body_row + 3;
    let max_rows = frame.body_height.saturating_sub(3);
    let selected = app
        .planet_overlay
        .selected
        .min(planets.len().saturating_sub(1));
    let scroll = clamp_scroll(app.planet_overlay.scroll, selected, max_rows, planets.len());

    let starbase_coords = app
        .game_data
        .bases
        .records
        .iter()
        .filter(|base| base.owner_empire_raw() == owner_slot && base.active_flag_raw() != 0)
        .map(|base| base.coords_raw())
        .collect::<std::collections::BTreeSet<_>>();

    for (visible_idx, planet) in planets.iter().skip(scroll).take(max_rows).enumerate() {
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
        let player = app
            .game_data
            .player
            .records
            .get(app.player_record_index_1_based.saturating_sub(1));
        write_clipped(
            buf,
            row,
            frame.body_col,
            frame.body_width,
            &format_planet_row(
                planet,
                starbase_coords.contains(&planet.coords_raw()),
                player.map(|player| player.tax_rate()).unwrap_or(0),
            ),
            style,
        );
    }

    if planets.is_empty() {
        write_clipped(
            buf,
            list_start,
            frame.body_col,
            frame.body_width,
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

fn format_planet_row(planet: &PlanetRecord, has_starbase: bool, tax_rate: u8) -> String {
    let coords = planet.coords_raw();
    let present = planet.present_production_points().unwrap_or(0);
    let potential = planet.potential_production_points();
    let stored = planet.stored_production_points();
    let revenue = yearly_tax_revenue(present, tax_rate);
    let growth = yearly_growth_delta(present, potential, tax_rate, has_starbase) as i16;
    let queue = build_queue_total(planet);
    let docked = docked_total(planet);
    let name = planet.planet_name();

    format!(
        "({:02},{:02}) {:<13} {:>3} {:>4} {:>6} {:>3} {:>+4} {:>5} {:>4} {:>3} {:>3} {:>3}",
        coords[0],
        coords[1],
        truncate(&name, 13),
        potential,
        present,
        stored,
        revenue,
        growth,
        queue,
        docked,
        u8::from(has_starbase),
        planet.army_count_raw(),
        planet.ground_batteries_raw(),
    )
}

fn build_queue_total(planet: &PlanetRecord) -> u32 {
    (0..10)
        .map(|slot| {
            let points = u32::from(planet.build_count_raw(slot));
            let kind = ProductionItemKind::from_raw(planet.build_kind_raw(slot));
            let Some(cost) = kind.build_cost() else {
                return 0;
            };
            if matches!(
                kind,
                ProductionItemKind::Army | ProductionItemKind::GroundBattery
            ) {
                points / cost
            } else {
                points / cost
            }
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
