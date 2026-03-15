use std::fs;
use std::path::Path;

use ec_data::{CampaignStore, CoreGameData, MaintenanceEvents};

use crate::commands::reports::build_database_dat;

pub(crate) fn with_runtime_game_mut<T, F>(
    dir: &Path,
    mutate: F,
) -> Result<T, Box<dyn std::error::Error>>
where
    F: FnOnce(&mut CoreGameData) -> Result<T, Box<dyn std::error::Error>>,
{
    let store = CampaignStore::open_default_in_dir(dir)?;
    if !store.has_snapshots()? {
        store.import_directory_snapshot(dir)?;
    }
    let mut state = store
        .load_latest_runtime_state()?
        .ok_or("campaign store has no snapshots")?;
    let result = mutate(&mut state.game_data)?;
    let database = build_database_dat(
        &state.game_data,
        &state.game_data.planets,
        &MaintenanceEvents::default(),
        Some(&state.database),
    );
    store.save_runtime_state(
        &state.game_data,
        &database,
        &state.results_bytes,
        &state.messages_bytes,
        &state.queued_mail,
    )?;
    Ok(result)
}

pub(crate) fn with_runtime_game_mut_and_export<T, F>(
    dir: &Path,
    mutate: F,
) -> Result<T, Box<dyn std::error::Error>>
where
    F: FnOnce(&mut CoreGameData) -> Result<T, Box<dyn std::error::Error>>,
{
    let store = CampaignStore::open_default_in_dir(dir)?;
    let result = with_runtime_game_mut(dir, mutate)?;
    let state = store
        .load_latest_runtime_state()?
        .ok_or("campaign store has no snapshots")?;
    write_partial_runtime_projection(dir, &state.game_data)?;
    Ok(result)
}

pub(crate) fn with_runtime_game_mut_and_export_core<T, F>(
    dir: &Path,
    mutate: F,
) -> Result<T, Box<dyn std::error::Error>>
where
    F: FnOnce(&mut CoreGameData) -> Result<T, Box<dyn std::error::Error>>,
{
    let store = CampaignStore::open_default_in_dir(dir)?;
    let result = with_runtime_game_mut(dir, mutate)?;
    let state = store
        .load_latest_runtime_state()?
        .ok_or("campaign store has no snapshots")?;
    write_partial_runtime_projection(dir, &state.game_data)?;
    fs::write(dir.join("CONQUEST.DAT"), state.game_data.conquest.to_bytes())?;
    Ok(result)
}

fn write_partial_runtime_projection(
    dir: &Path,
    game_data: &CoreGameData,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(dir.join("PLAYER.DAT"), game_data.player.to_bytes())?;
    fs::write(dir.join("PLANETS.DAT"), game_data.planets.to_bytes())?;
    fs::write(dir.join("FLEETS.DAT"), game_data.fleets.to_bytes())?;
    fs::write(dir.join("BASES.DAT"), game_data.bases.to_bytes())?;
    fs::write(dir.join("IPBM.DAT"), game_data.ipbm.to_bytes())?;
    fs::write(dir.join("SETUP.DAT"), game_data.setup.to_bytes())?;
    Ok(())
}
