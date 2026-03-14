mod common;

use common::{cleanup_dir, copy_fixture_dir, run_ec_cli, run_ec_cli_in_dir, unique_temp_dir};

#[test]
fn sysop_setup_programs_prints_known_f4_values() {
    let stdout = run_ec_cli(&["sysop", "setup-programs", "original/v1.5"]);
    assert!(stdout.contains("ECUTIL F4 Modify Program Options"));
    assert!(stdout.contains("C Snoop Enabled: Yes"));
}

#[test]
fn sysop_snoop_off_rewrites_setup_flag() {
    let target = unique_temp_dir("ec-cli-sysop-snoop");
    copy_fixture_dir("original/v1.5", &target);

    let stdout = run_ec_cli_in_dir(
        &["sysop", "snoop", target.to_str().unwrap(), "off"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Snoop enabled: no"));

    let stdout = run_ec_cli_in_dir(
        &["sysop", "snoop", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Snoop enabled: no"));

    cleanup_dir(&target);
}

#[test]
fn sysop_can_init_canonical_four_player_start() {
    let target = unique_temp_dir("ec-cli-sysop-init");
    let stdout = run_ec_cli(&["sysop", "new-game", target.to_str().unwrap()]);
    assert!(stdout.contains("Initialized new game"));
    assert!(stdout.contains("players=4"));
    assert!(target.join("DATABASE.DAT").exists());
    let game_data = ec_data::CoreGameData::load(&target).expect("generated game should load");
    assert_eq!(game_data.player.records[0].owner_mode_raw(), 0);
    assert_eq!(game_data.planets.records[0].planet_name(), "Not Named Yet");
    cleanup_dir(&target);
}

#[test]
fn sysop_new_game_accepts_player_count_flag() {
    let target = unique_temp_dir("ec-cli-sysop-new-game-players");
    let stdout = run_ec_cli(&[
        "sysop",
        "new-game",
        target.to_str().unwrap(),
        "--players",
        "2",
    ]);
    assert!(stdout.contains("Initialized new game"));
    assert!(stdout.contains("players=2"));
    cleanup_dir(&target);
}

#[test]
fn sysop_new_game_accepts_kdl_config() {
    let target = unique_temp_dir("ec-cli-sysop-new-game-config");
    let stdout = run_ec_cli(&[
        "sysop",
        "new-game",
        target.to_str().unwrap(),
        "--config",
        "rust/ec-data/config/setup.example.kdl",
    ]);
    assert!(stdout.contains("Initialized new game"));
    assert!(stdout.contains("setup.example.kdl"));
    assert!(target.join("DATABASE.DAT").exists());
    cleanup_dir(&target);
}

#[test]
fn sysop_new_game_allows_player_override_over_kdl() {
    let target = unique_temp_dir("ec-cli-sysop-new-game-config-override");
    let stdout = run_ec_cli(&[
        "sysop",
        "new-game",
        target.to_str().unwrap(),
        "--config",
        "rust/ec-data/config/setup.example.kdl",
        "--players",
        "2",
    ]);
    assert!(stdout.contains("Initialized new game"));
    assert!(stdout.contains("setup.example.kdl"));
    assert!(stdout.contains("players=2"));
    assert!(target.join("DATABASE.DAT").exists());
    cleanup_dir(&target);
}

#[test]
fn sysop_new_game_accepts_seed_and_reports_it() {
    let target = unique_temp_dir("ec-cli-sysop-new-game-seed");
    let stdout = run_ec_cli(&[
        "sysop",
        "new-game",
        target.to_str().unwrap(),
        "--players",
        "4",
        "--seed",
        "1515",
    ]);
    assert!(stdout.contains("Initialized new game"));
    assert!(stdout.contains("seed=1515"));
    cleanup_dir(&target);
}

#[test]
fn sysop_new_game_accepts_manual_nine_player_tier() {
    let target = unique_temp_dir("ec-cli-sysop-new-game-nine");
    let stdout = run_ec_cli(&[
        "sysop",
        "new-game",
        target.to_str().unwrap(),
        "--players",
        "9",
        "--seed",
        "2025",
    ]);
    assert!(stdout.contains("Initialized new game"));
    assert!(stdout.contains("players=9"));
    assert!(target.join("PLAYER.DAT").exists());
    cleanup_dir(&target);
}

#[test]
fn sysop_generate_gamestate_writes_preflight_clean_directory() {
    let target = unique_temp_dir("ec-cli-sysop-generate");
    let stdout = run_ec_cli(&[
        "sysop",
        "generate-gamestate",
        target.to_str().unwrap(),
        "4",
        "3001",
        "16:13",
        "30:6",
        "2:25",
        "26:26",
    ]);
    assert!(stdout.contains("Generated gamestate at:"));
    assert!(stdout.contains("Preflight validation: OK"));
    cleanup_dir(&target);
}
