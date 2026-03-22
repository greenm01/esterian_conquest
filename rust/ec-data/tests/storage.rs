mod common;

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use ec_data::{CampaignStore, DEFAULT_CAMPAIGN_DB_NAME};
use rusqlite::Connection;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn temp_dir(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ))
}

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) {
    fs::create_dir_all(dst).expect("create temp dir");
    for entry in fs::read_dir(src).expect("read src dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &target);
        } else {
            fs::copy(&path, &target).expect("copy file");
        }
    }
}

#[test]
fn sqlite_store_round_trips_directory_export() {
    let source = repo_root().join("fixtures/ecutil-init/v1.5");
    let imported = temp_dir("ec-data-storage-import");
    let exported = temp_dir("ec-data-storage-export");
    copy_dir_all(&source, &imported);

    let store = CampaignStore::open(imported.join(DEFAULT_CAMPAIGN_DB_NAME)).expect("open store");
    store
        .import_directory_snapshot(&imported)
        .expect("import directory");
    let year = store
        .export_latest_snapshot_to_dir(&exported)
        .expect("export snapshot");
    assert_eq!(year, 3000);

    for name in [
        "PLAYER.DAT",
        "PLANETS.DAT",
        "FLEETS.DAT",
        "BASES.DAT",
        "IPBM.DAT",
        "SETUP.DAT",
        "CONQUEST.DAT",
        "DATABASE.DAT",
        "MESSAGES.DAT",
        "RESULTS.DAT",
    ] {
        assert_eq!(
            fs::read(imported.join(name)).expect("source bytes"),
            fs::read(exported.join(name)).expect("exported bytes"),
            "mismatch for {name}",
        );
    }
}

#[test]
fn sqlite_store_persists_explicit_campaign_seed() {
    let source = repo_root().join("fixtures/ecutil-init/v1.5");
    let imported = temp_dir("ec-data-storage-seed-explicit");
    copy_dir_all(&source, &imported);

    let store = CampaignStore::open(imported.join(DEFAULT_CAMPAIGN_DB_NAME)).expect("open store");
    let expected_seed = 0xEC15_2026_0000_0042u64;
    store
        .import_directory_snapshot_with_seed(&imported, Some(expected_seed))
        .expect("import directory");

    let initial = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot");
    assert_eq!(initial.campaign_seed, expected_seed);

    store
        .save_runtime_state_structured(
            &initial.game_data,
            &initial.report_block_rows,
            &initial.queued_mail,
        )
        .expect("resave runtime state");

    let reloaded = store
        .load_latest_runtime_state()
        .expect("reload runtime state")
        .expect("runtime snapshot");
    assert_eq!(reloaded.campaign_seed, expected_seed);
}

#[test]
fn sqlite_store_generates_and_reuses_campaign_seed() {
    let source = repo_root().join("fixtures/ecutil-init/v1.5");
    let imported = temp_dir("ec-data-storage-seed-generated");
    copy_dir_all(&source, &imported);

    let store = CampaignStore::open(imported.join(DEFAULT_CAMPAIGN_DB_NAME)).expect("open store");
    store
        .import_directory_snapshot(&imported)
        .expect("import directory");

    let initial = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot");
    assert_ne!(initial.campaign_seed, 0);

    store
        .save_runtime_state_structured(
            &initial.game_data,
            &initial.report_block_rows,
            &initial.queued_mail,
        )
        .expect("resave runtime state");

    let reloaded = store
        .load_latest_runtime_state()
        .expect("reload runtime state")
        .expect("runtime snapshot");
    assert_eq!(reloaded.campaign_seed, initial.campaign_seed);
}

#[test]
fn sqlite_store_schema_has_no_blob_columns_or_compat_files_table() {
    let source = repo_root().join("fixtures/ecutil-init/v1.5");
    let imported = temp_dir("ec-data-storage-schema");
    copy_dir_all(&source, &imported);

    let store_path = imported.join(DEFAULT_CAMPAIGN_DB_NAME);
    let store = CampaignStore::open(&store_path).expect("open store");
    store
        .import_directory_snapshot(&imported)
        .expect("import directory");

    let conn = Connection::open(store_path).expect("open sqlite db");
    let table_names = conn
        .prepare(
            "SELECT name
             FROM sqlite_master
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
             ORDER BY name",
        )
        .expect("prepare table list")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query table list")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect table list");
    assert!(
        !table_names.iter().any(|name| name == "compat_files"),
        "compat_files table should be gone: {table_names:?}"
    );

    let schema_rows = conn
        .prepare(
            "SELECT sql
             FROM sqlite_master
             WHERE type = 'table' AND sql IS NOT NULL",
        )
        .expect("prepare schema query")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query schema")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect schema");
    for sql in schema_rows {
        assert!(
            !sql.to_ascii_uppercase().contains("BLOB"),
            "sqlite schema should not use BLOB columns: {sql}"
        );
    }
}
