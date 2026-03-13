mod common;

use common::{cleanup_dir, repo_root, run_ec_cli, run_ec_cli_in_dir, unique_temp_dir};
use std::fs;

#[test]
fn scenario_invade_recreates_known_valid_pre_fixture() {
    let target = unique_temp_dir("ec-cli-scenario-invade");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let stdout = run_ec_cli_in_dir(
        &["scenario", target.to_str().unwrap(), "invade"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Applied scenario: invade"));

    let fixture_pre = repo_root().join("fixtures/ecmaint-invade-heavy-pre/v1.5");
    for name in ["FLEETS.DAT", "PLANETS.DAT"] {
        let expected = fs::read(fixture_pre.join(name)).unwrap();
        let actual = fs::read(target.join(name)).unwrap();
        assert_eq!(
            actual, expected,
            "{name} does not match preserved invade pre-fixture"
        );
    }

    cleanup_dir(&target);
}

#[test]
fn validate_invade_accepts_known_valid_fixture() {
    let stdout = run_ec_cli(&[
        "validate",
        "fixtures/ecmaint-invade-heavy-pre/v1.5",
        "invade",
    ]);
    assert!(stdout.contains("Valid invade scenario"));
    assert!(stdout.contains("FLEET[3]: order=0x0a (InvadeWorld)"));
    assert!(stdout.contains("PLANET[14]: (15,13) empire=2 armies=142 batteries=15"));
}

#[test]
fn scenario_init_replayable_invade_matches_exact_preserved_pre_fixture() {
    let target = unique_temp_dir("ec-cli-invade-init-replayable");

    let stdout = run_ec_cli_in_dir(
        &[
            "scenario-init-replayable",
            "fixtures/ecmaint-post/v1.5",
            target.to_str().unwrap(),
            "invade",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Replayable scenario directory initialized at"));

    let fixture_pre = repo_root().join("fixtures/ecmaint-invade-heavy-pre/v1.5");
    for name in ["FLEETS.DAT", "PLANETS.DAT", "SETUP.DAT"] {
        let expected = fs::read(fixture_pre.join(name)).unwrap();
        let actual = fs::read(target.join(name)).unwrap();
        assert_eq!(
            actual, expected,
            "{name} does not match preserved invade pre-fixture"
        );
    }

    cleanup_dir(&target);
}

#[test]
fn invade_init_recreates_known_valid_pre_fixture() {
    let target = unique_temp_dir("ec-cli-invade-init-param");

    // Recreate the fixture parameters: (15,13) target, SC=100, BB=100, CA=50, DD=50, TT=50, armies=100
    let stdout = run_ec_cli_in_dir(
        &[
            "invade-init",
            "fixtures/ecmaint-post/v1.5",
            target.to_str().unwrap(),
            "15",
            "13",
            "100",
            "100",
            "50",
            "50",
            "50",
            "100",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Invade directory initialized at"));

    let fixture_pre = repo_root().join("fixtures/ecmaint-invade-heavy-pre/v1.5");
    for name in ["FLEETS.DAT", "PLANETS.DAT"] {
        let expected = fs::read(fixture_pre.join(name)).unwrap();
        let actual = fs::read(target.join(name)).unwrap();
        assert_eq!(
            actual, expected,
            "{name} does not match preserved invade pre-fixture via invade-init"
        );
    }

    cleanup_dir(&target);
}
