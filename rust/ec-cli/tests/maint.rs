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
    let messages = fs::read(target.join("MESSAGES.DAT")).expect("MESSAGES.DAT should exist");
    assert!(
        messages.is_empty(),
        "MESSAGES.DAT should remain empty for current canonical maint output"
    );

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
    assert!(text.contains("Fleet battle report"));
    assert!(text.contains("System("));
    let messages = fs::read(target.join("MESSAGES.DAT")).expect("MESSAGES.DAT should exist");
    assert!(
        messages.is_empty(),
        "MESSAGES.DAT should remain empty for current canonical maint output"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_destroyed_fleet_generates_lost_contact_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-lost-contact");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);

    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("We lost all contact with the"));
    assert!(text.contains("Fleet Command Center"));
    assert!(text.contains("flight recorder"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_destroyed_starbase_generates_lost_contact_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-lost-starbase");
    copy_fixture_dir("fixtures/ecmaint-starbase-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let starbase_coords = game_data.bases.records[0].coords_raw();
    let attacker = &mut game_data.fleets.records[4];
    attacker.set_current_location_coords_raw(starbase_coords);
    attacker.set_standing_order_code_raw(1);
    attacker.set_standing_order_target_coords_raw(starbase_coords);
    attacker.set_current_speed(0);
    attacker.raw[0x19] = 0x81;
    attacker.set_rules_of_engagement(10);
    attacker.set_destroyer_count(20);
    attacker.set_cruiser_count(10);
    attacker.set_battleship_count(5);
    attacker.set_scout_count(0);
    attacker.set_troop_transport_count(0);
    attacker.set_etac_count(0);
    game_data.save(&target).expect("mutated fixture should save");

    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("From Starbase"));
    assert!(text.contains("alerting all fleets"));
    assert!(text.contains("We lost all contact with Starbase"));
    assert!(text.contains("burnt flight recorder"));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    assert_eq!(game_data.player.records[0].starbase_count_raw(), 0);
    assert!(
        game_data
            .bases
            .records
            .iter()
            .all(|base| !(base.coords_raw() == starbase_coords && base.owner_empire_raw() == 1 && base.active_flag_raw() != 0))
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_colonization_generates_results_report_from_colony_event() {
    let target = unique_temp_dir("ec-cli-maint-rust-colonize");
    copy_fixture_dir("fixtures/ecmaint-fleet-pre/v1.5", &target);

    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    assert!(
        !results.is_empty(),
        "RESULTS.DAT should contain colonization summaries"
    );
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("From colony mission in System("));
    assert!(text.contains("successfully established"));
    assert!(text.contains("Not Named Yet"));
    let messages = fs::read(target.join("MESSAGES.DAT")).expect("MESSAGES.DAT should exist");
    assert!(
        messages.is_empty(),
        "MESSAGES.DAT should remain empty for current canonical maint output"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_colonization_blocked_by_owner_generates_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-colonize-blocked");
    copy_fixture_dir("fixtures/ecmaint-fleet-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let blocked = &mut game_data.planets.records[13];
    blocked.set_owner_empire_slot_raw(2);
    blocked.set_ownership_status_raw(2);
    blocked.set_planet_name("TargetPrime");
    blocked.set_army_count_raw(10);
    game_data.save(&target).expect("mutated fixture should save");

    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    assert!(
        !results.is_empty(),
        "RESULTS.DAT should contain blocked colonization summaries"
    );
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("From colony mission in System("));
    assert!(text.contains("ot establish a colony on planet"));
    assert!(text.contains("already occupie"));
    assert!(text.contains("TargetPrime"));
    assert!(text.contains("Empire #2"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_scout_sector_generates_results_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-scout-sector");
    copy_fixture_dir("fixtures/ecmaint-fleet-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let scout = &mut game_data.fleets.records[0];
    scout.set_standing_order_code_raw(10);
    scout.set_standing_order_target_coords_raw([15, 13]);
    scout.set_scout_count(1);
    scout.set_etac_count(0);
    game_data.save(&target).expect("mutated fixture should save");

    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    assert!(!results.is_empty(), "RESULTS.DAT should contain scout summaries");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Scouting mission report"));
    assert!(text.contains("beginning to scout this sector"));
    assert!(text.contains("Sector(15,13)"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_scout_system_generates_results_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-scout-system");
    copy_fixture_dir("fixtures/ecmaint-fleet-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let scout = &mut game_data.fleets.records[0];
    scout.set_standing_order_code_raw(11);
    scout.set_standing_order_target_coords_raw([15, 13]);
    scout.set_scout_count(1);
    scout.set_etac_count(0);
    game_data.save(&target).expect("mutated fixture should save");

    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    assert!(!results.is_empty(), "RESULTS.DAT should contain scout summaries");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Scouting mission report"));
    assert!(text.contains("Owner:"));
    assert!(text.contains("Ground batteries:"));
    assert!(text.contains("System(15,13)"));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let viewer_record = database.record(13, 0);
    assert_eq!(
        viewer_record.planet_name_bytes(),
        game_data.planets.records[13].planet_name().as_bytes()
    );
    assert_eq!(viewer_record.raw[0x15], game_data.planets.records[13].owner_empire_slot_raw());

    cleanup_dir(&target);
}

#[test]
fn maint_rust_view_world_generates_results_and_database_intel() {
    let target = unique_temp_dir("ec-cli-maint-rust-view-world");
    copy_fixture_dir("fixtures/ecmaint-fleet-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let viewer = &mut game_data.fleets.records[0];
    viewer.set_standing_order_code_raw(9);
    viewer.set_standing_order_target_coords_raw([15, 13]);
    viewer.set_scout_count(0);
    viewer.set_etac_count(0);
    game_data.save(&target).expect("mutated fixture should save");

    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Viewing mission report"));
    assert!(text.contains("long range"));
    assert!(text.contains("potential"));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let viewer_record = database.record(13, 0);
    assert_eq!(
        viewer_record.planet_name_bytes(),
        game_data.planets.records[13].planet_name().as_bytes()
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_guard_starbase_generates_arrival_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-guard-starbase");
    copy_fixture_dir("fixtures/ecmaint-starbase-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let coords = game_data.bases.records[0].coords_raw();
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw([coords[0].saturating_sub(1), coords[1]]);
    fleet.set_standing_order_code_raw(4);
    fleet.set_standing_order_target_coords_raw(coords);
    fleet.set_current_speed(3);
    game_data.save(&target).expect("mutated fixture should save");

    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Guard Starbase mission report"));
    assert!(text.contains("guard/escort"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_guard_blockade_generates_arrival_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-guard-blockade");
    copy_fixture_dir("fixtures/ecmaint-fleet-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let guard = &mut game_data.fleets.records[0];
    guard.set_standing_order_code_raw(5);
    guard.set_standing_order_target_coords_raw([15, 13]);
    guard.set_scout_count(0);
    guard.set_etac_count(0);
    game_data.save(&target).expect("mutated fixture should save");

    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Guard/Blockade World mission report"));
    assert!(text.contains("arrived at planet"));
    assert!(text.contains("assignment"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_bombardment_generates_attacker_side_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-bombard-report");
    copy_fixture_dir("fixtures/ecmaint-bombard-arrive/v1.5", &target);

    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Bombardment mission report"));
    assert!(text.contains("bombing run"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_invade_failure_generates_attacker_side_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-invade-report");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let target_world = &mut game_data.planets.records[13];
    target_world.set_as_owned_target_world(
        [15, 13],
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        40,
        0,
        0,
        2,
    );
    let attacker = &mut game_data.fleets.records[0];
    attacker.set_current_location_coords_raw([15, 13]);
    attacker.set_standing_order_code_raw(7);
    attacker.set_standing_order_target_coords_raw([15, 13]);
    attacker.set_current_speed(3);
    attacker.raw[0x19] = 0x80;
    attacker.set_rules_of_engagement(10);
    attacker.set_scout_count(0);
    attacker.set_battleship_count(0);
    attacker.set_cruiser_count(0);
    attacker.set_destroyer_count(1);
    attacker.set_troop_transport_count(2);
    attacker.set_army_count(2);
    attacker.set_etac_count(0);
    game_data.save(&target).expect("mutated fixture should save");

    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Invasion mission report"));
    assert!(text.contains("repulsed") || text.contains("landing was"));
    assert!(text.contains("Friendly losses:"));
    assert!(text.contains("Enemy losses:"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_blitz_success_generates_attacker_side_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-blitz-report");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let target_world = &mut game_data.planets.records[13];
    target_world.set_as_owned_target_world(
        [15, 13],
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        1,
        1,
        0,
        2,
    );
    let attacker = &mut game_data.fleets.records[0];
    attacker.set_current_location_coords_raw([15, 13]);
    attacker.set_standing_order_code_raw(8);
    attacker.set_standing_order_target_coords_raw([15, 13]);
    attacker.set_current_speed(3);
    attacker.raw[0x19] = 0x80;
    attacker.set_rules_of_engagement(10);
    attacker.set_scout_count(0);
    attacker.set_battleship_count(0);
    attacker.set_cruiser_count(0);
    attacker.set_destroyer_count(1);
    attacker.set_troop_transport_count(10);
    attacker.set_army_count(30);
    attacker.set_etac_count(0);
    game_data.save(&target).expect("mutated fixture should save");

    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Blitz mission report"));
    assert!(text.contains("Friendly losses:"));
    assert!(text.contains("Enemy losses:"));
    assert!(text.contains("during the landing"));
    assert_eq!(text.matches("Blitz mission report").count(), 1);

    cleanup_dir(&target);
}

#[test]
fn maint_rust_battle_abort_generates_move_abort_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-battle-abort");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.fleets.records[0].set_standing_order_code_raw(1);
    game_data.save(&target).expect("mutated fixture should save");

    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Move mission report"));
    assert!(text.contains("abort our mission") || text.contains("abort our"));
    assert!(text.contains("seek safety"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_battle_abort_scout_report_mentions_retreat_destination() {
    let target = unique_temp_dir("ec-cli-maint-rust-scout-abort");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.fleets.records[0].set_standing_order_code_raw(10);
    game_data.save(&target).expect("mutated fixture should save");

    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Scouting mission report"));
    assert!(text.contains("Sensor contact") || text.contains("contact shows"));
    assert!(text.contains("identified the alien fleet") || text.contains("located and ident"));
    assert!(text.contains("withdraw toward") || text.contains("seeking safety"));
    assert!(text.contains("planet \"") || text.contains("System("));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_rendezvous_arrival_generates_waiting_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-rendezvous-wait");
    copy_fixture_dir("fixtures/ecmaint-fleet-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_standing_order_code_raw(14);
    fleet.set_standing_order_target_coords_raw([15, 13]);
    game_data.save(&target).expect("mutated fixture should save");

    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Rendezvous mission report"));
    assert!(text.contains("waiting for more fleets"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_join_merge_generates_join_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-join-merge");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.player.records[0].raw[0x00] = 0xff;
    let coords = game_data.fleets.records[0].current_location_coords_raw();
    game_data.fleets.records[1].set_current_location_coords_raw(coords);
    game_data.fleets.records[1].set_standing_order_code_raw(13);
    game_data.save(&target).expect("mutated fixture should save");

    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Join mission report"));
    assert!(text.contains("now merging"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_rendezvous_merge_generates_absorbing_report() {
    let target = unique_temp_dir("ec-cli-maint-rust-rendezvous-absorb");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.player.records[0].raw[0x00] = 0xff;
    let coords = game_data.fleets.records[0].current_location_coords_raw();
    game_data.fleets.records[0].set_standing_order_code_raw(14);
    game_data.fleets.records[1].set_current_location_coords_raw(coords);
    game_data.fleets.records[1].set_standing_order_code_raw(14);
    game_data.save(&target).expect("mutated fixture should save");

    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Rendezvous mission report"));
    assert!(text.contains("absorbing the") || text.contains("merging with the"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_join_contact_uses_join_report_label() {
    let target = unique_temp_dir("ec-cli-maint-rust-join-contact");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.fleets.records[0].set_standing_order_code_raw(13);
    game_data.save(&target).expect("mutated fixture should save");

    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Join mission report"));
    assert!(text.contains("Sensor contact") || text.contains("contact shows"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_guard_contact_uses_guard_report_label() {
    let target = unique_temp_dir("ec-cli-maint-rust-guard-contact");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.fleets.records[0].set_standing_order_code_raw(5);
    game_data.save(&target).expect("mutated fixture should save");

    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Guard/Blockade World mission report"));
    assert!(text.contains("Sensor contact") || text.contains("contact shows"));

    cleanup_dir(&target);
}
