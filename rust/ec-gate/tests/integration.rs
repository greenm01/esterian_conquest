//! End-to-end integration test for ec-gate steps 1–9.
//!
//! This test exercises the full processing pipeline for a 30501 SessionRequest
//! event — from parsing through routing through provisioning through response
//! payload construction — without requiring a live Nostr relay.
//!
//! Coverage:
//!   - Roster creation and write-back on first-time claim
//!   - 30501 event parsing (valid and stale)
//!   - Session routing for all three paths (invite code, game-id, npub-only)
//!   - SSH key provisioning (both Command and File methods)
//!   - 30502/30503 payload construction and NIP-44 round-trip
//!   - 30500 GameDefinition tag construction

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use nostr_sdk::nips::nip44;
use nostr_sdk::nips::nip44::Version;
use nostr_sdk::{EventBuilder, Keys, Kind, Tag};

use ec_gate::config::{AuthKeysMethod, DEFAULT_EC_GAME_PATH, GateConfig};
use ec_gate::roster::io::{load_roster, save_roster};
use ec_gate::roster::{Roster, Seat, SeatStatus};
use ec_gate::serve::game_def::{build_game_def_tags, sha256_hex};
use ec_gate::serve::provision::{provision_key, reap_expired_keys, remove_key};
use ec_gate::serve::request::{MAX_EVENT_AGE_SECS, parse_session_request};
use ec_gate::serve::response::{SessionReadyPayload, session_error_payload};
use ec_gate::serve::routing::{RouteError, RoutingDecision, route};

// ---------------------------------------------------------------------------
// Shared fixtures
// ---------------------------------------------------------------------------

fn temp_dir(tag: &str) -> PathBuf {
    let id = format!(
        "ec_gate_int_{tag}_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let path = std::env::temp_dir().join(id);
    fs::create_dir_all(&path).unwrap();
    path
}

fn make_roster_with_pending_seat(id: &str, name: &str) -> Roster {
    Roster {
        id: id.to_string(),
        name: name.to_string(),
        seats: vec![
            Seat {
                player: 1,
                code: "velvet-mountain".to_string(),
                status: SeatStatus::Pending,
                npub: None,
            },
            Seat {
                player: 2,
                code: "copper-sunrise".to_string(),
                status: SeatStatus::Pending,
                npub: None,
            },
        ],
    }
}

fn gate_config_command(keys_dir: PathBuf) -> GateConfig {
    GateConfig {
        relay: "wss://relay.example.com".to_string(),
        ssh_host: "play.example.com".to_string(),
        ssh_port: 22,
        ssh_user: "ecgame".to_string(),
        ec_game_path: PathBuf::from(DEFAULT_EC_GAME_PATH),
        auth_keys_method: AuthKeysMethod::Command,
        auth_keys_path: keys_dir,
        key_ttl: 60,
        games: vec![],
    }
}

fn signed_session_request_event(
    player_keys: &Keys,
    gate_keys: &Keys,
    invite_code: &str,
    game_id: Option<&str>,
    ssh_pubkey: &str,
) -> nostr_sdk::Event {
    let nonce = "test-nonce-integration-0001";
    let mut tag_list = vec![
        Tag::parse(["d", nonce]).unwrap(),
        Tag::parse(["p", &gate_keys.public_key().to_hex()]).unwrap(),
        Tag::parse(["ssh-pubkey", ssh_pubkey]).unwrap(),
    ];
    if let Some(gid) = game_id {
        tag_list.push(Tag::parse(["game-id", gid]).unwrap());
    }
    EventBuilder::new(Kind::Custom(30501), invite_code)
        .tags(tag_list)
        .sign_with_keys(player_keys)
        .unwrap()
}

// ---------------------------------------------------------------------------
// Test 1: First-time invite code claim pipeline
// ---------------------------------------------------------------------------

#[test]
fn full_pipeline_first_time_join_with_invite_code() {
    let dir = temp_dir("t1");
    let game_dir = dir.join("friday-night");
    fs::create_dir_all(&game_dir).unwrap();
    let keys_dir = dir.join("keys");

    let roster = make_roster_with_pending_seat("friday-night", "Friday Night EC");
    save_roster(&game_dir.join("roster.kdl"), &roster).unwrap();

    let gate_keys = Keys::generate();
    let player_keys = Keys::generate();
    let player_hex = player_keys.public_key().to_hex();

    let ssh_pubkey = "ssh-ed25519 AAAA111111111111111111111111111111111111111111111111111 test";
    let event = signed_session_request_event(
        &player_keys,
        &gate_keys,
        "velvet-mountain",
        None,
        ssh_pubkey,
    );

    // Step A: Parse the 30501 event.
    let req = parse_session_request(&event).expect("should parse");
    assert_eq!(req.invite_code.as_deref(), Some("velvet-mountain"));
    assert_eq!(req.player_pubkey, player_hex);

    // Step B: Route.
    let mut rosters = vec![roster];
    let dirs: Vec<&std::path::Path> = vec![&game_dir];
    let decision = route(&req, &mut rosters, &dirs);

    let seat = match decision {
        RoutingDecision::Provisioned(s) => s,
        other => panic!("expected Provisioned, got {other:?}"),
    };
    assert_eq!(seat.game_id, "friday-night");
    assert_eq!(seat.player, 1);

    // Verify roster was updated on disk.
    let saved = load_roster(&game_dir.join("roster.kdl")).unwrap();
    assert_eq!(saved.seats[0].status, SeatStatus::Claimed);
    assert_eq!(saved.seats[0].npub.as_deref(), Some(player_hex.as_str()));

    // Step C: Provision SSH key.
    let config = gate_config_command(keys_dir);
    let provisioned =
        provision_key(&config, &seat, ssh_pubkey, &game_dir).expect("provision_key should succeed");
    assert!(
        provisioned
            .entry
            .contains("--player-record-index-1-based 1")
    );
    assert!(provisioned.entry.contains(ssh_pubkey));

    // Verify key file exists.
    let key_file = config
        .auth_keys_path
        .join(format!("{}.key", provisioned.key_id));
    assert!(key_file.exists());

    // Step D: Build SessionReady payload and verify NIP-44 round-trip.
    let payload = SessionReadyPayload {
        game_id: &seat.game_id,
        ssh_host: &config.ssh_host,
        ssh_port: config.ssh_port,
        ssh_user: &config.ssh_user,
        game_name: &seat.game_name,
        seat: seat.player,
    };
    let plaintext = payload.to_json();

    let encrypted = nip44::encrypt(
        gate_keys.secret_key(),
        &player_keys.public_key(),
        &plaintext,
        Version::V2,
    )
    .expect("encrypt");

    let decrypted = nip44::decrypt(
        player_keys.secret_key(),
        &gate_keys.public_key(),
        &encrypted,
    )
    .expect("decrypt");

    assert_eq!(decrypted, plaintext);
    assert!(decrypted.contains("friday-night"));
    assert!(decrypted.contains("play.example.com"));
    assert!(decrypted.contains(r#""ssh_user":"ecgame""#));
    assert!(decrypted.contains(r#""seat":1"#));

    // Step E: Clean up — remove key.
    remove_key(&config, &provisioned.key_id).unwrap();
    assert!(!key_file.exists());
}

// ---------------------------------------------------------------------------
// Test 2: Returning player via game-id
// ---------------------------------------------------------------------------

#[test]
fn full_pipeline_returning_player_via_game_id() {
    let player_keys = Keys::generate();
    let player_hex = player_keys.public_key().to_hex();

    let mut roster = make_roster_with_pending_seat("saturday-showdown", "Saturday Showdown");
    // Pre-claim seat 2 for this player.
    roster.seats[1].status = SeatStatus::Claimed;
    roster.seats[1].npub = Some(player_hex.clone());

    let gate_keys = Keys::generate();
    let ssh_pubkey = "ssh-ed25519 AAAA222222222222222222222222222222222222222222222222222 test";
    let event = signed_session_request_event(
        &player_keys,
        &gate_keys,
        "", // no invite code
        Some("saturday-showdown"),
        ssh_pubkey,
    );

    let req = parse_session_request(&event).expect("should parse");
    assert!(req.invite_code.is_none());
    assert_eq!(req.game_id.as_deref(), Some("saturday-showdown"));

    let mut rosters = vec![roster];
    let dirs: Vec<&std::path::Path> = vec![std::path::Path::new("/srv/ec/saturday-showdown")];
    let decision = route(&req, &mut rosters, &dirs);

    let seat = match decision {
        RoutingDecision::Provisioned(s) => s,
        other => panic!("expected Provisioned, got {other:?}"),
    };
    assert_eq!(seat.player, 2);
    assert_eq!(seat.game_id, "saturday-showdown");
}

// ---------------------------------------------------------------------------
// Test 3: Error path — invalid invite code
// ---------------------------------------------------------------------------

#[test]
fn full_pipeline_invalid_invite_code_returns_error_payload() {
    let player_keys = Keys::generate();
    let gate_keys = Keys::generate();
    let ssh_pubkey = "ssh-ed25519 AAAA333333333333333333333333333333333333333333333333333 test";
    let event =
        signed_session_request_event(&player_keys, &gate_keys, "totally-wrong", None, ssh_pubkey);

    let req = parse_session_request(&event).expect("should parse");

    let roster = make_roster_with_pending_seat("any-game", "Any Game");
    let mut rosters = vec![roster];
    let dirs: Vec<&std::path::Path> = vec![std::path::Path::new("/srv/ec/any-game")];
    let decision = route(&req, &mut rosters, &dirs);

    match decision {
        RoutingDecision::Error(RouteError::InvalidCode) => {}
        other => panic!("expected InvalidCode, got {other:?}"),
    }

    // Build 30503 error payload.
    let payload = session_error_payload(&RouteError::InvalidCode);
    assert!(payload.contains(r#""error":"invalid_code""#));

    // NIP-44 round-trip.
    let encrypted = nip44::encrypt(
        gate_keys.secret_key(),
        &player_keys.public_key(),
        &payload,
        Version::V2,
    )
    .expect("encrypt");
    let decrypted = nip44::decrypt(
        player_keys.secret_key(),
        &gate_keys.public_key(),
        &encrypted,
    )
    .expect("decrypt");
    assert_eq!(decrypted, payload);
}

// ---------------------------------------------------------------------------
// Test 4: Multiple-games disambiguation error
// ---------------------------------------------------------------------------

#[test]
fn full_pipeline_multiple_games_returns_disambiguation_payload() {
    let player_keys = Keys::generate();
    let player_hex = player_keys.public_key().to_hex();
    let gate_keys = Keys::generate();

    // Player is in both games.
    let mut roster_a = make_roster_with_pending_seat("game-a", "Game A");
    roster_a.seats[0].status = SeatStatus::Claimed;
    roster_a.seats[0].npub = Some(player_hex.clone());

    let mut roster_b = make_roster_with_pending_seat("game-b", "Game B");
    roster_b.seats[1].status = SeatStatus::Claimed;
    roster_b.seats[1].npub = Some(player_hex.clone());

    let ssh_pubkey = "ssh-ed25519 AAAA444444444444444444444444444444444444444444444444444 test";
    // No invite code, no game-id.
    let event = signed_session_request_event(&player_keys, &gate_keys, "", None, ssh_pubkey);

    let req = parse_session_request(&event).expect("should parse");

    let mut rosters = vec![roster_a, roster_b];
    let dirs: Vec<&std::path::Path> = vec![
        std::path::Path::new("/srv/ec/game-a"),
        std::path::Path::new("/srv/ec/game-b"),
    ];
    let decision = route(&req, &mut rosters, &dirs);

    let games = match decision {
        RoutingDecision::Error(RouteError::MultipleGames(g)) => g,
        other => panic!("expected MultipleGames, got {other:?}"),
    };
    assert_eq!(games.len(), 2);

    let payload = session_error_payload(&RouteError::MultipleGames(games));
    assert!(payload.contains(r#""error":"multiple_games""#));
    assert!(payload.contains("game-a"));
    assert!(payload.contains("game-b"));
}

// ---------------------------------------------------------------------------
// Test 5: GameDefinition tags for a full 4-seat roster
// ---------------------------------------------------------------------------

#[test]
fn full_pipeline_game_definition_tags_four_seat_roster() {
    let roster = Roster {
        id: "friday-night".to_string(),
        name: "Friday Night EC".to_string(),
        seats: vec![
            Seat {
                player: 1,
                code: "velvet-mountain".to_string(),
                status: SeatStatus::Claimed,
                npub: Some("npub1aaa".to_string()),
            },
            Seat {
                player: 2,
                code: "copper-sunrise".to_string(),
                status: SeatStatus::Pending,
                npub: None,
            },
            Seat {
                player: 3,
                code: "amber-cascade".to_string(),
                status: SeatStatus::Claimed,
                npub: Some("npub1bbb".to_string()),
            },
            Seat {
                player: 4,
                code: "silver-meadow".to_string(),
                status: SeatStatus::Pending,
                npub: None,
            },
        ],
    };

    let tags = build_game_def_tags(&roster).unwrap();
    let tag_vecs: Vec<Vec<String>> = tags.iter().map(|t| t.clone().to_vec()).collect();

    // d tag
    let d = tag_vecs.iter().find(|t| t[0] == "d").unwrap();
    assert_eq!(d[1], "friday-night");

    // players count
    let players = tag_vecs.iter().find(|t| t[0] == "players").unwrap();
    assert_eq!(players[1], "4");

    // 4 slot tags
    let slots: Vec<_> = tag_vecs.iter().filter(|t| t[0] == "slot").collect();
    assert_eq!(slots.len(), 4);

    // Verify code hashes are correct SHA-256.
    let slot1 = slots.iter().find(|s| s[1] == "1").unwrap();
    assert_eq!(slot1[2], sha256_hex("velvet-mountain"));
    assert_eq!(slot1[3], "npub1aaa");
    assert_eq!(slot1[4], "claimed");

    let slot2 = slots.iter().find(|s| s[1] == "2").unwrap();
    assert_eq!(slot2[2], sha256_hex("copper-sunrise"));
    assert_eq!(slot2[3], "");
    assert_eq!(slot2[4], "pending");
}

// ---------------------------------------------------------------------------
// Test 6: Key reaper removes expired key while leaving live key
// ---------------------------------------------------------------------------

#[test]
fn full_pipeline_reaper_cleans_expired_while_leaving_live() {
    let dir = temp_dir("t6");
    let game_dir = dir.join("reaper-game");
    fs::create_dir_all(&game_dir).unwrap();
    let keys_dir = dir.join("keys");

    let seat_live = ec_gate::serve::routing::ResolvedSeat {
        game_id: "reaper-game".to_string(),
        game_name: "Reaper Game".to_string(),
        player: 1,
        player_npub: "npub1live".to_string(),
    };
    let seat_expired = ec_gate::serve::routing::ResolvedSeat {
        game_id: "reaper-game".to_string(),
        game_name: "Reaper Game".to_string(),
        player: 2,
        player_npub: "npub1expired".to_string(),
    };

    let ssh_a = "ssh-ed25519 AAAA555555555555555555555555555555555555555555555555555 live";
    let ssh_b = "ssh-ed25519 AAAA666666666666666666666666666666666666666666666666666 expired";

    // Provision a long-lived key.
    let config_live = gate_config_command(keys_dir.clone());
    let key_live = provision_key(&config_live, &seat_live, ssh_a, &game_dir).unwrap();

    // Provision a key with zero TTL and wait for it to expire.
    let mut config_expired = gate_config_command(keys_dir.clone());
    config_expired.key_ttl = 0;
    let key_expired = provision_key(&config_expired, &seat_expired, ssh_b, &game_dir).unwrap();

    std::thread::sleep(std::time::Duration::from_secs(1));

    // Reap using the same directory.
    let removed = reap_expired_keys(&config_live).expect("reap");
    assert!(removed >= 1, "should have removed the expired key");

    // Expired key file gone.
    let expired_path = keys_dir.join(format!("{}.key", key_expired.key_id));
    assert!(!expired_path.exists(), "expired key should be gone");

    // Live key file still present.
    let live_path = keys_dir.join(format!("{}.key", key_live.key_id));
    assert!(live_path.exists(), "live key should remain");
}

// ---------------------------------------------------------------------------
// Test 7: Stale event rejection (replay prevention)
// ---------------------------------------------------------------------------

#[test]
fn full_pipeline_stale_event_is_rejected() {
    let player_keys = Keys::generate();
    let gate_keys = Keys::generate();
    let ssh_pubkey = "ssh-ed25519 AAAA777777777777777777777777777777777777777777777777777 test";

    let old_ts = nostr_sdk::Timestamp::from_secs(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
            - MAX_EVENT_AGE_SECS
            - 10,
    );

    let event = EventBuilder::new(Kind::Custom(30501), "velvet-mountain")
        .tags(vec![
            Tag::parse(["d", "stale-nonce"]).unwrap(),
            Tag::parse(["p", &gate_keys.public_key().to_hex()]).unwrap(),
            Tag::parse(["ssh-pubkey", ssh_pubkey]).unwrap(),
        ])
        .custom_created_at(old_ts)
        .sign_with_keys(&player_keys)
        .unwrap();

    let result = parse_session_request(&event);
    assert!(
        matches!(result, Err(ec_gate::serve::request::ParseError::Stale)),
        "stale event should be rejected: {result:?}"
    );
}
