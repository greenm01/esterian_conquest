//! Left panel: Treasury, Production/Potential, Revenue, Growth.

use nc_data::{yearly_growth_delta, yearly_tax_revenue};
use nc_ui::PlayfieldBuffer;

use crate::app::state::DashApp;
use crate::layout;
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let (ox, oy) = layout::frame_offset(app);
    let col = ox + 2;
    let start_row = layout::left_economy_title_row(oy);
    let width = layout::left_panel_content_width();

    layout::write_width_clipped(buf, start_row, col, width, "ECONOMY", theme::section_title_style());

    let player_idx = app.player_record_index_1_based.saturating_sub(1);
    let Some(player) = app.game_data.player.records.get(player_idx) else { return };
    let tax = player.tax_rate();
    let owner_slot = app.player_record_index_1_based as u8;

    let mut total_treasury: u32 = 0;
    let mut total_present: u32 = 0;
    let mut total_potential: u32 = 0;
    for planet in &app.game_data.planets.records {
        if planet.owner_empire_slot_raw() != owner_slot { continue; }
        total_treasury += planet.stored_goods_raw();
        total_present += planet.present_production_points().unwrap_or(0) as u32;
        total_potential += planet.potential_production_points() as u32;
    }

    let revenue = yearly_tax_revenue(total_present as u16, tax);
    let growth: i32 = if total_present < total_potential {
        yearly_growth_delta(total_present as u16, total_potential as u16, tax, false) as i32
    } else { 0 };

    layout::write_width_clipped(buf, start_row + 1, col, width, &format!(" Treasury:{:>7}", total_treasury), theme::value_style());
    layout::write_width_clipped(buf, start_row + 2, col, width, &format!(" Prod:{}/{}", total_present, total_potential), theme::value_style());
    layout::write_width_clipped(buf, start_row + 3, col, width, &format!(" Revenue:{:>7}", revenue), theme::value_style());
    let gs = if growth > 0 { theme::friendly_style() } else if growth < 0 { theme::enemy_style() } else { theme::dim_style() };
    layout::write_width_clipped(buf, start_row + 4, col, width, &format!(" Growth:{:>+7}", growth), gs);
}
