use nc_nostr::claim::{
    SeatClaimErrorPayload, build_seat_claim_request_event, parse_seat_claim_error,
    parse_seat_claim_request,
};
use nc_nostr::discovery::{InviteResolution, select_discovered_game_from_events};
use nc_nostr::session::{
    SessionErrorPayload, SessionReadyPayload, SessionUiMode, build_session_request_event,
    parse_session_error, parse_session_ready, parse_session_request,
};
use nostr_sdk::{EventBuilder, Keys, Kind, PublicKey, Tag};

#[test]
fn session_request_round_trip_parses_expected_fields() {
    let player_keys = Keys::generate();
    let gate_keys = Keys::generate();
    let gate_pubkey = PublicKey::parse(&gate_keys.public_key().to_hex()).expect("gate pubkey");
    let event = build_session_request_event(
        &player_keys,
        &gate_pubkey,
        "abc123",
        "ssh-ed25519 AAAATEST",
        Some("velvet-mountain"),
        Some("game-1"),
    )
    .expect("build event");

    let parsed = parse_session_request(&event).expect("parse request");
    assert_eq!(parsed.nonce, "abc123");
    assert_eq!(parsed.ssh_pubkey, "ssh-ed25519 AAAATEST");
    assert_eq!(parsed.invite_code.as_deref(), Some("velvet-mountain"));
    assert_eq!(parsed.game_id.as_deref(), Some("game-1"));
}

#[test]
fn session_payloads_round_trip_through_json_helpers() {
    let ready = SessionReadyPayload {
        game_id: "game-1".to_string(),
        ssh_host: "play.example.com".to_string(),
        ssh_port: 22,
        ssh_user: "nc".to_string(),
        host_fingerprint: "SHA256:abc".to_string(),
        game_name: "Alpha".to_string(),
        seat: 2,
        player_name: "Empire Two".to_string(),
        session_ui: SessionUiMode::FullscreenNcDash,
    };
    assert_eq!(
        parse_session_ready(&ready.to_json()).expect("parse ready"),
        ready
    );

    let error = SessionErrorPayload {
        error: "multiple_games".to_string(),
        message: "pick one".to_string(),
        games: vec![nc_nostr::session::GameEntry {
            game_id: "g1".to_string(),
            name: "Alpha".to_string(),
            seat: 2,
        }],
    };
    assert_eq!(
        parse_session_error(&error.to_json()).expect("parse error"),
        error
    );
}

#[test]
fn seat_claim_helpers_round_trip() {
    let player_keys = Keys::generate();
    let gate_keys = Keys::generate();
    let gate_pubkey = PublicKey::parse(&gate_keys.public_key().to_hex()).expect("gate pubkey");
    let event = build_seat_claim_request_event(
        &player_keys,
        &gate_pubkey,
        "claim123",
        "Velvet-Mountain",
        Some("game-7"),
    )
    .expect("build claim request");
    let parsed = parse_seat_claim_request(&event).expect("parse claim request");
    assert_eq!(parsed.nonce, "claim123");
    assert_eq!(parsed.invite_code, "velvet-mountain");
    assert_eq!(parsed.game_id.as_deref(), Some("game-7"));

    let error = SeatClaimErrorPayload {
        error: "invalid_code".to_string(),
        message: "bad".to_string(),
    };
    assert_eq!(
        parse_seat_claim_error(&error.to_json()).expect("parse claim error"),
        error
    );
}

#[test]
fn discovery_prefers_same_identity_rejoin() {
    let gate_keys = Keys::generate();
    let player_keys = Keys::generate();
    let invite_hash = nc_nostr::hash::sha256_hex("velvet-mountain".as_bytes());
    let tags = vec![
        Tag::parse(["d", "game-1"]).unwrap(),
        Tag::parse(["name", "Alpha"]).unwrap(),
        Tag::parse(["ssh-host", "play.example.com"]).unwrap(),
        Tag::parse(["ssh-port", "22"]).unwrap(),
        Tag::parse([
            "slot",
            "2",
            &invite_hash,
            &player_keys.public_key().to_hex(),
            "claimed",
        ])
        .unwrap(),
    ];
    let event = EventBuilder::new(Kind::Custom(30500), "")
        .tags(tags)
        .sign_with_keys(&gate_keys)
        .unwrap();

    let discovered = select_discovered_game_from_events(
        [&event],
        "play.example.com",
        22,
        "wss://relay.example.com",
        "velvet-mountain",
        Some(&player_keys.public_key().to_hex()),
    )
    .expect("discover game");

    assert_eq!(discovered.game_id, "game-1");
    assert_eq!(discovered.seat, 2);
    assert_eq!(discovered.resolution, InviteResolution::SameIdentityRejoin);
}
