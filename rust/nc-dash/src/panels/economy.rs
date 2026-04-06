//! Left panel: Treasury, production, revenue, and generated PP growth.

use nc_data::{yearly_growth_delta, yearly_tax_revenue};
use nc_ui::{CellStyle, PlayfieldBuffer};

use crate::app::state::DashApp;
use crate::layout::{self, PanelWidgetFrame};
use crate::theme;

pub(crate) const TITLE: &str = "ECONOMY";

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: PanelWidgetFrame) {
    layout::write_panel_title(buf, frame, TITLE, theme::section_title_style());

    for (row_idx, (text, style)) in body_rows(app).into_iter().enumerate() {
        if row_idx >= frame.body.height {
            break;
        }
        layout::write_panel_body_line(buf, frame, row_idx, &text, style);
    }
}

pub(crate) fn body_rows(app: &DashApp) -> Vec<(String, CellStyle)> {
    let player_idx = app.player_record_index_1_based.saturating_sub(1);
    let Some(player) = app.game_data.player.records.get(player_idx) else {
        return Vec::new();
    };
    let tax = player.tax_rate();
    let owner_slot = app.player_record_index_1_based as u8;

    let mut total_treasury: u32 = 0;
    let mut total_present: u32 = 0;
    let mut total_potential: u32 = 0;
    for planet in &app.game_data.planets.records {
        if planet.owner_empire_slot_raw() != owner_slot {
            continue;
        }
        total_treasury += planet.stored_goods_raw();
        total_present += planet.present_production_points().unwrap_or(0) as u32;
        total_potential += planet.potential_production_points() as u32;
    }

    let economy = app.game_data.empire_economy_summary(app.player_record_index_1_based);

    let revenue = yearly_tax_revenue(total_present as u16, tax);
    let growth: i32 = if total_present < total_potential {
        yearly_growth_delta(total_present as u16, total_potential as u16, tax, false) as i32
    } else {
        0
    };
    let growth_percent = if total_present == 0 {
        String::from("?")
    } else {
        format!(
            "{:.1}%",
            (f64::from(growth.max(0) as u16) / f64::from(total_present as u16)) * 100.0
        )
    };
    let gs = if growth > 0 {
        theme::friendly_style()
    } else if growth < 0 {
        theme::enemy_style()
    } else {
        theme::dim_style()
    };

    vec![
        (
            layout::format_left_column_value("Treasury", &total_treasury.to_string()),
            theme::value_style(),
        ),
        (
            layout::format_left_column_value("Prod", &total_present.to_string()),
            theme::value_style(),
        ),
        (
            layout::format_left_column_value("Pot Prod", &total_potential.to_string()),
            theme::value_style(),
        ),
        (
            layout::format_left_column_value("Revenue", &revenue.to_string()),
            theme::value_style(),
        ),
        (
            layout::format_left_column_value("PP Gen", &format!("{growth:+}")),
            gs,
        ),
        (
            layout::format_left_column_value("% Growth", &growth_percent),
            gs,
        ),
        (
            layout::format_left_column_value("Efficiency", &format!("{:.1}%", economy.efficiency_percent)),
            theme::value_style(),
        ),
        (
            layout::format_left_column_value("Prod Rank", &format!("#{}", economy.rank_by_present_production)),
            theme::value_style(),
        ),
        (
            layout::format_left_column_value("Plnt Rank", &format!("#{}", economy.rank_by_planets)),
            theme::value_style(),
        ),
        (
            layout::format_left_column_value("Cmd Limit", &format!("{} / {}", economy.current_fleets_and_bases, economy.max_fleets_and_bases)),
            theme::value_style(),
        ),
    ]
}
