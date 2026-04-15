mod common;

use nc_nostr::claim::{SeatClaimRequestPayload, parse_seat_claim_request};
use nc_nostr::first_join::{FirstJoinSetupRequestPayload, parse_first_join_setup_request};
use nc_nostr::handle_check::{HandleCheckRequestPayload, parse_handle_check_request};
use nc_nostr::invite_request::{InviteRequestPayload, parse_invite_request};
use nc_nostr::private_payload::encrypt_private_json;
use nc_nostr::state_sync::{StateRequestPayload, parse_state_request};
use nc_nostr::turn_commands::{TurnCommandsPayload, parse_turn_commands};
use nostr_sdk::{EventBuilder, Keys, Kind, Tag};

fn make_test_keys() -> Keys {
    Keys::generate()
}

fn build_private_event<T: serde::Serialize>(
    player_keys: &Keys,
    host_keys: &Keys,
    kind: u16,
    payload: &T,
    tags: Vec<Tag>,
) -> nostr_sdk::Event {
    let encrypted = encrypt_private_json(player_keys, &host_keys.public_key(), payload)
        .expect("encrypt payload");
    EventBuilder::new(Kind::Custom(kind), encrypted)
        .tags(tags)
        .sign_with_keys(player_keys)
        .expect("should sign")
}

#[test]
fn test_parse_state_request_30507() {
    let player_keys = make_test_keys();
    let host_keys = make_test_keys();

    let tags = vec![
        Tag::parse(["d", "state-req-001"]).unwrap(),
        Tag::parse(["p", &host_keys.public_key().to_hex()]).unwrap(),
        Tag::parse(["game-id", "test-game"]).unwrap(),
    ];

    let event = build_private_event(
        &player_keys,
        &host_keys,
        30507,
        &StateRequestPayload {
            last_turn: Some(5),
            last_hash: Some("abc123".to_string()),
            handle: Some("pilot".to_string()),
        },
        tags,
    );

    let parsed = parse_state_request(host_keys.secret_key(), &event).expect("should parse");

    assert_eq!(parsed.request_id, "state-req-001");
    assert_eq!(parsed.game_id, "test-game");
    assert_eq!(parsed.last_turn, Some(5));
    assert_eq!(parsed.last_hash, Some("abc123".to_string()));
    assert_eq!(parsed.handle.as_deref(), Some("pilot"));
}

#[test]
fn test_parse_invite_request_30513() {
    let player_keys = make_test_keys();
    let host_keys = make_test_keys();

    let tags = vec![
        Tag::parse(["d", "invite-req-001"]).unwrap(),
        Tag::parse(["p", &host_keys.public_key().to_hex()]).unwrap(),
        Tag::parse(["game-id", "test-game"]).unwrap(),
    ];

    let event = build_private_event(
        &player_keys,
        &host_keys,
        30513,
        &InviteRequestPayload {
            message: "I'd like to join this game.".to_string(),
            handle: Some("traveler".to_string()),
        },
        tags,
    );
    let tag_values: Vec<Vec<String>> = event.tags.iter().map(|tag| tag.clone().to_vec()).collect();
    assert!(
        !tag_values
            .iter()
            .any(|tag| tag.first().map(String::as_str) == Some("handle"))
    );
    assert!(!event.content.contains("I'd like to join this game."));

    let parsed = parse_invite_request(host_keys.secret_key(), &event).expect("should parse");

    assert_eq!(parsed.request_id, "invite-req-001");
    assert_eq!(parsed.game_id, "test-game");
    assert_eq!(parsed.message, "I'd like to join this game.");
    assert_eq!(parsed.handle.as_deref(), Some("traveler"));
}

#[test]
fn test_parse_turn_commands_30522() {
    let player_keys = make_test_keys();
    let host_keys = make_test_keys();

    let tags = vec![
        Tag::parse(["d", "turn-submit-001"]).unwrap(),
        Tag::parse(["p", &host_keys.public_key().to_hex()]).unwrap(),
        Tag::parse(["game-id", "test-game"]).unwrap(),
        Tag::parse(["turn", "7"]).unwrap(),
    ];

    let event = build_private_event(
        &player_keys,
        &host_keys,
        30522,
        &TurnCommandsPayload {
            commands: "fleet 1 { order speed=3 }".to_string(),
            handle: Some("marshal".to_string()),
        },
        tags,
    );
    let tag_values: Vec<Vec<String>> = event.tags.iter().map(|tag| tag.clone().to_vec()).collect();
    assert!(
        !tag_values
            .iter()
            .any(|tag| tag.first().map(String::as_str) == Some("handle"))
    );
    assert!(!event.content.contains("fleet 1 { order speed=3 }"));

    let parsed = parse_turn_commands(host_keys.secret_key(), &event).expect("should parse");

    assert_eq!(parsed.submit_id, "turn-submit-001");
    assert_eq!(parsed.game_id, "test-game");
    assert_eq!(parsed.turn, 7);
    assert_eq!(parsed.commands, "fleet 1 { order speed=3 }");
    assert_eq!(parsed.handle.as_deref(), Some("marshal"));
}

#[test]
fn test_parse_seat_claim_30510() {
    let player_keys = make_test_keys();
    let host_keys = make_test_keys();

    let tags = vec![
        Tag::parse(["d", "nonce-12345"]).unwrap(),
        Tag::parse(["p", &host_keys.public_key().to_hex()]).unwrap(),
        Tag::parse(["game-id", "test-game"]).unwrap(),
    ];

    let event = build_private_event(
        &player_keys,
        &host_keys,
        30510,
        &SeatClaimRequestPayload {
            invite: "invite-code-abc123".to_string(),
            handle: Some("claimer".to_string()),
        },
        tags,
    );
    assert!(!event.content.contains("invite-code-abc123"));

    let parsed = parse_seat_claim_request(host_keys.secret_key(), &event).expect("should parse");

    assert_eq!(parsed.nonce, "nonce-12345");
    assert_eq!(parsed.invite_code, "invite-code-abc123");
    assert_eq!(parsed.game_id, Some("test-game".to_string()));
    assert_eq!(parsed.handle.as_deref(), Some("claimer"));
}

#[test]
fn test_parse_handle_check_30525() {
    let player_keys = make_test_keys();
    let host_keys = make_test_keys();

    let tags = vec![
        Tag::parse(["d", "handle-check-001"]).unwrap(),
        Tag::parse(["p", &host_keys.public_key().to_hex()]).unwrap(),
    ];

    let event = build_private_event(
        &player_keys,
        &host_keys,
        30525,
        &HandleCheckRequestPayload {
            handle: "StarRider".to_string(),
        },
        tags,
    );

    let parsed =
        parse_handle_check_request(host_keys.secret_key(), &event).expect("should parse");

    assert_eq!(parsed.request_id, "handle-check-001");
    assert_eq!(parsed.handle, "StarRider");
    assert_eq!(parsed.player_pubkey, player_keys.public_key().to_hex());
}

#[test]
fn test_parse_first_join_setup_30527() {
    let player_keys = make_test_keys();
    let host_keys = make_test_keys();

    let tags = vec![
        Tag::parse(["d", "first-join-001"]).unwrap(),
        Tag::parse(["p", &host_keys.public_key().to_hex()]).unwrap(),
        Tag::parse(["game-id", "test-game"]).unwrap(),
    ];

    let event = build_private_event(
        &player_keys,
        &host_keys,
        30527,
        &FirstJoinSetupRequestPayload {
            empire_name: "Terran Union".to_string(),
            homeworld_name: "Sol".to_string(),
        },
        tags,
    );

    let parsed =
        parse_first_join_setup_request(host_keys.secret_key(), &event).expect("should parse");

    assert_eq!(parsed.request_id, "first-join-001");
    assert_eq!(parsed.game_id, "test-game");
    assert_eq!(parsed.empire_name, "Terran Union");
    assert_eq!(parsed.homeworld_name, "Sol");
}

#[test]
fn test_state_request_missing_tags() {
    let player_keys = make_test_keys();
    let host_keys = make_test_keys();

    let tags = vec![Tag::parse(["d", "bad-req"]).unwrap()];
    let event = build_private_event(
        &player_keys,
        &host_keys,
        30507,
        &StateRequestPayload {
            last_turn: None,
            last_hash: None,
            handle: None,
        },
        tags,
    );

    let result = parse_state_request(host_keys.secret_key(), &event);
    assert!(result.is_none(), "should fail without required tags");
}

#[test]
fn test_invite_request_without_required_tags() {
    let player_keys = make_test_keys();
    let host_keys = make_test_keys();

    let tags = vec![Tag::parse(["d", "req-001"]).unwrap()];
    let event = build_private_event(
        &player_keys,
        &host_keys,
        30513,
        &InviteRequestPayload {
            message: "missing game-id".to_string(),
            handle: None,
        },
        tags,
    );

    let result = parse_invite_request(host_keys.secret_key(), &event);
    assert!(result.is_none(), "should fail without game-id tag");
}
