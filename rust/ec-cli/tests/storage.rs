mod common;

use std::fs;

use common::{cleanup_dir, run_ec_cli, unique_temp_dir};

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

    cleanup_dir(&source);
    cleanup_dir(&exported);
}
