use nc_connect::config::{ConnectConfig, RelayEntry, RelayStatus, ServerBookmark};
use nc_connect::connect::resolve::{
    DEFAULT_SSH_PORT, ParsedInviteCode, derive_relay_url, parse_invite_code, resolve_invite,
    resolve_server,
};

// ── parse_invite_code ────────────────────────────────────────────────────────

#[test]
fn parse_canonical_invite() {
    let p = parse_invite_code("velvet-mountain@relay.example.com").unwrap();
    assert_eq!(
        p,
        ParsedInviteCode {
            words: "velvet-mountain".into(),
            relay_host: "relay.example.com".into(),
            relay_port: None,
        }
    );
}

#[test]
fn parse_with_host_and_port() {
    let p = parse_invite_code("velvet-mountain@relay.example.com:7447").unwrap();
    assert_eq!(
        p,
        ParsedInviteCode {
            words: "velvet-mountain".into(),
            relay_host: "relay.example.com".into(),
            relay_port: Some(7447),
        }
    );
}

#[test]
fn parse_with_localhost_port() {
    let p = parse_invite_code("red-fox@localhost:7777").unwrap();
    assert_eq!(
        p,
        ParsedInviteCode {
            words: "red-fox".into(),
            relay_host: "localhost".into(),
            relay_port: Some(7777),
        }
    );
}

#[test]
fn parse_trims_whitespace() {
    let p = parse_invite_code("  velvet-mountain@relay.example.com  ").unwrap();
    assert_eq!(p.words, "velvet-mountain");
    assert_eq!(p.relay_host, "relay.example.com");
}

#[test]
fn parse_empty_is_err() {
    assert!(parse_invite_code("").is_err());
    assert!(parse_invite_code("   ").is_err());
}

#[test]
fn parse_empty_words_at_sign_is_err() {
    assert!(parse_invite_code("@play.example.com").is_err());
}

#[test]
fn parse_invalid_port_is_err() {
    assert!(parse_invite_code("red-fox@host:99999").is_err());
    assert!(parse_invite_code("red-fox@host:abc").is_err());
}

#[test]
fn parse_ipv6_with_port() {
    let p = parse_invite_code("blue-sky@[::1]:7777").unwrap();
    assert_eq!(
        p,
        ParsedInviteCode {
            words: "blue-sky".into(),
            relay_host: "[::1]".into(),
            relay_port: Some(7777),
        }
    );
}

#[test]
fn parse_ipv6_without_port() {
    let p = parse_invite_code("blue-sky@[::1]").unwrap();
    assert_eq!(
        p,
        ParsedInviteCode {
            words: "blue-sky".into(),
            relay_host: "[::1]".into(),
            relay_port: None,
        }
    );
}

// ── derive_relay_url ─────────────────────────────────────────────────────────

#[test]
fn relay_url_public_host_uses_wss() {
    assert_eq!(
        derive_relay_url("play.example.com"),
        "wss://play.example.com:7777"
    );
}

#[test]
fn relay_url_localhost_uses_ws() {
    assert_eq!(derive_relay_url("localhost"), "ws://localhost:7777");
}

#[test]
fn relay_url_127_uses_ws() {
    assert_eq!(derive_relay_url("127.0.0.1"), "ws://127.0.0.1:7777");
}

#[test]
fn relay_url_10_dot_uses_ws() {
    assert_eq!(derive_relay_url("10.0.0.1"), "ws://10.0.0.1:7777");
}

#[test]
fn relay_url_192_168_uses_ws() {
    assert_eq!(derive_relay_url("192.168.1.5"), "ws://192.168.1.5:7777");
}

#[test]
fn relay_url_172_16_uses_ws() {
    assert_eq!(derive_relay_url("172.16.0.1"), "ws://172.16.0.1:7777");
}

#[test]
fn relay_url_172_31_uses_ws() {
    assert_eq!(
        derive_relay_url("172.31.255.255"),
        "ws://172.31.255.255:7777"
    );
}

#[test]
fn relay_url_172_32_uses_wss() {
    // 172.32.x.x is outside 172.16/12, so public.
    assert_eq!(derive_relay_url("172.32.0.1"), "wss://172.32.0.1:7777");
}

#[test]
fn relay_url_ipv6_loopback_uses_ws() {
    assert_eq!(derive_relay_url("[::1]"), "ws://[::1]:7777");
}

// ── resolve_invite ───────────────────────────────────────────────────────────

#[test]
fn resolve_invite_uses_inline_host() {
    let config = ConnectConfig::empty();
    let t = resolve_invite("red-fox@relay.example.com:7447", &config).unwrap();
    assert_eq!(t.server_host, "");
    assert_eq!(t.server_port, DEFAULT_SSH_PORT);
    assert_eq!(t.invite_code, Some("red-fox".into()));
    assert_eq!(t.game_id, None);
    assert_eq!(t.relay_url, "wss://relay.example.com:7447");
}

#[test]
fn resolve_invite_ignores_default_config_relay() {
    let config = ConnectConfig {
        relays: vec![RelayEntry {
            url: "wss://relay.custom.com".into(),
            is_default: true,
            status: RelayStatus::Unknown,
            last_error: None,
            last_checked: None,
        }],
        relay: Some("wss://relay.custom.com".into()),
        servers: vec![],
        default_server: None,
        maps_dir: None,
        lock_timeout_minutes: None,
        log_file: None,
        log_level: None,
    };
    let t = resolve_invite("red-fox@relay.example.com", &config).unwrap();
    assert_eq!(t.relay_url, "wss://relay.example.com");
}

#[test]
fn resolve_invite_requires_relay_suffix() {
    let config = ConnectConfig::empty();
    assert!(resolve_invite("red-fox", &config).is_err());
}

#[test]
fn resolve_invite_localhost_uses_ws() {
    let config = ConnectConfig::empty();
    let t = resolve_invite("red-fox@localhost:7777", &config).unwrap();
    assert_eq!(t.relay_url, "ws://localhost:7777");
}

// ── resolve_server ───────────────────────────────────────────────────────────

fn config_with_bookmark(name: &str, host: &str, port: u16) -> ConnectConfig {
    ConnectConfig {
        relays: vec![],
        relay: None,
        servers: vec![ServerBookmark {
            name: name.to_string(),
            host: host.to_string(),
            port,
        }],
        default_server: None,
        maps_dir: None,
        lock_timeout_minutes: None,
        log_file: None,
        log_level: None,
    }
}

#[test]
fn resolve_server_by_bookmark_name() {
    let config = config_with_bookmark("friday", "play.example.com", 22);
    let t = resolve_server("friday", &config).unwrap();
    assert_eq!(t.server_host, "play.example.com");
    assert_eq!(t.server_port, 22);
    assert_eq!(t.invite_code, None);
    assert_eq!(t.game_id, None);
}

#[test]
fn resolve_server_by_host_only() {
    let config = ConnectConfig::empty();
    let t = resolve_server("play.example.com", &config).unwrap();
    assert_eq!(t.server_host, "play.example.com");
    assert_eq!(t.server_port, DEFAULT_SSH_PORT);
}

#[test]
fn resolve_server_by_host_and_port() {
    let config = ConnectConfig::empty();
    let t = resolve_server("play.example.com:2222", &config).unwrap();
    assert_eq!(t.server_host, "play.example.com");
    assert_eq!(t.server_port, 2222);
}

#[test]
fn resolve_server_prefers_bookmark_over_literal() {
    // "friday" is also a valid hostname literal but we resolve bookmark first.
    let config = config_with_bookmark("friday", "play.example.com", 2222);
    let t = resolve_server("friday", &config).unwrap();
    assert_eq!(t.server_host, "play.example.com");
    assert_eq!(t.server_port, 2222);
}

#[test]
fn resolve_server_relay_from_config() {
    let config = ConnectConfig {
        relays: vec![RelayEntry {
            url: "wss://relay.custom.com".into(),
            is_default: true,
            status: RelayStatus::Unknown,
            last_error: None,
            last_checked: None,
        }],
        relay: Some("wss://relay.custom.com".into()),
        servers: vec![],
        default_server: None,
        maps_dir: None,
        lock_timeout_minutes: None,
        log_file: None,
        log_level: None,
    };
    let t = resolve_server("play.example.com", &config).unwrap();
    assert_eq!(t.relay_url, "wss://relay.custom.com");
}

#[test]
fn resolve_server_empty_is_err() {
    let config = ConnectConfig::empty();
    assert!(resolve_server("", &config).is_err());
    assert!(resolve_server("   ", &config).is_err());
}

#[test]
fn resolve_server_derived_relay_for_localhost() {
    let config = ConnectConfig::empty();
    let t = resolve_server("localhost:2222", &config).unwrap();
    assert_eq!(t.relay_url, "ws://localhost:7777");
}

// ── parse_invite_code strict format validation ────────────────────────────────

#[test]
fn parse_uppercase_words_normalized_to_lowercase() {
    let p = parse_invite_code("VELVET-MOUNTAIN@relay.example.com").unwrap();
    assert_eq!(p.words, "velvet-mountain");
}

#[test]
fn parse_mixed_case_words_normalized() {
    let p = parse_invite_code("Velvet-Mountain@relay.example.com").unwrap();
    assert_eq!(p.words, "velvet-mountain");
}

#[test]
fn parse_no_hyphen_is_err() {
    assert!(parse_invite_code("velvetmountain").is_err());
}

#[test]
fn parse_three_words_is_err() {
    assert!(parse_invite_code("velvet-mountain-peak").is_err());
}

#[test]
fn parse_hyphen_only_is_err() {
    assert!(parse_invite_code("-").is_err());
}

#[test]
fn parse_leading_hyphen_is_err() {
    assert!(parse_invite_code("-mountain").is_err());
}

#[test]
fn parse_trailing_hyphen_is_err() {
    assert!(parse_invite_code("velvet-").is_err());
}

#[test]
fn parse_digits_in_words_is_err() {
    assert!(parse_invite_code("velvet1-mountain").is_err());
    assert!(parse_invite_code("velvet-mountain2").is_err());
}

#[test]
fn parse_uppercase_with_host_normalized() {
    let p = parse_invite_code("RED-FOX@relay.example.com").unwrap();
    assert_eq!(p.words, "red-fox");
    assert_eq!(p.relay_host, "relay.example.com".to_string());
}
