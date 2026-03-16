mod common;

use common::{
    cleanup_dir, copy_fixture_dir, run_classic_ecgame_smoke, run_classic_ecgame_smoke_with_alias,
    run_ec_cli, run_ec_cli_in_dir, unique_temp_dir,
};

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
fn sysop_maintenance_days_update_runtime_store_and_export() {
    let target = unique_temp_dir("ec-cli-sysop-maintenance-days");
    let exported = unique_temp_dir("ec-cli-sysop-maintenance-days-exported");
    let stdout = run_ec_cli(&["sysop", "new-game", target.to_str().unwrap()]);
    assert!(stdout.contains("Initialized new game"));

    let stdout = run_ec_cli_in_dir(
        &[
            "sysop",
            "maintenance-days",
            target.to_str().unwrap(),
            "set",
            "mon",
            "wed",
            "fri",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Directory:"));
    assert!(stdout.contains("Maintenance days:"));
    assert!(stdout.contains("mon=yes"));
    assert!(stdout.contains("wed=yes"));
    assert!(stdout.contains("fri=yes"));

    run_ec_cli_in_dir(
        &["db-export", target.to_str().unwrap(), exported.to_str().unwrap()],
        common::rust_workspace(),
    );

    let original = run_ec_cli_in_dir(
        &["sysop", "maintenance-days", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    let reexported = run_ec_cli_in_dir(
        &["sysop", "maintenance-days", exported.to_str().unwrap()],
        common::rust_workspace(),
    );
    let original_lines = original.lines().skip(1).collect::<Vec<_>>();
    let reexported_lines = reexported.lines().skip(1).collect::<Vec<_>>();
    assert_eq!(original_lines, reexported_lines);

    cleanup_dir(&target);
    cleanup_dir(&exported);
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
    assert_eq!(game_data.player.records[0].tax_rate(), 50);
    assert_eq!(game_data.planets.records[0].economy_marker_raw(), 50);
    assert_eq!(
        game_data.planets.records[0].present_production_points(),
        Some(100)
    );
    assert!(target.join("ECGAME.EXE").exists());
    assert!(target.join("ECMAINT.EXE").exists());
    assert!(target.join("ECUTIL.EXE").exists());
    cleanup_dir(&target);
}

#[test]
#[ignore = "launches classic ECGAME through dosbox-x"]
fn sysop_new_game_launches_classic_ecgame_smoke() {
    let target = unique_temp_dir("ec-cli-sysop-new-game-ecgame");
    let stdout = run_ec_cli(&["sysop", "new-game", target.to_str().unwrap()]);
    assert!(stdout.contains("Initialized new game"));

    run_classic_ecgame_smoke(&target, 1);

    cleanup_dir(&target);
}

#[test]
#[ignore = "launches classic ECGAME through dosbox-x"]
fn classic_login_prepare_supports_matched_preloaded_ecgame_smoke() {
    let target = unique_temp_dir("ec-cli-classic-login-preloaded-ecgame");
    let stdout = run_ec_cli(&["sysop", "new-game", target.to_str().unwrap()]);
    assert!(stdout.contains("Initialized new game"));

    let prepare = run_ec_cli(&[
        "classic-login-prepare",
        target.to_str().unwrap(),
        "2",
        "SYSOP",
        "foo",
    ]);
    assert!(prepare.contains("Prepared classic login for player 2"));

    run_classic_ecgame_smoke_with_alias(&target, 2, "SYSOP");

    cleanup_dir(&target);
}

#[test]
#[ignore = "launches classic ECGAME through dosbox-x"]
fn hybrid_campaign_loop_reopens_classic_client_after_rust_maintenance() {
    let target = unique_temp_dir("ec-cli-hybrid-campaign-loop");

    let stdout = run_ec_cli(&["sysop", "new-game", target.to_str().unwrap()]);
    assert!(stdout.contains("Initialized new game"));

    run_classic_ecgame_smoke(&target, 1);

    let prepare = run_ec_cli(&[
        "classic-login-prepare",
        target.to_str().unwrap(),
        "1",
        "SYSOP",
        "foo",
    ]);
    assert!(prepare.contains("Prepared classic login for player 1"));

    run_classic_ecgame_smoke_with_alias(&target, 1, "SYSOP");

    let maint = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(maint.contains("Rust maintenance complete."));

    run_classic_ecgame_smoke_with_alias(&target, 1, "SYSOP");

    cleanup_dir(&target);
}

#[test]
#[ignore = "launches classic ECGAME through dosbox-x"]
fn returning_player_fixture_reopens_in_classic_ecgame_with_matching_alias() {
    let target = unique_temp_dir("ec-cli-returning-player-ecgame");
    copy_fixture_dir("original/v1.5", &target);

    run_classic_ecgame_smoke_with_alias(&target, 1, "HANNIBAL");

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
