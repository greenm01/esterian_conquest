//! Regression tests for gate configuration KDL parsing and path resolution.

use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use ec_gate::config::io::{config_path, load_config, parse_config_str, render_config, save_config};
use ec_gate::config::{AuthKeysMethod, DEFAULT_EC_GAME_PATH};

// --- Canonical round-trip ---

const CANONICAL_CONFIG: &str = r#"
relay "wss://relay.example.com"
ssh-host "play.example.com"
ssh-port 22
ssh-user "ecgame"
ec-game-path "/opt/ec/bin/ec-game"
auth-keys-method "command"
auth-keys-path "/var/lib/ec-gate/keys"
key-ttl 60
game "/srv/ec/friday-night"
game "/srv/ec/saturday-showdown"
"#;

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[test]
fn parse_canonical_config() {
    let cfg = parse_config_str(CANONICAL_CONFIG).expect("parse failed");
    assert_eq!(cfg.relay, "wss://relay.example.com");
    assert_eq!(cfg.ssh_host, "play.example.com");
    assert_eq!(cfg.ssh_port, 22);
    assert_eq!(cfg.ssh_user, "ecgame");
    assert_eq!(cfg.ec_game_path, PathBuf::from("/opt/ec/bin/ec-game"));
    assert_eq!(cfg.auth_keys_method, AuthKeysMethod::Command);
    assert_eq!(cfg.auth_keys_path, PathBuf::from("/var/lib/ec-gate/keys"));
    assert_eq!(cfg.key_ttl, 60);
    assert_eq!(
        cfg.games,
        vec![
            PathBuf::from("/srv/ec/friday-night"),
            PathBuf::from("/srv/ec/saturday-showdown"),
        ]
    );
}

#[test]
fn parse_auth_keys_method_file() {
    let kdl = r#"
relay "wss://r.example.com"
ssh-host "h.example.com"
ssh-port 2222
ssh-user "ecgame"
auth-keys-method "file"
auth-keys-path "/home/ecgame/.ssh/authorized_keys"
key-ttl 120
game "/srv/ec/game1"
"#;
    let cfg = parse_config_str(kdl).expect("parse failed");
    assert_eq!(cfg.auth_keys_method, AuthKeysMethod::File);
    assert_eq!(cfg.ssh_port, 2222);
    assert_eq!(cfg.ec_game_path, PathBuf::from(DEFAULT_EC_GAME_PATH));
    assert_eq!(cfg.key_ttl, 120);
    assert_eq!(cfg.games, vec![PathBuf::from("/srv/ec/game1")]);
}

#[test]
fn parse_single_game() {
    let kdl = r#"
relay "wss://r.example.com"
ssh-host "h.example.com"
ssh-port 22
ssh-user "ecgame"
auth-keys-method "command"
auth-keys-path "/var/lib/ec-gate/keys"
key-ttl 30
game "/srv/ec/only-game"
"#;
    let cfg = parse_config_str(kdl).expect("parse failed");
    assert_eq!(cfg.games.len(), 1);
    assert_eq!(cfg.games[0], PathBuf::from("/srv/ec/only-game"));
}

// --- Error cases ---

#[test]
fn parse_missing_relay_is_error() {
    let kdl = r#"
ssh-host "h.example.com"
ssh-port 22
ssh-user "ecgame"
auth-keys-method "command"
auth-keys-path "/var/lib/ec-gate/keys"
key-ttl 60
game "/srv/ec/game1"
"#;
    let result = parse_config_str(kdl);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("relay"));
}

#[test]
fn parse_missing_ssh_host_is_error() {
    let kdl = r#"
relay "wss://r.example.com"
ssh-port 22
ssh-user "ecgame"
auth-keys-method "command"
auth-keys-path "/var/lib/ec-gate/keys"
key-ttl 60
game "/srv/ec/game1"
"#;
    let result = parse_config_str(kdl);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("ssh-host"));
}

#[test]
fn parse_missing_games_is_allowed() {
    let kdl = r#"
relay "wss://r.example.com"
ssh-host "h.example.com"
ssh-port 22
ssh-user "ecgame"
auth-keys-method "command"
auth-keys-path "/var/lib/ec-gate/keys"
key-ttl 60
"#;
    let cfg = parse_config_str(kdl).expect("parse failed");
    assert!(cfg.games.is_empty(), "config should allow zero games");
}

#[test]
fn parse_unknown_auth_keys_method_is_error() {
    let kdl = r#"
relay "wss://r.example.com"
ssh-host "h.example.com"
ssh-port 22
ssh-user "ecgame"
auth-keys-method "magic"
auth-keys-path "/var/lib/ec-gate/keys"
key-ttl 60
game "/srv/ec/game1"
"#;
    let result = parse_config_str(kdl);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("auth-keys-method"));
}

#[test]
fn parse_invalid_kdl_syntax_is_error() {
    let result = parse_config_str("this is not { valid kdl }}}");
    assert!(result.is_err());
}

// --- File I/O ---

#[test]
fn load_config_from_file() {
    let dir = std::env::temp_dir().join("ec-gate-config-test");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.kdl");
    fs::write(&path, CANONICAL_CONFIG).unwrap();

    let cfg = load_config(&path).expect("load failed");
    assert_eq!(cfg.relay, "wss://relay.example.com");
    assert_eq!(cfg.games.len(), 2);

    fs::remove_file(&path).ok();
    fs::remove_dir(&dir).ok();
}

#[test]
fn render_and_save_config_round_trip() {
    let dir = std::env::temp_dir().join("ec-gate-config-save-test");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.kdl");
    let cfg = parse_config_str(CANONICAL_CONFIG).expect("parse failed");

    save_config(&path, &cfg).expect("save failed");
    let round_trip = load_config(&path).expect("reload failed");
    let rendered = render_config(&round_trip);

    assert_eq!(round_trip.relay, cfg.relay);
    assert_eq!(round_trip.games, cfg.games);
    assert!(rendered.contains("relay \"wss://relay.example.com\""));
    assert!(rendered.contains("game \"/srv/ec/friday-night\""));

    fs::remove_file(&path).ok();
    fs::remove_dir(&dir).ok();
}

#[test]
fn load_config_missing_file_is_error() {
    let path = PathBuf::from("/tmp/ec-gate-no-such-config-file.kdl");
    let result = load_config(&path);
    assert!(result.is_err());
}

// --- Path resolution ---

#[test]
fn config_path_returns_xdg_config_home_when_set() {
    let _guard = env_lock().lock().expect("env lock");
    // XDG_CONFIG_HOME controls the user-level fallback, but the implementation
    // still prefers /etc/ec-gate when that directory exists.
    let tmp = std::env::temp_dir().join("ec-gate-xdg-test");
    let previous_xdg = std::env::var_os("XDG_CONFIG_HOME");
    // SAFETY: guarded by the test-local environment mutex.
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", tmp.to_str().unwrap());
    }
    let path = config_path();
    if let Some(value) = previous_xdg {
        // SAFETY: guarded by the test-local environment mutex.
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", value);
        }
    } else {
        // SAFETY: guarded by the test-local environment mutex.
        unsafe {
            std::env::remove_var("XDG_CONFIG_HOME");
        }
    }

    if PathBuf::from("/etc/ec-gate").exists() {
        assert_eq!(path, PathBuf::from("/etc/ec-gate/config.kdl"));
    } else {
        assert_eq!(path, tmp.join("ec-gate").join("config.kdl"));
    }
}

#[test]
fn config_path_falls_back_to_home_config() {
    let _guard = env_lock().lock().expect("env lock");
    // Remove XDG_CONFIG_HOME so the fallback path is used.
    let previous_xdg = std::env::var_os("XDG_CONFIG_HOME");
    // SAFETY: guarded by the test-local environment mutex.
    unsafe {
        std::env::remove_var("XDG_CONFIG_HOME");
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let path = config_path();
    if let Some(value) = previous_xdg {
        // SAFETY: guarded by the test-local environment mutex.
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", value);
        }
    }
    // Path should be under HOME/.config/ec-gate/config.kdl (unless /etc/ec-gate exists).
    if !PathBuf::from("/etc/ec-gate").exists() {
        assert_eq!(
            path,
            PathBuf::from(&home)
                .join(".config")
                .join("ec-gate")
                .join("config.kdl")
        );
    }
}
