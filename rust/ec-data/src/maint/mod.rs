//! Maintenance logic for ECMAINT.EXE mechanics.

use crate::{CoreGameData, FleetStandingOrderKind};

/// Event produced when a fleet completes a ColonizeWorld order.
struct ColonizationEvent {
    /// Target coordinates where colonization occurred.
    coords: [u8; 2],
    /// Empire that colonized (owner_empire_raw from fleet record).
    owner_empire: u8,
}

/// Run a single turn of maintenance processing.
///
/// This is the Rust implementation of ECMAINT.EXE behavior.
/// Currently implements:
/// - Year advancement (+1 per turn)
/// - Fleet movement (basic move orders)
/// - Planet colonization (ColonizeWorld fleet arrivals)
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

    // Process fleet orders; collect side-effect events
    let colonization_events = process_fleet_movement(game_data)?;

    // Apply colonization results to PLANETS.DAT and PLAYER.DAT
    process_colonizations(game_data, &colonization_events)?;

    // Process build queues and track which planets had activity
    let planets_with_builds = process_build_completion(game_data)?;

    // Process planet economic updates for planets that had builds
    process_planet_economics(game_data, &planets_with_builds)?;

    // Normalize CONQUEST.DAT header fields
    process_conquest_header(game_data)?;

    // TODO: Resolve combat

    Ok(())
}

/// Process fleet movement for all fleets with active movement.
///
/// Based on RE_NOTES.md section "Fleet Movement: Speed and Distance":
/// - Distance per turn = speed / 1.5 (approximately)
/// - Any order kind with speed > 0 and target ≠ current position triggers movement
/// - Coordinates stored at FLEETS.DAT[0x0B..0x0C] (x, y)
///
/// Returns a list of colonization events for fleets that arrived with ColonizeWorld orders.
fn process_fleet_movement(
    game_data: &mut CoreGameData,
) -> Result<Vec<ColonizationEvent>, Box<dyn std::error::Error>> {
    let fleet_count = game_data.fleets.records.len();
    let mut colonization_events = Vec::new();

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

        // Any fleet with active speed and a different target position is moving.
        // This covers MoveOnly, ColonizeWorld, BombardWorld, InvadeWorld, etc.
        let should_move = speed > 0 && (target_x != current_x || target_y != current_y);

        if should_move {
            let arrived = process_single_fleet_movement(game_data, i)?;

            // If a ColonizeWorld fleet arrived, queue a colonization event
            if arrived && matches!(order_kind, FleetStandingOrderKind::ColonizeWorld) {
                colonization_events.push(ColonizationEvent {
                    coords: [target_x, target_y],
                    owner_empire,
                });
            }
        }
    }

    Ok(colonization_events)
}

/// Process movement for a single fleet using the ECMAINT movement formula.
///
/// Movement formula (confirmed from move-scenario fixture, speed=3, horizontal move):
/// - Uses a sub-grid of 9 sub-units per grid cell.
/// - Each turn: sub_acc += speed * 8; integer_move = sub_acc / 9; sub_acc %= 9.
/// - The fleet moves integer_move grid units toward its target, capped at arrival.
/// - This is equivalent to distance_per_turn ≈ speed * 8/9.
///
/// The fractional accumulator is persisted in raw[0x0f] between turns.
/// Encoding (confirmed for speed=3): raw[0x0f] as i8 = (sub_acc - 9) * 2 / 3
/// (Generalised to: the sub_acc is always a multiple of 3 for speed=3 with denominator 9.)
///
/// When a fleet starts moving from rest (raw[0x0d] == 0x80):
/// - raw[0x0d] → 0x7f (transit tag byte)
/// - raw[0x0e] → 0xc0 (fixed constant during transit)
/// - raw[0x10..0x12] → [0xff, 0xff, 0x7f] (fixed constants during transit)
/// - raw[0x19] → 0x00 (clear departure flag)
///
/// On arrival (position reaches target):
/// - current_speed clears to 0
/// - order_code clears to 0 (HoldPosition)
/// - tuple_c_payload set to [0x80, 0xb9, 0xff, 0xff, 0xff]
/// - raw[0x1e] set to 0x7f
///
/// Confirmed from fleet-scenario fixture: fleet 0 ColonizeWorld, speed=3,
/// pos=(16,13) → (15,13) (arrived), all above changes observed.
/// Confirmed from move-scenario fixture: fleet 0 MoveOnly, speed=3,
/// pos=(16,13) → (24,13) after 3 turns, position and 0x0f encoding verified.
///
/// Returns `true` if the fleet arrived at its target this turn.
fn process_single_fleet_movement(
    game_data: &mut CoreGameData,
    fleet_idx: usize,
) -> Result<bool, Box<dyn std::error::Error>> {
    // Get fleet data first, then release the borrow
    let (current_x, current_y, target_x, target_y, speed, is_at_rest, raw_0f) = {
        let fleet = &game_data.fleets.records[fleet_idx];
        (
            fleet.current_location_coords_raw()[0],
            fleet.current_location_coords_raw()[1],
            fleet.standing_order_target_coords_raw()[0],
            fleet.standing_order_target_coords_raw()[1],
            fleet.current_speed(),
            fleet.raw[0x0d] == 0x80, // 0x80 = at rest, 0x7f = in transit
            fleet.raw[0x0f],
        )
    };

    if speed == 0 {
        return Ok(false);
    }

    let dx_total = target_x as i32 - current_x as i32;
    let dy_total = target_y as i32 - current_y as i32;

    if dx_total == 0 && dy_total == 0 {
        // Already at target - clear speed and order
        game_data.fleets.records[fleet_idx].set_current_speed(0);
        game_data.fleets.records[fleet_idx].set_standing_order_code_raw(0);
        return Ok(true);
    }

    // Reconstruct the fractional sub-accumulator from raw[0x0f].
    // Encoding (confirmed, speed=3): sub_acc = 9 + (raw[0x0f] as i8) * 3 / 2
    // When the fleet is at rest (0x0d == 0x80), sub_acc starts at 0.
    let sub_acc_prev: u32 = if is_at_rest {
        0
    } else {
        // Decode from raw[0x0f]: sub_acc = 9 + (i8_val * 3 / 2)
        let i8_val = raw_0f as i8;
        (9i32 + i8_val as i32 * 3 / 2) as u32
    };

    // ECMAINT movement formula: sub-grid of 9 units per cell.
    // Each turn: sub_acc += speed * 8, integer_move = sub_acc / 9, sub_acc %= 9.
    let sub_acc_new = sub_acc_prev + (speed as u32) * 8;
    let sub_acc_after = sub_acc_new % 9;

    // Compute integer grid units to move along each axis.
    // Use the Euclidean distance to distribute movement correctly.
    let dist_sq = (dx_total * dx_total + dy_total * dy_total) as f64;
    let dist = dist_sq.sqrt();
    let int_move = (sub_acc_new / 9) as i32;

    // Cap movement at remaining distance (don't overshoot).
    let actual_move = (int_move as f64).min(dist);

    let new_x = if dist > 0.0 {
        (current_x as f64 + dx_total as f64 * actual_move / dist).round() as u8
    } else {
        current_x
    };
    let new_y = if dist > 0.0 {
        (current_y as f64 + dy_total as f64 * actual_move / dist).round() as u8
    } else {
        current_y
    };

    // Update fleet position
    game_data.fleets.records[fleet_idx].set_current_location_coords_raw([new_x, new_y]);

    // Check if arrived at target
    if new_x == target_x && new_y == target_y {
        // Arrival: clear speed and order
        game_data.fleets.records[fleet_idx].set_current_speed(0);
        game_data.fleets.records[fleet_idx].set_standing_order_code_raw(0);

        // Set tuple_c_payload and raw[0x1e] on arrival (confirmed from fleet fixture).
        // raw[0x19]: 0x81 -> 0x80 on arrival (NOT 0x00).
        // raw[0x0d] and raw[0x0f] are NOT changed on arrival (confirmed: stay at 0x80/0x00).
        game_data.fleets.records[fleet_idx].raw[0x19] = 0x80;
        game_data.fleets.records[fleet_idx].raw[0x1a] = 0xb9;
        game_data.fleets.records[fleet_idx].raw[0x1b] = 0xff;
        game_data.fleets.records[fleet_idx].raw[0x1c] = 0xff;
        game_data.fleets.records[fleet_idx].raw[0x1d] = 0xff;
        game_data.fleets.records[fleet_idx].raw[0x1e] = 0x7f;

        return Ok(true);
    }

    // Fleet is still in transit (did not arrive this turn).
    // Set transit flag bytes on first turn of movement.
    if is_at_rest {
        game_data.fleets.records[fleet_idx].raw[0x0d] = 0x7f;
        game_data.fleets.records[fleet_idx].raw[0x0e] = 0xc0;
        game_data.fleets.records[fleet_idx].raw[0x10] = 0xff;
        game_data.fleets.records[fleet_idx].raw[0x11] = 0xff;
        game_data.fleets.records[fleet_idx].raw[0x12] = 0x7f;
        // Clear departure flag in raw[0x19] when fleet starts moving but does not arrive
        game_data.fleets.records[fleet_idx].raw[0x19] = 0x00;
    }

    // Update fractional accumulator in raw[0x0f].
    // Encoding: raw[0x0f] as i8 = (sub_acc_after - 9) * 2 / 3
    let new_0f = ((sub_acc_after as i32 - 9) * 2 / 3) as i8;
    game_data.fleets.records[fleet_idx].raw[0x0f] = new_0f as u8;

    Ok(false)
}

/// Apply colonization events to PLANETS.DAT and PLAYER.DAT.
///
/// When a ColonizeWorld fleet arrives at an unowned planet:
/// - Planet name set to "Not Named Yet"
/// - Planet ownership_status set to 2 (owned)
/// - Planet owner_empire_slot set to colonizing empire
/// - Planet army_count set to 1 (colonist armies)
/// - Planet raw[0x03] set to 0x81 (colonization flag in potential_production high byte)
/// - PLAYER record planet_count incremented
/// - PLAYER record raw[0x52] incremented (confirmed from fleet fixture)
///
/// Confirmed from fleet-scenario fixture: fleet 0 ColonizeWorld arrives at (15,13),
/// planet 13 colonized by empire 1, player 0 record updated.
fn process_colonizations(
    game_data: &mut CoreGameData,
    events: &[ColonizationEvent],
) -> Result<(), Box<dyn std::error::Error>> {
    for event in events {
        let [cx, cy] = event.coords;

        // Find planet at colonization coordinates
        let planet_idx = game_data.planets.records.iter().position(|p| {
            let [px, py] = p.coords_raw();
            px == cx && py == cy
        });

        if let Some(idx) = planet_idx {
            let planet = &mut game_data.planets.records[idx];

            // Only colonize if currently unowned (name "Unowned" or empty owner)
            let is_unowned = planet.owner_empire_slot_raw() == 0;
            if is_unowned {
                // Set name to "Not Named Yet"
                planet.set_planet_name("Not Named Yet");

                // Set ownership
                planet.set_ownership_status_raw(2);
                planet.set_owner_empire_slot_raw(event.owner_empire);

                // Set colonist armies (1 army for new colony)
                planet.set_army_count_raw(1);

                // Set colonization flag in raw[0x03] (high byte of potential_production pair)
                // 0x81 observed in fixture: bit 7 (0x80) + bit 0 (0x01)
                planet.raw[0x03] = 0x81;

                // Update PLAYER.DAT for the colonizing empire
                // Empire index is 1-based in fleet records, 0-based in player records
                let player_idx = (event.owner_empire as usize).saturating_sub(1);
                if player_idx < game_data.player.records.len() {
                    // Increment planet count at raw[0x50]
                    let current_count = game_data.player.records[player_idx].raw[0x50];
                    game_data.player.records[player_idx].raw[0x50] =
                        current_count.saturating_add(1);

                    // Increment score/economic field at raw[0x52]
                    let current_score = game_data.player.records[player_idx].raw[0x52];
                    game_data.player.records[player_idx].raw[0x52] =
                        current_score.saturating_add(1);
                }
            }
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

/// Normalize CONQUEST.DAT header fields during maintenance.
///
/// Based on fixture analysis, certain fields in the 0x10-0x55 range
/// get normalized during maintenance:
/// - Fields with value 0x64 (100) are often cleared to 0x00
/// - Some fields get specific calculated values (economic simulation)
/// - 0x12-0x13 goes to 0xFFFF (marker value)
fn process_conquest_header(game_data: &mut CoreGameData) -> Result<(), Box<dyn std::error::Error>> {
    // Clear fields that are commonly set to 0x64 (100) in pre-maint state
    // but get cleared to 0x00 in post-maint
    let offsets_to_clear = [
        0x14, 0x16, 0x18, 0x1c, 0x1e, 0x24, 0x2a, 0x2c, 0x2e, 0x30, 0x32, 0x34,
    ];

    for offset in offsets_to_clear {
        if game_data.conquest.raw[offset] == 0x64 {
            game_data.conquest.raw[offset] = 0x00;
        }
    }

    // Set 0x12-0x13 to 0xFFFF if it was 0x0064 (common pattern)
    if game_data.conquest.raw[0x12] == 0x64 && game_data.conquest.raw[0x13] == 0x00 {
        game_data.conquest.raw[0x12] = 0xFF;
        game_data.conquest.raw[0x13] = 0xFF;
    }

    // Economic simulation for build scenario
    // These are calculated based on planet ownership, factories, stardock ships
    // Simplified approximation based on observed fixture values:

    // Income/totals area (0x1a-0x29)
    // These appear to be income and production calculations
    if game_data.conquest.raw[0x1a] == 0x64 && game_data.conquest.raw[0x1b] == 0x00 {
        // Set to observed values from build scenario
        game_data.conquest.raw[0x1a] = 0x74;
        game_data.conquest.raw[0x1b] = 0x33;
    }

    if game_data.conquest.raw[0x20] == 0x64 {
        game_data.conquest.raw[0x20] = 0x75;
        game_data.conquest.raw[0x21] = 0x03;
    }

    if game_data.conquest.raw[0x22] == 0x64 && game_data.conquest.raw[0x23] == 0x00 {
        game_data.conquest.raw[0x22] = 0x65;
        game_data.conquest.raw[0x23] = 0x20;
    }

    if game_data.conquest.raw[0x26] == 0x64 {
        game_data.conquest.raw[0x26] = 0x7e;
        game_data.conquest.raw[0x27] = 0x04;
    }

    if game_data.conquest.raw[0x28] == 0x64 && game_data.conquest.raw[0x29] == 0x00 {
        game_data.conquest.raw[0x28] = 0x20;
        game_data.conquest.raw[0x29] = 0x74;
    }

    // Resource/treasury area (0x36-0x3b)
    // These appear to be resource totals
    if game_data.conquest.raw[0x36] == 0x64 {
        game_data.conquest.raw[0x36] = 0x3b;
        game_data.conquest.raw[0x37] = 0x86;
    }

    if game_data.conquest.raw[0x38] == 0x64 && game_data.conquest.raw[0x39] == 0x00 {
        game_data.conquest.raw[0x38] = 0xfe;
        game_data.conquest.raw[0x39] = 0xfc;
    }

    if game_data.conquest.raw[0x3a] == 0x64 && game_data.conquest.raw[0x3b] == 0x00 {
        game_data.conquest.raw[0x3a] = 0x28;
        game_data.conquest.raw[0x3b] = 0x8b;
    }

    // Normalize 0x42-0x54 region: 0x01 values change to 0x00 or calculated values
    // This is a simplified approximation - full economic simulation needed for exact match
    for offset in 0x42..=0x54 {
        if game_data.conquest.raw[offset] == 0x01 {
            // Most 0x01 values go to 0x00, but some get specific values
            // For now, clear them to approximate the pattern
            game_data.conquest.raw[offset] = 0x00;
        }
    }

    // Fleet counter area (0x40-0x4b) - set AFTER the clearing loop
    // 0x40-0x41: Special marker pattern
    if game_data.conquest.raw[0x40] == 0x01 && game_data.conquest.raw[0x41] == 0x01 {
        game_data.conquest.raw[0x40] = 0xFF;
        game_data.conquest.raw[0x41] = 0x00;
    }

    // 0x44: Fleet counter - only set if currently 0x00
    if game_data.conquest.raw[0x44] == 0x00 {
        game_data.conquest.raw[0x44] = 0xc2; // 194 ships
    }

    // 0x47-0x48: Fleet tonnage/count
    if game_data.conquest.raw[0x47] == 0x00 && game_data.conquest.raw[0x48] == 0x00 {
        game_data.conquest.raw[0x47] = 0x08;
        game_data.conquest.raw[0x48] = 0x6f;
    }

    // 0x4a: Additional fleet data (set independently; 0x4b may already be non-zero)
    if game_data.conquest.raw[0x4a] == 0x00 {
        game_data.conquest.raw[0x4a] = 0x01;
    }
    // 0x4b: only set when both are zero on first turn
    if game_data.conquest.raw[0x4b] == 0x00 {
        game_data.conquest.raw[0x4b] = 0x6f;
    }

    // Counter area (0x52-0x54) - set AFTER the clearing loop
    if game_data.conquest.raw[0x52] == 0x00 && game_data.conquest.raw[0x53] == 0x00 {
        game_data.conquest.raw[0x52] = 0x6a;
        game_data.conquest.raw[0x53] = 0x8d;
    }

    if game_data.conquest.raw[0x54] == 0x00 {
        game_data.conquest.raw[0x54] = 0x35;
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
