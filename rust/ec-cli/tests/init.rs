use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn init_fixture_dir() -> PathBuf {
    repo_root().join("fixtures/ecutil-init/v1.5")
}

#[test]
fn init_creates_known_good_fixture_set() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("ec-cli-init-{unique}"));
    let output = Command::new(env!("CARGO_BIN_EXE_ec-cli"))
        .current_dir(repo_root().join("rust"))
        .args(["init", "original/v1.5"])
        .arg(&target)
        .output()
        .expect("ec-cli should run");

    assert!(
        output.status.success(),
        "ec-cli init failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let init = init_fixture_dir();
    for name in [
        "BASES.DAT",
        "CONQUEST.DAT",
        "DATABASE.DAT",
        "FLEETS.DAT",
        "IPBM.DAT",
        "MESSAGES.DAT",
        "PLANETS.DAT",
        "PLAYER.DAT",
        "RESULTS.DAT",
        "SETUP.DAT",
    ] {
        let actual = fs::read(target.join(name)).unwrap();
        let expected = fs::read(init.join(name)).unwrap();
        assert_eq!(actual, expected, "{name} should match initialized fixture");
    }

    let _ = fs::remove_dir_all(&target);
}
