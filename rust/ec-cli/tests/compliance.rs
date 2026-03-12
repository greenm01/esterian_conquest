mod common;

use common::{cleanup_dir, run_ec_cli, run_ec_cli_in_dir, unique_temp_dir};
use ec_data::CoreGameData;

#[test]
fn compliance_report_summarizes_known_post_fixture_failures() {
    let stdout = run_ec_cli(&["compliance-report", "fixtures/ecmaint-post/v1.5"]);
    assert!(stdout.contains("Compliance Report"));
    assert!(stdout.contains("FAIL guard-starbase-linkage:"));
    assert!(stdout.contains("OK   ipbm-count-length"));
    assert!(stdout.contains("Key words: player.starbase_count=0 player.ipbm_count=0"));
    assert!(stdout.contains("guarding_fleet_count=0"));
}

#[test]
fn core_report_summarizes_known_post_fixture_counts() {
    let stdout = run_ec_cli(&["core-report", "fixtures/ecmaint-post/v1.5"]);
    assert!(stdout.contains("Core State Report"));
    assert!(stdout.contains("player_record_count=5"));
    assert!(stdout.contains("planet_record_count=20"));
    assert!(stdout.contains("fleet_record_count=16"));
    assert!(stdout.contains("base_record_count=0"));
    assert!(stdout.contains("ipbm_record_count=0"));
    assert!(stdout.contains("initialized_fleet_blocks=true"));
    assert!(stdout.contains("initialized_fleet_payloads=true"));
    assert!(stdout.contains("initialized_fleet_missions=true"));
    assert!(stdout.contains("initialized_homeworld_alignment=true"));
    assert!(stdout.contains("initialized_planet_ownership=true"));
    assert!(stdout.contains("homeworld_seed_payloads=true"));
    assert!(stdout.contains("unowned_planet_payloads=true"));
    assert!(stdout.contains("empty_auxiliary_state=true"));
    assert!(stdout.contains("setup_baseline=true"));
    assert!(stdout.contains("conquest_baseline=true"));
    assert!(stdout.contains("initialized_fleet_block_head_ids=[1, 5, 9, 13]"));
    assert!(stdout.contains("player1_starbase_count=0"));
    assert!(stdout.contains("player1_owned_base_record_count=0"));
    assert!(stdout.contains("player1_ipbm_count=0"));
    assert!(stdout.contains("player 01: owned_planet_count=1 homeworld_seed_coords=Some([16, 13]) starbase_count=0 owned_base_count=0 ipbm_count=0 fleet_chain_head=1"));
}

#[test]
fn core_validate_accepts_known_post_fixture_state() {
    let stdout = run_ec_cli(&["core-validate", "fixtures/ecmaint-post/v1.5"]);
    assert!(stdout.contains("Valid core state"));
    assert!(stdout.contains("base_record_count = 0"));
    assert!(stdout.contains("ipbm_record_count = 0"));
    assert!(stdout.contains("initialized_fleet_blocks = true"));
    assert!(stdout.contains("initialized_fleet_payloads = true"));
    assert!(stdout.contains("initialized_fleet_missions = true"));
    assert!(stdout.contains("initialized_homeworld_alignment = true"));
    assert!(stdout.contains("initialized_planet_ownership = true"));
    assert!(stdout.contains("homeworld_seed_payloads = true"));
    assert!(stdout.contains("unowned_planet_payloads = true"));
    assert!(stdout.contains("empty_auxiliary_state = true"));
    assert!(stdout.contains("setup_baseline = true"));
    assert!(stdout.contains("conquest_baseline = true"));
    assert!(stdout.contains("player1_starbase_count = 0"));
    assert!(stdout.contains("player1_owned_base_record_count = 0"));
    assert!(stdout.contains("player1_ipbm_count = 0"));
}

#[test]
fn core_sync_counts_repairs_player1_count_words() {
    let target = unique_temp_dir("ec-cli-core-sync");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut data = CoreGameData::load(&target).unwrap();
    data.player.records[0].set_starbase_count_raw(3);
    data.player.records[0].set_ipbm_count_raw(2);
    data.save(&target).unwrap();

    let stderr = common::run_ec_cli_failure_in_dir(
        &["core-validate", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(stderr.contains("PLAYER[1]-owned BASES.DAT record count expected 3, got 0"));
    assert!(stderr.contains("IPBM.DAT record count expected 2, got 0"));

    let sync_stdout = run_ec_cli_in_dir(
        &["core-sync-counts", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(sync_stdout.contains("Core counts synchronized"));
    assert!(sync_stdout.contains("player1_starbase_count = 0"));
    assert!(sync_stdout.contains("player1_owned_base_record_count = 0"));
    assert!(sync_stdout.contains("player1_ipbm_count = 0"));
    assert!(sync_stdout.contains("initialized_fleet_blocks = true"));
    assert!(sync_stdout.contains("initialized_fleet_payloads = true"));
    assert!(sync_stdout.contains("initialized_fleet_missions = true"));
    assert!(sync_stdout.contains("initialized_homeworld_alignment = true"));
    assert!(sync_stdout.contains("initialized_planet_ownership = true"));
    assert!(sync_stdout.contains("homeworld_seed_payloads = true"));
    assert!(sync_stdout.contains("unowned_planet_payloads = true"));
    assert!(sync_stdout.contains("empty_auxiliary_state = true"));
    assert!(sync_stdout.contains("setup_baseline = true"));
    assert!(sync_stdout.contains("conquest_baseline = true"));
    assert!(sync_stdout.contains("player 02: owned_planet_count = 1 homeworld_seed_coords = Some([4, 13]) starbase_count = 0 owned_base_count = 0 fleet_chain_head = 25956"));

    let validate_stdout = run_ec_cli_in_dir(
        &["core-validate", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(validate_stdout.contains("Valid core state"));

    cleanup_dir(&target);
}

#[test]
fn core_sync_baseline_repairs_control_and_count_fields() {
    let target = unique_temp_dir("ec-cli-core-sync-baseline");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut data = CoreGameData::load(&target).unwrap();
    data.player.records[0].set_starbase_count_raw(3);
    data.player.records[0].set_ipbm_count_raw(2);
    data.setup.raw[..5].copy_from_slice(b"BAD!!");
    data.setup.set_remote_timeout_enabled(false);
    data.conquest.raw[0..2].copy_from_slice(&2999u16.to_le_bytes());
    data.conquest.raw[2] = 9;
    data.conquest.set_maintenance_schedule_enabled([false; 7]);
    data.save(&target).unwrap();

    let sync_stdout = run_ec_cli_in_dir(
        &["core-sync-baseline", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(sync_stdout.contains("Core baseline synchronized"));
    assert!(sync_stdout.contains("setup_baseline = true"));
    assert!(sync_stdout.contains("conquest_baseline = true"));

    let validate_stdout = run_ec_cli_in_dir(
        &["core-validate", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(validate_stdout.contains("Valid core state"));

    cleanup_dir(&target);
}

#[test]
fn core_sync_initialized_fleets_repairs_fleet_baseline() {
    let target = unique_temp_dir("ec-cli-core-sync-fleets");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut data = CoreGameData::load(&target).unwrap();
    data.fleets.records.clear();
    data.fleets.records.push(ec_data::FleetRecord::new_zeroed());
    data.save(&target).unwrap();

    let sync_stdout = run_ec_cli_in_dir(
        &["core-sync-initialized-fleets", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(sync_stdout.contains("Initialized fleet baseline synchronized"));
    assert!(sync_stdout.contains("initialized_fleet_blocks = true"));
    assert!(sync_stdout.contains("initialized_fleet_payloads = true"));
    assert!(sync_stdout.contains("initialized_fleet_missions = true"));
    assert!(sync_stdout.contains("initialized_homeworld_alignment = true"));

    let validate_stdout = run_ec_cli_in_dir(
        &["core-validate", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(validate_stdout.contains("Valid core state"));

    cleanup_dir(&target);
}

#[test]
fn compliance_report_summarizes_valid_parameterized_guard_starbase_directory() {
    let target = unique_temp_dir("ec-cli-compliance-report");

    run_ec_cli_in_dir(
        &["guard-starbase-init", target.to_str().unwrap(), "12", "9"],
        common::rust_workspace(),
    );

    let stdout = run_ec_cli_in_dir(
        &["compliance-report", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(stdout.contains("OK   guard-starbase-linkage"));
    assert!(stdout.contains("OK   ipbm-count-length"));
    assert!(stdout.contains("guarding_fleet_count=1"));
    assert!(stdout.contains("guarding_fleet[1]: guard_index=1 target=[12, 9]"));
    assert!(stdout.contains("fleet1.local_slot=1 fleet1.id=1"));
    assert!(stdout.contains("base1.summary=1 base1.id=1 base1.chain=1 coords=[12, 9]"));

    cleanup_dir(&target);
}

#[test]
fn compliance_batch_report_summarizes_batch_directory_status() {
    let target = unique_temp_dir("ec-cli-compliance-batch");

    run_ec_cli_in_dir(
        &["guard-starbase-batch-init", target.to_str().unwrap(), "12:9", "14:7"],
        common::rust_workspace(),
    );

    let stdout = run_ec_cli_in_dir(
        &["compliance-batch-report", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Compliance Batch Report"));
    assert!(stdout.contains("x12-y09: fleet-order=fail planet-build=fail guard-starbase=ok ipbm=ok"));
    assert!(stdout.contains("x14-y07: fleet-order=fail planet-build=fail guard-starbase=ok ipbm=ok"));

    cleanup_dir(&target);
}

#[test]
fn validate_preserved_all_classifies_known_build_fixture() {
    let stdout = run_ec_cli(&[
        "validate-preserved",
        "fixtures/ecmaint-build-pre/v1.5",
        "all",
    ]);
    assert!(stdout.contains("OK   planet-build"));
    assert!(stdout.contains("FAIL fleet-order:"));
    assert!(stdout.contains("FAIL guard-starbase:"));
}

#[test]
fn validate_all_classifies_known_fleet_fixture() {
    let stdout = run_ec_cli(&["validate", "fixtures/ecmaint-fleet-pre/v1.5", "all"]);
    assert!(stdout.contains("OK   fleet-order"));
    assert!(stdout.contains("FAIL planet-build:"));
    assert!(stdout.contains("FAIL guard-starbase:"));
}

#[test]
fn validate_all_rejects_post_maint_fixture() {
    let stderr = common::run_ec_cli_failure_in_dir(
        &["validate", "fixtures/ecmaint-post/v1.5", "all"],
        common::rust_workspace(),
    );
    assert!(stderr.contains("directory does not match any known accepted scenario"));
}

#[test]
fn validate_preserved_rejects_post_maint_fixture() {
    let stderr = common::run_ec_cli_failure_in_dir(
        &["validate-preserved", "fixtures/ecmaint-post/v1.5", "all"],
        common::rust_workspace(),
    );
    assert!(stderr.contains("directory does not exactly match any preserved accepted scenario"));
}
