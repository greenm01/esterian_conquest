mod common;

use nc_nostr::claim::parse_seat_claim_request;
use nc_nostr::invite_request::parse_invite_request;
use nc_nostr::state_sync::parse_state_request;
use nc_nostr::turn_commands::parse_turn_commands;
use nostr_sdk::{Keys, Kind, Tag};

fn make_test_keys() -> Keys {
    Keys::generate()
}

#[test]
fn test_parse_state_request_30507() {
    let keys = make_test_keys();
    let pubkey = keys.public_key();

    let tags = vec![
        Tag::parse(["d", "state-req-001"]).unwrap(),
        Tag::parse(["p", &pubkey.to_hex()]).unwrap(),
        Tag::parse(["game-id", "test-game"]).unwrap(),
    ];

    let event = nostr_sdk::EventBuilder::new(
        Kind::Custom(30507),
        r#"{"last_turn":5,"last_hash":"abc123"}"#,
    )
    .tags(tags)
    .sign_with_keys(&keys)
    .expect("should sign");

    let parsed = parse_state_request(&event).expect("should parse");

    assert_eq!(parsed.request_id, "state-req-001");
    assert_eq!(parsed.game_id, "test-game");
    assert_eq!(parsed.last_turn, Some(5));
    assert_eq!(parsed.last_hash, Some("abc123".to_string()));
}

#[test]
fn test_parse_invite_request_30513() {
    let keys = make_test_keys();

    let tags = vec![
        Tag::parse(["d", "invite-req-001"]).unwrap(),
        Tag::parse(["p", &keys.public_key().to_hex()]).unwrap(),
        Tag::parse(["game-id", "test-game"]).unwrap(),
    ];

    let event = nostr_sdk::EventBuilder::new(Kind::Custom(30513), "I'd like to join this game.")
        .tags(tags)
        .sign_with_keys(&keys)
        .expect("should sign");

    let parsed = parse_invite_request(&event).expect("should parse");

    assert_eq!(parsed.request_id, "invite-req-001");
    assert_eq!(parsed.game_id, "test-game");
    assert_eq!(parsed.message, "I'd like to join this game.");
}

#[test]
fn test_parse_turn_commands_30522() {
    let keys = make_test_keys();
    let pubkey = keys.public_key();

    let tags = vec![
        Tag::parse(["d", "turn-submit-001"]).unwrap(),
        Tag::parse(["p", &pubkey.to_hex()]).unwrap(),
        Tag::parse(["game-id", "test-game"]).unwrap(),
        Tag::parse(["turn", "7"]).unwrap(),
    ];

    let event = nostr_sdk::EventBuilder::new(Kind::Custom(30522), "fleet 1 { order speed=3 }")
        .tags(tags)
        .sign_with_keys(&keys)
        .expect("should sign");

    let parsed = parse_turn_commands(&event).expect("should parse");

    assert_eq!(parsed.submit_id, "turn-submit-001");
    assert_eq!(parsed.game_id, "test-game");
    assert_eq!(parsed.turn, 7);
    assert_eq!(parsed.commands, "fleet 1 { order speed=3 }");
}

#[test]
fn test_parse_seat_claim_30510() {
    let keys = make_test_keys();
    let pubkey = keys.public_key();

    let tags = vec![
        Tag::parse(["d", "nonce-12345"]).unwrap(),
        Tag::parse(["p", &pubkey.to_hex()]).unwrap(),
        Tag::parse(["game-id", "test-game"]).unwrap(),
    ];

    let event = nostr_sdk::EventBuilder::new(Kind::Custom(30510), "invite-code-abc123")
        .tags(tags)
        .sign_with_keys(&keys)
        .expect("should sign");

    let parsed = parse_seat_claim_request(&event).expect("should parse");

    assert_eq!(parsed.nonce, "nonce-12345");
    assert_eq!(parsed.invite_code, "invite-code-abc123");
    assert_eq!(parsed.game_id, Some("test-game".to_string()));
}

#[test]
fn test_state_request_missing_tags() {
    let keys = make_test_keys();

    let tags = vec![Tag::parse(["d", "bad-req"]).unwrap()];

    let event = nostr_sdk::EventBuilder::new(Kind::Custom(30507), "{}")
        .tags(tags)
        .sign_with_keys(&keys)
        .expect("should sign");

    let result = parse_state_request(&event);
    assert!(result.is_none(), "should fail without required tags");
}

#[test]
fn test_invite_request_without_required_tags() {
    let keys = make_test_keys();

    let tags = vec![Tag::parse(["d", "req-001"]).unwrap()];

    let event = nostr_sdk::EventBuilder::new(Kind::Custom(30513), "missing game-id")
        .tags(tags)
        .sign_with_keys(&keys)
        .expect("should sign");

    let result = parse_invite_request(&event);
    assert!(result.is_none(), "should fail without game-id tag");
}
