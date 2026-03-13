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

    // Process build queues and track which planets had activity
    let planets_with_builds = process_build_completion(game_data)?;

    // Process planet economic updates for planets that had builds
    process_planet_economics(game_data, &planets_with_builds)?;

    // TODO: Resolve combat

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

/// Process build queue completion for all planets.
///
/// Build production is based on planet's industrial capacity:
/// - Production rate = factories_word + potential_production bonus
/// - Each build queue item decrements by production rate per turn
/// - When build_count reaches 0, ship moves to stardock
///
/// Returns a list of planet indices that had build activity.
fn process_build_completion(
    game_data: &mut CoreGameData,
) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
    let planet_count = game_data.planets.records.len();
    let mut planets_with_builds = Vec::new();

    for planet_idx in 0..planet_count {
        // Calculate production rate based on factories and potential
        let factories = game_data.planets.records[planet_idx].factories_word_raw();
        let potential =
            u16::from_le_bytes(game_data.planets.records[planet_idx].potential_production_raw());

        // Production = factories + (potential / 2) as simple approximation
        // TODO: Verify exact formula from RE_NOTES or fixtures
        let production_rate = factories + (potential / 2);
        let production_rate_u8 = production_rate.min(255) as u8;

        // Process up to 10 build slots per planet
        let mut had_builds = false;
        for slot in 0..10 {
            let build_count = game_data.planets.records[planet_idx].build_count_raw(slot);

            if build_count > 0 {
                had_builds = true;
                // Decrement by production rate (or remaining count if less)
                let decrement = build_count.min(production_rate_u8);
                let new_count = build_count.saturating_sub(decrement);

                game_data.planets.records[planet_idx].set_build_count_raw(slot, new_count);

                // If build completed (reached 0), move to stardock
                if new_count == 0 {
                    let build_kind = game_data.planets.records[planet_idx].build_kind_raw(slot);

                    // Find first empty stardock slot
                    let mut _moved = false;
                    for stardock_slot in 0..10 {
                        let existing_kind =
                            game_data.planets.records[planet_idx].stardock_kind_raw(stardock_slot);
                        if existing_kind == 0 {
                            // Empty slot found
                            game_data.planets.records[planet_idx]
                                .set_stardock_kind_raw(stardock_slot, build_kind);
                            // Set count based on ship type (default to 3 for now)
                            game_data.planets.records[planet_idx]
                                .set_stardock_count_raw(stardock_slot, 3);
                            _moved = true;
                            break;
                        }
                    }

                    // Clear the build slot
                    game_data.planets.records[planet_idx].set_build_kind_raw(slot, 0);
                }
            }
        }

        if had_builds {
            planets_with_builds.push(planet_idx);
        }
    }

    Ok(planets_with_builds)
}

/// Process planet economic updates during maintenance.
///
/// Only applies to planets that had build queue activity.
/// Currently handles:
/// - Tax rate reset (cleared to 0)
/// - Factories word normalization (high byte cleared)
fn process_planet_economics(
    game_data: &mut CoreGameData,
    planets_with_builds: &[usize],
) -> Result<(), Box<dyn std::error::Error>> {
    for &planet_idx in planets_with_builds {
        // Reset tax rate to 0 (observed in fixture analysis)
        game_data.planets.records[planet_idx].set_planet_tax_rate_raw(0);

        // Normalize factories word - clear the high byte
        // Observed: 0x4886 (34376) -> 0x4800 (72), so high byte 0x86 cleared to 0x00
        // But low byte 0x48 stays
        game_data.planets.records[planet_idx].raw[0x09] = 0x00;
        // Keep low byte at 0x08 as is
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
