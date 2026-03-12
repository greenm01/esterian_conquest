use std::path::Path;

use ec_data::CoreGameData;

pub(crate) fn print_core_report(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = CoreGameData::load(dir)?;
    let starbase_total = data.player1_starbase_count_current_known();
    let owned_base_total = data.player1_owned_base_record_count_current_known();
    let ipbm_total = data.player1_ipbm_count_current_known();
    let player_owned_planet_counts = data.player_owned_planet_counts_current_known();
    let player_starbase_counts = data.player_starbase_counts_current_known();
    let player_owned_base_counts = data.player_owned_base_record_counts_current_known();
    let player_homeworld_seed_coords = data.player_homeworld_seed_coords_current_known();
    let player_fleet_chain_heads = data.player_fleet_chain_heads_current_known();
    let initialized_fleet_block_head_ids = data.current_known_initialized_fleet_block_head_ids();

    println!("Core State Report");
    println!("  dir={}", dir.display());
    println!("  player_record_count={}", data.player.records.len());
    println!("  planet_record_count={}", data.planets.records.len());
    println!("  fleet_record_count={}", data.fleets.records.len());
    println!("  base_record_count={}", data.bases.records.len());
    println!("  ipbm_record_count={}", data.ipbm.records.len());
    println!("  conquest.game_year={}", data.conquest.game_year());
    println!(
        "  initialized_fleet_blocks={}",
        data.looks_like_initialized_fleet_blocks_current_known()
    );
    println!(
        "  initialized_fleet_payloads={}",
        data.current_known_initialized_fleet_payload_errors().is_empty()
    );
    println!(
        "  initialized_fleet_missions={}",
        data.current_known_initialized_fleet_mission_errors().is_empty()
    );
    println!(
        "  initialized_homeworld_alignment={}",
        data.current_known_initialized_homeworld_alignment_errors().is_empty()
    );
    println!(
        "  initialized_planet_ownership={}",
        data.current_known_initialized_planet_ownership_errors().is_empty()
    );
    println!(
        "  homeworld_seed_payloads={}",
        data.current_known_homeworld_seed_payload_errors().is_empty()
    );
    println!(
        "  unowned_planet_payloads={}",
        data.current_known_unowned_planet_payload_errors().is_empty()
    );
    println!(
        "  empty_auxiliary_state={}",
        data.current_known_empty_auxiliary_state_errors().is_empty()
    );
    println!(
        "  initialized_fleet_block_head_ids={:?}",
        initialized_fleet_block_head_ids
    );
    println!("  player1_starbase_count={}", starbase_total);
    println!("  player1_owned_base_record_count={}", owned_base_total);
    println!("  player1_ipbm_count={}", ipbm_total);

    for (idx, record) in data.player.records.iter().enumerate() {
        println!(
            "  player {:02}: owned_planet_count={} homeworld_seed_coords={:?} starbase_count={} owned_base_count={} ipbm_count={} fleet_chain_head={}",
            idx + 1,
            player_owned_planet_counts[idx],
            player_homeworld_seed_coords.get(idx).copied().flatten(),
            player_starbase_counts[idx],
            player_owned_base_counts[idx],
            record.ipbm_count_raw(),
            player_fleet_chain_heads[idx]
        );
    }

    Ok(())
}

pub(crate) fn validate_core_state(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = CoreGameData::load(dir)?;
    let errors = data.current_known_core_state_errors();

    if errors.is_empty() {
        println!("Valid core state");
        println!("  base_record_count = {}", data.bases.records.len());
        println!(
            "  player1_starbase_count = {}",
            data.player1_starbase_count_current_known()
        );
        println!(
            "  player1_owned_base_record_count = {}",
            data.player1_owned_base_record_count_current_known()
        );
        println!("  ipbm_record_count = {}", data.ipbm.records.len());
        println!(
            "  player1_ipbm_count = {}",
            data.player1_ipbm_count_current_known()
        );
        println!(
            "  initialized_fleet_blocks = {}",
            data.looks_like_initialized_fleet_blocks_current_known()
        );
        println!(
            "  initialized_fleet_payloads = {}",
            data.current_known_initialized_fleet_payload_errors().is_empty()
        );
        println!(
            "  initialized_fleet_missions = {}",
            data.current_known_initialized_fleet_mission_errors().is_empty()
        );
        println!(
            "  initialized_homeworld_alignment = {}",
            data.current_known_initialized_homeworld_alignment_errors().is_empty()
        );
        println!(
            "  initialized_planet_ownership = {}",
            data.current_known_initialized_planet_ownership_errors().is_empty()
        );
        println!(
            "  homeworld_seed_payloads = {}",
            data.current_known_homeworld_seed_payload_errors().is_empty()
        );
        println!(
            "  unowned_planet_payloads = {}",
            data.current_known_unowned_planet_payload_errors().is_empty()
        );
        println!(
            "  empty_auxiliary_state = {}",
            data.current_known_empty_auxiliary_state_errors().is_empty()
        );
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}

pub(crate) fn sync_core_counts(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut data = CoreGameData::load(dir)?;
    data.sync_player1_current_known_counts();

    data.save(dir)?;

    println!("Core counts synchronized");
    println!(
        "  player1_starbase_count = {}",
        data.player1_starbase_count_current_known()
    );
    println!(
        "  player1_owned_base_record_count = {}",
        data.player1_owned_base_record_count_current_known()
    );
    println!(
        "  player1_ipbm_count = {}",
        data.player1_ipbm_count_current_known()
    );
    println!(
        "  initialized_fleet_blocks = {}",
        data.looks_like_initialized_fleet_blocks_current_known()
    );
    println!(
        "  initialized_fleet_payloads = {}",
        data.current_known_initialized_fleet_payload_errors().is_empty()
    );
    println!(
        "  initialized_fleet_missions = {}",
        data.current_known_initialized_fleet_mission_errors().is_empty()
    );
    println!(
        "  initialized_homeworld_alignment = {}",
        data.current_known_initialized_homeworld_alignment_errors().is_empty()
    );
    println!(
        "  initialized_planet_ownership = {}",
        data.current_known_initialized_planet_ownership_errors().is_empty()
    );
    println!(
        "  homeworld_seed_payloads = {}",
        data.current_known_homeworld_seed_payload_errors().is_empty()
    );
    println!(
        "  unowned_planet_payloads = {}",
        data.current_known_unowned_planet_payload_errors().is_empty()
    );
    println!(
        "  empty_auxiliary_state = {}",
        data.current_known_empty_auxiliary_state_errors().is_empty()
    );
    for (idx, (starbase_count, owned_base_count)) in data
        .player_starbase_counts_current_known()
        .into_iter()
        .zip(data.player_owned_base_record_counts_current_known())
        .enumerate()
    {
        let fleet_chain_head = data.player_fleet_chain_heads_current_known()[idx];
        println!(
            "  player {:02}: owned_planet_count = {} homeworld_seed_coords = {:?} starbase_count = {} owned_base_count = {} fleet_chain_head = {}",
            idx + 1,
            data.player_owned_planet_counts_current_known()[idx],
            data.player_homeworld_seed_coords_current_known()
                .get(idx)
                .copied()
                .flatten(),
            starbase_count,
            owned_base_count,
            fleet_chain_head
        );
    }
    Ok(())
}
