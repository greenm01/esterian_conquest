use nc_connect::connect::resolve::ResolvedTarget;
use nc_connect::connect::session_state::{
    SessionStateErrorPayload, SessionStatePayload, should_retry_game_metadata_via_handshake,
};

#[test]
fn parse_session_state_payload_json() {
    let json = r#"{"game_id":"friday-night","game_name":"Friday Night EC","seat":2,"player_name":"Empire of Sol"}"#;
    let payload: SessionStatePayload =
        serde_json::from_str(json).expect("session state should parse");
    assert_eq!(payload.game_id, "friday-night");
    assert_eq!(payload.game_name, "Friday Night EC");
    assert_eq!(payload.seat, 2);
    assert_eq!(payload.player_name, "Empire of Sol");
}

#[test]
fn parse_session_state_error_payload_json() {
    let json =
        r#"{"error":"unknown_player","message":"Your identity is not enrolled in that game."}"#;
    let payload: SessionStateErrorPayload =
        serde_json::from_str(json).expect("session state error should parse");
    assert_eq!(payload.error, "unknown_player");
    assert_eq!(
        payload.message,
        "Your identity is not enrolled in that game."
    );
}

#[test]
fn pending_invite_targets_do_not_retry_metadata_via_handshake() {
    let target = ResolvedTarget {
        server_host: "localhost".to_string(),
        server_port: 22,
        relay_url: "ws://localhost:8080".to_string(),
        invite_code: Some("velvet-mountain".to_string()),
        game_id: Some("stress-campaign".to_string()),
        gate_npub: None,
    };

    assert!(!should_retry_game_metadata_via_handshake(&target));
}

#[test]
fn reconnect_targets_still_retry_metadata_via_handshake() {
    let target = ResolvedTarget {
        server_host: "localhost".to_string(),
        server_port: 22,
        relay_url: "ws://localhost:8080".to_string(),
        invite_code: None,
        game_id: Some("stress-campaign".to_string()),
        gate_npub: None,
    };

    assert!(should_retry_game_metadata_via_handshake(&target));
}
