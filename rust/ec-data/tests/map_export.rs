use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ec_compat::import_directory_snapshot;
use ec_data::{
    CampaignStore, DEFAULT_CAMPAIGN_DB_NAME, STARMAP_CSV_FILE_NAME, STARMAP_DETAILS_CSV_FILE_NAME,
    STARMAP_TEXT_FILE_NAME, build_player_map_export_data,
};

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

fn copy_dir_all(src: &Path, dst: &Path) {
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
fn shared_player_map_export_builder_returns_fixed_three_file_bundle() {
    let source = repo_root().join("fixtures/ecutil-init/v1.5");
    let imported = temp_dir("ec-data-map-export");
    copy_dir_all(&source, &imported);

    let store = CampaignStore::open(imported.join(DEFAULT_CAMPAIGN_DB_NAME)).expect("open store");
    import_directory_snapshot(&store, &imported).expect("import directory");

    let export = build_player_map_export_data(&imported, 1).expect("build player export");
    let files = export.fixed_named_files();

    assert_eq!(files.len(), 3);
    assert_eq!(files[0].name, STARMAP_TEXT_FILE_NAME);
    assert_eq!(files[1].name, STARMAP_CSV_FILE_NAME);
    assert_eq!(files[2].name, STARMAP_DETAILS_CSV_FILE_NAME);
    assert!(files.iter().all(|file| !file.contents.is_empty()));
}
