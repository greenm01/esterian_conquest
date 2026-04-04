//! Regression tests for SSH key provisioning (step 7).

use std::fs;
use std::path::PathBuf;

use nc_gate::config::{AuthKeysMethod, DEFAULT_NC_GAME_PATH, GateConfig};
use nc_gate::serve::provision::{provision_key, reap_expired_keys, remove_key};
use nc_gate::serve::routing::ResolvedSeat;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn temp_dir() -> PathBuf {
    let base = std::env::temp_dir();
    let id = format!(
        "nc_gate_prov_test_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let path = base.join(id);
    fs::create_dir_all(&path).unwrap();
    path
}

fn config_command(auth_keys_path: PathBuf) -> GateConfig {
    GateConfig {
        relay: "wss://relay.example.com".to_string(),
        ssh_host: "play.example.com".to_string(),
        ssh_port: 22,
        ssh_user: "ecgame".to_string(),
        nc_game_path: PathBuf::from(DEFAULT_NC_GAME_PATH),
        nc_game_log_file: None,
        nc_game_log_level: None,
        auth_keys_method: AuthKeysMethod::Command,
        auth_keys_path,
        key_ttl: 60,
        games: vec![],
    }
}

fn config_file(auth_keys_path: PathBuf) -> GateConfig {
    GateConfig {
        relay: "wss://relay.example.com".to_string(),
        ssh_host: "play.example.com".to_string(),
        ssh_port: 22,
        ssh_user: "ecgame".to_string(),
        nc_game_path: PathBuf::from(DEFAULT_NC_GAME_PATH),
        nc_game_log_file: None,
        nc_game_log_level: None,
        auth_keys_method: AuthKeysMethod::File,
        auth_keys_path,
        key_ttl: 60,
        games: vec![],
    }
}

fn seat(game_id: &str, player: usize) -> ResolvedSeat {
    ResolvedSeat {
        game_id: game_id.to_string(),
        game_name: "Test Game".to_string(),
        player,
        player_npub: "npub1test000".to_string(),
        first_claim: false,
    }
}

const TEST_SSH_PUBKEY: &str =
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIBGk6testTestTestTestTestTestTestTestTestAA test";
const TEST_SESSION_TOKEN: &str = "session-test-token";

const GAME_DIR: &str = "/srv/ec/friday-night";

fn provision(
    config: &GateConfig,
    seat: &ResolvedSeat,
    ssh_pubkey: &str,
    game_dir: &PathBuf,
) -> nc_gate::serve::provision::ProvisionedKey {
    provision_key(config, seat, ssh_pubkey, game_dir, TEST_SESSION_TOKEN, None)
        .expect("provision_key should succeed")
}

// ---------------------------------------------------------------------------
// Command method tests
// ---------------------------------------------------------------------------

#[test]
fn command_provision_creates_key_file() {
    let dir = temp_dir();
    let config = config_command(dir.join("keys"));
    let game_dir = PathBuf::from(GAME_DIR);

    let provisioned = provision(
        &config,
        &seat("friday-night", 2),
        TEST_SSH_PUBKEY,
        &game_dir,
    );

    let key_path = config
        .auth_keys_path
        .join(format!("{}.key", provisioned.key_id));
    assert!(key_path.exists(), "key file should be created");
}

#[test]
fn command_key_file_contains_expires_and_entry() {
    let dir = temp_dir();
    let config = config_command(dir.join("keys"));
    let game_dir = PathBuf::from(GAME_DIR);

    let provisioned = provision(
        &config,
        &seat("friday-night", 2),
        TEST_SSH_PUBKEY,
        &game_dir,
    );

    let key_path = config
        .auth_keys_path
        .join(format!("{}.key", provisioned.key_id));
    let contents = fs::read_to_string(&key_path).unwrap();

    assert!(
        contents.starts_with("expires="),
        "first line should be expires=<ts>"
    );
    assert!(
        contents.contains(TEST_SSH_PUBKEY),
        "key file should contain the SSH pubkey"
    );
}

#[test]
fn command_key_entry_has_command_restriction() {
    let dir = temp_dir();
    let config = config_command(dir.join("keys"));
    let game_dir = PathBuf::from(GAME_DIR);

    let provisioned = provision(
        &config,
        &seat("friday-night", 2),
        TEST_SSH_PUBKEY,
        &game_dir,
    );

    assert!(
        provisioned.entry.contains("command="),
        "entry should contain command= restriction"
    );
    assert!(
        provisioned
            .entry
            .contains(&format!(r#"command="'{}'"#, DEFAULT_NC_GAME_PATH)),
        "entry should launch nc-game directly as the forced external command"
    );
    assert!(
        provisioned.entry.contains("--player 2"),
        "entry should contain the seat index"
    );
    assert!(
        provisioned
            .entry
            .contains("--session-token 'session-test-token'"),
        "entry should contain the DB-backed session token"
    );
    assert!(
        provisioned.entry.contains(GAME_DIR),
        "entry should contain the game dir"
    );
    assert!(
        provisioned.entry.contains(DEFAULT_NC_GAME_PATH),
        "entry should contain the nc-game binary path"
    );
    assert!(
        provisioned.entry.contains("no-port-forwarding"),
        "entry should contain no-port-forwarding"
    );
}

#[test]
fn command_key_entry_includes_hosted_invite_code_when_present() {
    let dir = temp_dir();
    let config = config_command(dir.join("keys"));
    let game_dir = PathBuf::from(GAME_DIR);

    let provisioned = provision_key(
        &config,
        &seat("friday-night", 2),
        TEST_SSH_PUBKEY,
        &game_dir,
        TEST_SESSION_TOKEN,
        Some("velvet-mountain"),
    )
    .expect("provision_key should succeed");

    assert!(
        provisioned
            .entry
            .contains("--hosted-invite-code 'velvet-mountain'")
    );
}

#[test]
fn command_key_entry_exports_nc_game_log_env_when_configured() {
    let dir = temp_dir();
    let mut config = config_command(dir.join("keys"));
    config.nc_game_log_file = Some(PathBuf::from("/var/log/nc-game audit.log"));
    config.nc_game_log_level = Some(nc_log::LogLevel::Trace);
    let game_dir = PathBuf::from(GAME_DIR);

    let provisioned = provision(
        &config,
        &seat("friday-night", 2),
        TEST_SSH_PUBKEY,
        &game_dir,
    );

    assert!(
        provisioned
            .entry
            .contains("NC_GAME_LOG_FILE='/var/log/nc-game audit.log'")
    );
    assert!(provisioned.entry.contains("NC_GAME_LOG_LEVEL='trace'"));
    assert!(provisioned.entry.contains("/usr/bin/env "));
    assert!(provisioned.entry.contains("'/usr/local/bin/nc-game'"));
    assert!(
        !provisioned.entry.contains(
            "env NC_GAME_LOG_FILE='/var/log/nc-game audit.log' NC_GAME_LOG_LEVEL='trace' exec"
        ),
        "entry should not ask env to launch a literal exec program"
    );
}

#[test]
fn command_remove_key_deletes_file() {
    let dir = temp_dir();
    let config = config_command(dir.join("keys"));
    let game_dir = PathBuf::from(GAME_DIR);

    let provisioned = provision(
        &config,
        &seat("friday-night", 1),
        TEST_SSH_PUBKEY,
        &game_dir,
    );

    remove_key(&config, &provisioned.key_id).expect("remove_key should succeed");

    let key_path = config
        .auth_keys_path
        .join(format!("{}.key", provisioned.key_id));
    assert!(!key_path.exists(), "key file should be removed");
}

#[test]
fn command_remove_key_nonexistent_is_ok() {
    let dir = temp_dir();
    let config = config_command(dir.join("keys"));
    // Should not error even if the file never existed.
    remove_key(&config, "nonexistent_key_id").expect("remove nonexistent key should be ok");
}

#[test]
fn command_reap_removes_expired_entry() {
    let dir = temp_dir();
    let mut config = config_command(dir.join("keys"));
    config.key_ttl = 0; // expires immediately

    let game_dir = PathBuf::from(GAME_DIR);
    let provisioned = provision(
        &config,
        &seat("friday-night", 1),
        TEST_SSH_PUBKEY,
        &game_dir,
    );

    // Sleep 1s so the timestamp is definitely past.
    std::thread::sleep(std::time::Duration::from_secs(1));

    let removed = reap_expired_keys(&config).expect("reap should succeed");
    assert!(removed >= 1, "reap should remove the expired entry");

    let key_path = config
        .auth_keys_path
        .join(format!("{}.key", provisioned.key_id));
    assert!(!key_path.exists(), "expired key file should be gone");
}

#[test]
fn command_reap_leaves_non_expired_entry() {
    let dir = temp_dir();
    let config = config_command(dir.join("keys"));

    let game_dir = PathBuf::from(GAME_DIR);
    let provisioned = provision(
        &config,
        &seat("friday-night", 1),
        TEST_SSH_PUBKEY,
        &game_dir,
    );

    let removed = reap_expired_keys(&config).expect("reap should succeed");
    assert_eq!(removed, 0, "non-expired entry should not be reaped");

    let key_path = config
        .auth_keys_path
        .join(format!("{}.key", provisioned.key_id));
    assert!(key_path.exists(), "non-expired key file should remain");
}

#[test]
fn command_reap_empty_dir_is_ok() {
    let dir = temp_dir();
    let config = config_command(dir.join("keys_empty"));
    // Directory does not exist yet — should be fine.
    let removed = reap_expired_keys(&config).expect("reap on empty/missing dir should be ok");
    assert_eq!(removed, 0);
}

// ---------------------------------------------------------------------------
// File method tests
// ---------------------------------------------------------------------------

#[test]
fn file_provision_appends_block() {
    let dir = temp_dir();
    let config = config_file(dir.join("authorized_keys"));
    let game_dir = PathBuf::from(GAME_DIR);

    let provisioned = provision(
        &config,
        &seat("friday-night", 3),
        TEST_SSH_PUBKEY,
        &game_dir,
    );

    let contents = fs::read_to_string(&config.auth_keys_path).unwrap();
    assert!(
        contents.contains(&provisioned.key_id),
        "authorized_keys should contain the key_id marker"
    );
    assert!(
        contents.contains(TEST_SSH_PUBKEY),
        "authorized_keys should contain the SSH pubkey"
    );
    assert!(
        contents.contains("command="),
        "authorized_keys should contain command= restriction"
    );
}

#[test]
fn file_remove_key_strips_block() {
    let dir = temp_dir();
    let config = config_file(dir.join("authorized_keys"));
    let game_dir = PathBuf::from(GAME_DIR);

    let provisioned = provision(
        &config,
        &seat("friday-night", 1),
        TEST_SSH_PUBKEY,
        &game_dir,
    );

    remove_key(&config, &provisioned.key_id).expect("remove_key should succeed");

    let contents = fs::read_to_string(&config.auth_keys_path).unwrap_or_default();
    assert!(
        !contents.contains(TEST_SSH_PUBKEY),
        "SSH pubkey should be removed from authorized_keys"
    );
    assert!(
        !contents.contains(&provisioned.key_id),
        "key_id marker should be removed from authorized_keys"
    );
}

#[test]
fn file_remove_key_leaves_other_entries() {
    let dir = temp_dir();
    let config = config_file(dir.join("authorized_keys"));
    let game_dir = PathBuf::from(GAME_DIR);

    let ssh_a = "ssh-ed25519 AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA comment_a";
    let ssh_b = "ssh-ed25519 BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB comment_b";

    let key_a = provision(&config, &seat("friday-night", 1), ssh_a, &game_dir);
    let _key_b = provision(&config, &seat("friday-night", 2), ssh_b, &game_dir);

    remove_key(&config, &key_a.key_id).expect("remove key A");

    let contents = fs::read_to_string(&config.auth_keys_path).unwrap();
    assert!(!contents.contains(ssh_a), "key A pubkey should be gone");
    assert!(contents.contains(ssh_b), "key B pubkey should remain");
}

#[test]
fn file_reap_removes_expired_block() {
    let dir = temp_dir();
    let mut config = config_file(dir.join("authorized_keys"));
    config.key_ttl = 0;

    let game_dir = PathBuf::from(GAME_DIR);
    let provisioned = provision(
        &config,
        &seat("friday-night", 1),
        TEST_SSH_PUBKEY,
        &game_dir,
    );

    std::thread::sleep(std::time::Duration::from_secs(1));

    let removed = reap_expired_keys(&config).expect("reap should succeed");
    assert!(removed >= 1, "reap should remove the expired block");

    let contents = fs::read_to_string(&config.auth_keys_path).unwrap_or_default();
    assert!(
        !contents.contains(&provisioned.key_id),
        "expired block marker should be gone"
    );
}

#[test]
fn file_reap_leaves_non_expired_block() {
    let dir = temp_dir();
    let config = config_file(dir.join("authorized_keys"));
    let game_dir = PathBuf::from(GAME_DIR);

    let provisioned = provision(
        &config,
        &seat("friday-night", 1),
        TEST_SSH_PUBKEY,
        &game_dir,
    );

    let removed = reap_expired_keys(&config).expect("reap should succeed");
    assert_eq!(removed, 0, "non-expired block should not be reaped");

    let contents = fs::read_to_string(&config.auth_keys_path).unwrap();
    assert!(
        contents.contains(&provisioned.key_id),
        "non-expired block marker should remain"
    );
}

#[test]
fn file_reap_missing_file_is_ok() {
    let dir = temp_dir();
    let config = config_file(dir.join("no_such_authorized_keys"));
    let removed = reap_expired_keys(&config).expect("reap on missing file should be ok");
    assert_eq!(removed, 0);
}
