use nc_nostr::invite_request::{parse_invite_request, InviteRequestPayload};
use nc_nostr::state_sync::{parse_state_request, StateRequestPayload};
use nc_nostr::turn_commands::{parse_turn_commands, TurnCommandsPayload};
use nc_nostr::private_payload::encrypt_private_json;
use nostr_sdk::{EventBuilder, Keys, Kind, Tag};

#[test]
fn invite_request_uses_hex_player_pubkey() {
    let player = Keys::generate();
    let host = Keys::generate();
    let encrypted = encrypt_private_json(
        &player,
        &host.public_key(),
        &InviteRequestPayload {
            message: "let me in".to_string(),
            handle: Some("StarRider".to_string()),
        },
    )
    .expect("encrypt invite request");

    let event = EventBuilder::new(Kind::Custom(30513), encrypted)
        .tags(vec![
            Tag::parse(["d", "req-001"]).unwrap(),
            Tag::parse(["game-id", "friday-night"]).unwrap(),
            Tag::parse(["p", &host.public_key().to_hex()]).unwrap(),
        ])
        .sign_with_keys(&player)
        .unwrap();

    let parsed = parse_invite_request(host.secret_key(), &event).expect("parse invite request");
    assert_eq!(parsed.player_pubkey, player.public_key().to_hex());
}

#[test]
fn state_request_uses_hex_player_pubkey() {
    let player = Keys::generate();
    let host = Keys::generate();
    let encrypted = encrypt_private_json(
        &player,
        &host.public_key(),
        &StateRequestPayload {
            last_turn: Some(7),
            last_hash: Some("abc123".to_string()),
            handle: Some("StarRider".to_string()),
        },
    )
    .expect("encrypt state request");

    let event = EventBuilder::new(Kind::Custom(30507), encrypted)
        .tags(vec![
            Tag::parse(["d", "state-001"]).unwrap(),
            Tag::parse(["game-id", "friday-night"]).unwrap(),
            Tag::parse(["p", &host.public_key().to_hex()]).unwrap(),
        ])
        .sign_with_keys(&player)
        .unwrap();

    let parsed = parse_state_request(host.secret_key(), &event).expect("parse state request");
    assert_eq!(parsed.player_pubkey, player.public_key().to_hex());
}

#[test]
fn turn_commands_use_hex_player_pubkey() {
    let player = Keys::generate();
    let host = Keys::generate();
    let encrypted = encrypt_private_json(
        &player,
        &host.public_key(),
        &TurnCommandsPayload {
            commands: "fleet 1 { order speed=3 }".to_string(),
            handle: Some("StarRider".to_string()),
        },
    )
    .expect("encrypt turn commands");

    let event = EventBuilder::new(Kind::Custom(30522), encrypted)
        .tags(vec![
            Tag::parse(["d", "submit-001"]).unwrap(),
            Tag::parse(["game-id", "friday-night"]).unwrap(),
            Tag::parse(["turn", "5"]).unwrap(),
            Tag::parse(["p", &host.public_key().to_hex()]).unwrap(),
        ])
        .sign_with_keys(&player)
        .unwrap();

    let parsed = parse_turn_commands(host.secret_key(), &event).expect("parse turn commands");
    assert_eq!(parsed.player_pubkey, player.public_key().to_hex());
}
