use nc_nostr::invite_request::{
    InviteRequestPayload, InviteRequestReceipt, InviteRequestReceiptStatus, parse_invite_request,
};
use nc_nostr::private_payload::encrypt_private_json;
use nc_nostr::state_sync::{
    StateErrorCode, StateErrorPayload, StateRequestPayload, parse_state_error,
    parse_state_request,
};
use nc_nostr::turn_commands::{TurnCommandsPayload, parse_turn_commands};
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

#[test]
fn invite_request_receipt_supports_game_full_status() {
    let receipt = InviteRequestReceipt {
        request_id: "req-001".to_string(),
        game_id: "sandbox-smoke".to_string(),
        status: InviteRequestReceiptStatus::GameFull,
        message: "This sandbox is full right now.".to_string(),
    };

    let json = serde_json::to_string(&receipt).expect("serialize receipt");
    let parsed: InviteRequestReceipt = serde_json::from_str(&json).expect("parse receipt");

    assert_eq!(parsed.status, InviteRequestReceiptStatus::GameFull);
    assert_eq!(parsed.status.as_str(), "game_full");
}

#[test]
fn state_error_payload_round_trips_as_private_30520() {
    let player = Keys::generate();
    let host = Keys::generate();
    let encrypted = encrypt_private_json(
        &host,
        &player.public_key(),
        &StateErrorPayload {
            game_id: "sandbox-smoke".to_string(),
            code: StateErrorCode::NotAPlayer,
            message: "You no longer have a claimed seat in this game.".to_string(),
        },
    )
    .expect("encrypt state error");

    let event = EventBuilder::new(Kind::Custom(30520), encrypted)
        .tags(vec![
            Tag::parse(["d", "state-error"]).unwrap(),
            Tag::parse(["game-id", "sandbox-smoke"]).unwrap(),
            Tag::parse(["error", "not_a_player"]).unwrap(),
            Tag::parse(["p", &player.public_key().to_hex()]).unwrap(),
        ])
        .sign_with_keys(&host)
        .unwrap();

    let parsed = parse_state_error(player.secret_key(), &event).expect("parse state error");
    assert_eq!(parsed.code, StateErrorCode::NotAPlayer);
    assert_eq!(
        parsed.message,
        "You no longer have a claimed seat in this game."
    );
}
