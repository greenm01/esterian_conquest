mod common;

use common::{cleanup_dir, repo_root, run_ec_cli, run_ec_cli_in_dir, unique_temp_dir};
use std::fs;

#[test]
fn scenario_bombard_recreates_known_valid_bombard_pre_fixture() {
    let target = unique_temp_dir("ec-cli-scenario-bombard");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let stdout = run_ec_cli_in_dir(
        &["scenario", target.to_str().unwrap(), "bombard"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Applied scenario: bombard"));

    let fixture_pre = repo_root().join("fixtures/ecmaint-bombard-pre/v1.5");
    for name in ["FLEETS.DAT", "PLANETS.DAT"] {
        let expected = fs::read(fixture_pre.join(name)).unwrap();
        let actual = fs::read(target.join(name)).unwrap();
        assert_eq!(
            actual, expected,
            "{name} does not match preserved bombard pre-fixture"
        );
    }

    cleanup_dir(&target);
}

#[test]
fn validate_bombard_accepts_known_valid_fixture() {
    let stdout = run_ec_cli(&["validate", "fixtures/ecmaint-bombard-pre/v1.5", "bombard"]);
    assert!(stdout.contains("Valid bombard scenario"));
    assert!(stdout.contains("FLEET[3].order = 0x06 (BombardWorld)"));
    assert!(stdout.contains("PLANET[14]: homeworld seed empire=2 armies=10 batteries=4"));
}

#[test]
fn scenario_init_replayable_bombard_matches_exact_preserved_pre_fixture() {
    let target = unique_temp_dir("ec-cli-bombard-init-replayable");

    let stdout = run_ec_cli_in_dir(
        &[
            "scenario-init-replayable",
            "fixtures/ecmaint-post/v1.5",
            target.to_str().unwrap(),
            "bombard",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Replayable scenario directory initialized at"));

    let fixture_pre = repo_root().join("fixtures/ecmaint-bombard-pre/v1.5");
    for name in [
        "PLAYER.DAT",
        "PLANETS.DAT",
        "FLEETS.DAT",
        "SETUP.DAT",
        "CONQUEST.DAT",
        "DATABASE.DAT",
    ] {
        let expected = fs::read(fixture_pre.join(name)).unwrap();
        let actual = fs::read(target.join(name)).unwrap();
        assert_eq!(
            actual, expected,
            "{name} does not match preserved bombard pre-fixture"
        );
    }

    cleanup_dir(&target);
}
