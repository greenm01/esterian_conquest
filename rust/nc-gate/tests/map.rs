use nc_data::PlayerMapExportData;
use nc_gate::serve::map::{
    MAP_PUSH_KIND, MapFilePayload, build_map_bundle_payload, build_map_bundle_payload_for_values,
    parse_map_request,
};
use nc_gate::serve::routing::ResolvedSeat;
use nostr_sdk::{EventBuilder, Keys, Kind, Tag};

#[test]
fn parse_map_request_reads_nonce_and_game_id() {
    let player_keys = Keys::generate();
    let event = EventBuilder::new(Kind::Custom(30504), "")
        .tags(vec![
            Tag::parse(["d", "map-nonce-1"]).unwrap(),
            Tag::parse(["p", &player_keys.public_key().to_hex()]).unwrap(),
            Tag::parse(["game-id", "friday-night"]).unwrap(),
        ])
        .sign_with_keys(&player_keys)
        .unwrap();

    let request = parse_map_request(&event).expect("map request should parse");
    assert_eq!(request.nonce, "map-nonce-1");
    assert_eq!(request.game_id, "friday-night");
    assert_eq!(request.player_pubkey, player_keys.public_key().to_hex());
}

#[test]
fn build_map_bundle_payload_compresses_each_file() {
    let seat = ResolvedSeat {
        game_id: "friday-night".into(),
        game_name: "Friday Night EC".into(),
        player: 2,
        player_npub: "player".into(),
        first_claim: false,
    };
    let export = PlayerMapExportData {
        ascii_export: "STAR MAP".into(),
        csv_export: "x,y\n1,2".into(),
        csv_details_export: "x,y,known_name\n1,2,Foosville".into(),
    };

    let payload = build_map_bundle_payload(&seat, &export).expect("payload should build");
    assert_eq!(payload.game_id, "friday-night");
    assert_eq!(payload.game_name, "Friday Night EC");
    assert_eq!(payload.seat, 2);
    assert_eq!(payload.files.len(), 3);
    assert!(payload.files.iter().all(|file| file.codec == "zstd+base64"));
    assert!(payload.files.iter().all(|file| !file.sha256.is_empty()));
    assert!(payload.files.iter().all(|file| !file.content.is_empty()));
    assert!(payload.files.iter().any(|file| {
        file == &MapFilePayload {
            name: "starmap.txt".into(),
            codec: "zstd+base64".into(),
            sha256: file.sha256.clone(),
            content: file.content.clone(),
        }
    }));
}

#[test]
fn build_map_bundle_payload_for_values_matches_proactive_map_push_shape() {
    let export = PlayerMapExportData {
        ascii_export: "STAR MAP".into(),
        csv_export: "x,y\n1,2".into(),
        csv_details_export: "x,y,known_name\n1,2,Foosville".into(),
    };

    let payload =
        build_map_bundle_payload_for_values("friday-night", "Friday Night EC", 2, &export)
            .expect("payload should build");
    assert_eq!(MAP_PUSH_KIND, 30512);
    assert_eq!(payload.game_id, "friday-night");
    assert_eq!(payload.game_name, "Friday Night EC");
    assert_eq!(payload.seat, 2);
    assert_eq!(payload.files.len(), 3);
}
