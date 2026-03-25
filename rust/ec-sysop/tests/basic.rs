use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use ec_data::{CampaignStore, CoreGameData};

static TEMP_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "{prefix}-{}-{}-{}",
        std::process::id(),
        TEMP_DIR_SEQ.fetch_add(1, Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create temp dir");
    root
}

fn run_ec_sysop(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_ec-sysop"))
        .args(args)
        .output()
        .expect("ec-sysop should run");

    assert!(
        output.status.success(),
        "ec-sysop failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("stdout should be utf-8")
}

fn run_ec_sysop_failure(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_ec-sysop"))
        .args(args)
        .output()
        .expect("ec-sysop should run");

    assert!(
        !output.status.success(),
        "ec-sysop unexpectedly succeeded: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stderr).expect("stderr should be utf-8")
}

#[test]
fn ec_sysop_new_game_initializes_default_campaign() {
    let target = unique_temp_dir("ec-sysop-new-game");

    let stdout = run_ec_sysop(&["new-game", target.to_str().expect("utf-8 path")]);
    assert!(stdout.contains("Initialized new game"));
    assert!(stdout.contains("players=4"));
    assert!(target.join("DATABASE.DAT").exists());

    let game_data = CoreGameData::load(&target).expect("generated game should load");
    assert_eq!(game_data.player.records[0].owner_mode_raw(), 0);
    assert_eq!(game_data.planets.records[0].planet_name(), "Not Named Yet");

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn ec_sysop_new_game_rejects_internal_setup_preset_flag() {
    let target = unique_temp_dir("ec-sysop-new-game-invalid-config");
    let stderr = run_ec_sysop_failure(&[
        "new-game",
        target.to_str().expect("utf-8 path"),
        "--config",
        "ec-cli/config/setup.example.kdl",
    ]);
    assert!(stderr.contains("--config is only supported"));
    let _ = fs::remove_dir_all(&target);
}

#[test]
fn ec_sysop_maint_runs_rust_maintenance() {
    let target = unique_temp_dir("ec-sysop-maint");

    run_ec_sysop(&[
        "new-game",
        target.to_str().expect("utf-8 path"),
        "--seed",
        "1515",
    ]);

    let stdout = run_ec_sysop(&["maint", target.to_str().expect("utf-8 path"), "1"]);
    assert!(stdout.contains("Rust maintenance complete."));

    let store = CampaignStore::open_default_in_dir(&target).expect("open campaign store");
    let runtime = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot should exist");
    assert_eq!(runtime.game_data.conquest.game_year(), 3001);

    let _ = fs::remove_dir_all(&target);
}
