pub mod mapgen;

use ec_data::{CoreGameData, SetupConfig, SetupConfigError};

pub use mapgen::{
    build_seeded_initialized_game, build_seeded_new_game, generate_map, GeneratedMap,
    GeneratedWorld, MapMetrics,
};

pub fn build_game_data_from_setup_config(
    config: &SetupConfig,
    runtime_seed: u64,
) -> Result<CoreGameData, SetupConfigError> {
    let seed = config.seed.unwrap_or(runtime_seed);
    let mut data = build_seeded_new_game(config.player_count, 3000, seed)
        .map_err(|err| SetupConfigError::Parse(err.to_string()))?;

    data.setup.set_snoop_enabled(config.setup_options.snoop);
    data.setup
        .set_local_timeout_enabled(config.setup_options.local_timeout);
    data.setup
        .set_remote_timeout_enabled(config.setup_options.remote_timeout);
    data.setup
        .set_max_time_between_keys_minutes_raw(config.setup_options.max_key_gap_minutes);
    data.setup
        .set_minimum_time_granted_minutes_raw(config.setup_options.minimum_time_minutes);
    data.setup
        .set_purge_after_turns_raw(config.setup_options.purge_after_turns);
    data.setup
        .set_autopilot_inactive_turns_raw(config.setup_options.autopilot_after_turns);
    for idx in 0..4 {
        data.setup
            .set_com_irq_raw(idx, config.port_setup.com_irq[idx]);
        data.setup.set_com_hardware_flow_control_enabled(
            idx,
            config.port_setup.hardware_flow_control[idx],
        );
    }
    data.conquest
        .set_maintenance_schedule_enabled(config.maintenance_days);
    data.conquest.set_player_count(config.player_count);
    Ok(data)
}
