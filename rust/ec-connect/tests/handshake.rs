use ec_connect::connect::handshake::{
    GameEntry, SessionErrorPayload, SessionReadyPayload, parse_session_error, parse_session_ready,
    random_nonce_hex,
};

// ── parse_session_ready ───────────────────────────────────────────────────────

#[test]
fn parse_ready_full_payload() {
    let json = r#"{"game_id":"friday-night","ssh_host":"play.example.com","ssh_port":22,"ssh_user":"ecgame","host_fingerprint":"SHA256:abc","game_name":"Friday Night EC","seat":2,"player_name":"Empire of Sol"}"#;
    let p = parse_session_ready(json).unwrap();
    assert_eq!(p.game_id, "friday-night");
    assert_eq!(p.ssh_host, "play.example.com");
    assert_eq!(p.ssh_port, 22);
    assert_eq!(p.ssh_user, "ecgame");
    assert_eq!(p.host_fingerprint, "SHA256:abc");
    assert_eq!(p.game_name, "Friday Night EC");
    assert_eq!(p.seat, 2);
    assert_eq!(p.player_name, "Empire of Sol");
}

#[test]
fn parse_ready_missing_fingerprint_defaults_to_empty() {
    let json = r#"{"game_id":"g","ssh_host":"h","ssh_port":22,"game_name":"N","seat":1,"player_name":"P"}"#;
    let p = parse_session_ready(json).unwrap();
    assert_eq!(p.ssh_user, "");
    assert_eq!(p.host_fingerprint, "");
}

#[test]
fn parse_ready_missing_player_name_defaults_to_empty() {
    let json = r#"{"game_id":"g","ssh_host":"h","ssh_port":2222,"game_name":"N","seat":3}"#;
    let p = parse_session_ready(json).unwrap();
    assert_eq!(p.player_name, "");
    assert_eq!(p.ssh_port, 2222);
    assert_eq!(p.seat, 3);
}

#[test]
fn parse_ready_missing_game_id_is_err() {
    let json = r#"{"ssh_host":"h","ssh_port":22,"game_name":"N","seat":1}"#;
    assert!(parse_session_ready(json).is_err());
}

#[test]
fn parse_ready_missing_ssh_port_is_err() {
    let json = r#"{"game_id":"g","ssh_host":"h","game_name":"N","seat":1}"#;
    assert!(parse_session_ready(json).is_err());
}

#[test]
fn parse_ready_escaped_strings() {
    let json =
        r#"{"game_id":"g","ssh_host":"h","ssh_port":22,"game_name":"Night \"EC\"","seat":1}"#;
    let p = parse_session_ready(json).unwrap();
    assert_eq!(p.game_name, "Night \"EC\"");
}

// ── parse_session_error ───────────────────────────────────────────────────────

#[test]
fn parse_error_simple() {
    let json =
        r#"{"error":"invalid_code","message":"The invite code 'velvet-mountain' is not valid."}"#;
    let p = parse_session_error(json).unwrap();
    assert_eq!(p.error, "invalid_code");
    assert_eq!(p.message, "The invite code 'velvet-mountain' is not valid.");
    assert!(p.games.is_empty());
}

#[test]
fn parse_error_multiple_games() {
    let json = r#"{"error":"multiple_games","message":"Multiple games.","games":[{"game_id":"friday-night","name":"Friday Night EC","seat":2},{"game_id":"saturday-showdown","name":"Saturday Showdown","seat":5}]}"#;
    let p = parse_session_error(json).unwrap();
    assert_eq!(p.error, "multiple_games");
    assert_eq!(p.games.len(), 2);
    assert_eq!(
        p.games[0],
        GameEntry {
            game_id: "friday-night".into(),
            name: "Friday Night EC".into(),
            seat: 2,
        }
    );
    assert_eq!(
        p.games[1],
        GameEntry {
            game_id: "saturday-showdown".into(),
            name: "Saturday Showdown".into(),
            seat: 5,
        }
    );
}

#[test]
fn parse_error_non_multiple_games_has_empty_game_list() {
    let json = r#"{"error":"rate_limited","message":"Too many requests."}"#;
    let p = parse_session_error(json).unwrap();
    assert_eq!(p.error, "rate_limited");
    assert!(p.games.is_empty());
}

#[test]
fn parse_error_missing_error_field_is_err() {
    let json = r#"{"message":"something"}"#;
    assert!(parse_session_error(json).is_err());
}

#[test]
fn parse_error_missing_message_field_is_err() {
    let json = r#"{"error":"invalid_code"}"#;
    assert!(parse_session_error(json).is_err());
}

#[test]
fn parse_error_escaped_message() {
    let json = r#"{"error":"unknown","message":"line1\nline2"}"#;
    let p = parse_session_error(json).unwrap();
    assert_eq!(p.message, "line1\nline2");
}

// ── round-trip: ec-gate format → parse_session_ready ─────────────────────────

/// The gate serializes payloads without host_fingerprint or player_name (older
/// gate), ensure we still parse cleanly.
#[test]
fn parse_ready_gate_compact_format() {
    // This matches ec-gate's SessionReadyPayload::to_json() output.
    let json = r#"{"game_id":"friday-night","ssh_host":"play.example.com","ssh_port":22,"ssh_user":"ecgame","game_name":"Friday Night EC","seat":2}"#;
    let p = parse_session_ready(json).unwrap();
    assert_eq!(p.game_id, "friday-night");
    assert_eq!(p.seat, 2);
    assert_eq!(p.ssh_user, "ecgame");
    assert_eq!(p.host_fingerprint, "");
    assert_eq!(p.player_name, "");
}

// ── random_nonce_hex ──────────────────────────────────────────────────────────

#[test]
fn nonce_is_64_hex_chars() {
    let n = random_nonce_hex();
    assert_eq!(n.len(), 64);
    assert!(n.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn two_nonces_differ() {
    let n1 = random_nonce_hex();
    let n2 = random_nonce_hex();
    assert_ne!(n1, n2);
}

// ── SessionReadyPayload / SessionErrorPayload PartialEq ───────────────────────

#[test]
fn session_ready_payload_equality() {
    let a = SessionReadyPayload {
        game_id: "g".into(),
        ssh_host: "h".into(),
        ssh_port: 22,
        ssh_user: "ecgame".into(),
        host_fingerprint: "fp".into(),
        game_name: "N".into(),
        seat: 1,
        player_name: "P".into(),
    };
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn session_error_payload_equality() {
    let a = SessionErrorPayload {
        error: "e".into(),
        message: "m".into(),
        games: vec![],
    };
    let b = a.clone();
    assert_eq!(a, b);
}
