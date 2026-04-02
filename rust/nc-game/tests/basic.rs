use std::path::PathBuf;
use std::process::Stdio;
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use std::time::Duration;
use std::{fs, path::Path};
use std::{io::Write, process::Child};

use nc_compat::import_directory_snapshot;
use nc_data::{CampaignSettings, CampaignStore, CoreGameData};
use nc_game::terminal::ColorMode;

static TEMP_DIR_SEQ: AtomicU64 = AtomicU64::new(0);
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn temp_fixture_copy() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "nc-game-basic-{}-{}-{}",
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
    store
        .save_campaign_settings(&CampaignSettings::new("fixture-game", "Esterian Conquest"))
        .expect("seed campaign settings");
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
    let store = CampaignStore::open_default_in_dir(root).expect("open campaign store");
    let mut settings = store.load_campaign_settings().expect("load settings");
    settings.reservations = vec![nc_data::SeatReservation {
        player_record_index_1_based: player,
        alias: alias.to_string(),
    }];
    store
        .save_campaign_settings(&settings)
        .expect("write settings");
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

fn wait_for_exit(child: &mut Child, timeout: Duration) -> std::process::ExitStatus {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait().expect("poll child status") {
            return status;
        }
        if std::time::Instant::now() >= deadline {
            child.kill().expect("kill hung child");
            return child.wait().expect("wait after kill");
        }
        thread::sleep(Duration::from_millis(25));
    }
}

struct EnvGuard {
    key: &'static str,
    prior: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: Option<&str>) -> Self {
        let prior = std::env::var(key).ok();
        unsafe {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        Self { key, prior }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            match &self.prior {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }
}

#[test]
fn client_renders_startup_splash_from_fixture() {
    let fixture_dir = temp_fixture_copy();
    let output = Command::new(env!("CARGO_BIN_EXE_nc-game"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--player",
            "1",
        ])
        .output()
        .expect("nc-game should run");

    assert!(
        output.status.success(),
        "nc-game failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "successful nc-game launch should be silent on stderr: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("_   _  ___  ____"));
    assert!(stdout.contains(&format!("NC v{}", env!("CARGO_PKG_VERSION"))));
    assert!(stdout.contains("View the game introduction? Y/[N] ->"));
    assert!(!stdout.contains("config.kdl"));
    assert!(!stdout.contains("theme.kdl"));
}

#[test]
fn reserved_dropfile_alias_can_launch_without_player_flag() {
    let fixture_dir = temp_fixture_copy();
    write_reserved_config(&fixture_dir, "SYSOP", 1);
    let dropfile = write_dropfile(&fixture_dir, "SYSOP");

    let output = Command::new(env!("CARGO_BIN_EXE_nc-game"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--dropfile",
            dropfile.to_str().expect("dropfile path should be utf-8"),
        ])
        .output()
        .expect("nc-game should run");

    assert!(
        output.status.success(),
        "nc-game failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "successful nc-game launch should be silent on stderr: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn reserved_dropfile_alias_rejects_mismatched_explicit_player() {
    let fixture_dir = temp_fixture_copy();
    write_reserved_config(&fixture_dir, "SYSOP", 1);
    let dropfile = write_dropfile(&fixture_dir, "SYSOP");

    let output = Command::new(env!("CARGO_BIN_EXE_nc-game"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--player",
            "2",
            "--dropfile",
            dropfile.to_str().expect("dropfile path should be utf-8"),
        ])
        .output()
        .expect("nc-game should run");

    assert!(!output.status.success(), "nc-game should reject mismatch");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("--player 2 does not match reserved seat 1"),
        "stderr={:?}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn unreserved_dropfile_alias_without_player_opens_first_time_menu() {
    let fixture_dir = temp_fixture_copy();
    write_reserved_config(&fixture_dir, "SYSOP", 1);
    let dropfile = write_dropfile(&fixture_dir, "RIVAL");

    let output = Command::new(env!("CARGO_BIN_EXE_nc-game"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--dropfile",
            dropfile.to_str().expect("dropfile path should be utf-8"),
        ])
        .output()
        .expect("nc-game should run");

    assert!(
        output.status.success(),
        "nc-game failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "successful nc-game launch should be silent on stderr: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("FIRST TIME MENU:"));
    assert!(stdout.contains("FIRST TIME COMMAND"));
}

#[test]
fn piped_dropfile_launch_stays_interactive_without_tty_stdout() {
    let fixture_dir = temp_fixture_copy();
    write_reserved_config(&fixture_dir, "SYSOP", 1);
    let dropfile = write_dropfile(&fixture_dir, "RIVAL");

    let mut child = Command::new(env!("CARGO_BIN_EXE_nc-game"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--dropfile",
            dropfile.to_str().expect("dropfile path should be utf-8"),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("nc-game should start");

    let mut stdin = child.stdin.take().expect("stdin should be piped");
    let writer = thread::spawn(move || {
        thread::sleep(Duration::from_millis(150));
        stdin.write_all(b"j").expect("write join key");
        stdin.flush().expect("flush join key");
        thread::sleep(Duration::from_millis(200));
        stdin.write_all(&[0x1b]).expect("write escape key");
        stdin.flush().expect("flush escape key");
        thread::sleep(Duration::from_millis(200));
        stdin.write_all(b"q").expect("write quit key");
        stdin.flush().expect("flush quit key");
    });

    let status = wait_for_exit(&mut child, Duration::from_secs(5));
    writer.join().expect("writer thread should finish");
    let output = child.wait_with_output().expect("collect child output");

    assert!(
        status.success(),
        "nc-game failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "successful nc-game launch should be silent on stderr: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("FIRST TIME MENU:"));
    assert!(stdout.contains("EMPIRE NAME"));
}

#[test]
fn explicit_player_out_of_range_is_refused() {
    let fixture_dir = temp_fixture_copy();

    let output = Command::new(env!("CARGO_BIN_EXE_nc-game"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--player",
            "9",
        ])
        .output()
        .expect("nc-game should run");

    assert!(
        !output.status.success(),
        "nc-game should reject bad --player"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--player 9 exceeds player count 4"),
        "stderr={:?}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn detect_color_mode_treats_modern_ssh_terms_as_rich_color() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let _colorterm = EnvGuard::set("COLORTERM", None);
    let _term = EnvGuard::set("TERM", Some("xterm-kitty"));

    assert_eq!(nc_game::cli::detect_color_mode(), ColorMode::TrueColor);
}

#[test]
fn detect_color_mode_uses_color256_for_non_dumb_term_without_legacy_hint() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let _colorterm = EnvGuard::set("COLORTERM", None);
    let _term = EnvGuard::set("TERM", Some("xterm"));

    assert_eq!(nc_game::cli::detect_color_mode(), ColorMode::Color256);
}

#[test]
fn detect_color_mode_keeps_ansi16_for_dumb_term() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let _colorterm = EnvGuard::set("COLORTERM", None);
    let _term = EnvGuard::set("TERM", Some("dumb"));

    assert_eq!(nc_game::cli::detect_color_mode(), ColorMode::Ansi16);
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

    let output = Command::new(env!("CARGO_BIN_EXE_nc-game"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--dropfile",
            dropfile.to_str().expect("dropfile path should be utf-8"),
        ])
        .output()
        .expect("nc-game should run");

    assert!(
        !output.status.success(),
        "nc-game should reject handle conflict"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("conflicts with stored player handle 'OTHER'"),
        "stderr={:?}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn nc_game_opt_in_log_file_captures_startup_without_stderr_noise() {
    let fixture_dir = temp_fixture_copy();
    let log_path = fixture_dir.join("nc-game.log");

    let output = Command::new(env!("CARGO_BIN_EXE_nc-game"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--player",
            "1",
            "--log-file",
            log_path.to_str().expect("log path should be utf-8"),
            "--log-level",
            "debug",
        ])
        .output()
        .expect("nc-game should run");

    assert!(
        output.status.success(),
        "nc-game failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "successful nc-game launch should stay silent on stderr: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let log = fs::read_to_string(&log_path).expect("log file should be created");
    assert!(log.contains("nc-game logging initialized"));
    assert!(log.contains("loaded nc-game app"));
}

#[test]
fn nc_game_rejects_invalid_log_level() {
    let fixture_dir = temp_fixture_copy();

    let output = Command::new(env!("CARGO_BIN_EXE_nc-game"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--player",
            "1",
            "--log-level",
            "loud",
        ])
        .output()
        .expect("nc-game should run");

    assert!(!output.status.success(), "invalid log level should fail");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("unknown log level 'loud'; expected error, warn, info, debug, or trace"),
        "stderr={:?}",
        String::from_utf8_lossy(&output.stderr)
    );
}
