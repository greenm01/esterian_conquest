//! Left panel: Treasury, Production/Potential, Revenue, Growth.

use nc_data::{yearly_growth_delta, yearly_tax_revenue};
use nc_ui::PlayfieldBuffer;

use crate::app::state::DashApp;
use crate::layout::LEFT_WIDTH;
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let start_row = 2;
    let col = 2;

    buf.write_text(start_row, col, "ECONOMY", theme::section_title_style());

    let player_idx = app.player_record_index_1_based.saturating_sub(1);
    let Some(player) = app.game_data.player.records.get(player_idx) else {
        return;
    };

    let tax = player.tax_rate();
    let owner_slot = app.player_record_index_1_based as u8;

    // Aggregate empire-wide totals.
    let mut total_treasury: u32 = 0;
    let mut total_present: u32 = 0;
    let mut total_potential: u32 = 0;

    for planet in &app.game_data.planets.records {
        if planet.owner_empire_slot_raw() != owner_slot {
            continue;
        }
        total_treasury += planet.stored_goods_raw();
        let present = planet.present_production_points().unwrap_or(0) as u32;
        let potential = planet.potential_production_points() as u32;
        total_present += present;
        total_potential += potential;
    }

    let revenue = yearly_tax_revenue(total_present as u16, tax);
    let growth: i32 = if total_present < total_potential {
        let has_starbase = false; // simplified — full calc would check per-planet
        let delta = yearly_growth_delta(
            total_present as u16,
            total_potential as u16,
            tax,
            has_starbase,
        );
        delta as i32
    } else {
        0
    };

    let _ = LEFT_WIDTH;
    buf.write_text(start_row + 1, col, &format!(" Treasury:{:>8}", total_treasury), theme::value_style());
    buf.write_text(
        start_row + 2,
        col,
        &format!(" Prod:{}/{}", total_present, total_potential),
        theme::value_style(),
    );
    buf.write_text(start_row + 3, col, &format!(" Revenue:{:>8}", revenue), theme::value_style());
    let growth_style = if growth > 0 { theme::friendly_style() } else if growth < 0 { theme::enemy_style() } else { theme::dim_style() };
    buf.write_text(start_row + 4, col, &format!(" Growth:{:>+8}", growth), growth_style);
}
