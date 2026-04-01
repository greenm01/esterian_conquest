use nc_connect::connect::seat_claim::{SeatClaimErrorPayload, parse_seat_claim_error};

#[test]
fn parse_seat_claim_error_payload_json() {
    let json = r#"{"error":"invalid_code","message":"The invite code is not valid."}"#;
    let payload = parse_seat_claim_error(json).expect("seat claim error should parse");
    assert_eq!(
        payload,
        SeatClaimErrorPayload {
            error: "invalid_code".to_string(),
            message: "The invite code is not valid.".to_string(),
        }
    );
}
