//! Left panel: active fleet + starbase list with 2-letter order codes.

use nc_data::Order;
use nc_ui::PlayfieldBuffer;

use crate::app::state::DashApp;
use crate::theme;

/// Two-letter order abbreviation per player manual Appendix C.
fn order_abbrev(order: Order) -> &'static str {
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
        Order::BombardWorld | Order::InvadeWorld | Order::BlitzWorld => crate::theme::enemy_style(),
        Order::PatrolSector | Order::GuardStarbase | Order::GuardBlockadeWorld => crate::theme::friendly_style(),
        Order::MoveOnly | Order::SeekHome | Order::JoinAnotherFleet | Order::RendezvousSector => crate::theme::alert_style(),
        Order::ScoutSector | Order::ScoutSolarSystem | Order::ViewWorld => crate::theme::label_style(),
        _ => crate::theme::dim_style(),
    }
}

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let start_row = buf.height() / 2; // dynamic: below planet list
    let start_row = start_row.max(15).min(buf.height().saturating_sub(6));
    let col = 2;

    buf.write_text(start_row, col, "ACTIVE FLEETS", crate::theme::section_title_style());

    let owner_slot = app.player_record_index_1_based as u8;
    let max_rows = buf.height().saturating_sub(start_row + 2);

    let mut row = start_row + 1;
    let mut shown = 0;

    for fleet in &app.game_data.fleets.records {
        if fleet.owner_empire_raw() != owner_slot || !fleet.has_any_force() {
            continue;
        }
        if shown < app.fleets_scroll {
            shown += 1;
            continue;
        }
        if shown >= max_rows.max(1) + app.fleets_scroll {
            break;
        }

        let fleet_num = fleet.local_slot_word_raw();
        let coords = fleet.current_location_coords_raw();
        let order = fleet.standing_order_kind();
        let abbrev = order_abbrev(order);
        let style = order_style(order);

        let line = format!(" #{:<3} ({:02},{:02}) {}", fleet_num, coords[0], coords[1], abbrev);
        // Write label in value style, then overwrite abbrev in order color.
        buf.write_text(row, col, &line[..line.len().saturating_sub(2)], crate::theme::value_style());
        buf.write_text(row, col + line.len().saturating_sub(2), abbrev, style);

        row += 1;
        shown += 1;
    }

    // Starbases.
    for base in &app.game_data.bases.records {
        if base.owner_empire_raw() != owner_slot || base.active_flag_raw() == 0 {
            continue;
        }
        if shown < app.fleets_scroll {
            shown += 1;
            continue;
        }
        if shown >= max_rows.max(1) + app.fleets_scroll {
            break;
        }
        let coords = base.coords_raw();
        let line = format!(" SB{:<2} ({:02},{:02}) Gs", base.base_id_raw(), coords[0], coords[1]);
        buf.write_text(row, col, &line, crate::theme::friendly_style());
        row += 1;
        shown += 1;
    }

    if row == start_row + 1 {
        buf.write_text(start_row + 1, col, " (none)", crate::theme::dim_style());
    }
}
