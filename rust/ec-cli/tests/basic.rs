mod common;

use common::{run_ec_cli, run_ec_cli_in_dir, unique_temp_dir, cleanup_dir};
use std::fs;

#[test]
fn match_identifies_original_fixture() {
    let stdout = run_ec_cli(&["match", "original/v1.5"]);
    assert!(stdout.contains("MATCH original/v1.5"));
}

#[test]
fn match_identifies_initialized_fixture() {
    let stdout = run_ec_cli(&["match", "fixtures/ecutil-init/v1.5"]);
    assert!(stdout.contains("MATCH fixtures/ecutil-init/v1.5"));
}

#[test]
fn scenario_list_prints_known_scenarios() {
    let stdout = run_ec_cli(&["scenario", "original/v1.5", "list"]);
    assert!(stdout.contains("Known scenarios:"));
    assert!(stdout.contains("fleet-order: accepted fleet movement/order fixture"));
    assert!(stdout.contains("planet-build: accepted planet build-queue fixture"));
    assert!(stdout.contains("guard-starbase: accepted one-base guard-starbase fixture"));
}

#[test]
fn scenario_show_prints_fixture_metadata() {
    let stdout = run_ec_cli(&["scenario", "original/v1.5", "show", "guard-starbase"]);
    assert!(stdout.contains("Scenario: guard-starbase"));
    assert!(stdout.contains("Description: accepted one-base guard-starbase fixture"));
    assert!(stdout.contains("fixtures/ecmaint-starbase-pre/v1.5"));
    assert!(stdout.contains("PLAYER.DAT"));
    assert!(stdout.contains("FLEETS.DAT"));
    assert!(stdout.contains("BASES.DAT"));
}

#[test]
fn headers_prints_known_setup_and_conquest_values() {
    let stdout = run_ec_cli(&["headers", "original/v1.5"]);
    assert!(stdout.contains("SETUP.version=EC151"));
    assert!(stdout.contains("SETUP.option_prefix=[04, 03, 04, 03, 01, 01, 01, 01]"));
    assert!(stdout.contains("SETUP.com_irqs=[4, 3, 4, 3]"));
    assert!(stdout.contains("SETUP.com_flow_control=[true, true, true, true]"));
    assert!(stdout.contains("SETUP.snoop_enabled=true"));
    assert!(stdout.contains("SETUP.local_timeout_enabled=false"));
    assert!(stdout.contains("SETUP.remote_timeout_enabled=true"));
    assert!(stdout.contains("SETUP.max_time_between_keys_minutes_raw=10"));
    assert!(stdout.contains("SETUP.minimum_time_granted_minutes_raw=0"));
    assert!(stdout.contains("SETUP.purge_after_turns_raw=0"));
    assert!(stdout.contains("SETUP.autopilot_inactive_turns_raw=0"));
    assert!(stdout.contains("CONQUEST.game_year=3022"));
    assert!(stdout.contains("CONQUEST.player_count=4"));
    assert!(stdout.contains("CONQUEST.player_config_word=0104"));
    assert!(stdout.contains("CONQUEST.maintenance_schedule=[01, 01, 01, 01, 01, 01, 01]"));
    assert!(stdout.contains("CONQUEST.header_len=85"));
    assert!(stdout.contains("0bce"));
}

#[test]
fn headers_accepts_relative_fixture_paths_from_rust_workspace() {
    let stdout = run_ec_cli(&["headers", "../fixtures/ecutil-init/v1.5"]);
    assert!(stdout.contains("CONQUEST.game_year=3000"));
    assert!(stdout.contains("CONQUEST.player_count=4"));
}

#[test]
fn inspect_summarizes_core_directory_state() {
    let stdout = run_ec_cli(&["inspect", "fixtures/ecmaint-post/v1.5"]);
    assert!(stdout.contains("Directory:"));
    assert!(stdout.contains("Players:"));
    assert!(stdout.contains("Planets:"));
    assert!(stdout.contains("Fleets:"));
    assert!(stdout.contains("Bases:"));
    assert!(stdout.contains("IPBM:"));
}

#[test]
fn compare_reports_expected_initialized_to_post_maint_shape() {
    let stdout = run_ec_cli(&[
        "compare",
        "fixtures/ecutil-init/v1.5",
        "fixtures/ecmaint-post/v1.5",
    ]);
    assert!(stdout.contains("SETUP.DAT: size 522 vs 522, differing bytes 0"));
    assert!(stdout.contains("CONQUEST.DAT: size 2085 vs 2085, differing bytes 51"));
    assert!(stdout.contains("DATABASE.DAT: size 8000 vs 8000, differing bytes 80"));
    assert!(stdout.contains("FLEETS.DAT: size 864 vs 864, differing bytes 0"));
}

#[test]
fn scenario_init_all_materializes_all_known_scenarios() {
    let target = unique_temp_dir("ec-cli-all-scenarios");

    let stdout = run_ec_cli_in_dir(
        &["scenario-init-all", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Initialized all known scenarios under"));

    let manifest = fs::read_to_string(target.join("SCENARIOS.txt")).unwrap();
    assert!(manifest.contains("source="));
    assert!(manifest.contains("fixtures/ecmaint-post/v1.5"));
    assert!(manifest.contains("fleet-order"));
    assert!(manifest.contains("planet-build"));
    assert!(manifest.contains("guard-starbase"));

    let fleet_validate = run_ec_cli_in_dir(
        &["validate", target.join("fleet-order").to_str().unwrap(), "fleet-order"],
        common::rust_workspace(),
    );
    assert!(fleet_validate.contains("Valid fleet-order scenario"));

    let build_validate = run_ec_cli_in_dir(
        &["validate", target.join("planet-build").to_str().unwrap(), "planet-build"],
        common::rust_workspace(),
    );
    assert!(build_validate.contains("Valid planet-build scenario"));

    let starbase_validate = run_ec_cli_in_dir(
        &["validate", target.join("guard-starbase").to_str().unwrap(), "guard-starbase"],
        common::rust_workspace(),
    );
    assert!(starbase_validate.contains("Valid guard-starbase scenario"));

    cleanup_dir(&target);
}

#[test]
fn scenario_init_compose_materializes_combined_directory() {
    let target = unique_temp_dir("ec-cli-scenario-compose");

    let stdout = run_ec_cli_in_dir(
        &[
            "scenario-init-compose",
            target.to_str().unwrap(),
            "fleet-order",
            "planet-build",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Applied scenarios: fleet-order, planet-build"));
    assert!(stdout.contains("Scenario chain directory initialized at"));

    let fleet_validate = run_ec_cli_in_dir(
        &["validate", target.to_str().unwrap(), "fleet-order"],
        common::rust_workspace(),
    );
    assert!(fleet_validate.contains("Valid fleet-order scenario"));

    let build_validate = run_ec_cli_in_dir(
        &["validate", target.to_str().unwrap(), "planet-build"],
        common::rust_workspace(),
    );
    assert!(build_validate.contains("Valid planet-build scenario"));

    cleanup_dir(&target);
}

#[test]
fn core_init_current_known_baseline_materializes_valid_directory() {
    let target = unique_temp_dir("ec-cli-core-init-current-known");

    let stdout = run_ec_cli_in_dir(
        &["core-init-current-known-baseline", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Current-known baseline directory initialized at"));
    assert!(stdout.contains("source snapshot:"));
    assert!(stdout.contains("fixtures/ecmaint-post/v1.5"));
    assert!(stdout.contains("initialized_fleet_blocks = true"));
    assert!(stdout.contains("homeworld_seed_payloads = true"));
    assert!(stdout.contains("setup_baseline = true"));
    assert!(stdout.contains("conquest_baseline = true"));

    let validate_stdout = run_ec_cli_in_dir(
        &["core-validate", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(validate_stdout.contains("Valid core state"));

    cleanup_dir(&target);
}

#[test]
fn core_diff_current_known_baseline_reports_mutated_files() {
    let target = unique_temp_dir("ec-cli-core-diff-current-known");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut data = ec_data::CoreGameData::load(&target).unwrap();
    data.setup.raw[..5].copy_from_slice(b"BAD!!");
    data.planets.records[14].set_planet_tax_rate_raw(3);
    data.save(&target).unwrap();

    let stdout = run_ec_cli_in_dir(
        &["core-diff-current-known-baseline", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Current-known Baseline Diff"));
    assert!(stdout.contains("SETUP.DAT: differing_bytes="));
    assert!(stdout.contains("PLANETS.DAT: differing_bytes="));

    cleanup_dir(&target);
}

#[test]
fn core_diff_current_known_baseline_offsets_reports_mutated_offsets() {
    let target = unique_temp_dir("ec-cli-core-diff-current-known-offsets");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut data = ec_data::CoreGameData::load(&target).unwrap();
    data.setup.raw[..5].copy_from_slice(b"BAD!!");
    data.planets.records[14].set_planet_tax_rate_raw(3);
    data.save(&target).unwrap();

    let stdout = run_ec_cli_in_dir(
        &[
            "core-diff-current-known-baseline-offsets",
            target.to_str().unwrap(),
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Current-known Baseline Diff Offsets"));
    assert!(stdout.contains("SETUP.DAT: differing_offsets=[0, 1, 2, 3, 4"));
    assert!(stdout.contains("PLANETS.DAT: differing_offsets="));

    cleanup_dir(&target);
}
