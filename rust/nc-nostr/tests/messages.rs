use nc_nostr::contact_message::{ContactMessage, decrypt_contact_message};
use nc_nostr::lobby_notice::{LobbyNotice, parse_lobby_notice};
use nc_nostr::player_message::{
    PlayerMessage, PlayerMessageRequest, decrypt_player_message, parse_player_message_request,
};
use nc_nostr::private_payload::encrypt_private_json;
use nc_nostr::thread_message::{SenderRole, SysopThreadMessage, decrypt_thread_message};
use nostr_sdk::{EventBuilder, Keys, Kind, Tag, ToBech32};

#[test]
fn parse_lobby_notice_round_trips_payload_and_d_tag() {
    let keys = Keys::generate();
    let notice = LobbyNotice {
        notice_id: "notice-001".to_string(),
        sender_npub: keys.public_key().to_bech32().expect("npub"),
        sender_handle: Some("nc-host".to_string()),
        body: "Maintenance runs at 19:00 UTC tonight.".to_string(),
        created_at: 1_770_000_000,
    };
    let event = EventBuilder::new(Kind::Custom(30516), serde_json::to_string(&notice).unwrap())
        .tags(vec![Tag::parse(["d", "notice-001"]).unwrap()])
        .sign_with_keys(&keys)
        .unwrap();

    let parsed = parse_lobby_notice(&event).expect("parse notice");
    assert_eq!(parsed.notice_id, "notice-001");
    assert_eq!(parsed.sender_handle.as_deref(), Some("nc-host"));
    assert!(parsed.body.contains("Maintenance runs"));
}

#[test]
fn decrypt_thread_message_round_trips_encrypted_payload() {
    let sender = Keys::generate();
    let recipient = Keys::generate();
    let payload = SysopThreadMessage {
        message_id: "thread-001".to_string(),
        game_id: "friday-night".to_string(),
        sender_role: SenderRole::Sysop,
        sender_pubkey: sender.public_key().to_hex(),
        sender_npub: sender.public_key().to_bech32().expect("npub"),
        sender_handle: Some("nc-host".to_string()),
        body: "You are approved for the open seat.".to_string(),
        created_at: 1_770_000_000,
    };
    let encrypted =
        encrypt_private_json(&sender, &recipient.public_key(), &payload).expect("encrypt");
    let event = EventBuilder::new(Kind::Custom(30517), &encrypted)
        .tags(vec![
            Tag::parse(["d", "thread-001"]).unwrap(),
            Tag::parse(["game-id", "friday-night"]).unwrap(),
            Tag::parse(["p", &recipient.public_key().to_hex()]).unwrap(),
        ])
        .sign_with_keys(&sender)
        .unwrap();

    let parsed = decrypt_thread_message(recipient.secret_key(), &event).expect("decrypt");
    assert_eq!(parsed.message_id, "thread-001");
    assert_eq!(parsed.game_id, "friday-night");
    assert_eq!(parsed.sender_role, SenderRole::Sysop);
    assert_eq!(parsed.sender_pubkey, sender.public_key().to_hex());
    assert_eq!(parsed.sender_handle.as_deref(), Some("nc-host"));
}

#[test]
fn decrypt_contact_message_round_trips_encrypted_payload() {
    let sender = Keys::generate();
    let recipient = Keys::generate();
    let payload = ContactMessage {
        message_id: "contact-001".to_string(),
        sender_pubkey: sender.public_key().to_hex(),
        sender_npub: sender.public_key().to_bech32().expect("npub"),
        sender_label: Some("nc_sysop".to_string()),
        body: "Ping from direct contact.".to_string(),
        created_at: 1_770_000_001,
    };
    let encrypted =
        encrypt_private_json(&sender, &recipient.public_key(), &payload).expect("encrypt");
    let event = EventBuilder::new(Kind::Custom(30518), &encrypted)
        .tags(vec![
            Tag::parse(["d", "contact-001"]).unwrap(),
            Tag::parse(["p", &recipient.public_key().to_hex()]).unwrap(),
        ])
        .sign_with_keys(&sender)
        .unwrap();

    let parsed = decrypt_contact_message(recipient.secret_key(), &event).expect("decrypt");
    assert_eq!(parsed.message_id, "contact-001");
    assert_eq!(parsed.sender_label.as_deref(), Some("nc_sysop"));
    assert_eq!(parsed.sender_pubkey, sender.public_key().to_hex());
}

#[test]
fn player_message_helpers_round_trip() {
    let sender = Keys::generate();
    let recipient = Keys::generate();
    let request = PlayerMessageRequest {
        message_id: "mail-001".to_string(),
        game_id: "friday-night".to_string(),
        sender_pubkey: sender.public_key().to_hex(),
        recipient_empire_id: 2,
        body: "Hold the border.".to_string(),
        created_at: 1_770_000_002,
    };
    let encrypted =
        encrypt_private_json(&sender, &recipient.public_key(), &request).expect("encrypt");
    let request_event = EventBuilder::new(Kind::Custom(30523), &encrypted)
        .tags(vec![
            Tag::parse(["d", "mail-001"]).unwrap(),
            Tag::parse(["game-id", "friday-night"]).unwrap(),
            Tag::parse(["p", &recipient.public_key().to_hex()]).unwrap(),
        ])
        .sign_with_keys(&sender)
        .unwrap();
    let parsed_request =
        parse_player_message_request(recipient.secret_key(), &request_event).expect("parse");
    assert_eq!(parsed_request.message_id, "mail-001");
    assert_eq!(parsed_request.recipient_empire_id, 2);
    assert_eq!(parsed_request.sender_pubkey, sender.public_key().to_hex());

    let message = PlayerMessage {
        message_id: "mail-001".to_string(),
        game_id: "friday-night".to_string(),
        sender_empire_id: 1,
        sender_empire_name: "Terran Union".to_string(),
        recipient_empire_id: 2,
        recipient_empire_name: "Rigel Empire".to_string(),
        body: "Hold the border.".to_string(),
        created_at: 1_770_000_003,
    };
    let encrypted =
        encrypt_private_json(&sender, &recipient.public_key(), &message).expect("encrypt");
    let message_event = EventBuilder::new(Kind::Custom(30523), &encrypted)
        .tags(vec![
            Tag::parse(["d", "mail-001"]).unwrap(),
            Tag::parse(["game-id", "friday-night"]).unwrap(),
            Tag::parse(["p", &recipient.public_key().to_hex()]).unwrap(),
        ])
        .sign_with_keys(&sender)
        .unwrap();
    let parsed_message =
        decrypt_player_message(recipient.secret_key(), &message_event).expect("decrypt");
    assert_eq!(parsed_message.sender_empire_name, "Terran Union");
    assert_eq!(parsed_message.recipient_empire_name, "Rigel Empire");
}
