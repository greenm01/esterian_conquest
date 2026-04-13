mod common;

use nc_host::supervisor::routing::{RoutingError, route_event};
use nostr_sdk::{Keys, Kind, Tag};

fn make_test_keys() -> Keys {
    Keys::generate()
}

fn make_host_keys() -> Keys {
    Keys::generate()
}

#[test]
fn test_route_event_no_p_tag() {
    let host_keys = make_host_keys();
    let player_keys = make_test_keys();

    let tags = vec![
        Tag::parse(["d", "test"]).unwrap(),
        Tag::parse(["game-id", "test-game"]).unwrap(),
    ];

    let event = nostr_sdk::EventBuilder::new(Kind::Custom(30513), "test")
        .tags(tags)
        .sign_with_keys(&player_keys)
        .unwrap();

    let games_root = std::path::PathBuf::from("/tmp/nonexistent");
    let result = route_event(event, &games_root, &host_keys.public_key());

    assert!(matches!(result, Err(RoutingError::NotAddressedToHost)));
}

#[test]
fn test_route_event_missing_game_id() {
    let host_keys = make_host_keys();
    let player_keys = make_test_keys();
    let host_hex = host_keys.public_key().to_hex();

    let tags = vec![
        Tag::parse(["d", "test"]).unwrap(),
        Tag::parse(["p", &host_hex]).unwrap(),
    ];

    let event = nostr_sdk::EventBuilder::new(Kind::Custom(30513), "test")
        .tags(tags)
        .sign_with_keys(&player_keys)
        .unwrap();

    let games_root = std::path::PathBuf::from("/tmp/nonexistent");
    let result = route_event(event, &games_root, &host_keys.public_key());

    assert!(matches!(result, Err(RoutingError::InvalidEvent(_))));
}

#[test]
fn test_route_event_unknown_game() {
    let host_keys = make_host_keys();
    let player_keys = make_test_keys();
    let host_hex = host_keys.public_key().to_hex();

    let tags = vec![
        Tag::parse(["d", "test"]).unwrap(),
        Tag::parse(["p", &host_hex]).unwrap(),
        Tag::parse(["game-id", "nonexistent-game-12345"]).unwrap(),
    ];

    let event = nostr_sdk::EventBuilder::new(Kind::Custom(30513), "test")
        .tags(tags)
        .sign_with_keys(&player_keys)
        .unwrap();

    let games_root = std::path::PathBuf::from("/tmp/nonexistent");
    let result = route_event(event, &games_root, &host_keys.public_key());

    assert!(matches!(result, Err(RoutingError::UnknownGame(_))));
}

#[test]
fn test_route_event_invalid_kind() {
    let host_keys = make_host_keys();
    let player_keys = make_test_keys();
    let host_hex = host_keys.public_key().to_hex();

    let tags = vec![
        Tag::parse(["d", "test"]).unwrap(),
        Tag::parse(["p", &host_hex]).unwrap(),
        Tag::parse(["game-id", "test-game"]).unwrap(),
    ];

    // Use kind 1 (text note) instead of a hosted kind
    let event = nostr_sdk::EventBuilder::new(Kind::TextNote, "test")
        .tags(tags)
        .sign_with_keys(&player_keys)
        .unwrap();

    let games_root = std::path::PathBuf::from("/tmp/nonexistent");
    let result = route_event(event, &games_root, &host_keys.public_key());

    // Should fail because game doesn't exist
    assert!(matches!(result, Err(RoutingError::UnknownGame(_))));
}
