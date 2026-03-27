use ec_connect::connect::map_fetch::{MapBundlePayload, MapErrorPayload, MapFilePayload};

#[test]
fn parse_map_bundle_payload_json() {
    let json = r#"{"game_id":"friday-night","game_name":"Friday Night EC","seat":2,"files":[{"name":"starmap.txt","codec":"zstd+base64","sha256":"abc123","content":"Zm9v"}]}"#;
    let payload: MapBundlePayload = serde_json::from_str(json).expect("map bundle should parse");
    assert_eq!(payload.game_id, "friday-night");
    assert_eq!(payload.game_name, "Friday Night EC");
    assert_eq!(payload.seat, 2);
    assert_eq!(
        payload.files,
        vec![MapFilePayload {
            name: "starmap.txt".into(),
            codec: "zstd+base64".into(),
            sha256: "abc123".into(),
            content: "Zm9v".into(),
        }]
    );
}

#[test]
fn parse_map_error_payload_json() {
    let json = r#"{"error":"payload_too_large","message":"The starmap bundle is too large to deliver in one message."}"#;
    let payload: MapErrorPayload = serde_json::from_str(json).expect("map error should parse");
    assert_eq!(payload.error, "payload_too_large");
    assert_eq!(
        payload.message,
        "The starmap bundle is too large to deliver in one message."
    );
}
