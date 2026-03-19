use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use ec_data::{
    CampaignRuntimeState, CampaignStore, CoreGameData, DEFAULT_CAMPAIGN_DB_NAME, MaintenanceEvents,
    PlanetIntelSnapshot,
};

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
    let planet_intel_by_viewer = load_runtime_intel_by_viewer(&store, &state.game_data)?;
    let result = mutate(&mut state.game_data)?;
    let database = build_database_dat(
        &state.game_data,
        &state.game_data.planets,
        &planet_intel_by_viewer,
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
    if let Some(state) = store.load_latest_runtime_state()? {
        return Ok(state);
    }

    store.import_directory_snapshot(dir)?;
    Ok(store
        .load_latest_runtime_state()?
        .ok_or("campaign store has no snapshots after importing directory")?)
}

pub(crate) fn load_runtime_game_data(
    dir: &Path,
) -> Result<CoreGameData, Box<dyn std::error::Error>> {
    let store_path = dir.join(DEFAULT_CAMPAIGN_DB_NAME);
    if store_path.exists() {
        let store = CampaignStore::open(store_path)?;
        if let Some(state) = store.load_latest_runtime_state()? {
            return Ok(state.game_data);
        }
    }

    Ok(CoreGameData::load(dir)?)
}

pub(crate) fn export_runtime_snapshot_in_place(
    dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    CampaignStore::open_default_in_dir(dir)?.export_latest_snapshot_to_dir(dir)?;
    Ok(())
}

pub(crate) fn export_runtime_core_projection_in_place(
    dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let game_data = load_runtime_game_data(dir)?;
    fs::write(dir.join("PLAYER.DAT"), game_data.player.to_bytes())?;
    fs::write(dir.join("PLANETS.DAT"), game_data.planets.to_bytes())?;
    fs::write(dir.join("FLEETS.DAT"), game_data.fleets.to_bytes())?;
    fs::write(dir.join("BASES.DAT"), game_data.bases.to_bytes())?;
    fs::write(dir.join("IPBM.DAT"), game_data.ipbm.to_bytes())?;
    fs::write(dir.join("SETUP.DAT"), game_data.setup.to_bytes())?;
    Ok(())
}

pub(crate) fn with_runtime_game_mut_and_export<T, F>(
    dir: &Path,
    mutate: F,
) -> Result<T, Box<dyn std::error::Error>>
where
    F: FnOnce(&mut CoreGameData) -> Result<T, Box<dyn std::error::Error>>,
{
    with_runtime_game_mut(dir, mutate)
}

pub(crate) fn load_runtime_intel_by_viewer(
    campaign_store: &CampaignStore,
    game_data: &CoreGameData,
) -> Result<Vec<BTreeMap<usize, PlanetIntelSnapshot>>, Box<dyn std::error::Error>> {
    (1..=game_data.conquest.player_count())
        .map(|viewer_empire_id| {
            Ok(campaign_store
                .latest_planet_intel_for_viewer(viewer_empire_id)?
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>())
        })
        .collect()
}

pub(crate) fn with_runtime_game_mut_and_export_core<T, F>(
    dir: &Path,
    mutate: F,
) -> Result<T, Box<dyn std::error::Error>>
where
    F: FnOnce(&mut CoreGameData) -> Result<T, Box<dyn std::error::Error>>,
{
    with_runtime_game_mut(dir, mutate)
}
