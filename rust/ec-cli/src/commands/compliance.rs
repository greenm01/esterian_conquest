use std::fs;
use std::path::Path;

use ec_data::CoreGameData;

use crate::commands::guard_starbase::validate_guard_starbase_scenario;
use crate::commands::ipbm::validate_ipbm;

pub(crate) fn print_compliance_report(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Compliance Report");
    println!("  dir={}", dir.display());
    println!();

    match validate_guard_starbase_scenario(dir) {
        Ok(()) => println!("OK   guard-starbase-linkage"),
        Err(err) => println!("FAIL guard-starbase-linkage: {err}"),
    }

    match validate_ipbm(dir) {
        Ok(()) => println!("OK   ipbm-count-length"),
        Err(err) => println!("FAIL ipbm-count-length: {err}"),
    }

    let data = CoreGameData::load(dir)?;
    let player1 = &data.player.records[0];
    println!();
    println!(
        "Key words: player.starbase_count={} player.ipbm_count={}",
        player1.starbase_count_raw(),
        player1.ipbm_count_raw()
    );
    if let Some(fleet1) = data.fleets.records.first() {
        println!(
            "  fleet1.local_slot={} fleet1.id={} fleet1.guard={}/{} target={:?}",
            fleet1.local_slot_word_raw(),
            fleet1.fleet_id_word_raw(),
            fleet1.guard_starbase_index_raw(),
            fleet1.guard_starbase_enable_raw(),
            fleet1.standing_order_target_coords_raw()
        );
    }
    if let Some(base1) = data.bases.records.first() {
        println!(
            "  base1.summary={} base1.id={} base1.chain={} coords={:?}",
            base1.summary_word_raw(),
            base1.base_id_raw(),
            base1.chain_word_raw(),
            base1.coords_raw()
        );
    } else {
        println!("  base1=<none>");
    }
    println!("  ipbm.record_count={}", data.ipbm.records.len());
    Ok(())
}

pub(crate) fn print_compliance_batch_report(
    root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Compliance Batch Report");
    println!("  root={}", root.display());
    let mut dirs = fs::read_dir(root)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            entry.file_type().ok().filter(|ty| ty.is_dir())?;
            Some(entry.path())
        })
        .collect::<Vec<_>>();
    dirs.sort();

    for dir in dirs {
        print!("{}: ", dir.file_name().unwrap_or_default().to_string_lossy());
        let guard_ok = match CoreGameData::load(&dir) {
            Ok(data) => data.guard_starbase_onebase_errors_current_known().is_empty(),
            _ => false,
        };
        let ipbm_ok = match CoreGameData::load(&dir) {
            Ok(data) => data.ipbm_count_length_errors_current_known().is_empty(),
            _ => false,
        };
        let fleet_ok = match CoreGameData::load(&dir) {
            Ok(data) => data
                .fleet_order_errors_current_known(1, 0x03, 0x0C, [0x0F, 0x0D], None, None)
                .is_empty(),
            _ => false,
        };
        let build_ok = match CoreGameData::load(&dir) {
            Ok(data) => data
                .planet_build_errors_current_known(15, 0x03, 0x01)
                .is_empty(),
            _ => false,
        };
        println!(
            "fleet-order={} planet-build={} guard-starbase={} ipbm={}",
            if fleet_ok { "ok" } else { "fail" },
            if build_ok { "ok" } else { "fail" },
            if guard_ok { "ok" } else { "fail" },
            if ipbm_ok { "ok" } else { "fail" }
        );
    }

    Ok(())
}
