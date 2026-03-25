use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, path::Path};

use ec_compat::import_directory_snapshot;
use ec_data::{CampaignStore, CoreGameData};

static TEMP_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn temp_fixture_copy() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ec-game-basic-{}-{}-{}",
        std::process::id(),
        TEMP_DIR_SEQ.fetch_add(1, Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ));
    copy_dir_all(&repo_root().join("fixtures/ecutil-init/v1.5"), &root);
    let store = CampaignStore::open_default_in_dir(&root).expect("open campaign store");
    import_directory_snapshot(&store, &root).expect("seed sqlite snapshot");
    root
}

fn write_dropfile(root: &Path, alias: &str) -> PathBuf {
    let path = root.join("DOOR32.SYS");
    fs::write(
        &path,
        format!("2\n1\n57600\nEnigma\n1\nReal Name\n{alias}\n10\n15\n1\n80\n25\n"),
    )
    .expect("write dropfile");
    path
}

fn write_reserved_config(root: &Path, alias: &str, player: usize) {
    fs::write(
        root.join("config.kdl"),
        format!(
            "game_name \"Esterian Conquest\"\n\
             snoop #true\n\
             session {{\n\
                 max_idle_minutes 10\n\
                 minimum_time_minutes 0\n\
                 local_timeout #false\n\
                 remote_timeout #true\n\
             }}\n\
             inactivity {{\n\
                 purge_after_turns 0\n\
                 autopilot_after_turns 0\n\
             }}\n\
             reservations {{\n\
                 seat player={player} alias=\"{alias}\"\n\
             }}\n"
        ),
    )
    .expect("write config");
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
    let output = Command::new(env!("CARGO_BIN_EXE_ec-game"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--player",
            "1",
        ])
        .output()
        .expect("ec-game should run");

    assert!(
        output.status.success(),
        "ec-game failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("#######"));
    assert!(stdout.contains(&format!("EC v{}", env!("CARGO_PKG_VERSION"))));
    assert!(stdout.contains("View the game introduction? Y/[N] ->"));
}

#[test]
fn reserved_dropfile_alias_can_launch_without_player_flag() {
    let fixture_dir = temp_fixture_copy();
    write_reserved_config(&fixture_dir, "SYSOP", 1);
    let dropfile = write_dropfile(&fixture_dir, "SYSOP");

    let output = Command::new(env!("CARGO_BIN_EXE_ec-game"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--dropfile",
            dropfile.to_str().expect("dropfile path should be utf-8"),
        ])
        .output()
        .expect("ec-game should run");

    assert!(
        output.status.success(),
        "ec-game failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn reserved_dropfile_alias_rejects_mismatched_explicit_player() {
    let fixture_dir = temp_fixture_copy();
    write_reserved_config(&fixture_dir, "SYSOP", 1);
    let dropfile = write_dropfile(&fixture_dir, "SYSOP");

    let output = Command::new(env!("CARGO_BIN_EXE_ec-game"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--player",
            "2",
            "--dropfile",
            dropfile.to_str().expect("dropfile path should be utf-8"),
        ])
        .output()
        .expect("ec-game should run");

    assert!(!output.status.success(), "ec-game should reject mismatch");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("--player 2 does not match reserved seat 1"),
        "stderr={:?}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn unreserved_dropfile_alias_without_player_still_requires_player() {
    let fixture_dir = temp_fixture_copy();
    write_reserved_config(&fixture_dir, "SYSOP", 1);
    let dropfile = write_dropfile(&fixture_dir, "RIVAL");

    let output = Command::new(env!("CARGO_BIN_EXE_ec-game"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--dropfile",
            dropfile.to_str().expect("dropfile path should be utf-8"),
        ])
        .output()
        .expect("ec-game should run");

    assert!(!output.status.success(), "ec-game should require --player");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("reserve the dropfile alias in config.kdl"),
        "stderr={:?}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn reserved_dropfile_alias_rejects_conflicting_stored_player_handle() {
    let fixture_dir = temp_fixture_copy();
    let mut data = CoreGameData::load(&fixture_dir).expect("load fixture");
    data.join_player(1, "Codex Dominion")
        .expect("join player for mismatch test");
    data.player.records[0].set_assigned_player_handle_raw("OTHER");
    data.save(&fixture_dir).expect("save fixture");
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    import_directory_snapshot(&store, &fixture_dir).expect("refresh sqlite snapshot");

    write_reserved_config(&fixture_dir, "SYSOP", 1);
    let dropfile = write_dropfile(&fixture_dir, "SYSOP");

    let output = Command::new(env!("CARGO_BIN_EXE_ec-game"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--dropfile",
            dropfile.to_str().expect("dropfile path should be utf-8"),
        ])
        .output()
        .expect("ec-game should run");

    assert!(
        !output.status.success(),
        "ec-game should reject handle conflict"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("conflicts with stored player handle 'OTHER'"),
        "stderr={:?}",
        String::from_utf8_lossy(&output.stderr)
    );
}
