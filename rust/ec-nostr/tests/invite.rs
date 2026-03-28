use ec_nostr::invite::{InvitePayload, decode_invite, encode_invite, is_bech32_invite};

fn simple_payload() -> InvitePayload {
    InvitePayload {
        relay_url: "wss://play.example.com:7777".to_string(),
        words: "velvet-mountain".to_string(),
        ssh_host: "play.example.com".to_string(),
        ssh_port: 22,
        game_id: None,
        gate_npub: None,
    }
}

// ── round-trip ───────────────────────────────────────────────────────────────

#[test]
fn round_trip_simple() {
    let payload = simple_payload();
    let encoded = encode_invite(&payload).unwrap();
    let decoded = decode_invite(&encoded).unwrap();
    assert_eq!(decoded, payload);
}

#[test]
fn round_trip_with_game_id() {
    let payload = InvitePayload {
        relay_url: "wss://relay.nostr.com".to_string(),
        words: "copper-sunrise".to_string(),
        ssh_host: "play.example.com".to_string(),
        ssh_port: 2222,
        game_id: Some("friday-night".to_string()),
        gate_npub: None,
    };
    let decoded = decode_invite(&encode_invite(&payload).unwrap()).unwrap();
    assert_eq!(decoded, payload);
}

#[test]
fn round_trip_with_gate_npub() {
    let npub = [42u8; 32];
    let payload = InvitePayload {
        relay_url: "wss://play.example.com:7777".to_string(),
        words: "amber-cascade".to_string(),
        ssh_host: "play.example.com".to_string(),
        ssh_port: 22,
        game_id: None,
        gate_npub: Some(npub),
    };
    let decoded = decode_invite(&encode_invite(&payload).unwrap()).unwrap();
    assert_eq!(decoded, payload);
}

#[test]
fn round_trip_all_fields() {
    let npub = [0xab; 32];
    let payload = InvitePayload {
        relay_url: "wss://relay.nostr.com:7777".to_string(),
        words: "jade-horizon".to_string(),
        ssh_host: "play.example.com".to_string(),
        ssh_port: 22,
        game_id: Some("saturday-game".to_string()),
        gate_npub: Some(npub),
    };
    let decoded = decode_invite(&encode_invite(&payload).unwrap()).unwrap();
    assert_eq!(decoded, payload);
}

#[test]
fn round_trip_preserves_ssh_host_and_port() {
    let payload = InvitePayload {
        relay_url: "wss://relay.example.com:7777".to_string(),
        words: "velvet-mountain".to_string(),
        ssh_host: "game.example.com".to_string(),
        ssh_port: 2222,
        game_id: None,
        gate_npub: None,
    };
    let decoded = decode_invite(&encode_invite(&payload).unwrap()).unwrap();
    assert_eq!(decoded.ssh_host, "game.example.com");
    assert_eq!(decoded.ssh_port, 2222);
}

// ── format ───────────────────────────────────────────────────────────────────

#[test]
fn encoded_starts_with_ecinv1() {
    let encoded = encode_invite(&simple_payload()).unwrap();
    assert!(encoded.starts_with("ecinv1"), "got: {encoded}");
}

#[test]
fn is_bech32_invite_recognizes_prefix() {
    let encoded = encode_invite(&simple_payload()).unwrap();
    assert!(is_bech32_invite(&encoded));
    assert!(!is_bech32_invite("velvet-mountain"));
    assert!(!is_bech32_invite("velvet-mountain@play.example.com"));
    assert!(!is_bech32_invite("npub1abc"));
}

// ── error cases ──────────────────────────────────────────────────────────────

#[test]
fn corrupted_checksum_is_err() {
    let mut encoded = encode_invite(&simple_payload()).unwrap();
    let last = encoded.pop().unwrap();
    let replacement = if last == 'a' { 'z' } else { 'a' };
    encoded.push(replacement);
    assert!(decode_invite(&encoded).is_err());
}

#[test]
fn wrong_hrp_is_err() {
    assert!(decode_invite("npub1deadbeef").is_err());
}

#[test]
fn empty_string_is_err() {
    assert!(decode_invite("").is_err());
}

// ── localhost relay ───────────────────────────────────────────────────────────

#[test]
fn round_trip_localhost() {
    let payload = InvitePayload {
        relay_url: "ws://localhost:8080".to_string(),
        words: "velvet-mountain".to_string(),
        ssh_host: "localhost".to_string(),
        ssh_port: 22,
        game_id: None,
        gate_npub: None,
    };
    let decoded = decode_invite(&encode_invite(&payload).unwrap()).unwrap();
    assert_eq!(decoded.relay_url, "ws://localhost:8080");
    assert_eq!(decoded.ssh_host, "localhost");
    assert_eq!(decoded.ssh_port, 22);
}
