//! Left panel: active fleet summary stats.

use crate::app::state::DashApp;
use crate::layout::{self, PanelWidgetFrame};
use nc_data::{Order, STARDOCK_SLOT_COUNT};
use nc_ui::{CellStyle, PlayfieldBuffer};
use nc_ui::theme::classic::status_value_style;

pub(crate) const TITLE: &str = "MY FLEETS";

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: PanelWidgetFrame) {
    layout::write_panel_title(buf, frame, TITLE, crate::theme::section_title_style());

    for (i, (row, style)) in body_rows(app).into_iter().enumerate() {
        if i >= frame.body.height {
            break;
        }
        layout::write_panel_body_line(buf, frame, i, &row, style);
    }
}

pub(crate) fn body_rows(app: &DashApp) -> Vec<(String, CellStyle)> {
    let owner_slot = app.player_record_index_1_based as u8;

    let mut total_fleets = 0;
    let mut total_ships = 0;
    let mut docked = 0;
    let mut in_transit = 0;
    let mut hostile = 0;
    let mut defensive = 0;
    let mut idle = 0;

    for planet in &app.game_data.planets.records {
        if planet.owner_empire_slot_raw() != owner_slot {
            continue;
        }

        for slot in 0..STARDOCK_SLOT_COUNT {
            docked += u32::from(planet.stardock_count_raw(slot));
        }
    }

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

    vec![
        (
            layout::format_left_column_value("Tot Fleets", &total_fleets.to_string()),
            status_value_style(),
        ),
        (
            layout::format_left_column_value("Tot Ships", &total_ships.to_string()),
            status_value_style(),
        ),
        (
            layout::format_left_column_value("Docked", &docked.to_string()),
            status_value_style(),
        ),
        (
            layout::format_left_column_value("In Transit", &in_transit.to_string()),
            status_value_style(),
        ),
        (
            layout::format_left_column_value("Hostile", &hostile.to_string()),
            status_value_style(),
        ),
        (
            layout::format_left_column_value("Defensive", &defensive.to_string()),
            status_value_style(),
        ),
        (
            layout::format_left_column_value("Idle", &idle.to_string()),
            status_value_style(),
        ),
    ]
}
