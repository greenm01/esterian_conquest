use std::path::PathBuf;
use std::process::Command;

use ec_client::startup::StartupArtConfig;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn startup_config_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config/startup.default.kdl")
}

#[test]
fn startup_default_kdl_parses() {
    let config =
        StartupArtConfig::load_kdl(&startup_config_path()).expect("startup config should parse");
    assert!(config.bbs_art_path.exists(), "bbs art should exist");
    assert!(config.ec_game_art_path.exists(), "ec art should exist");
}

#[test]
fn client_renders_startup_splash_from_fixture() {
    let fixture_dir = repo_root().join("fixtures/ecutil-init/v1.5");
    let output = Command::new(env!("CARGO_BIN_EXE_ec-client"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--player",
            "1",
        ])
        .output()
        .expect("ec-client should run");

    assert!(
        output.status.success(),
        "ec-client failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("THE BATTLE FIELD BBS"));
    assert!(stdout.contains("Press any key to continue."));
}
