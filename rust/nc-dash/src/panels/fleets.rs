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

    let label_width = layout::label_value_width([
        "Total Fleets",
        "Total Ships",
        "In Transit",
        "Hostile",
        "Defensive",
        "Idle",
    ]);
    let summary_rows = vec![
        layout::format_label_value("Total Fleets", label_width, &format!("{total_fleets:>4}")),
        layout::format_label_value("Total Ships", label_width, &format!("{total_ships:>4}")),
        layout::format_label_value("In Transit", label_width, &format!("{in_transit:>4}")),
        layout::format_label_value("Hostile", label_width, &format!("{hostile:>4}")),
        layout::format_label_value("Defensive", label_width, &format!("{defensive:>4}")),
        layout::format_label_value("Idle", label_width, &format!("{idle:>4}")),
    ];

    for (i, row) in summary_rows.iter().enumerate() {
        if i >= frame.body.height {
            break;
        }
        layout::write_panel_body_line(buf, frame, i, row, status_value_style());
    }
}
