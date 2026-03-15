use std::path::Path;

use crate::commands::runtime::with_runtime_game_mut_and_export;

pub(crate) fn set_player_name(
    dir: &Path,
    player_record_index_1_based: usize,
    handle: &str,
    empire_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    with_runtime_game_mut_and_export(dir, |data| {
        let owner_empire = player_record_index_1_based as u8;
        let player = data
            .player
            .records
            .get_mut(player_record_index_1_based - 1)
            .ok_or_else(|| format!("player record index out of range: {player_record_index_1_based}"))?;
        player.set_owner_empire_raw(owner_empire);
        player.set_occupied_flag(owner_empire);
        player.set_assigned_player_handle_raw(handle);
        player.set_controlled_empire_name_raw(empire_name);
        player.set_autopilot_flag(0);
        Ok(())
    })?;

    println!(
        "Player {} renamed: handle='{}' empire='{}'",
        player_record_index_1_based, handle, empire_name
    );
    Ok(())
}
