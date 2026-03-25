use ec_compat::{
    ensure_classic_auxiliary_files, import_directory_snapshot_with_seed,
    write_default_database_dat_for_game_data,
};
use ec_data::{CampaignStore, generate_campaign_seed};
use ec_engine::build_seeded_new_game;
use std::fs;
use std::path::Path;

use crate::setup_preset::SetupPresetConfig;
use crate::workspace::seed_classic_runtime_files;

pub(crate) fn init_new_game(
    target: &Path,
    player_count: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    init_new_game_with_seed(target, player_count, runtime_seed())
}

pub(crate) fn init_new_game_with_seed(
    target: &Path,
    player_count: u8,
    seed: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = build_seeded_new_game(player_count, 3000, seed)?;

    fs::create_dir_all(target)?;
    data.save(target)?;
    write_default_database_dat_for_game_data(target, &data)?;
    ensure_classic_auxiliary_files(target)?;

    seed_classic_runtime_files(target)?;
    let store = CampaignStore::open_default_in_dir(target)?;
    import_directory_snapshot_with_seed(&store, target, Some(seed))?;

    Ok(())
}

pub(crate) fn init_new_game_from_config(
    target: &Path,
    config_path: &Path,
    player_count_override: Option<u8>,
    seed_override: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = SetupPresetConfig::load_kdl(config_path)?;
    let config = if let Some(player_count) = player_count_override {
        config.with_player_count_override(player_count)?
    } else {
        config
    };
    let seed = seed_override.unwrap_or_else(runtime_seed);
    let data = build_seeded_new_game(config.player_count, 3000, config.seed.unwrap_or(seed))?;

    fs::create_dir_all(target)?;
    data.save(target)?;
    write_default_database_dat_for_game_data(target, &data)?;
    ensure_classic_auxiliary_files(target)?;

    seed_classic_runtime_files(target)?;
    let store = CampaignStore::open_default_in_dir(target)?;
    import_directory_snapshot_with_seed(&store, target, Some(seed))?;

    Ok(())
}

fn runtime_seed() -> u64 {
    generate_campaign_seed()
}
