use std::path::Path;

use ec_data::CoreGameData;

use crate::commands::runtime::{with_runtime_game_mut, with_runtime_game_mut_and_export};

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
            .ok_or_else(|| {
                format!("player record index out of range: {player_record_index_1_based}")
            })?;
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

pub(crate) fn prepare_classic_login(
    dir: &Path,
    player_record_index_1_based: usize,
    caller_alias: &str,
    empire_name_override: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let current_empire_name = current_empire_name(dir, player_record_index_1_based)?;
    let empire_name = empire_name_override.unwrap_or(&current_empire_name);

    with_runtime_game_mut_and_export(dir, |data| {
        let owner_empire = player_record_index_1_based as u8;
        let player = data
            .player
            .records
            .get_mut(player_record_index_1_based - 1)
            .ok_or_else(|| {
                format!("player record index out of range: {player_record_index_1_based}")
            })?;
        player.set_owner_empire_raw(owner_empire);
        player.set_occupied_flag(owner_empire);
        player.set_assigned_player_handle_raw(caller_alias);
        if let Some(empire_name) = empire_name_override {
            player.set_controlled_empire_name_raw(empire_name);
        }
        player.set_autopilot_flag(0);
        Ok(())
    })?;

    println!(
        "Prepared classic login for player {}: caller_alias='{}' empire='{}'",
        player_record_index_1_based, caller_alias, empire_name
    );
    Ok(())
}

fn current_empire_name(
    dir: &Path,
    player_record_index_1_based: usize,
) -> Result<String, Box<dyn std::error::Error>> {
    with_runtime_game_mut(dir, |data: &mut CoreGameData| {
        let player = data
            .player
            .records
            .get(player_record_index_1_based - 1)
            .ok_or_else(|| {
                format!("player record index out of range: {player_record_index_1_based}")
            })?;
        Ok(player.controlled_empire_name_summary())
    })
}
