mod arrivals;
mod salvage;
mod stepper;

use super::MovementEvents;
use crate::{CoreGameData, Order, VisibleHazardIntel};
use arrivals::handle_fleet_arrival;
use ec_data::fleet_motion_state::decode_exact_position;
use salvage::{queue_salvage_resolution, remap_movement_event_fleet_indices_after_removal};
use stepper::process_single_fleet_movement;

/// Process fleet movement for all fleets with active movement.
///
/// Based on docs/dev/archive/RE_NOTES.md section "Fleet Movement: Speed and Distance":
/// - Distance per turn = speed / 1.5 (approximately)
/// - Any order kind with speed > 0 and target ≠ current position triggers movement
/// - Coordinates stored at FLEETS.DAT[0x0B..0x0C] (x, y)
///
/// Returns a list of colonization events for fleets that arrived with ColonizeWorld orders.
pub(super) fn process_fleet_movement(
    game_data: &mut CoreGameData,
    visible_hazards_by_empire: &[VisibleHazardIntel],
) -> Result<MovementEvents, Box<dyn std::error::Error>> {
    let fleet_count = game_data.fleets.records.len();
    let mut movement_events = MovementEvents::default();
    let mut to_remove = vec![false; fleet_count];

    for i in 0..fleet_count {
        let (target_x, target_y, current_x, current_y, speed, order_kind, owner_empire) = {
            let fleet = &game_data.fleets.records[i];
            (
                fleet.standing_order_target_coords_raw()[0],
                fleet.standing_order_target_coords_raw()[1],
                fleet.current_location_coords_raw()[0],
                fleet.current_location_coords_raw()[1],
                fleet.current_speed(),
                fleet.standing_order_kind(),
                fleet.owner_empire_raw(),
            )
        };
        if matches!(order_kind, Order::Salvage) && target_x == current_x && target_y == current_y {
            let planet_idx = game_data
                .planets
                .records
                .iter()
                .position(|planet| planet.coords_raw() == [target_x, target_y]);
            queue_salvage_resolution(
                game_data,
                &mut movement_events,
                &mut to_remove,
                i,
                owner_empire,
                planet_idx,
                [target_x, target_y],
            )?;
            continue;
        }
        // A fleet moves when it has a non-HoldPosition order, speed > 0,
        // and either has not yet reached its visible target sector or is still
        // carrying unresolved exact in-transit state after rounding into it.
        // order_code 0x00 = HoldPosition — fleet stays put even if speed > 0
        // and target != current.
        // Note: BombardWorld/InvadeWorld fleets also move to their target before executing;
        // they are allowed here — arrival handling decides whether the order persists and
        // whether movement state stops or stays armed for the next phase.
        let order_code = game_data.fleets.records[i].standing_order_code_raw();
        let has_unresolved_exact_transit = target_x == current_x
            && target_y == current_y
            && decode_exact_position(&game_data.fleets.records[i]).is_some();
        let should_move = speed > 0
            && order_code != 0x00
            && ((target_x != current_x || target_y != current_y) || has_unresolved_exact_transit);

        if should_move {
            let arrived = process_single_fleet_movement(game_data, i, visible_hazards_by_empire)?;

            if arrived {
                handle_fleet_arrival(
                    game_data,
                    &mut movement_events,
                    &mut to_remove,
                    i,
                    order_kind,
                    owner_empire,
                    [target_x, target_y],
                )?;
            }
        }
    }

    if to_remove.iter().any(|remove| *remove) {
        remap_movement_event_fleet_indices_after_removal(&mut movement_events, &to_remove);
        super::remove_selected_fleets(game_data, &to_remove);
    }

    Ok(movement_events)
}
