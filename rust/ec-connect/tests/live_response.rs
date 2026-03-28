use ec_connect::connect::live_response::{build_response_filter, is_matching_response_event};
use nostr_sdk::{EventBuilder, Keys, Kind, Tag, Timestamp};

#[test]
fn response_filter_targets_gate_player_and_since() {
    let gate_keys = Keys::generate();
    let player_keys = Keys::generate();
    let filter = build_response_filter(
        &gate_keys.public_key(),
        &player_keys.public_key(),
        [Kind::Custom(30502), Kind::Custom(30503)],
        Timestamp::from(1234),
    );

    let json = serde_json::to_value(&filter).unwrap();
    assert_eq!(json["authors"][0], gate_keys.public_key().to_hex());
    assert_eq!(json["#p"][0], player_keys.public_key().to_hex());
    assert_eq!(json["kinds"][0], 30502);
    assert_eq!(json["kinds"][1], 30503);
    assert_eq!(json["since"], 1234);
}

#[test]
fn matching_response_event_requires_kind_author_p_tag_and_nonce() {
    let gate_keys = Keys::generate();
    let player_keys = Keys::generate();
    let nonce = "nonce-123";
    let event = EventBuilder::new(Kind::Custom(30502), "ciphertext")
        .tags(vec![
            Tag::parse(["d", nonce]).unwrap(),
            Tag::parse(["p", &player_keys.public_key().to_hex()]).unwrap(),
        ])
        .sign_with_keys(&gate_keys)
        .unwrap();

    assert!(is_matching_response_event(
        &event,
        &[Kind::Custom(30502), Kind::Custom(30503)],
        &gate_keys.public_key(),
        &player_keys.public_key(),
        nonce,
    ));
    assert!(!is_matching_response_event(
        &event,
        &[Kind::Custom(30503)],
        &gate_keys.public_key(),
        &player_keys.public_key(),
        nonce,
    ));
    assert!(!is_matching_response_event(
        &event,
        &[Kind::Custom(30502), Kind::Custom(30503)],
        &gate_keys.public_key(),
        &player_keys.public_key(),
        "wrong-nonce",
    ));
}
