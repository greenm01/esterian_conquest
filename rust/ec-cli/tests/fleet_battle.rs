mod common;

use common::{cleanup_dir, repo_root, run_ec_cli, run_ec_cli_in_dir, unique_temp_dir};
use std::fs;

#[test]
fn scenario_fleet_battle_recreates_known_valid_pre_fixture() {
    let target = unique_temp_dir("ec-cli-scenario-fleet-battle");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let stdout = run_ec_cli_in_dir(
        &["scenario", target.to_str().unwrap(), "fleet-battle"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Applied scenario: fleet-battle"));

    let fixture_pre = repo_root().join("fixtures/ecmaint-fleet-battle-pre/v1.5");
    for name in ["FLEETS.DAT", "PLANETS.DAT"] {
        let expected = fs::read(fixture_pre.join(name)).unwrap();
        let actual = fs::read(target.join(name)).unwrap();
        assert_eq!(
            actual, expected,
            "{name} does not match preserved fleet-battle pre-fixture"
        );
    }

    cleanup_dir(&target);
}

#[test]
fn validate_fleet_battle_accepts_known_valid_fixture() {
    let stdout = run_ec_cli(&[
        "validate",
        "fixtures/ecmaint-fleet-battle-pre/v1.5",
        "fleet-battle",
    ]);
    assert!(stdout.contains("Valid fleet-battle scenario"));
    assert!(stdout.contains("FLEET[1]: loc=(10,10)"));
    assert!(stdout.contains("FLEET[9]: order=0x01 (MoveOnly)"));
    assert!(stdout.contains("PLANET[14]: (15,13) empire=2 armies=142 batteries=15"));
}

#[test]
fn scenario_init_replayable_fleet_battle_matches_exact_preserved_pre_fixture() {
    let target = unique_temp_dir("ec-cli-fleet-battle-init-replayable");

    let stdout = run_ec_cli_in_dir(
        &[
            "scenario-init-replayable",
            "fixtures/ecmaint-post/v1.5",
            target.to_str().unwrap(),
            "fleet-battle",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Replayable scenario directory initialized at"));

    let fixture_pre = repo_root().join("fixtures/ecmaint-fleet-battle-pre/v1.5");
    for name in ["FLEETS.DAT", "PLANETS.DAT", "SETUP.DAT"] {
        let expected = fs::read(fixture_pre.join(name)).unwrap();
        let actual = fs::read(target.join(name)).unwrap();
        assert_eq!(
            actual, expected,
            "{name} does not match preserved fleet-battle pre-fixture"
        );
    }

    cleanup_dir(&target);
}
