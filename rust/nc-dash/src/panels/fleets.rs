//! Left panel: active fleet + starbase list with 2-letter order codes.

use crate::app::state::DashApp;
use crate::layout::{self, PanelWidgetFrame};
use crate::theme;
use nc_data::Order;
use nc_ui::PlayfieldBuffer;

pub fn order_abbrev(order: Order) -> &'static str {
    match order {
        Order::HoldPosition => "Hd",
        Order::MoveOnly => "Mv",
        Order::SeekHome => "Sk",
        Order::PatrolSector => "Pa",
        Order::GuardStarbase => "Gs",
        Order::GuardBlockadeWorld => "Gb",
        Order::BombardWorld => "Bo",
        Order::InvadeWorld => "In",
        Order::BlitzWorld => "Bz",
        Order::ViewWorld => "Vw",
        Order::ScoutSector => "Ss",
        Order::ScoutSolarSystem => "Sy",
        Order::ColonizeWorld => "Co",
        Order::JoinAnotherFleet => "Jn",
        Order::RendezvousSector => "Rz",
        Order::Salvage => "Sa",
        Order::Unknown(_) => "??",
    }
}

fn order_style(order: Order) -> nc_ui::CellStyle {
    match order {
        Order::BombardWorld | Order::InvadeWorld | Order::BlitzWorld => theme::enemy_style(),
        Order::PatrolSector | Order::GuardStarbase | Order::GuardBlockadeWorld => {
            theme::friendly_style()
        }
        Order::MoveOnly | Order::SeekHome | Order::JoinAnotherFleet | Order::RendezvousSector => {
            theme::alert_style()
        }
        Order::ScoutSector | Order::ScoutSolarSystem | Order::ViewWorld => theme::label_style(),
        _ => theme::dim_style(),
    }
}

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: PanelWidgetFrame) {
    layout::write_panel_title(buf, frame, "ACTIVE FLEETS", theme::section_title_style());

    let owner_slot = app.player_record_index_1_based as u8;
    let max_rows = frame.body.height;
    let mut row_offset = 0usize;
    let mut shown = 0;

    for fleet in &app.game_data.fleets.records {
        if fleet.owner_empire_raw() != owner_slot || !fleet.has_any_force() {
            continue;
        }
        if shown < app.fleets_scroll {
            shown += 1;
            continue;
        }
        if row_offset >= max_rows {
            break;
        }
        let num = fleet.local_slot_word_raw();
        let c = fleet.current_location_coords_raw();
        let order = fleet.standing_order_kind();
        let ab = order_abbrev(order);
        layout::write_panel_body_line(
            buf,
            frame,
            row_offset,
            &format!("#{:<3} ({:02},{:02}) {:>2}", num, c[0], c[1], ab),
            order_style(order),
        );
        row_offset += 1;
        shown += 1;
    }
    for base in &app.game_data.bases.records {
        if base.owner_empire_raw() != owner_slot || base.active_flag_raw() == 0 {
            continue;
        }
        if shown < app.fleets_scroll {
            shown += 1;
            continue;
        }
        if row_offset >= max_rows {
            break;
        }
        let c = base.coords_raw();
        layout::write_panel_body_line(
            buf,
            frame,
            row_offset,
            &format!("SB{:<2} ({:02},{:02}) Gs", base.base_id_raw(), c[0], c[1]),
            theme::friendly_style(),
        );
        row_offset += 1;
        shown += 1;
    }
    if row_offset == 0 {
        layout::write_panel_body_line(buf, frame, 0, "(none)", theme::dim_style());
    }
}
