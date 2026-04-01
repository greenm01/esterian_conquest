use nc_gate::serve::state::{
    ParseError, SessionStateErrorPayload, SessionStatePayload, parse_session_state_request,
};
use nostr_sdk::{EventBuilder, Keys, Kind, Tag};

fn signed_state_request(game_id: &str) -> nostr_sdk::Event {
    let keys = Keys::generate();
    EventBuilder::new(Kind::Custom(30507), "")
        .tags(vec![
            Tag::parse(["d", "test-state-nonce"]).unwrap(),
            Tag::parse(["game-id", game_id]).unwrap(),
        ])
        .sign_with_keys(&keys)
        .unwrap()
}

#[test]
fn parse_session_state_request_basic() {
    let event = signed_state_request("friday-night");
    let req = parse_session_state_request(&event).expect("state request should parse");
    assert_eq!(req.nonce, "test-state-nonce");
    assert_eq!(req.game_id, "friday-night");
    assert_eq!(req.player_pubkey, event.pubkey.to_hex());
}

#[test]
fn parse_session_state_request_requires_game_id() {
    let keys = Keys::generate();
    let event = EventBuilder::new(Kind::Custom(30507), "")
        .tags(vec![Tag::parse(["d", "test-state-nonce"]).unwrap()])
        .sign_with_keys(&keys)
        .unwrap();
    assert_eq!(
        parse_session_state_request(&event).unwrap_err(),
        ParseError::MissingGameId
    );
}

#[test]
fn session_state_payload_json_round_trip() {
    let payload = SessionStatePayload {
        game_id: "friday-night".to_string(),
        game_name: "Friday Night EC".to_string(),
        seat: 2,
        player_name: "Empire of Sol".to_string(),
    };
    let json = serde_json::to_string(&payload).expect("serialize session state");
    let parsed: SessionStatePayload = serde_json::from_str(&json).expect("parse session state");
    assert_eq!(parsed, payload);
}

#[test]
fn session_state_error_payload_json_round_trip() {
    let payload = SessionStateErrorPayload {
        error: "unknown_player".to_string(),
        message: "Your identity is not enrolled in that game.".to_string(),
    };
    let json = serde_json::to_string(&payload).expect("serialize state error");
    let parsed: SessionStateErrorPayload =
        serde_json::from_str(&json).expect("parse session state error");
    assert_eq!(parsed, payload);
}
