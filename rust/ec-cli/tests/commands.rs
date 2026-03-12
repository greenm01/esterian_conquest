use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run_ec_cli_in_dir(args: &[&str], current_dir: PathBuf) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_ec-cli"))
        .current_dir(current_dir)
        .args(args)
        .output()
        .expect("ec-cli should run");

    assert!(
        output.status.success(),
        "ec-cli failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("stdout should be utf-8")
}

fn run_ec_cli(args: &[&str]) -> String {
    run_ec_cli_in_dir(args, repo_root().join("rust"))
}

fn run_ec_cli_failure_in_dir(args: &[&str], current_dir: PathBuf) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_ec-cli"))
        .current_dir(current_dir)
        .args(args)
        .output()
        .expect("ec-cli should run");

    assert!(
        !output.status.success(),
        "ec-cli unexpectedly succeeded: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stderr).expect("stderr should be utf-8")
}

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
fn maintenance_days_set_rewrites_conquest_schedule() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("ec-cli-maint-{unique}"));
    fs::create_dir_all(&target).unwrap();

    let fixture = repo_root().join("fixtures/ecmaint-post/v1.5");
    for name in [
        "BASES.DAT",
        "CONQUEST.DAT",
        "DATABASE.DAT",
        "FLEETS.DAT",
        "IPBM.DAT",
        "MESSAGES.DAT",
        "PLANETS.DAT",
        "PLAYER.DAT",
        "RESULTS.DAT",
        "SETUP.DAT",
    ] {
        fs::copy(fixture.join(name), target.join(name)).unwrap();
    }

    let stdout = run_ec_cli_in_dir(
        &["maintenance-days", target.to_str().unwrap(), "set", "sun", "tue", "thu", "sat"],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("sun=yes mon=no tue=yes wed=no thu=yes fri=no sat=yes"));
    assert!(stdout.contains("Maintenance raw: [01, 00, ca, 00, 0a, 00, 26]"));

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn snoop_off_rewrites_setup_flag() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("ec-cli-snoop-{unique}"));
    fs::create_dir_all(&target).unwrap();

    let fixture = repo_root().join("original/v1.5");
    fs::copy(fixture.join("SETUP.DAT"), target.join("SETUP.DAT")).unwrap();

    let stdout = run_ec_cli_in_dir(
        &["snoop", target.to_str().unwrap(), "off"],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("Snoop enabled: no"));

    let stdout = run_ec_cli_in_dir(
        &["snoop", target.to_str().unwrap()],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("Snoop enabled: no"));

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn purge_after_rewrites_setup_raw_value() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("ec-cli-purge-{unique}"));
    fs::create_dir_all(&target).unwrap();

    let fixture = repo_root().join("original/v1.5");
    fs::copy(fixture.join("SETUP.DAT"), target.join("SETUP.DAT")).unwrap();

    let stdout = run_ec_cli_in_dir(
        &["purge-after", target.to_str().unwrap(), "1"],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("Purge after turns (raw): 1"));

    let stdout = run_ec_cli_in_dir(
        &["purge-after", target.to_str().unwrap()],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("Purge after turns (raw): 1"));

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn setup_programs_prints_mapped_f4_values() {
    let stdout = run_ec_cli(&["setup-programs", "original/v1.5"]);
    assert!(stdout.contains("ECUTIL F4 Modify Program Options"));
    assert!(stdout.contains("A Purge messages & reports after: 0 turn(s)"));
    assert!(stdout.contains("B Autopilot any empires inactive for: 0 turn(s)"));
    assert!(stdout.contains("C Snoop Enabled: Yes"));
    assert!(stdout.contains("D Enable timeout for local users: No"));
    assert!(stdout.contains("E Enable timeout for remote users: Yes"));
    assert!(stdout.contains("F Maximum time between key strokes: 10 minute(s)"));
    assert!(stdout.contains("G Minimum time granted: 0 minute(s)"));
}

#[test]
fn remaining_f4_commands_rewrite_setup_fields() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("ec-cli-f4-{unique}"));
    fs::create_dir_all(&target).unwrap();

    let fixture = repo_root().join("original/v1.5");
    fs::copy(fixture.join("SETUP.DAT"), target.join("SETUP.DAT")).unwrap();

    let stdout = run_ec_cli_in_dir(
        &["local-timeout", target.to_str().unwrap(), "on"],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("Local timeout enabled: yes"));

    let stdout = run_ec_cli_in_dir(
        &["remote-timeout", target.to_str().unwrap(), "off"],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("Remote timeout enabled: no"));

    let stdout = run_ec_cli_in_dir(
        &["max-key-gap", target.to_str().unwrap(), "15"],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("Maximum time between key strokes (minutes): 15"));

    let stdout = run_ec_cli_in_dir(
        &["minimum-time", target.to_str().unwrap(), "69"],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("Minimum time granted (minutes): 69"));

    let stdout = run_ec_cli_in_dir(
        &["autopilot-after", target.to_str().unwrap(), "3"],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("Autopilot inactive turns (raw): 3"));

    let stdout = run_ec_cli_in_dir(
        &["setup-programs", target.to_str().unwrap()],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("D Enable timeout for local users: Yes"));
    assert!(stdout.contains("E Enable timeout for remote users: No"));
    assert!(stdout.contains("F Maximum time between key strokes: 15 minute(s)"));
    assert!(stdout.contains("G Minimum time granted: 69 minute(s)"));
    assert!(stdout.contains("B Autopilot any empires inactive for: 3 turn(s)"));

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn port_setup_prints_known_f5_values() {
    let stdout = run_ec_cli(&["port-setup", "original/v1.5"]);
    assert!(stdout.contains("ECUTIL F5 Modem / Com Port Setup"));
    assert!(stdout.contains("COM 1 IRQ: 4"));
    assert!(stdout.contains("COM 2 IRQ: 3"));
    assert!(stdout.contains("COM 3 IRQ: 4"));
    assert!(stdout.contains("COM 4 IRQ: 3"));
    assert!(stdout.contains("COM 1 Hardware Flow Control: Yes"));
    assert!(stdout.contains("COM 4 Hardware Flow Control: Yes"));
}

#[test]
fn flow_control_off_rewrites_setup_flag() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("ec-cli-flow-{unique}"));
    fs::create_dir_all(&target).unwrap();

    let fixture = repo_root().join("original/v1.5");
    fs::copy(fixture.join("SETUP.DAT"), target.join("SETUP.DAT")).unwrap();

    let stdout = run_ec_cli_in_dir(
        &["flow-control", target.to_str().unwrap(), "com1", "off"],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("COM 1 Hardware Flow Control: No"));

    let stdout = run_ec_cli_in_dir(
        &["port-setup", target.to_str().unwrap()],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("COM 1 Hardware Flow Control: No"));
    assert!(stdout.contains("COM 2 Hardware Flow Control: Yes"));

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn com_irq_rewrites_setup_value() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("ec-cli-irq-{unique}"));
    fs::create_dir_all(&target).unwrap();

    let fixture = repo_root().join("original/v1.5");
    fs::copy(fixture.join("SETUP.DAT"), target.join("SETUP.DAT")).unwrap();

    let stdout = run_ec_cli_in_dir(
        &["com-irq", target.to_str().unwrap(), "com1", "7"],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("COM 1 IRQ: 7"));

    let stdout = run_ec_cli_in_dir(
        &["port-setup", target.to_str().unwrap()],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("COM 1 IRQ: 7"));
    assert!(stdout.contains("COM 2 IRQ: 3"));

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn fleet_order_recreates_known_valid_fleet_pre_fixture() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("ec-cli-fleet-order-{unique}"));
    fs::create_dir_all(&target).unwrap();

    let fixture = repo_root().join("fixtures/ecmaint-post/v1.5");
    for name in [
        "BASES.DAT",
        "CONQUEST.DAT",
        "DATABASE.DAT",
        "FLEETS.DAT",
        "IPBM.DAT",
        "MESSAGES.DAT",
        "PLANETS.DAT",
        "PLAYER.DAT",
        "RESULTS.DAT",
        "SETUP.DAT",
    ] {
        fs::copy(fixture.join(name), target.join(name)).unwrap();
    }

    let stdout = run_ec_cli_in_dir(
        &["fleet-order", target.to_str().unwrap(), "1", "3", "12", "15", "13"],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("Fleet record 1 updated: speed=3 order=0x0c target=(15, 13)"));

    let expected = repo_root().join("fixtures/ecmaint-fleet-pre/v1.5/FLEETS.DAT");
    let actual = fs::read(target.join("FLEETS.DAT")).unwrap();
    assert_eq!(actual, fs::read(expected).unwrap());

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn planet_build_recreates_known_valid_build_pre_fixture() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("ec-cli-planet-build-{unique}"));
    fs::create_dir_all(&target).unwrap();

    let fixture = repo_root().join("fixtures/ecmaint-post/v1.5");
    for name in [
        "BASES.DAT",
        "CONQUEST.DAT",
        "DATABASE.DAT",
        "FLEETS.DAT",
        "IPBM.DAT",
        "MESSAGES.DAT",
        "PLANETS.DAT",
        "PLAYER.DAT",
        "RESULTS.DAT",
        "SETUP.DAT",
    ] {
        fs::copy(fixture.join(name), target.join(name)).unwrap();
    }

    let stdout = run_ec_cli_in_dir(
        &["planet-build", target.to_str().unwrap(), "15", "0x03", "0x01"],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("Planet record 15 updated: build_slot=0x03 build_kind=0x01"));

    let expected = repo_root().join("fixtures/ecmaint-build-pre/v1.5/PLANETS.DAT");
    let actual = fs::read(target.join("PLANETS.DAT")).unwrap();
    assert_eq!(actual, fs::read(expected).unwrap());

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn scenario_fleet_order_recreates_known_valid_fleet_pre_fixture() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("ec-cli-scenario-fleet-order-{unique}"));
    fs::create_dir_all(&target).unwrap();

    let fixture = repo_root().join("fixtures/ecmaint-post/v1.5");
    for name in [
        "BASES.DAT",
        "CONQUEST.DAT",
        "DATABASE.DAT",
        "FLEETS.DAT",
        "IPBM.DAT",
        "MESSAGES.DAT",
        "PLANETS.DAT",
        "PLAYER.DAT",
        "RESULTS.DAT",
        "SETUP.DAT",
    ] {
        fs::copy(fixture.join(name), target.join(name)).unwrap();
    }

    let stdout = run_ec_cli_in_dir(
        &["scenario", target.to_str().unwrap(), "fleet-order"],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("Applied scenario: fleet-order"));

    let expected = repo_root().join("fixtures/ecmaint-fleet-pre/v1.5/FLEETS.DAT");
    let actual = fs::read(target.join("FLEETS.DAT")).unwrap();
    assert_eq!(actual, fs::read(expected).unwrap());

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn scenario_planet_build_recreates_known_valid_build_pre_fixture() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("ec-cli-scenario-planet-build-{unique}"));
    fs::create_dir_all(&target).unwrap();

    let fixture = repo_root().join("fixtures/ecmaint-post/v1.5");
    for name in [
        "BASES.DAT",
        "CONQUEST.DAT",
        "DATABASE.DAT",
        "FLEETS.DAT",
        "IPBM.DAT",
        "MESSAGES.DAT",
        "PLANETS.DAT",
        "PLAYER.DAT",
        "RESULTS.DAT",
        "SETUP.DAT",
    ] {
        fs::copy(fixture.join(name), target.join(name)).unwrap();
    }

    let stdout = run_ec_cli_in_dir(
        &["scenario", target.to_str().unwrap(), "planet-build"],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("Applied scenario: planet-build"));

    let expected = repo_root().join("fixtures/ecmaint-build-pre/v1.5/PLANETS.DAT");
    let actual = fs::read(target.join("PLANETS.DAT")).unwrap();
    assert_eq!(actual, fs::read(expected).unwrap());

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn guard_starbase_scenario_recreates_known_valid_starbase_pre_fixture() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("ec-cli-guard-starbase-{unique}"));
    fs::create_dir_all(&target).unwrap();

    let fixture = repo_root().join("fixtures/ecmaint-post/v1.5");
    for name in [
        "BASES.DAT",
        "CONQUEST.DAT",
        "DATABASE.DAT",
        "FLEETS.DAT",
        "IPBM.DAT",
        "MESSAGES.DAT",
        "PLANETS.DAT",
        "PLAYER.DAT",
        "RESULTS.DAT",
        "SETUP.DAT",
    ] {
        fs::copy(fixture.join(name), target.join(name)).unwrap();
    }

    let stdout = run_ec_cli_in_dir(
        &["scenario", target.to_str().unwrap(), "guard-starbase"],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("Applied scenario: guard-starbase"));

    let expected_player = repo_root().join("fixtures/ecmaint-starbase-pre/v1.5/PLAYER.DAT");
    let expected_fleets = repo_root().join("fixtures/ecmaint-starbase-pre/v1.5/FLEETS.DAT");
    let expected_bases = repo_root().join("fixtures/ecmaint-starbase-pre/v1.5/BASES.DAT");

    assert_eq!(fs::read(target.join("PLAYER.DAT")).unwrap(), fs::read(expected_player).unwrap());
    assert_eq!(fs::read(target.join("FLEETS.DAT")).unwrap(), fs::read(expected_fleets).unwrap());
    assert_eq!(fs::read(target.join("BASES.DAT")).unwrap(), fs::read(expected_bases).unwrap());

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn validate_guard_starbase_accepts_known_valid_fixture() {
    let stdout = run_ec_cli(&["validate", "fixtures/ecmaint-starbase-pre/v1.5", "guard-starbase"]);
    assert!(stdout.contains("Valid guard-starbase scenario"));
    assert!(stdout.contains("BASES.DAT matches the accepted one-base guard-starbase record"));
}

#[test]
fn validate_fleet_order_accepts_known_valid_fixture() {
    let stdout = run_ec_cli(&["validate", "fixtures/ecmaint-fleet-pre/v1.5", "fleet-order"]);
    assert!(stdout.contains("Valid fleet-order scenario"));
    assert!(stdout.contains("FLEET[1].order = 0x0c"));
}

#[test]
fn validate_planet_build_accepts_known_valid_fixture() {
    let stdout = run_ec_cli(&["validate", "fixtures/ecmaint-build-pre/v1.5", "planet-build"]);
    assert!(stdout.contains("Valid planet-build scenario"));
    assert!(stdout.contains("PLANET[15].build_kind = 0x01"));
}

#[test]
fn validate_guard_starbase_rejects_post_maint_fixture() {
    let stderr = run_ec_cli_failure_in_dir(
        &["validate", "fixtures/ecmaint-post/v1.5", "guard-starbase"],
        repo_root().join("rust"),
    );
    assert!(stderr.contains("PLAYER[1].starbase_count_raw expected 1, got 0"));
    assert!(stderr.contains("FLEET[1].order expected 0x04, got 0x05"));
    assert!(stderr.contains("BASES.DAT expected 1 record, got 0"));
}

#[test]
fn validate_fleet_order_rejects_post_maint_fixture() {
    let stderr = run_ec_cli_failure_in_dir(
        &["validate", "fixtures/ecmaint-post/v1.5", "fleet-order"],
        repo_root().join("rust"),
    );
    assert!(stderr.contains("FLEET[1].order expected 0x0c, got 0x05"));
    assert!(stderr.contains("FLEET[1].target expected (15, 13), got [16, 13]"));
}

#[test]
fn validate_planet_build_rejects_post_maint_fixture() {
    let stderr = run_ec_cli_failure_in_dir(
        &["validate", "fixtures/ecmaint-post/v1.5", "planet-build"],
        repo_root().join("rust"),
    );
    assert!(stderr.contains("PLANET[15].build_slot expected 0x03, got 0x00"));
    assert!(stderr.contains("PLANET[15].build_kind expected 0x01, got 0x00"));
}

#[test]
fn scenario_init_guard_starbase_materializes_runnable_directory() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("ec-cli-guard-starbase-init-{unique}"));

    let stdout = run_ec_cli_in_dir(
        &[
            "scenario-init",
            "fixtures/ecmaint-post/v1.5",
            target.to_str().unwrap(),
            "guard-starbase",
        ],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("Applied scenario: guard-starbase"));
    assert!(stdout.contains("Scenario directory initialized at"));

    let expected_player = repo_root().join("fixtures/ecmaint-starbase-pre/v1.5/PLAYER.DAT");
    let expected_fleets = repo_root().join("fixtures/ecmaint-starbase-pre/v1.5/FLEETS.DAT");
    let expected_bases = repo_root().join("fixtures/ecmaint-starbase-pre/v1.5/BASES.DAT");
    let expected_setup = repo_root().join("fixtures/ecmaint-post/v1.5/SETUP.DAT");

    assert_eq!(fs::read(target.join("PLAYER.DAT")).unwrap(), fs::read(expected_player).unwrap());
    assert_eq!(fs::read(target.join("FLEETS.DAT")).unwrap(), fs::read(expected_fleets).unwrap());
    assert_eq!(fs::read(target.join("BASES.DAT")).unwrap(), fs::read(expected_bases).unwrap());
    assert_eq!(fs::read(target.join("SETUP.DAT")).unwrap(), fs::read(expected_setup).unwrap());

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn scenario_init_guard_starbase_accepts_omitted_source() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("ec-cli-guard-starbase-default-{unique}"));

    let stdout = run_ec_cli_in_dir(
        &["scenario-init", target.to_str().unwrap(), "guard-starbase"],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("Applied scenario: guard-starbase"));
    assert!(stdout.contains("Scenario directory initialized at"));

    let validate = run_ec_cli_in_dir(
        &["validate", target.to_str().unwrap(), "guard-starbase"],
        repo_root().join("rust"),
    );
    assert!(validate.contains("Valid guard-starbase scenario"));

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn scenario_init_fleet_order_materializes_runnable_directory() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("ec-cli-fleet-order-init-{unique}"));

    let stdout = run_ec_cli_in_dir(
        &[
            "scenario-init",
            "fixtures/ecmaint-post/v1.5",
            target.to_str().unwrap(),
            "fleet-order",
        ],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("Applied scenario: fleet-order"));
    assert!(stdout.contains("Scenario directory initialized at"));

    let expected_fleets = repo_root().join("fixtures/ecmaint-fleet-pre/v1.5/FLEETS.DAT");
    let expected_planets = repo_root().join("fixtures/ecmaint-post/v1.5/PLANETS.DAT");

    assert_eq!(fs::read(target.join("FLEETS.DAT")).unwrap(), fs::read(expected_fleets).unwrap());
    assert_eq!(fs::read(target.join("PLANETS.DAT")).unwrap(), fs::read(expected_planets).unwrap());

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn scenario_init_planet_build_materializes_runnable_directory() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("ec-cli-planet-build-init-{unique}"));

    let stdout = run_ec_cli_in_dir(
        &[
            "scenario-init",
            "fixtures/ecmaint-post/v1.5",
            target.to_str().unwrap(),
            "planet-build",
        ],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("Applied scenario: planet-build"));
    assert!(stdout.contains("Scenario directory initialized at"));

    let expected_planets = repo_root().join("fixtures/ecmaint-build-pre/v1.5/PLANETS.DAT");
    let expected_fleets = repo_root().join("fixtures/ecmaint-post/v1.5/FLEETS.DAT");

    assert_eq!(fs::read(target.join("PLANETS.DAT")).unwrap(), fs::read(expected_planets).unwrap());
    assert_eq!(fs::read(target.join("FLEETS.DAT")).unwrap(), fs::read(expected_fleets).unwrap());

    let _ = fs::remove_dir_all(&target);
}
