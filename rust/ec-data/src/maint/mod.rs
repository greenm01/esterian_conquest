//! Maintenance logic for year advancement and economic simulation.

use crate::CoreGameData;

/// Run a single turn of maintenance processing.
///
/// This is the Rust implementation of ECMAINT.EXE behavior.
/// Currently implements:
/// - Year advancement (+1 per turn)
///
/// TODO: Implement full mechanic parity:
/// - Fleet movement resolution
/// - Combat resolution  
/// - Build completion
/// - Economic simulation
/// - Database regeneration
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

    // TODO: Process fleet orders
    // TODO: Resolve combat
    // TODO: Complete builds
    // TODO: Update economy
    // TODO: Regenerate DATABASE.DAT

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
