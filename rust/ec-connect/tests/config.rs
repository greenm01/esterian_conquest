//! Regression tests for config I/O (step 3).

use std::path::PathBuf;

use ec_connect::config::io::{
    load_config_from, parse_config_str, render_config, save_config_to, seed_default_relay_at,
};
use ec_connect::config::{ConnectConfig, ServerBookmark, validate_relay_url};

// ---------------------------------------------------------------------------
// parse_config_str
// ---------------------------------------------------------------------------

#[test]
fn parse_empty_string_returns_empty_config() {
    let config = parse_config_str("").unwrap();
    assert!(config.relay.is_none());
    assert!(config.servers.is_empty());
    assert!(config.default_server.is_none());
    assert!(config.maps_dir.is_none());
    assert!(config.lock_timeout_minutes.is_none());
}

#[test]
fn parse_full_config() {
    let kdl = r#"
relay "wss://relay.example.com"
server "friday" host="play.example.com" port=22
server "local" host="localhost" port=2222
default "friday"
maps-dir "/tmp/ec-maps"
lock-timeout-minutes 7
"#;
    let config = parse_config_str(kdl).unwrap();
    assert_eq!(config.relay.as_deref(), Some("wss://relay.example.com"));
    assert_eq!(config.servers.len(), 2);
    assert_eq!(config.servers[0].name, "friday");
    assert_eq!(config.servers[0].host, "play.example.com");
    assert_eq!(config.servers[0].port, 22);
    assert_eq!(config.servers[1].name, "local");
    assert_eq!(config.servers[1].host, "localhost");
    assert_eq!(config.servers[1].port, 2222);
    assert_eq!(config.default_server.as_deref(), Some("friday"));
    assert_eq!(config.maps_dir, Some(PathBuf::from("/tmp/ec-maps")));
    assert_eq!(config.lock_timeout_minutes, Some(7));
}

#[test]
fn parse_server_without_port_defaults_to_22() {
    let kdl = "server \"home\" host=\"192.168.1.10\"\n";
    let config = parse_config_str(kdl).unwrap();
    assert_eq!(config.servers[0].port, 22);
}

#[test]
fn parse_relay_only() {
    let config = parse_config_str("relay \"wss://nostr.example.com\"\n").unwrap();
    assert_eq!(config.relay.as_deref(), Some("wss://nostr.example.com"));
    assert!(config.servers.is_empty());
    assert!(config.default_server.is_none());
}

#[test]
fn parse_unknown_nodes_are_ignored() {
    let kdl = "relay \"wss://r.example.com\"\nfuture-thing foo=\"bar\"\n";
    let config = parse_config_str(kdl).unwrap();
    assert_eq!(config.relay.as_deref(), Some("wss://r.example.com"));
}

#[test]
fn parse_relay_missing_arg_is_err() {
    let kdl = "relay\n";
    assert!(parse_config_str(kdl).is_err());
}

#[test]
fn parse_server_missing_host_is_err() {
    let kdl = "server \"x\" port=22\n";
    assert!(parse_config_str(kdl).is_err());
}

// ---------------------------------------------------------------------------
// render_config
// ---------------------------------------------------------------------------

#[test]
fn render_empty_config_is_empty_string() {
    let config = ConnectConfig::empty();
    assert_eq!(render_config(&config), "");
}

#[test]
fn render_full_config_roundtrip() {
    let config = ConnectConfig {
        relay: Some("wss://relay.example.com".to_string()),
        servers: vec![
            ServerBookmark {
                name: "alpha".to_string(),
                host: "alpha.example.com".to_string(),
                port: 22,
            },
            ServerBookmark {
                name: "local".to_string(),
                host: "localhost".to_string(),
                port: 2222,
            },
        ],
        default_server: Some("alpha".to_string()),
        maps_dir: Some(PathBuf::from("/tmp/ec-maps")),
        lock_timeout_minutes: Some(7),
    };

    let rendered = render_config(&config);
    let parsed = parse_config_str(&rendered).unwrap();

    assert_eq!(parsed.relay.as_deref(), Some("wss://relay.example.com"));
    assert_eq!(parsed.servers.len(), 2);
    assert_eq!(parsed.servers[0].name, "alpha");
    assert_eq!(parsed.servers[0].port, 22);
    assert_eq!(parsed.servers[1].name, "local");
    assert_eq!(parsed.servers[1].port, 2222);
    assert_eq!(parsed.default_server.as_deref(), Some("alpha"));
    assert_eq!(parsed.maps_dir, Some(PathBuf::from("/tmp/ec-maps")));
    assert_eq!(parsed.lock_timeout_minutes, Some(7));
}

#[test]
fn render_escapes_special_chars() {
    let config = ConnectConfig {
        relay: Some("wss://re\"lay.example.com".to_string()),
        servers: vec![],
        default_server: None,
        maps_dir: None,
        lock_timeout_minutes: None,
    };
    let rendered = render_config(&config);
    // Must parse back without error.
    let parsed = parse_config_str(&rendered).unwrap();
    assert_eq!(parsed.relay.as_deref(), Some("wss://re\"lay.example.com"));
}

// ---------------------------------------------------------------------------
// ConnectConfig helpers
// ---------------------------------------------------------------------------

#[test]
fn server_lookup_by_name() {
    let config = ConnectConfig {
        relay: None,
        servers: vec![ServerBookmark {
            name: "prod".to_string(),
            host: "prod.example.com".to_string(),
            port: 22,
        }],
        default_server: None,
        maps_dir: None,
        lock_timeout_minutes: None,
    };
    let s = config.server("prod").unwrap();
    assert_eq!(s.host, "prod.example.com");
    assert!(config.server("missing").is_none());
}

#[test]
fn validate_relay_url_accepts_ws_and_wss() {
    assert_eq!(
        validate_relay_url("ws://localhost:8080").unwrap(),
        Some("ws://localhost:8080".to_string())
    );
    assert_eq!(
        validate_relay_url("wss://relay.example.com").unwrap(),
        Some("wss://relay.example.com".to_string())
    );
}

#[test]
fn validate_relay_url_allows_blank_for_clear() {
    assert_eq!(validate_relay_url("   ").unwrap(), None);
}

#[test]
fn validate_relay_url_rejects_non_websocket_scheme() {
    let err = validate_relay_url("https://relay.example.com").unwrap_err();
    assert!(err.contains("ws:// or wss://"));
}

#[test]
fn validate_relay_url_rejects_malformed_string_with_invite_tail() {
    let err = validate_relay_url(
        "ws://localhost:80800wd6r5wps8qcquem0v3nxzargv4ez6emp0fjsjmr0vdskc6r0wd6q",
    )
    .unwrap_err();
    assert!(err.contains("valid ws:// or wss:// URL"));
}

// ---------------------------------------------------------------------------
// save_config_to / load_config_from
// ---------------------------------------------------------------------------

fn tmp_config_path(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("ec_connect_test_config_{name}.kdl"));
    p
}

#[test]
fn load_config_from_missing_file_returns_empty() {
    let path = tmp_config_path("missing_xyz_99999");
    let _ = std::fs::remove_file(&path);
    let config = load_config_from(&path).unwrap();
    assert!(config.relay.is_none());
    assert!(config.servers.is_empty());
}

#[test]
fn save_load_config_roundtrip() {
    let path = tmp_config_path("roundtrip");
    let _ = std::fs::remove_file(&path);

    let config = ConnectConfig {
        relay: Some("wss://r.test".to_string()),
        servers: vec![ServerBookmark {
            name: "test".to_string(),
            host: "localhost".to_string(),
            port: 2222,
        }],
        default_server: Some("test".to_string()),
        maps_dir: Some(PathBuf::from("/tmp/ec-maps")),
        lock_timeout_minutes: Some(9),
    };

    save_config_to(&config, &path).unwrap();
    let loaded = load_config_from(&path).unwrap();

    assert_eq!(loaded.relay.as_deref(), Some("wss://r.test"));
    assert_eq!(loaded.servers.len(), 1);
    assert_eq!(loaded.servers[0].port, 2222);
    assert_eq!(loaded.default_server.as_deref(), Some("test"));
    assert_eq!(loaded.maps_dir, Some(PathBuf::from("/tmp/ec-maps")));
    assert_eq!(loaded.lock_timeout_minutes, Some(9));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn seed_default_relay_sets_value_when_unset() {
    let path = tmp_config_path("seed_when_unset");
    let _ = std::fs::remove_file(&path);

    let changed = seed_default_relay_at("wss://relay.example.com", &path).unwrap();
    let loaded = load_config_from(&path).unwrap();

    assert!(changed);
    assert_eq!(loaded.relay.as_deref(), Some("wss://relay.example.com"));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn seed_default_relay_keeps_existing_valid_default() {
    let path = tmp_config_path("seed_keep_existing");
    let _ = std::fs::remove_file(&path);

    save_config_to(
        &ConnectConfig {
            relay: Some("wss://existing.example.com".to_string()),
            servers: vec![],
            default_server: None,
            maps_dir: None,
            lock_timeout_minutes: None,
        },
        &path,
    )
    .unwrap();

    let changed = seed_default_relay_at("wss://relay.example.com", &path).unwrap();
    let loaded = load_config_from(&path).unwrap();

    assert!(!changed);
    assert_eq!(loaded.relay.as_deref(), Some("wss://existing.example.com"));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn seed_default_relay_replaces_invalid_existing_default() {
    let path = tmp_config_path("seed_replace_invalid");
    let _ = std::fs::remove_file(&path);

    std::fs::write(&path, "relay \"ws://localhost:80800wd6r5wps8qc\"\n").unwrap();

    let changed = seed_default_relay_at("wss://relay.example.com", &path).unwrap();
    let loaded = load_config_from(&path).unwrap();

    assert!(changed);
    assert_eq!(loaded.relay.as_deref(), Some("wss://relay.example.com"));

    let _ = std::fs::remove_file(&path);
}
