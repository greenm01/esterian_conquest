use nc_gate::serve::claim::{
    ParseSeatClaimError, parse_seat_claim_request, seat_claim_error_payload,
};
use nc_nostr::claim::SeatClaimRequestPayload;
use nc_nostr::private_payload::encrypt_private_json;
use nostr_sdk::{EventBuilder, Keys, Kind, Tag};

fn signed_claim_request(
    player_keys: &Keys,
    gate_keys: &Keys,
    invite_code: &str,
    game_id: Option<&str>,
) -> nostr_sdk::Event {
    let mut tags = vec![
        Tag::parse(["d", "claim-nonce"]).unwrap(),
        Tag::parse(["p", &gate_keys.public_key().to_hex()]).unwrap(),
    ];
    if let Some(game_id) = game_id {
        tags.push(Tag::parse(["game-id", game_id]).unwrap());
    }
    let encrypted = encrypt_private_json(
        player_keys,
        &gate_keys.public_key(),
        &SeatClaimRequestPayload {
            invite: invite_code.to_string(),
            handle: None,
        },
    )
    .unwrap();
    EventBuilder::new(Kind::Custom(30510), encrypted)
        .tags(tags)
        .sign_with_keys(player_keys)
        .unwrap()
}

#[test]
fn parse_seat_claim_request_basic() {
    let player_keys = Keys::generate();
    let gate_keys = Keys::generate();
    let event = signed_claim_request(&player_keys, &gate_keys, "velvet-mountain", Some("friday-night"));
    let request = parse_seat_claim_request(gate_keys.secret_key(), &event)
        .expect("claim request should parse");
    assert_eq!(request.nonce, "claim-nonce");
    assert_eq!(request.invite_code, "velvet-mountain");
    assert_eq!(request.game_id.as_deref(), Some("friday-night"));
    assert_eq!(request.player_pubkey, event.pubkey.to_hex());
}

#[test]
fn parse_seat_claim_request_requires_invite_code() {
    let player_keys = Keys::generate();
    let gate_keys = Keys::generate();
    let event = signed_claim_request(&player_keys, &gate_keys, "", None);
    assert_eq!(
        parse_seat_claim_request(gate_keys.secret_key(), &event).unwrap_err(),
        ParseSeatClaimError::MissingInviteCode
    );
}

#[test]
fn seat_claim_error_payload_includes_code_and_message() {
    let payload = seat_claim_error_payload("invalid_code", "The invite code is not valid.");
    assert!(payload.contains(r#""error":"invalid_code""#));
    assert!(payload.contains(r#""message":"The invite code is not valid.""#));
}
