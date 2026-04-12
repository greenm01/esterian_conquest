mod common;

use common::create_test_game;
use nc_daemon::lobby::catalog_publish::publish_game_definition;
use nc_nostr::game_definition::build_game_definition_tags;
use nc_nostr::invite_request::{build_invite_decision_tags, InviteDecision, InviteDecisionPayload};
use nc_nostr::state_sync::{build_state_response_tags, GameState};

#[test]
fn game_definition_tags_are_per_field_and_slots_are_multivalue() {
    let (_temp, _game_dir, store) = create_test_game("catalog-test", 4);
    let def = publish_game_definition(&store, "catalog-test", Some("Test Host"))
        .expect("definition should build")
        .expect("definition should exist");

    let tags = build_game_definition_tags(&def);

    assert!(tags.iter().any(|t| t == &vec!["d".to_string(), "catalog-test".to_string()]));
    assert!(tags.iter().any(|t| t.first().map(String::as_str) == Some("name")));
    assert!(tags.iter().any(|t| t.first().map(String::as_str) == Some("recruiting")));

    let slot = tags
        .iter()
        .find(|t| t.first().map(String::as_str) == Some("slot"))
        .expect("slot tag should exist");
    assert_eq!(slot.len(), 5);
}

#[test]
fn invite_decision_tags_do_not_leak_invite() {
    let payload = InviteDecisionPayload {
        request_id: "req-42".to_string(),
        game_id: "friday-night".to_string(),
        decision: InviteDecision::Approved {
            invite: "amber-river@relay.example.com".to_string(),
        },
        message: "Seat 2 is yours.".to_string(),
    };

    let tags = build_invite_decision_tags(&payload);
    assert!(tags.iter().all(|(key, _)| *key != "invite"));
}

#[test]
fn state_response_tags_do_not_leak_player_identity() {
    let state = GameState {
        game_id: "friday-night".to_string(),
        turn: 12,
        year: 3012,
        player_seat: 4,
        player_name: "Fourth Empire".to_string(),
        state_hash: "abc123".to_string(),
        state: serde_json::json!({}),
        queued_mail: vec![],
        report_blocks: vec![],
    };

    let tags = build_state_response_tags(&state);
    assert!(tags.iter().all(|(key, _)| *key != "player-seat"));
    assert!(tags.iter().all(|(key, _)| *key != "player-name"));
}
