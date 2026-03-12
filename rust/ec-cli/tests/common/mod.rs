#![allow(dead_code)]

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
