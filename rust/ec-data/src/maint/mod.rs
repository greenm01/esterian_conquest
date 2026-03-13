//! Maintenance logic for ECMAINT.EXE mechanics.

use crate::{CoreGameData, FleetStandingOrderKind};

/// Run a single turn of maintenance processing.
///
/// This is the Rust implementation of ECMAINT.EXE behavior.
/// Currently implements:
/// - Year advancement (+1 per turn)
/// - Fleet movement (basic move orders)
///
/// Note: DATABASE.DAT regeneration is handled separately in the CLI layer
/// since it's not part of CoreGameData.
///
/// # Arguments
/// * `game_data` - Mutable reference to the game state to modify
///
/// # Returns
/// Ok(()) on success, or an error if maintenance fails
pub fn run_maintenance_turn(
    game_data: &mut CoreGameData,
) -> Result<(), Box<dyn std::error::Error>> {
    // Advance game year by 1
    let current_year = game_data.conquest.game_year();
    let new_year = current_year + 1;
    game_data.conquest.set_game_year(new_year);

    // Process fleet orders
    process_fleet_movement(game_data)?;

    // TODO: Resolve combat
    // TODO: Complete builds
    // TODO: Update economy

    Ok(())
}

/// Process fleet movement for all fleets with move orders.
///
/// Based on RE_NOTES.md section "Fleet Movement: Speed and Distance":
/// - Distance per turn = speed / 1.5 (approximately)
/// - First turn has a "startup penalty" reducing distance by ~1 unit
/// - Coordinates stored at FLEETS.DAT[0x0B..0x0C] (x, y)
fn process_fleet_movement(game_data: &mut CoreGameData) -> Result<(), Box<dyn std::error::Error>> {
    let fleet_count = game_data.fleets.records.len();

    for i in 0..fleet_count {
        let order_kind = game_data.fleets.records[i].standing_order_kind();

        // Process movement orders (MoveOnly or RendezvousSector)
        match order_kind {
            FleetStandingOrderKind::MoveOnly | FleetStandingOrderKind::RendezvousSector => {
                process_single_fleet_movement(game_data, i)?;
            }
            _ => {}
        }
    }

    Ok(())
}

/// Process movement for a single fleet using the ECMAINT movement formula.
///
/// Movement formula (from RE_NOTES.md):
/// - Target distance per turn = speed / 1.5
/// - First turn: max(1, floor(speed / 1.5))
/// - Subsequent turns: ceil(speed / 1.5) or follows observed pattern
///
/// Observed patterns over 3 passes:
/// - Speed 1: 1, 0, 1 (total 2, avg 0.67)
/// - Speed 2: 1, 2, 2 (total 5, avg 1.67)
/// - Speed 3: 2, 3, 3 (total 8, avg 2.67)
fn process_single_fleet_movement(
    game_data: &mut CoreGameData,
    fleet_idx: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let fleet = &game_data.fleets.records[fleet_idx];

    // Get current position
    let current_x = fleet.current_location_coords_raw()[0];
    let current_y = fleet.current_location_coords_raw()[1];

    // Get target position from order
    let target_x = fleet.standing_order_target_coords_raw()[0];
    let target_y = fleet.standing_order_target_coords_raw()[1];

    // Get movement speed
    let speed = fleet.current_speed();

    if speed == 0 {
        return Ok(());
    }

    // Calculate distance to target
    let dx = target_x as f64 - current_x as f64;
    let dy = target_y as f64 - current_y as f64;
    let distance_to_target = (dx * dx + dy * dy).sqrt();

    if distance_to_target <= 0.5 {
        // Already at target - clear speed
        game_data.fleets.records[fleet_idx].set_current_speed(0);
        return Ok(());
    }

    // Calculate movement distance based on ECMAINT formula
    // Base formula: speed / 1.5 ≈ speed * 0.666...
    let speed_f64 = speed as f64;
    let target_distance = speed_f64 / 1.5;

    // Movement distance is the minimum of:
    // 1. Distance to target (don't overshoot)
    // 2. Calculated movement based on speed
    let move_distance = distance_to_target.min(target_distance);

    if move_distance > 0.0 {
        // Calculate movement vector
        let ratio = move_distance / distance_to_target;
        let new_x = (current_x as f64 + dx * ratio).round() as u8;
        let new_y = (current_y as f64 + dy * ratio).round() as u8;

        // Update fleet position
        game_data.fleets.records[fleet_idx].set_current_location_coords_raw([new_x, new_y]);

        // Check if arrived at target
        if new_x == target_x && new_y == target_y {
            game_data.fleets.records[fleet_idx].set_current_speed(0);
        }
    }

    Ok(())
}

/// Run maintenance for multiple turns.
///
/// # Arguments
/// * `game_data` - Mutable reference to the game state
/// * `turns` - Number of turns to process
///
/// # Returns
/// The final year after all turns, or an error
pub fn run_maintenance_turns(
    game_data: &mut CoreGameData,
    turns: u16,
) -> Result<u16, Box<dyn std::error::Error>> {
    for _ in 0..turns {
        run_maintenance_turn(game_data)?;
    }
    Ok(game_data.conquest.game_year())
}
