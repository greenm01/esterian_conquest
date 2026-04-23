use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use nc_helm::App;

fn unique_temp_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("nc-helm-{label}-{}-{nanos}", std::process::id()))
}

#[test]
fn missing_game_dir_reports_clear_error() {
    let dir = unique_temp_path("missing-dir");
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("stale temp dir should remove cleanly");
    }

    let err = App::new_local_dashboard(&dir).expect_err("missing dir should fail");
    let message = err.to_string();
    assert!(message.contains("game directory not found"));
    assert!(message.contains(dir.to_string_lossy().as_ref()));
}

#[test]
fn missing_ncgame_db_reports_clear_error_without_creating_db() {
    let dir = unique_temp_path("missing-db");
    fs::create_dir_all(&dir).expect("temp dir should create");

    let err = App::new_local_dashboard(&dir).expect_err("missing db should fail");
    let message = err.to_string();
    assert!(message.contains("missing runtime database"));
    assert!(message.contains("ncgame.db"));
    assert!(
        !dir.join("ncgame.db").exists(),
        "validation should not silently create ncgame.db"
    );

    fs::remove_dir_all(&dir).expect("temp dir should remove cleanly");
}
