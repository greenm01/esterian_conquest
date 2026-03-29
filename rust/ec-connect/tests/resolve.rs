use ec_connect::config::{ConnectConfig, RelayEntry, RelayStatus, ServerBookmark};
use ec_connect::connect::resolve::{
    DEFAULT_SSH_PORT, ParsedInviteCode, derive_relay_url, parse_invite_code, resolve_invite,
    resolve_server,
};
use ec_nostr::invite::{InvitePayload, encode_invite};

// ── parse_invite_code ────────────────────────────────────────────────────────

#[test]
fn parse_bare_words() {
    let p = parse_invite_code("velvet-mountain").unwrap();
    assert_eq!(
        p,
        ParsedInviteCode {
            words: "velvet-mountain".into(),
            server: None,
            relay_url: None,
            game_id: None,
            gate_npub: None,
        }
    );
}

#[test]
fn parse_with_host() {
    let p = parse_invite_code("velvet-mountain@play.example.com").unwrap();
    assert_eq!(
        p,
        ParsedInviteCode {
            words: "velvet-mountain".into(),
            server: Some(("play.example.com".into(), DEFAULT_SSH_PORT)),
            relay_url: None,
            game_id: None,
            gate_npub: None,
        }
    );
}

#[test]
fn parse_with_host_and_port() {
    let p = parse_invite_code("velvet-mountain@play.example.com:2222").unwrap();
    assert_eq!(
        p,
        ParsedInviteCode {
            words: "velvet-mountain".into(),
            server: Some(("play.example.com".into(), 2222)),
            relay_url: None,
            game_id: None,
            gate_npub: None,
        }
    );
}

#[test]
fn parse_with_localhost_port() {
    let p = parse_invite_code("red-fox@localhost:2222").unwrap();
    assert_eq!(
        p,
        ParsedInviteCode {
            words: "red-fox".into(),
            server: Some(("localhost".into(), 2222)),
            relay_url: None,
            game_id: None,
            gate_npub: None,
        }
    );
}

#[test]
fn parse_trims_whitespace() {
    let p = parse_invite_code("  velvet-mountain  ").unwrap();
    assert_eq!(p.words, "velvet-mountain");
    assert_eq!(p.server, None);
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
    let p = parse_invite_code("blue-sky@[::1]:2222").unwrap();
    assert_eq!(
        p,
        ParsedInviteCode {
            words: "blue-sky".into(),
            server: Some(("[::1]".into(), 2222)),
            relay_url: None,
            game_id: None,
            gate_npub: None,
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
            server: Some(("[::1]".into(), DEFAULT_SSH_PORT)),
            relay_url: None,
            game_id: None,
            gate_npub: None,
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

fn config_with_default(host: &str, port: u16, relay: Option<&str>) -> ConnectConfig {
    let relay = relay.map(|s| s.to_string());
    ConnectConfig {
        relays: relay
            .as_ref()
            .map(|url| {
                vec![RelayEntry {
                    url: url.clone(),
                    is_default: true,
                    status: RelayStatus::Unknown,
                    last_error: None,
                    last_checked: None,
                }]
            })
            .unwrap_or_default(),
        relay,
        servers: vec![ServerBookmark {
            name: "default".into(),
            host: host.to_string(),
            port,
        }],
        default_server: Some("default".into()),
        maps_dir: None,
        lock_timeout_minutes: None,
    }
}

#[test]
fn resolve_invite_uses_inline_host() {
    let config = ConnectConfig::empty();
    let t = resolve_invite("red-fox@play.example.com:2222", &config).unwrap();
    assert_eq!(t.server_host, "play.example.com");
    assert_eq!(t.server_port, 2222);
    assert_eq!(t.invite_code, Some("red-fox".into()));
    assert_eq!(t.game_id, None);
    assert_eq!(t.relay_url, "wss://play.example.com:7777");
}

#[test]
fn resolve_invite_uses_default_bookmark_when_no_inline_host() {
    let config = config_with_default("play.example.com", 22, None);
    let t = resolve_invite("red-fox", &config).unwrap();
    assert_eq!(t.server_host, "play.example.com");
    assert_eq!(t.server_port, 22);
    assert_eq!(t.invite_code, Some("red-fox".into()));
}

#[test]
fn resolve_invite_prefers_inline_host_over_default() {
    let config = config_with_default("other.example.com", 22, None);
    let t = resolve_invite("red-fox@play.example.com", &config).unwrap();
    assert_eq!(t.server_host, "play.example.com");
}

#[test]
fn resolve_invite_uses_config_relay_over_derived() {
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
    };
    let t = resolve_invite("red-fox@play.example.com", &config).unwrap();
    assert_eq!(t.relay_url, "wss://relay.custom.com");
}

#[test]
fn resolve_invite_no_server_is_err() {
    let config = ConnectConfig::empty();
    assert!(resolve_invite("red-fox", &config).is_err());
}

#[test]
fn resolve_invite_missing_default_bookmark_is_err() {
    let config = ConnectConfig {
        relays: vec![],
        relay: None,
        servers: vec![],
        default_server: Some("ghost".into()),
        maps_dir: None,
        lock_timeout_minutes: None,
    };
    assert!(resolve_invite("red-fox", &config).is_err());
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
    let p = parse_invite_code("VELVET-MOUNTAIN").unwrap();
    assert_eq!(p.words, "velvet-mountain");
}

#[test]
fn parse_mixed_case_words_normalized() {
    let p = parse_invite_code("Velvet-Mountain").unwrap();
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
    let p = parse_invite_code("RED-FOX@play.example.com").unwrap();
    assert_eq!(p.words, "red-fox");
    assert_eq!(
        p.server,
        Some(("play.example.com".to_string(), DEFAULT_SSH_PORT))
    );
}

// ── bech32 invite parsing ─────────────────────────────────────────────────────

#[test]
fn parse_bech32_invite_simple() {
    let payload = InvitePayload {
        relay_url: "wss://play.example.com:7777".to_string(),
        words: "velvet-mountain".to_string(),
        ssh_host: "play.example.com".to_string(),
        ssh_port: 22,
        game_id: None,
        gate_npub: None,
    };
    let encoded = encode_invite(&payload).unwrap();
    let p = parse_invite_code(&encoded).unwrap();
    assert_eq!(p.words, "velvet-mountain");
    assert_eq!(p.relay_url, Some("wss://play.example.com:7777".to_string()));
    assert_eq!(p.game_id, None);
    assert_eq!(p.gate_npub, None);
    // ssh_host/ssh_port from bech32 are surfaced via the server field.
    assert_eq!(p.server, Some(("play.example.com".into(), 22)));
}

#[test]
fn parse_bech32_invite_with_game_id() {
    let payload = InvitePayload {
        relay_url: "wss://play.example.com:7777".to_string(),
        words: "copper-sunrise".to_string(),
        ssh_host: "play.example.com".to_string(),
        ssh_port: 22,
        game_id: Some("friday-night".to_string()),
        gate_npub: None,
    };
    let encoded = encode_invite(&payload).unwrap();
    let p = parse_invite_code(&encoded).unwrap();
    assert_eq!(p.words, "copper-sunrise");
    assert_eq!(p.game_id, Some("friday-night".to_string()));
}

#[test]
fn parse_bech32_invite_with_gate_npub() {
    let npub_bytes = [0xabu8; 32];
    let payload = InvitePayload {
        relay_url: "wss://play.example.com:7777".to_string(),
        words: "jade-horizon".to_string(),
        ssh_host: "play.example.com".to_string(),
        ssh_port: 22,
        game_id: None,
        gate_npub: Some(npub_bytes),
    };
    let encoded = encode_invite(&payload).unwrap();
    let p = parse_invite_code(&encoded).unwrap();
    assert!(p.gate_npub.is_some());
    // Gate npub is hex-encoded in ParsedInviteCode.
    let expected_hex: String = npub_bytes.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(p.gate_npub.unwrap(), expected_hex);
}

#[test]
fn parse_bech32_corrupted_checksum_is_err() {
    let payload = InvitePayload {
        relay_url: "wss://play.example.com:7777".to_string(),
        words: "velvet-mountain".to_string(),
        ssh_host: "play.example.com".to_string(),
        ssh_port: 22,
        game_id: None,
        gate_npub: None,
    };
    let mut encoded = encode_invite(&payload).unwrap();
    let last = encoded.pop().unwrap();
    encoded.push(if last == 'a' { 'z' } else { 'a' });
    assert!(parse_invite_code(&encoded).is_err());
}

#[test]
fn parse_plain_and_bech32_both_work() {
    assert!(parse_invite_code("velvet-mountain").is_ok());
    let payload = InvitePayload {
        relay_url: "wss://play.example.com:7777".to_string(),
        words: "velvet-mountain".to_string(),
        ssh_host: "play.example.com".to_string(),
        ssh_port: 22,
        game_id: None,
        gate_npub: None,
    };
    let encoded = encode_invite(&payload).unwrap();
    assert!(parse_invite_code(&encoded).is_ok());
}

// ── bech32 invite resolution ──────────────────────────────────────────────────

#[test]
fn resolve_bech32_invite_uses_embedded_relay() {
    let payload = InvitePayload {
        relay_url: "wss://relay.nostr.example.com".to_string(),
        words: "velvet-mountain".to_string(),
        ssh_host: "play.example.com".to_string(),
        ssh_port: 2222,
        game_id: None,
        gate_npub: None,
    };
    let encoded = encode_invite(&payload).unwrap();
    let config = config_with_default("other.example.com", 22, None);
    let t = resolve_invite(&encoded, &config).unwrap();
    assert_eq!(t.relay_url, "wss://relay.nostr.example.com");
    assert_eq!(t.server_host, "play.example.com");
    assert_eq!(t.server_port, 2222);
    assert_eq!(t.invite_code, Some("velvet-mountain".to_string()));
}

#[test]
fn resolve_bech32_invite_propagates_game_id() {
    let payload = InvitePayload {
        relay_url: "wss://play.example.com:7777".to_string(),
        words: "copper-sunrise".to_string(),
        ssh_host: "play.example.com".to_string(),
        ssh_port: 22,
        game_id: Some("friday-night".to_string()),
        gate_npub: None,
    };
    let encoded = encode_invite(&payload).unwrap();
    let config = config_with_default("play.example.com", 22, None);
    let t = resolve_invite(&encoded, &config).unwrap();
    assert_eq!(t.game_id, Some("friday-night".to_string()));
}
