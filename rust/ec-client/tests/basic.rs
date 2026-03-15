use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, path::Path};

use ec_client::screen::GAME_VERSION;
use ec_data::CampaignStore;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn temp_fixture_copy() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ec-client-basic-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ));
    copy_dir_all(&repo_root().join("fixtures/ecutil-init/v1.5"), &root);
    CampaignStore::open_default_in_dir(&root)
        .expect("open campaign store")
        .import_directory_snapshot(&root)
        .expect("seed sqlite snapshot");
    root
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
fn client_renders_startup_splash_from_fixture() {
    let fixture_dir = temp_fixture_copy();
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
    assert!(stdout.contains("#######"));
    assert!(stdout.contains(&format!("Esterian Conquest Ver {GAME_VERSION}")));
    assert!(stdout.contains("View the game introduction? Y/[N] ->"));
}
