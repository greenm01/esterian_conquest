//! End-to-end integration coverage for the hosted-seat gate path.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use nc_data::{CampaignSettings, CampaignStore, HostedSeat, HostedSeatStatus};
use nc_gate::config::{AuthKeysMethod, DEFAULT_NC_GAME_PATH, GateConfig};
use nc_gate::serve::catalog::{HostedGame, HostedGameEntry};
use nc_gate::serve::game_def::build_game_def_tags;
use nc_gate::serve::lease::find_active_identity_session;
use nc_gate::serve::provision::{provision_key, reap_expired_keys, remove_key};
use nc_gate::serve::request::parse_session_request;
use nc_gate::serve::response::{SessionReadyPayload, SessionUiMode, session_error_payload};
use nc_gate::serve::routing::{RouteError, RoutingDecision, route};
use nc_nostr::hash::sha256_hex;
use nc_nostr::timing::MAX_EVENT_AGE_SECS;
use nostr_sdk::nips::nip44;
use nostr_sdk::nips::nip44::Version;
use nostr_sdk::{EventBuilder, Keys, Kind, Tag};

fn temp_dir(tag: &str) -> PathBuf {
    let id = format!(
        "nc_gate_int_{tag}_{}_{}",
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

fn hosted_game_entry(
    dir: &std::path::Path,
    id: &str,
    name: &str,
    seats: Vec<HostedSeat>,
) -> HostedGameEntry {
    let store = CampaignStore::open_default_in_dir(dir).unwrap();
    store
        .save_campaign_settings(&CampaignSettings::new(id, name))
        .unwrap();
    store.replace_hosted_seats(&seats).unwrap();
    HostedGameEntry {
        dir: dir.to_path_buf(),
        game: HostedGame {
            game_id: id.to_string(),
            game_name: name.to_string(),
            seats,
        },
    }
}

fn pending_seat(player: usize, code: &str) -> HostedSeat {
    HostedSeat {
        player_record_index_1_based: player,
        invite_code: code.to_string(),
        status: HostedSeatStatus::Pending,
        player_npub: None,
    }
}

fn claimed_seat(player: usize, code: &str, npub: &str) -> HostedSeat {
    HostedSeat {
        player_record_index_1_based: player,
        invite_code: code.to_string(),
        status: HostedSeatStatus::Claimed,
        player_npub: Some(npub.to_string()),
    }
}

fn gate_config_command(keys_dir: PathBuf) -> GateConfig {
    GateConfig {
        relay: "wss://relay.example.com".to_string(),
        ssh_host: "play.example.com".to_string(),
        ssh_port: 22,
        ssh_user: "ecgame".to_string(),
        nc_game_path: PathBuf::from(DEFAULT_NC_GAME_PATH),
        nc_game_log_file: None,
        nc_game_log_level: None,
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

#[test]
fn full_pipeline_first_time_join_with_invite_code() {
    let dir = temp_dir("t1");
    let game_dir = dir.join("friday-night");
    fs::create_dir_all(&game_dir).unwrap();
    let keys_dir = dir.join("keys");
    let entry = hosted_game_entry(
        &game_dir,
        "friday-night",
        "Friday Night EC",
        vec![
            pending_seat(1, "velvet-mountain"),
            pending_seat(2, "copper-sunrise"),
        ],
    );

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

    let req = parse_session_request(&event).expect("should parse");
    let decision = route(&req, &[entry.clone()]);
    let seat = match decision {
        RoutingDecision::Provisioned(seat) => seat,
        other => panic!("expected Provisioned, got {other:?}"),
    };
    assert!(seat.first_claim);

    let store = CampaignStore::open_default_in_dir(&game_dir).unwrap();
    let pending = store.hosted_seats().unwrap();
    assert_eq!(pending[0].status, HostedSeatStatus::Pending);
    assert!(pending[0].player_npub.is_none());

    let claimed = store.claim_hosted_seat_for_player(1, &player_hex).unwrap();
    let claimed = claimed.expect("seat should exist");
    assert_eq!(claimed.status, HostedSeatStatus::Claimed);
    assert_eq!(claimed.player_npub.as_deref(), Some(player_hex.as_str()));

    let config = gate_config_command(keys_dir);
    let provisioned = provision_key(
        &config,
        &seat,
        ssh_pubkey,
        &game_dir,
        "session-test-token",
        Some("velvet-mountain"),
    )
    .unwrap();
    let key_file = config
        .auth_keys_path
        .join(format!("{}.key", provisioned.key_id));
    assert!(key_file.exists());

    let payload = SessionReadyPayload {
        game_id: &seat.game_id,
        ssh_host: &config.ssh_host,
        ssh_port: config.ssh_port,
        ssh_user: &config.ssh_user,
        game_name: &seat.game_name,
        seat: seat.player,
        player_name: "Empire of Sol",
        session_ui: SessionUiMode::ClassicNcGame,
    };
    let plaintext = payload.to_json();
    let encrypted = nip44::encrypt(
        gate_keys.secret_key(),
        &player_keys.public_key(),
        &plaintext,
        Version::V2,
    )
    .unwrap();
    let decrypted = nip44::decrypt(
        player_keys.secret_key(),
        &gate_keys.public_key(),
        &encrypted,
    )
    .unwrap();
    assert_eq!(decrypted, plaintext);

    remove_key(&config, &provisioned.key_id).unwrap();
    assert!(!key_file.exists());
}

#[test]
fn full_pipeline_returning_player_via_game_id() {
    let dir = temp_dir("t2");
    let game_dir = dir.join("saturday-showdown");
    fs::create_dir_all(&game_dir).unwrap();
    let player_keys = Keys::generate();
    let player_hex = player_keys.public_key().to_hex();
    let entry = hosted_game_entry(
        &game_dir,
        "saturday-showdown",
        "Saturday Showdown",
        vec![
            pending_seat(1, "velvet-mountain"),
            claimed_seat(2, "copper-sunrise", &player_hex),
        ],
    );
    let gate_keys = Keys::generate();
    let event = signed_session_request_event(
        &player_keys,
        &gate_keys,
        "",
        Some("saturday-showdown"),
        "ssh-ed25519 AAAA222222222222222222222222222222222222222222222222222 test",
    );
    let req = parse_session_request(&event).unwrap();
    match route(&req, &[entry]) {
        RoutingDecision::Provisioned(seat) => {
            assert_eq!(seat.player, 2);
            assert!(!seat.first_claim);
        }
        other => panic!("expected Provisioned, got {other:?}"),
    }
}

#[test]
fn full_pipeline_invalid_invite_code_returns_error_payload() {
    let player_keys = Keys::generate();
    let gate_keys = Keys::generate();
    let event = signed_session_request_event(
        &player_keys,
        &gate_keys,
        "totally-wrong",
        None,
        "ssh-ed25519 AAAA333333333333333333333333333333333333333333333333333 test",
    );
    let req = parse_session_request(&event).unwrap();
    let decision = route(
        &req,
        &[HostedGameEntry {
            dir: PathBuf::from("/tmp/any-game"),
            game: HostedGame {
                game_id: "any-game".to_string(),
                game_name: "Any Game".to_string(),
                seats: vec![pending_seat(1, "velvet-mountain")],
            },
        }],
    );
    assert_eq!(decision, RoutingDecision::Error(RouteError::InvalidCode));

    let payload = session_error_payload(&RouteError::InvalidCode);
    let encrypted = nip44::encrypt(
        gate_keys.secret_key(),
        &player_keys.public_key(),
        &payload,
        Version::V2,
    )
    .unwrap();
    let decrypted = nip44::decrypt(
        player_keys.secret_key(),
        &gate_keys.public_key(),
        &encrypted,
    )
    .unwrap();
    assert_eq!(decrypted, payload);
}

#[test]
fn full_pipeline_multiple_games_returns_disambiguation_payload() {
    let player_keys = Keys::generate();
    let player_hex = player_keys.public_key().to_hex();
    let gate_keys = Keys::generate();
    let event = signed_session_request_event(
        &player_keys,
        &gate_keys,
        "",
        None,
        "ssh-ed25519 AAAA444444444444444444444444444444444444444444444444444 test",
    );
    let req = parse_session_request(&event).unwrap();
    let decision = route(
        &req,
        &[
            HostedGameEntry {
                dir: PathBuf::from("/tmp/game-a"),
                game: HostedGame {
                    game_id: "game-a".to_string(),
                    game_name: "Game A".to_string(),
                    seats: vec![claimed_seat(1, "velvet-mountain", &player_hex)],
                },
            },
            HostedGameEntry {
                dir: PathBuf::from("/tmp/game-b"),
                game: HostedGame {
                    game_id: "game-b".to_string(),
                    game_name: "Game B".to_string(),
                    seats: vec![claimed_seat(2, "copper-sunrise", &player_hex)],
                },
            },
        ],
    );
    let games = match decision {
        RoutingDecision::Error(RouteError::MultipleGames(games)) => games,
        other => panic!("expected MultipleGames, got {other:?}"),
    };
    assert_eq!(games.len(), 2);
    let payload = session_error_payload(&RouteError::MultipleGames(games));
    assert!(payload.contains(r#""error":"multiple_games""#));
}

#[test]
fn full_pipeline_game_definition_tags_four_seat_game() {
    let game = HostedGame {
        game_id: "friday-night".to_string(),
        game_name: "Friday Night EC".to_string(),
        seats: vec![
            claimed_seat(1, "velvet-mountain", "npub1aaa"),
            pending_seat(2, "copper-sunrise"),
            claimed_seat(3, "amber-cascade", "npub1bbb"),
            pending_seat(4, "silver-meadow"),
        ],
    };

    let tags = build_game_def_tags(&game, "play.example.com", 22).unwrap();
    let tag_vecs: Vec<Vec<String>> = tags.iter().map(|tag| tag.clone().to_vec()).collect();
    let d = tag_vecs.iter().find(|tag| tag[0] == "d").unwrap();
    assert_eq!(d[1], "friday-night");
    let players = tag_vecs.iter().find(|tag| tag[0] == "players").unwrap();
    assert_eq!(players[1], "4");
    let slots: Vec<_> = tag_vecs.iter().filter(|tag| tag[0] == "slot").collect();
    assert_eq!(slots.len(), 4);
    let slot1 = slots.iter().find(|slot| slot[1] == "1").unwrap();
    assert_eq!(slot1[2], sha256_hex(b"velvet-mountain"));
    assert_eq!(slot1[3], "npub1aaa");
    assert_eq!(slot1[4], "claimed");
    assert!(tag_vecs.iter().all(|tag| tag[0] != "invite-bech32"));
}

#[test]
fn full_pipeline_reaper_cleans_expired_while_leaving_live() {
    let dir = temp_dir("t6");
    let game_dir = dir.join("reaper-game");
    fs::create_dir_all(&game_dir).unwrap();
    let keys_dir = dir.join("keys");

    let seat_live = nc_gate::serve::routing::ResolvedSeat {
        game_id: "reaper-game".to_string(),
        game_name: "Reaper Game".to_string(),
        player: 1,
        player_npub: "npub1live".to_string(),
        first_claim: false,
    };
    let seat_expired = nc_gate::serve::routing::ResolvedSeat {
        game_id: "reaper-game".to_string(),
        game_name: "Reaper Game".to_string(),
        player: 2,
        player_npub: "npub1expired".to_string(),
        first_claim: false,
    };

    let ssh_a = "ssh-ed25519 AAAA555555555555555555555555555555555555555555555555555 live";
    let ssh_b = "ssh-ed25519 AAAA666666666666666666666666666666666666666666666666666 expired";

    let config_live = gate_config_command(keys_dir.clone());
    let key_live = provision_key(
        &config_live,
        &seat_live,
        ssh_a,
        &game_dir,
        "session-live",
        None,
    )
    .unwrap();

    let mut config_expired = gate_config_command(keys_dir.clone());
    config_expired.key_ttl = 0;
    let key_expired = provision_key(
        &config_expired,
        &seat_expired,
        ssh_b,
        &game_dir,
        "session-expired",
        None,
    )
    .unwrap();

    std::thread::sleep(std::time::Duration::from_secs(1));

    let removed = reap_expired_keys(&config_live).unwrap();
    assert!(removed >= 1);

    let expired_path = keys_dir.join(format!("{}.key", key_expired.key_id));
    assert!(!expired_path.exists());
    let live_path = keys_dir.join(format!("{}.key", key_live.key_id));
    assert!(live_path.exists());
}

#[test]
fn full_pipeline_same_identity_pending_retry_replaces_bootstrap_lease() {
    let dir = temp_dir("pending_retry");
    let game_dir = dir.join("retry-game");
    fs::create_dir_all(&game_dir).unwrap();
    let entry = hosted_game_entry(
        &game_dir,
        "retry-game",
        "Retry Game",
        vec![pending_seat(1, "velvet-mountain")],
    );

    let gate_keys = Keys::generate();
    let player_keys = Keys::generate();
    let player_hex = player_keys.public_key().to_hex();
    let ssh_pubkey = "ssh-ed25519 AAAA111111111111111111111111111111111111111111111111111 retry";
    let store = CampaignStore::open_default_in_dir(&game_dir).unwrap();

    let first_event = signed_session_request_event(
        &player_keys,
        &gate_keys,
        "velvet-mountain",
        None,
        ssh_pubkey,
    );
    let first_req = parse_session_request(&first_event).expect("first request should parse");
    let first_seat = match route(&first_req, &[entry.clone()]) {
        RoutingDecision::Provisioned(seat) => seat,
        other => panic!("expected Provisioned, got {other:?}"),
    };
    assert!(first_seat.first_claim);
    let first_lease = store
        .create_pending_session_lease("session-a", 1, &player_hex, 100, 60)
        .expect("first pending lease should succeed");
    assert_eq!(first_lease.session_token, "session-a");

    let second_event = signed_session_request_event(
        &player_keys,
        &gate_keys,
        "velvet-mountain",
        None,
        ssh_pubkey,
    );
    let second_req = parse_session_request(&second_event).expect("second request should parse");
    let second_seat = match route(&second_req, &[entry]) {
        RoutingDecision::Provisioned(seat) => seat,
        other => panic!("expected Provisioned, got {other:?}"),
    };
    assert!(second_seat.first_claim);
    let second_lease = store
        .create_pending_session_lease("session-b", 1, &player_hex, 110, 60)
        .expect("same-identity pending retry should succeed");
    assert_eq!(second_lease.session_token, "session-b");

    assert!(matches!(
        store.load_session_lease("session-a", 110),
        Err(nc_data::SessionLeaseError::InvalidToken)
    ));
    let live = store
        .load_session_lease("session-b", 110)
        .expect("replacement lease should load");
    assert_eq!(live.state, nc_data::SessionLeaseState::PendingSsh);
    assert_eq!(live.player_record_index_1_based, 1);
}

#[test]
fn full_pipeline_same_identity_active_retry_replaces_live_lease() {
    let dir = temp_dir("active_retry");
    let game_dir = dir.join("retry-game");
    fs::create_dir_all(&game_dir).unwrap();
    let gate_keys = Keys::generate();
    let player_keys = Keys::generate();
    let player_hex = player_keys.public_key().to_hex();
    let entry = hosted_game_entry(
        &game_dir,
        "retry-game",
        "Retry Game",
        vec![claimed_seat(1, "velvet-mountain", &player_hex)],
    );

    let ssh_pubkey = "ssh-ed25519 AAAA111111111111111111111111111111111111111111111111111 retry";
    let store = CampaignStore::open_default_in_dir(&game_dir).unwrap();

    let first_event =
        signed_session_request_event(&player_keys, &gate_keys, "", Some("retry-game"), ssh_pubkey);
    let first_req = parse_session_request(&first_event).expect("first request should parse");
    let first_seat = match route(&first_req, std::slice::from_ref(&entry)) {
        RoutingDecision::Provisioned(seat) => seat,
        other => panic!("expected Provisioned, got {other:?}"),
    };
    assert!(!first_seat.first_claim);
    let first_lease = store
        .create_pending_session_lease("session-a", 1, &player_hex, 100, 60)
        .expect("first pending lease should succeed");
    assert_eq!(first_lease.session_token, "session-a");
    let first_live = store
        .activate_session_lease("session-a", 101, 60)
        .expect("activate first lease");
    assert_eq!(first_live.state, nc_data::SessionLeaseState::Active);

    let second_event =
        signed_session_request_event(&player_keys, &gate_keys, "", Some("retry-game"), ssh_pubkey);
    let second_req = parse_session_request(&second_event).expect("second request should parse");
    let second_seat = match route(&second_req, &[entry]) {
        RoutingDecision::Provisioned(seat) => seat,
        other => panic!("expected Provisioned, got {other:?}"),
    };
    assert!(!second_seat.first_claim);
    let second_lease = store
        .create_pending_session_lease("session-b", 1, &player_hex, 110, 60)
        .expect("same-identity active retry should succeed");
    assert_eq!(second_lease.session_token, "session-b");
    assert_eq!(second_lease.state, nc_data::SessionLeaseState::PendingSsh);

    assert!(matches!(
        store.load_session_lease("session-a", 110),
        Err(nc_data::SessionLeaseError::InvalidToken)
    ));
    let live = store
        .load_session_lease("session-b", 110)
        .expect("replacement lease should load");
    assert_eq!(live.player_record_index_1_based, 1);
}

#[test]
fn full_pipeline_finds_live_identity_session_across_hosted_games() {
    let dir = temp_dir("identity_busy");
    let game_a_dir = dir.join("game-a");
    let game_b_dir = dir.join("game-b");
    fs::create_dir_all(&game_a_dir).unwrap();
    fs::create_dir_all(&game_b_dir).unwrap();

    let entry_a = hosted_game_entry(
        &game_a_dir,
        "game-a",
        "Game A",
        vec![claimed_seat(1, "alpha-beta", "npub1player000")],
    );
    let entry_b = hosted_game_entry(
        &game_b_dir,
        "game-b",
        "Game B",
        vec![claimed_seat(2, "gamma-delta", "npub1player000")],
    );

    let store_a = CampaignStore::open_default_in_dir(&game_a_dir).unwrap();
    store_a
        .create_pending_session_lease("session-a", 1, "npub1player000", 100, 60)
        .unwrap();

    let live =
        find_active_identity_session(&[entry_a.clone(), entry_b.clone()], "npub1player000", 100)
            .expect("scan active identity session")
            .expect("expected live identity session");
    assert_eq!(live.game_id, "game-a");
    assert_eq!(live.player_record_index_1_based, 1);

    assert!(
        find_active_identity_session(&[entry_a, entry_b], "npub1player000", 161)
            .expect("expired session should be pruned")
            .is_none()
    );
}

#[test]
fn full_pipeline_stale_event_is_rejected() {
    let player_keys = Keys::generate();
    let gate_keys = Keys::generate();
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
            Tag::parse([
                "ssh-pubkey",
                "ssh-ed25519 AAAA777777777777777777777777777777777777777777777777777 test",
            ])
            .unwrap(),
        ])
        .custom_created_at(old_ts)
        .sign_with_keys(&player_keys)
        .unwrap();

    let result = parse_session_request(&event);
    assert!(matches!(
        result,
        Err(nc_gate::serve::request::ParseError::Stale)
    ));
}
