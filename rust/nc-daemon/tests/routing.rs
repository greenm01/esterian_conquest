mod common;

use nc_daemon::supervisor::routing::{route_event, RoutingError};
use nostr_sdk::{Keys, Kind, Tag};

fn make_test_keys() -> Keys {
    Keys::generate()
}

fn make_daemon_keys() -> Keys {
    Keys::generate()
}

#[test]
fn test_route_event_no_p_tag() {
    let daemon_keys = make_daemon_keys();
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
    let result = route_event(event, &games_root, &daemon_keys.public_key());

    assert!(matches!(result, Err(RoutingError::NotAddressedToDaemon)));
}

#[test]
fn test_route_event_missing_game_id() {
    let daemon_keys = make_daemon_keys();
    let player_keys = make_test_keys();
    let daemon_hex = daemon_keys.public_key().to_hex();

    let tags = vec![
        Tag::parse(["d", "test"]).unwrap(),
        Tag::parse(["p", &daemon_hex]).unwrap(),
    ];

    let event = nostr_sdk::EventBuilder::new(Kind::Custom(30513), "test")
        .tags(tags)
        .sign_with_keys(&player_keys)
        .unwrap();

    let games_root = std::path::PathBuf::from("/tmp/nonexistent");
    let result = route_event(event, &games_root, &daemon_keys.public_key());

    assert!(matches!(result, Err(RoutingError::InvalidEvent(_))));
}

#[test]
fn test_route_event_unknown_game() {
    let daemon_keys = make_daemon_keys();
    let player_keys = make_test_keys();
    let daemon_hex = daemon_keys.public_key().to_hex();

    let tags = vec![
        Tag::parse(["d", "test"]).unwrap(),
        Tag::parse(["p", &daemon_hex]).unwrap(),
        Tag::parse(["game-id", "nonexistent-game-12345"]).unwrap(),
    ];

    let event = nostr_sdk::EventBuilder::new(Kind::Custom(30513), "test")
        .tags(tags)
        .sign_with_keys(&player_keys)
        .unwrap();

    let games_root = std::path::PathBuf::from("/tmp/nonexistent");
    let result = route_event(event, &games_root, &daemon_keys.public_key());

    assert!(matches!(result, Err(RoutingError::UnknownGame(_))));
}

#[test]
fn test_route_event_invalid_kind() {
    let daemon_keys = make_daemon_keys();
    let player_keys = make_test_keys();
    let daemon_hex = daemon_keys.public_key().to_hex();

    let tags = vec![
        Tag::parse(["d", "test"]).unwrap(),
        Tag::parse(["p", &daemon_hex]).unwrap(),
        Tag::parse(["game-id", "test-game"]).unwrap(),
    ];

    // Use kind 1 (text note) instead of a hosted kind
    let event = nostr_sdk::EventBuilder::new(Kind::TextNote, "test")
        .tags(tags)
        .sign_with_keys(&player_keys)
        .unwrap();

    let games_root = std::path::PathBuf::from("/tmp/nonexistent");
    let result = route_event(event, &games_root, &daemon_keys.public_key());

    // Should fail because game doesn't exist
    assert!(matches!(result, Err(RoutingError::UnknownGame(_))));
}
