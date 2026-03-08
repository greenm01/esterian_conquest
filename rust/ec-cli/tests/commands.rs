use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run_ec_cli_in_dir(args: &[&str], current_dir: PathBuf) -> String {
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

fn run_ec_cli(args: &[&str]) -> String {
    run_ec_cli_in_dir(args, repo_root().join("rust"))
}

#[test]
fn match_identifies_original_fixture() {
    let stdout = run_ec_cli(&["match", "original/v1.5"]);
    assert!(stdout.contains("MATCH original/v1.5"));
}

#[test]
fn match_identifies_initialized_fixture() {
    let stdout = run_ec_cli(&["match", "fixtures/ecutil-init/v1.5"]);
    assert!(stdout.contains("MATCH fixtures/ecutil-init/v1.5"));
}

#[test]
fn headers_prints_known_setup_and_conquest_values() {
    let stdout = run_ec_cli(&["headers", "original/v1.5"]);
    assert!(stdout.contains("SETUP.version=EC151"));
    assert!(stdout.contains("SETUP.option_prefix=[04, 03, 04, 03, 01, 01, 01, 01]"));
    assert!(stdout.contains("SETUP.snoop_enabled=true"));
    assert!(stdout.contains("SETUP.local_timeout_enabled=false"));
    assert!(stdout.contains("SETUP.remote_timeout_enabled=true"));
    assert!(stdout.contains("SETUP.max_time_between_keys_minutes_raw=10"));
    assert!(stdout.contains("SETUP.minimum_time_granted_minutes_raw=0"));
    assert!(stdout.contains("SETUP.purge_after_turns_raw=0"));
    assert!(stdout.contains("SETUP.autopilot_inactive_turns_raw=0"));
    assert!(stdout.contains("CONQUEST.game_year=3022"));
    assert!(stdout.contains("CONQUEST.player_count=4"));
    assert!(stdout.contains("CONQUEST.player_config_word=0104"));
    assert!(stdout.contains("CONQUEST.maintenance_schedule=[01, 01, 01, 01, 01, 01, 01]"));
    assert!(stdout.contains("CONQUEST.header_len=85"));
    assert!(stdout.contains("0bce"));
}

#[test]
fn headers_accepts_relative_fixture_paths_from_rust_workspace() {
    let stdout = run_ec_cli(&["headers", "../fixtures/ecutil-init/v1.5"]);
    assert!(stdout.contains("CONQUEST.game_year=3000"));
    assert!(stdout.contains("CONQUEST.player_count=4"));
}

#[test]
fn compare_reports_expected_initialized_to_post_maint_shape() {
    let stdout = run_ec_cli(&[
        "compare",
        "fixtures/ecutil-init/v1.5",
        "fixtures/ecmaint-post/v1.5",
    ]);
    assert!(stdout.contains("SETUP.DAT: size 522 vs 522, differing bytes 0"));
    assert!(stdout.contains("CONQUEST.DAT: size 2085 vs 2085, differing bytes 51"));
    assert!(stdout.contains("DATABASE.DAT: size 8000 vs 8000, differing bytes 80"));
    assert!(stdout.contains("FLEETS.DAT: size 864 vs 864, differing bytes 0"));
}

#[test]
fn maintenance_days_set_rewrites_conquest_schedule() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("ec-cli-maint-{unique}"));
    fs::create_dir_all(&target).unwrap();

    let fixture = repo_root().join("fixtures/ecmaint-post/v1.5");
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
        fs::copy(fixture.join(name), target.join(name)).unwrap();
    }

    let stdout = run_ec_cli_in_dir(
        &["maintenance-days", target.to_str().unwrap(), "set", "sun", "tue", "thu", "sat"],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("sun=yes mon=no tue=yes wed=no thu=yes fri=no sat=yes"));
    assert!(stdout.contains("Maintenance raw: [01, 00, ca, 00, 0a, 00, 26]"));

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn snoop_off_rewrites_setup_flag() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("ec-cli-snoop-{unique}"));
    fs::create_dir_all(&target).unwrap();

    let fixture = repo_root().join("original/v1.5");
    fs::copy(fixture.join("SETUP.DAT"), target.join("SETUP.DAT")).unwrap();

    let stdout = run_ec_cli_in_dir(
        &["snoop", target.to_str().unwrap(), "off"],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("Snoop enabled: no"));

    let stdout = run_ec_cli_in_dir(
        &["snoop", target.to_str().unwrap()],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("Snoop enabled: no"));

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn purge_after_rewrites_setup_raw_value() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("ec-cli-purge-{unique}"));
    fs::create_dir_all(&target).unwrap();

    let fixture = repo_root().join("original/v1.5");
    fs::copy(fixture.join("SETUP.DAT"), target.join("SETUP.DAT")).unwrap();

    let stdout = run_ec_cli_in_dir(
        &["purge-after", target.to_str().unwrap(), "1"],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("Purge after turns (raw): 1"));

    let stdout = run_ec_cli_in_dir(
        &["purge-after", target.to_str().unwrap()],
        repo_root().join("rust"),
    );
    assert!(stdout.contains("Purge after turns (raw): 1"));

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn setup_programs_prints_mapped_f4_values() {
    let stdout = run_ec_cli(&["setup-programs", "original/v1.5"]);
    assert!(stdout.contains("ECUTIL F4 Modify Program Options"));
    assert!(stdout.contains("A Purge messages & reports after: 0 turn(s)"));
    assert!(stdout.contains("B Autopilot any empires inactive for: 0 turn(s)"));
    assert!(stdout.contains("C Snoop Enabled: Yes"));
    assert!(stdout.contains("D Enable timeout for local users: No"));
    assert!(stdout.contains("E Enable timeout for remote users: Yes"));
    assert!(stdout.contains("F Maximum time between key strokes: 10 minute(s)"));
    assert!(stdout.contains("G Minimum time granted: 0 minute(s)"));
}
