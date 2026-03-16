mod common;

use std::fs;

use common::{cleanup_dir, run_ec_cli, run_ecmaint_oracle, unique_temp_dir};

#[test]
fn db_import_and_export_round_trip_fixture() {
    let source = unique_temp_dir("ec-cli-db-import");
    let exported = unique_temp_dir("ec-cli-db-export");
    common::copy_fixture_dir("fixtures/ecutil-init/v1.5", &source);

    let import_stdout = run_ec_cli(&["db-import", source.to_str().unwrap()]);
    assert!(import_stdout.contains("Imported"));
    assert!(source.join("ecgame.db").exists());

    let export_stdout = run_ec_cli(&[
        "db-export",
        source.to_str().unwrap(),
        exported.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year 3000"));
    assert_eq!(
        fs::read(source.join("PLAYER.DAT")).unwrap(),
        fs::read(exported.join("PLAYER.DAT")).unwrap()
    );
    assert_eq!(
        fs::read(source.join("DATABASE.DAT")).unwrap(),
        fs::read(exported.join("DATABASE.DAT")).unwrap()
    );
    assert!(exported.join("ECGAME.EXE").exists());
    assert!(exported.join("ECMAINT.EXE").exists());
    assert!(exported.join("ECUTIL.EXE").exists());

    cleanup_dir(&source);
    cleanup_dir(&exported);
}

#[test]
fn sqlite_maint_exported_directory_is_accepted_by_ecmaint_oracle() {
    let source = unique_temp_dir("ec-cli-db-maint-source");
    let exported = unique_temp_dir("ec-cli-db-maint-export");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &source);

    let import_stdout = run_ec_cli(&["db-import", source.to_str().unwrap()]);
    assert!(import_stdout.contains("Imported"));

    let maint_stdout = run_ec_cli(&["maint-rust", source.to_str().unwrap(), "1"]);
    assert!(maint_stdout.contains("Rust maintenance complete."));

    let export_stdout = run_ec_cli(&[
        "db-export",
        source.to_str().unwrap(),
        exported.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year"));

    let oracle_stdout = run_ecmaint_oracle(&exported);
    assert!(!oracle_stdout.trim().is_empty());
    assert!(exported.join("ECGAME.EXE").exists());

    cleanup_dir(&source);
    cleanup_dir(&exported);
}

#[test]
fn db_export_preserves_classic_player_handle_identity() {
    let source = unique_temp_dir("ec-cli-db-export-player-handle-source");
    let exported = unique_temp_dir("ec-cli-db-export-player-handle-exported");

    let stdout = run_ec_cli(&["sysop", "new-game", source.to_str().unwrap()]);
    assert!(stdout.contains("Initialized new game"));

    let rename_stdout = run_ec_cli(&[
        "player-name",
        source.to_str().unwrap(),
        "1",
        "SYSOP",
        "Auroran Combine",
    ]);
    assert!(rename_stdout.contains("Player 1 renamed"));

    let export_stdout = run_ec_cli(&[
        "db-export",
        source.to_str().unwrap(),
        exported.to_str().unwrap(),
    ]);
    assert!(export_stdout.contains("Exported year 3000"));

    let exported_data = ec_data::CoreGameData::load(&exported).expect("exported game should load");
    assert_eq!(
        exported_data.player.records[0].assigned_player_handle_summary(),
        "SYSOP"
    );
    assert_eq!(
        exported_data.player.records[0].controlled_empire_name_summary(),
        "Auroran Combine"
    );

    cleanup_dir(&source);
    cleanup_dir(&exported);
}
