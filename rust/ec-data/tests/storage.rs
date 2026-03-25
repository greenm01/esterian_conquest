mod common;

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use ec_compat::{
    DatabaseDat, export_latest_snapshot_to_dir, import_directory_snapshot,
    import_directory_snapshot_with_seed,
};
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
    import_directory_snapshot(&store, &imported).expect("import directory");
    let year = export_latest_snapshot_to_dir(&store, &exported).expect("export snapshot");
    assert_eq!(year, 3000);

    for name in [
        "PLAYER.DAT",
        "PLANETS.DAT",
        "FLEETS.DAT",
        "BASES.DAT",
        "IPBM.DAT",
        "SETUP.DAT",
        "CONQUEST.DAT",
        "MESSAGES.DAT",
        "RESULTS.DAT",
    ] {
        assert_eq!(
            fs::read(imported.join(name)).expect("source bytes"),
            fs::read(exported.join(name)).expect("exported bytes"),
            "mismatch for {name}",
        );
    }

    let database_bytes =
        fs::read(exported.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    assert_eq!(
        fs::read(imported.join("DATABASE.DAT")).expect("source DATABASE.DAT"),
        database_bytes,
        "mismatch for DATABASE.DAT",
    );
    let database = DatabaseDat::parse(&database_bytes).expect("exported DATABASE.DAT should parse");
    assert_eq!(
        database.records.len(),
        80,
        "expected 4x20 DATABASE.DAT records"
    );
}

#[test]
fn sqlite_store_preserves_orbit_seed_rows_as_orbit_intel() {
    let source = repo_root().join("fixtures/ecutil-init/v1.5");
    let imported = temp_dir("ec-data-storage-orbit-seed");
    copy_dir_all(&source, &imported);

    let store = CampaignStore::open(imported.join(DEFAULT_CAMPAIGN_DB_NAME)).expect("open store");
    import_directory_snapshot(&store, &imported).expect("import directory");

    for (viewer_empire_id, planet_record_index) in
        [(1, 15usize), (2, 13usize), (3, 5usize), (4, 6usize)]
    {
        let snapshot = store
            .latest_planet_intel_for_viewer(viewer_empire_id)
            .expect("load viewer intel")
            .into_iter()
            .find(|row| row.planet_record_index_1_based == planet_record_index)
            .expect("orbit seed row should exist");
        assert!(
            snapshot.compat_is_orbit_seed,
            "viewer {viewer_empire_id} planet {planet_record_index} should stay tagged as orbit seed"
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
    import_directory_snapshot_with_seed(&store, &imported, Some(expected_seed))
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
    import_directory_snapshot(&store, &imported).expect("import directory");

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
    import_directory_snapshot(&store, &imported).expect("import directory");

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
    assert!(
        !table_names
            .iter()
            .any(|name| name == "compat_database_record_fields"),
        "compat_database_record_fields table should be gone: {table_names:?}"
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
    assert!(
        schema_rows
            .iter()
            .any(|sql| sql.contains("known_starbase_count INTEGER")),
        "planet_intel schema should include known_starbase_count"
    );
    for sql in schema_rows {
        assert!(
            !sql.to_ascii_uppercase().contains("BLOB"),
            "sqlite schema should not use BLOB columns: {sql}"
        );
    }
}
