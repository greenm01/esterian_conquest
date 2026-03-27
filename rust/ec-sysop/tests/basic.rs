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
    let output = run_ec_sysop_output(args, None);

    assert!(
        output.status.success(),
        "ec-sysop failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("stdout should be utf-8")
}

fn run_ec_sysop_failure(args: &[&str]) -> String {
    let output = run_ec_sysop_output(args, None);

    assert!(
        !output.status.success(),
        "ec-sysop unexpectedly succeeded: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stderr).expect("stderr should be utf-8")
}

fn run_ec_sysop_output(args: &[&str], cwd: Option<&PathBuf>) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ec-sysop"));
    cmd.args(args);
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }
    cmd.output().expect("ec-sysop should run")
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

#[test]
fn ec_sysop_new_game_accepts_year_flag() {
    let target = unique_temp_dir("ec-sysop-new-game-year");

    let stdout = run_ec_sysop(&[
        "new-game",
        target.to_str().expect("utf-8 path"),
        "--year",
        "3012",
        "--seed",
        "1515",
    ]);
    assert!(stdout.contains("Initialized new game"));
    assert!(stdout.contains("year=3012"));

    let game_data = CoreGameData::load(&target).expect("generated game should load");
    assert_eq!(game_data.conquest.game_year(), 3012);

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn ec_sysop_help_lists_public_subcommands() {
    let output = run_ec_sysop_output(&["--help"], None);
    assert!(output.status.success(), "help should succeed");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("new-game <target_dir>"));
    assert!(stdout.contains("maint <dir> [turns]"));
    assert!(stdout.contains("nostr init [--identity <path>]"));
    assert!(stdout.contains("nostr serve [--config <path>] [--identity <path>]"));
}

#[test]
fn ec_sysop_new_game_help_does_not_treat_help_as_target_dir() {
    let cwd = unique_temp_dir("ec-sysop-help-cwd");
    let output = run_ec_sysop_output(&["new-game", "--help"], Some(&cwd));
    assert!(output.status.success(), "new-game help should succeed");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("new-game <target_dir>"));
    assert!(!stdout.contains("Initialized new game"));
    assert!(
        !cwd.join("--help").exists(),
        "help must not create a campaign"
    );
    let _ = fs::remove_dir_all(&cwd);
}

#[test]
fn ec_sysop_maint_help_prints_usage_without_running_maintenance() {
    let cwd = unique_temp_dir("ec-sysop-maint-help");
    let output = run_ec_sysop_output(&["maint", "--help"], Some(&cwd));
    assert!(output.status.success(), "maint help should succeed");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stdout.contains("maint <dir> [turns]"));
    assert!(!stdout.contains("Running Rust maintenance"));
    assert!(stderr.is_empty(), "maint help should not emit an error");
    let _ = fs::remove_dir_all(&cwd);
}

#[test]
fn ec_sysop_unknown_subcommand_fails_with_full_usage() {
    let output = run_ec_sysop_output(&["badcmd"], None);
    assert!(!output.status.success(), "unknown subcommand should fail");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stdout.contains("new-game <target_dir>"));
    assert!(stdout.contains("maint <dir> [turns]"));
    assert!(stdout.contains("nostr init [--identity <path>]"));
    assert!(stderr.contains("unknown subcommand: badcmd"));
}

#[test]
fn ec_sysop_nostr_help_prints_usage() {
    let output = run_ec_sysop_output(&["nostr", "--help"], None);
    assert!(output.status.success(), "nostr help should succeed");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("nostr init [--identity <path>]"));
    assert!(stdout.contains("nostr serve [--config <path>] [--identity <path>]"));
}

#[test]
fn ec_sysop_nostr_init_help_prints_usage_without_creating_identity() {
    let cwd = unique_temp_dir("ec-sysop-nostr-init-help");
    let output = run_ec_sysop_output(&["nostr", "init", "--help"], Some(&cwd));
    assert!(output.status.success(), "nostr init help should succeed");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("nostr init [--identity <path>]"));
    assert!(
        fs::read_dir(&cwd)
            .expect("cwd should exist")
            .next()
            .is_none(),
        "nostr init help must not create files"
    );
    let _ = fs::remove_dir_all(&cwd);
}

#[test]
fn ec_sysop_nostr_serve_help_prints_usage() {
    let output = run_ec_sysop_output(&["nostr", "serve", "--help"], None);
    assert!(output.status.success(), "nostr serve help should succeed");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("nostr serve [--config <path>] [--identity <path>]"));
}

#[test]
fn ec_sysop_nostr_init_creates_identity_at_requested_path() {
    let root = unique_temp_dir("ec-sysop-nostr-init");
    let identity_path = root.join("identity.kdl");

    let stdout = run_ec_sysop(&[
        "nostr",
        "init",
        "--identity",
        identity_path.to_str().expect("utf-8 path"),
    ]);
    assert!(stdout.contains("Daemon identity created at:"));
    assert!(stdout.contains("Public key (npub): npub1"));
    assert!(identity_path.exists(), "identity file should be created");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn ec_sysop_nostr_init_is_safe_to_rerun() {
    let root = unique_temp_dir("ec-sysop-nostr-init-rerun");
    let identity_path = root.join("identity.kdl");

    let first = run_ec_sysop(&[
        "nostr",
        "init",
        "--identity",
        identity_path.to_str().expect("utf-8 path"),
    ]);
    assert!(first.contains("Daemon identity created at:"));

    let second = run_ec_sysop(&[
        "nostr",
        "init",
        "--identity",
        identity_path.to_str().expect("utf-8 path"),
    ]);
    assert!(second.contains("Daemon identity already exists at:"));
    assert!(second.contains("Created:"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn ec_sysop_help_mentions_logging_flags() {
    let output = run_ec_sysop_output(&["--help"], None);
    assert!(output.status.success(), "help should succeed");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("--log-file <path>"));
    assert!(stdout.contains("--log-level <error|warn|info|debug|trace>"));
}

#[test]
fn ec_sysop_opt_in_log_file_captures_command_lifecycle() {
    let root = unique_temp_dir("ec-sysop-log-file");
    let target = root.join("game");
    let log_path = root.join("ec-sysop.log");

    let output = run_ec_sysop_output(
        &[
            "--log-file",
            log_path.to_str().expect("utf-8 path"),
            "--log-level",
            "debug",
            "new-game",
            target.to_str().expect("utf-8 path"),
            "--seed",
            "1515",
        ],
        None,
    );

    assert!(
        output.status.success(),
        "ec-sysop failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("Initialized new game"),
        "stdout={:?}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        output.stderr.is_empty(),
        "successful ec-sysop run should not emit stderr: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let log = fs::read_to_string(&log_path).expect("log file should exist");
    assert!(log.contains("ec-sysop logging initialized"));
    assert!(log.contains("running ec-sysop new-game"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn ec_sysop_rejects_invalid_log_level() {
    let output = run_ec_sysop_output(&["--log-level", "loud", "--help"], None);
    assert!(!output.status.success(), "invalid log level should fail");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(
        stderr.contains("unknown log level 'loud'; expected error, warn, info, debug, or trace")
    );
}
