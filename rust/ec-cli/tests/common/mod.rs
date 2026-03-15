#![allow(dead_code)]

use ec_data::{CoreGameData, DiplomaticRelation};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn rust_workspace() -> PathBuf {
    repo_root().join("rust")
}

pub fn run_ec_cli(args: &[&str]) -> String {
    run_ec_cli_in_dir(args, rust_workspace())
}

pub fn run_ec_cli_in_dir(args: &[&str], current_dir: PathBuf) -> String {
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

pub fn run_ec_cli_failure_in_dir(args: &[&str], current_dir: PathBuf) -> String {
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

pub fn import_campaign_db(target: &Path) {
    run_ec_cli_in_dir(&["db-import", target.to_str().unwrap()], rust_workspace());
}

pub fn export_campaign_db(source_dir: &Path, target_dir: &Path) {
    run_ec_cli_in_dir(
        &[
            "db-export",
            source_dir.to_str().unwrap(),
            target_dir.to_str().unwrap(),
        ],
        rust_workspace(),
    );
}

pub fn run_maint_rust_with_export(target: &Path, turns: u16) -> String {
    import_campaign_db(target);
    let turns = turns.to_string();
    let stdout = run_ec_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), &turns],
        rust_workspace(),
    );
    export_campaign_db(target, target);
    stdout
}

pub fn run_maint_rust_failure_after_import(target: &Path, turns: u16) -> String {
    import_campaign_db(target);
    let turns = turns.to_string();
    run_ec_cli_failure_in_dir(
        &["maint-rust", target.to_str().unwrap(), &turns],
        rust_workspace(),
    )
}

pub fn run_ecmaint_oracle(dir: &Path) -> String {
    let output = Command::new("python3")
        .current_dir(repo_root())
        .args(["tools/ecmaint_oracle.py", "run", dir.to_str().unwrap()])
        .env("SDL_VIDEODRIVER", "dummy")
        .env("SDL_AUDIODRIVER", "dummy")
        .output()
        .expect("ecmaint oracle should run");

    assert!(
        output.status.success(),
        "ecmaint oracle failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("oracle stdout should be utf-8")
}

pub fn unique_temp_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("{prefix}-{unique}"));
    fs::create_dir_all(&target).unwrap();
    target
}

pub fn cleanup_dir(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

pub fn copy_fixture_dir(relative: &str, target: &Path) {
    let fixture = repo_root().join(relative);
    copy_dir_files(&fixture, target);
}

pub fn copy_dir_files(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if !entry.file_type().unwrap().is_file() {
            continue;
        }
        fs::copy(entry.path(), target.join(entry.file_name())).unwrap();
    }
}

pub fn write_mutual_enemy_diplomacy(target: &Path, left: u8, right: u8) {
    let text = format!(
        "relation from={} to={} status=\"enemy\"\nrelation from={} to={} status=\"enemy\"\n",
        left, right, right, left
    );
    fs::write(target.join("diplomacy.kdl"), text).unwrap();
}

pub fn set_mutual_enemy_in_player_dat(target: &Path, left: u8, right: u8) {
    let mut game_data = CoreGameData::load(target).unwrap();
    game_data
        .set_stored_diplomatic_relation(left, right, DiplomaticRelation::Enemy)
        .unwrap();
    game_data
        .set_stored_diplomatic_relation(right, left, DiplomaticRelation::Enemy)
        .unwrap();
    game_data.save(target).unwrap();
}
