use std::fs;
use std::path::Path;

use ec_data::{CampaignRuntimeState, CampaignStore, CoreGameData, MaintenanceEvents};

use crate::commands::reports::build_database_dat;

pub(crate) fn with_runtime_game_mut<T, F>(
    dir: &Path,
    mutate: F,
) -> Result<T, Box<dyn std::error::Error>>
where
    F: FnOnce(&mut CoreGameData) -> Result<T, Box<dyn std::error::Error>>,
{
    let store = CampaignStore::open_default_in_dir(dir)?;
    let mut state = load_runtime_state_preferring_live_directory(dir, &store)?;
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

pub(crate) fn load_runtime_state_preferring_live_directory(
    dir: &Path,
    store: &CampaignStore,
) -> Result<CampaignRuntimeState, Box<dyn std::error::Error>> {
    let state = match store.load_latest_runtime_state()? {
        Some(state) => state,
        None => {
            store.import_directory_snapshot(dir)?;
            store
                .load_latest_runtime_state()?
                .ok_or("campaign store has no snapshots after importing directory")?
        }
    };

    if directory_differs_from_runtime_state(dir, &state)? {
        store.import_directory_snapshot(dir)?;
        return Ok(store
            .load_latest_runtime_state()?
            .ok_or("campaign store has no snapshots after refreshing from directory")?);
    }

    Ok(state)
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
    fs::write(
        dir.join("CONQUEST.DAT"),
        state.game_data.conquest.to_bytes(),
    )?;
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

fn directory_differs_from_runtime_state(
    dir: &Path,
    state: &CampaignRuntimeState,
) -> Result<bool, Box<dyn std::error::Error>> {
    let expected_files = [
        ("PLAYER.DAT", state.game_data.player.to_bytes()),
        ("PLANETS.DAT", state.game_data.planets.to_bytes()),
        ("FLEETS.DAT", state.game_data.fleets.to_bytes()),
        ("BASES.DAT", state.game_data.bases.to_bytes()),
        ("IPBM.DAT", state.game_data.ipbm.to_bytes()),
        ("SETUP.DAT", state.game_data.setup.to_bytes()),
        ("CONQUEST.DAT", state.game_data.conquest.to_bytes()),
        ("DATABASE.DAT", state.database.to_bytes()),
        ("RESULTS.DAT", state.results_bytes.clone()),
        ("MESSAGES.DAT", state.messages_bytes.clone()),
    ];

    for (name, expected) in expected_files {
        let path = dir.join(name);
        let actual = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(true),
            Err(err) => return Err(err.into()),
        };
        if actual != expected {
            return Ok(true);
        }
    }

    Ok(false)
}
