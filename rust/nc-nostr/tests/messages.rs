use nc_nostr::lobby_notice::{parse_lobby_notice, LobbyNotice};
use nc_nostr::private_payload::encrypt_private_json;
use nc_nostr::thread_message::{decrypt_thread_message, SenderRole, SysopThreadMessage};
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
