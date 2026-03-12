use std::path::Path;

use ec_data::CoreGameData;

pub(crate) fn print_core_report(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = CoreGameData::load(dir)?;
    let starbase_total = data.player1_starbase_count_current_known();
    let owned_base_total = data.player1_owned_base_record_count_current_known();
    let ipbm_total = data.player1_ipbm_count_current_known();

    println!("Core State Report");
    println!("  dir={}", dir.display());
    println!("  player_record_count={}", data.player.records.len());
    println!("  planet_record_count={}", data.planets.records.len());
    println!("  fleet_record_count={}", data.fleets.records.len());
    println!("  base_record_count={}", data.bases.records.len());
    println!("  ipbm_record_count={}", data.ipbm.records.len());
    println!("  conquest.game_year={}", data.conquest.game_year());
    println!("  player1_starbase_count={}", starbase_total);
    println!("  player1_owned_base_record_count={}", owned_base_total);
    println!("  player1_ipbm_count={}", ipbm_total);

    for (idx, record) in data.player.records.iter().enumerate() {
        println!(
            "  player {:02}: starbase_count={} ipbm_count={} fleet_chain_head={}",
            idx + 1,
            record.starbase_count_raw(),
            record.ipbm_count_raw(),
            record.fleet_chain_head_raw()
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
    Ok(())
}
