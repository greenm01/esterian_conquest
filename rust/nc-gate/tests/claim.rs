use nc_gate::serve::claim::{
    ParseSeatClaimError, parse_seat_claim_request, seat_claim_error_payload,
};
use nostr_sdk::{EventBuilder, Keys, Kind, Tag};

fn signed_claim_request(invite_code: &str, game_id: Option<&str>) -> nostr_sdk::Event {
    let keys = Keys::generate();
    let mut tags = vec![
        Tag::parse(["d", "claim-nonce"]).unwrap(),
        Tag::parse(["p", &Keys::generate().public_key().to_hex()]).unwrap(),
    ];
    if let Some(game_id) = game_id {
        tags.push(Tag::parse(["game-id", game_id]).unwrap());
    }
    EventBuilder::new(Kind::Custom(30510), invite_code)
        .tags(tags)
        .sign_with_keys(&keys)
        .unwrap()
}

#[test]
fn parse_seat_claim_request_basic() {
    let event = signed_claim_request("velvet-mountain", Some("friday-night"));
    let request = parse_seat_claim_request(&event).expect("claim request should parse");
    assert_eq!(request.nonce, "claim-nonce");
    assert_eq!(request.invite_code, "velvet-mountain");
    assert_eq!(request.game_id.as_deref(), Some("friday-night"));
    assert_eq!(request.player_pubkey, event.pubkey.to_hex());
}

#[test]
fn parse_seat_claim_request_requires_invite_code() {
    let event = signed_claim_request("", None);
    assert_eq!(
        parse_seat_claim_request(&event).unwrap_err(),
        ParseSeatClaimError::MissingInviteCode
    );
}

#[test]
fn seat_claim_error_payload_includes_code_and_message() {
    let payload = seat_claim_error_payload("invalid_code", "The invite code is not valid.");
    assert!(payload.contains(r#""error":"invalid_code""#));
    assert!(payload.contains(r#""message":"The invite code is not valid.""#));
}
