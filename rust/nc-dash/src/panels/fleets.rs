//! Left panel: active fleet summary stats.

use crate::app::state::DashApp;
use crate::layout::{self, PanelWidgetFrame};
use nc_data::Order;
use nc_ui::PlayfieldBuffer;
use nc_ui::theme::classic::status_value_style;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: PanelWidgetFrame) {
    layout::write_panel_title(
        buf,
        frame,
        "ACTIVE FLEETS",
        crate::theme::section_title_style(),
    );

    let owner_slot = app.player_record_index_1_based as u8;

    let mut total_fleets = 0;
    let mut total_ships = 0;
    let mut in_transit = 0;
    let mut hostile = 0;
    let mut defensive = 0;
    let mut idle = 0;

    for fleet in &app.game_data.fleets.records {
        if fleet.owner_empire_raw() != owner_slot || !fleet.has_any_force() {
            continue;
        }

        total_fleets += 1;
        total_ships += fleet.battleship_count() as u32
            + fleet.cruiser_count() as u32
            + fleet.destroyer_count() as u32
            + fleet.scout_count() as u32
            + fleet.troop_transport_count() as u32
            + fleet.etac_count() as u32;

        match fleet.standing_order_kind() {
            Order::MoveOnly
            | Order::SeekHome
            | Order::JoinAnotherFleet
            | Order::RendezvousSector => {
                in_transit += 1;
            }
            Order::BombardWorld | Order::InvadeWorld | Order::BlitzWorld => {
                hostile += 1;
            }
            Order::PatrolSector | Order::GuardStarbase | Order::GuardBlockadeWorld => {
                defensive += 1;
            }
            Order::HoldPosition => {
                idle += 1;
            }
            _ => {}
        }
    }

    let summary_rows = vec![
        format!(" Total Fleets:   {:>4}", total_fleets),
        format!(" Total Ships:    {:>4}", total_ships),
        format!(" In Transit:     {:>4}", in_transit),
        format!(" Hostile:        {:>4}", hostile),
        format!(" Defensive:      {:>4}", defensive),
        format!(" Idle:           {:>4}", idle),
    ];

    for (i, row) in summary_rows.iter().enumerate() {
        if i >= frame.body.height {
            break;
        }
        layout::write_panel_body_line(buf, frame, i, row, status_value_style());
    }
}
