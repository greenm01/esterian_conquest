use std::path::Path;

use ec_data::CoreGameData;

pub(crate) fn print_core_report(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = CoreGameData::load(dir)?;
    let starbase_total = player1_starbases(&data);
    let ipbm_total = player1_ipbm(&data);

    println!("Core State Report");
    println!("  dir={}", dir.display());
    println!("  player_record_count={}", data.player.records.len());
    println!("  planet_record_count={}", data.planets.records.len());
    println!("  fleet_record_count={}", data.fleets.records.len());
    println!("  base_record_count={}", data.bases.records.len());
    println!("  ipbm_record_count={}", data.ipbm.records.len());
    println!("  conquest.game_year={}", data.conquest.game_year());
    println!("  player1_starbase_count={}", starbase_total);
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
    let errors = core_state_errors(&data);

    if errors.is_empty() {
        println!("Valid core state");
        println!("  base_record_count = {}", data.bases.records.len());
        println!("  player1_starbase_count = {}", player1_starbases(&data));
        println!("  ipbm_record_count = {}", data.ipbm.records.len());
        println!("  player1_ipbm_count = {}", player1_ipbm(&data));
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}

pub(crate) fn core_state_errors(data: &CoreGameData) -> Vec<String> {
    let mut errors = Vec::new();
    let starbase_total = player1_starbases(data);
    let ipbm_total = player1_ipbm(data);

    if data.bases.records.len() != starbase_total {
        errors.push(format!(
            "BASES.DAT record count expected {}, got {}",
            starbase_total,
            data.bases.records.len()
        ));
    }

    if data.ipbm.records.len() != ipbm_total {
        errors.push(format!(
            "IPBM.DAT record count expected {}, got {}",
            ipbm_total,
            data.ipbm.records.len()
        ));
    }

    errors
}

fn player1_starbases(data: &CoreGameData) -> usize {
    data.player
        .records
        .first()
        .map(|record| record.starbase_count_raw() as usize)
        .unwrap_or(0)
}

fn player1_ipbm(data: &CoreGameData) -> usize {
    data.player
        .records
        .first()
        .map(|record| record.ipbm_count_raw() as usize)
        .unwrap_or(0)
}
