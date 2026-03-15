use std::path::PathBuf;
use std::process::Command;

use ec_client::startup::StartupArt;
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
fn startup_ec_art_projection_keeps_version_and_art() {
    let config =
        StartupArtConfig::load_kdl(&startup_config_path()).expect("startup config should parse");
    let art = StartupArt::load(&config.ec_game_art_path).expect("ec art should load");
    let playfield = art.render();
    let lines = (0..playfield.height())
        .map(|row| playfield.plain_line(row))
        .collect::<Vec<_>>();
    let full = lines.join("\n");

    assert!(full.contains("v1.60"), "rendered art:\n{full}");
    assert!(!full.contains("Blade"));
    assert!(!full.contains("Registration #"));
    assert!(
        full.contains("ANSI ART CONTRIBUTIONS NEEDED")
            || full.contains("▒██████")
            || full.contains("██████")
            || full.contains("######"),
        "projected art lost the banner body: {full}"
    );
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
    assert!(stdout.contains("INSERT YOUR ANSI ART HERE"));
    assert!(stdout.contains("Slap a key."));
}
