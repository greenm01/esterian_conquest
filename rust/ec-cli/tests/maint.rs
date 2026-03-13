mod common;

use common::{cleanup_dir, copy_fixture_dir, run_ec_cli_in_dir, unique_temp_dir};
use ec_data::{CoreGameData, DatabaseDat};
use std::fs;

#[test]
fn maint_rust_econ_updates_database_owner_intel_from_post_combat_planet_state() {
    let target = unique_temp_dir("ec-cli-maint-rust-econ");
    copy_fixture_dir("fixtures/ecmaint-econ-pre/v1.5", &target);

    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Running Rust maintenance on:"));
    assert!(stdout.contains("Rust maintenance complete."));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");

    let (planet_idx, planet) = game_data
        .planets
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.coords_raw() == [15, 13])
        .expect("econ combat target should exist");
    let year_bytes = (game_data.conquest.game_year() - 1).to_le_bytes();
    let owner_player = planet.owner_empire_slot_raw().saturating_sub(1) as usize;
    let planet_name = planet.planet_name();

    let owner_record = database.record(planet_idx, owner_player);
    assert_eq!(owner_record.planet_name_bytes(), planet_name.as_bytes());
    assert_eq!(owner_record.raw[0x15], planet.owner_empire_slot_raw());
    assert_eq!(owner_record.raw[0x16], year_bytes[0]);
    assert_eq!(owner_record.raw[0x17], year_bytes[1]);
    assert_eq!(owner_record.raw[0x1e], 0x40 + planet.owner_empire_slot_raw());
    assert_eq!(owner_record.raw[0x23], planet.army_count_raw());
    assert_eq!(owner_record.raw[0x25], planet.ground_batteries_raw());

    let unrelated_player = (owner_player + 1) % 4;
    let unrelated_record = database.record(planet_idx, unrelated_player);
    assert_eq!(unrelated_record.planet_name_bytes(), b"UNKNOWN");
    assert_eq!(unrelated_record.raw[0x15], 0xff);

    cleanup_dir(&target);
}

#[test]
fn maint_rust_fleet_battle_generates_results_report_from_battle_events() {
    let target = unique_temp_dir("ec-cli-maint-rust-fleet-battle");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);

    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    assert!(!results.is_empty(), "RESULTS.DAT should contain battle summaries");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Fleet battle at System("));

    cleanup_dir(&target);
}
