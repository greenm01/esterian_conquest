use ec_connect::cache::{CachedGame, GameCache};
use ec_connect::connect::handshake::SessionReadyPayload;
use ec_connect::connect::resolve::ResolvedTarget;
use ec_connect::connect::session::{
    build_cached_game_from_ready_payload, build_pending_cached_game_from_ready_payload,
    format_bridge_error_message, hosted_onboarding_invariant_message,
    is_unfinished_first_join_error, merge_session_state, resolve_gate_npub,
    unfinished_first_join_error_message,
};
use ec_connect::connect::session_state::SessionStatePayload;

#[test]
fn missing_gate_lookup_uses_picker_first_message() {
    let err = resolve_gate_npub("play.example.com", &GameCache::empty(), None)
        .expect_err("missing gate should return an error");

    assert!(err.contains("joined game list"));
    assert!(err.contains("invite code"));
    assert!(err.contains("picker"));
    assert!(!err.contains("--gate"));
}

#[test]
fn localhost_ssh_auth_failure_uses_local_helper_hint() {
    let err = format_bridge_error_message("localhost", "SSH public-key authentication failed");

    assert!(err.contains("local game server"));
    assert!(err.contains("localhost testing"));
    assert!(err.contains("wrong SSH user or auth-keys path"));
}

#[test]
fn remote_bridge_error_keeps_generic_disconnect_message() {
    let err =
        format_bridge_error_message("play.example.com", "SSH public-key authentication failed");

    assert!(err.contains("Connection to game server was lost."));
    assert!(err.contains("Contact your sysop"));
    assert!(!err.contains("localhost testing"));
}

fn local_target() -> ResolvedTarget {
    ResolvedTarget {
        server_host: "localhost".to_string(),
        server_port: 22,
        relay_url: "ws://localhost:8080".to_string(),
        invite_code: Some("victim-sickness".to_string()),
        game_id: None,
        gate_npub: None,
    }
}

fn ready_payload() -> SessionReadyPayload {
    SessionReadyPayload {
        game_id: "stress-campaign".to_string(),
        ssh_host: "localhost".to_string(),
        ssh_port: 22,
        ssh_user: "niltempus".to_string(),
        host_fingerprint: "SHA256:test".to_string(),
        game_name: "Stress Campaign".to_string(),
        seat: 2,
        player_name: "Empire Prime".to_string(),
    }
}

#[test]
fn ready_payload_can_build_cache_row_for_claimed_reconnects() {
    let entry = build_cached_game_from_ready_payload(
        &ready_payload(),
        &local_target(),
        "npub1player",
        "npub1gate",
        "2026-03-30T20:00:00Z",
    );

    assert_eq!(entry.id, "stress-campaign");
    assert_eq!(entry.name, "Stress Campaign");
    assert_eq!(entry.player_name.as_deref(), Some("Empire Prime"));
    assert_eq!(entry.server, "localhost");
    assert_eq!(entry.port, 22);
    assert_eq!(entry.relay_url.as_deref(), Some("ws://localhost:8080"));
    assert_eq!(entry.seat, 2);
    assert_eq!(entry.npub, "npub1player");
    assert_eq!(entry.gate_npub, "npub1gate");
    assert_eq!(entry.joined, "2026-03-30T20:00:00Z");
    assert!(entry.last_connected.is_none());
}

#[test]
fn ready_payload_can_build_pending_cache_row_for_incomplete_first_join() {
    let entry = build_pending_cached_game_from_ready_payload(
        &ready_payload(),
        &local_target(),
        "npub1player",
        "npub1gate",
        "2026-03-30T20:00:00Z",
    );

    assert_eq!(entry.status, ec_connect::cache::CachedGameStatus::Pending);
    assert_eq!(entry.invite_code.as_deref(), Some("victim-sickness"));
}

#[test]
fn unfinished_first_join_error_is_stable_for_stale_row_recovery() {
    let err = unfinished_first_join_error_message();

    assert!(err.contains("not enrolled"));
    assert!(err.contains("left before naming your empire"));
    assert!(err.contains("invite code again"));
    assert!(is_unfinished_first_join_error(err));
    assert!(!is_unfinished_first_join_error(
        "unknown_player: player not found"
    ));
}

#[test]
fn hosted_onboarding_invariant_message_is_clean_single_paragraph() {
    let err = hosted_onboarding_invariant_message();

    assert!(err.contains("Hosted join failed before empire naming."));
    assert!(err.contains("wrong first-time screen"));
    assert!(err.contains("Retry the invite."));
    assert!(!err.contains('\n'));
}

#[test]
fn merge_session_state_updates_existing_row_without_resetting_joined() {
    let mut cache = GameCache {
        games: vec![build_cached_game_from_ready_payload(
            &ready_payload(),
            &local_target(),
            "npub1player",
            "npub1gate",
            "2026-03-30T20:00:00Z",
        )],
    };
    cache.games[0].last_connected = Some("2026-03-30T20:05:00Z".to_string());

    merge_session_state(
        &mut cache,
        &SessionStatePayload {
            game_id: "stress-campaign".to_string(),
            game_name: "Renamed Campaign".to_string(),
            seat: 5,
            player_name: "Renamed Empire".to_string(),
        },
        &local_target(),
        "npub1player",
        "npub1gate",
        "2026-03-30T20:10:00Z",
    );

    let game = &cache.games[0];
    assert_eq!(game.name, "Renamed Campaign");
    assert_eq!(game.player_name.as_deref(), Some("Renamed Empire"));
    assert_eq!(game.seat, 5);
    assert_eq!(game.joined, "2026-03-30T20:00:00Z");
    assert_eq!(game.last_connected.as_deref(), Some("2026-03-30T20:05:00Z"));
}

#[test]
fn merge_session_state_rehydrates_missing_cache_row() {
    let mut cache = GameCache::empty();

    merge_session_state(
        &mut cache,
        &SessionStatePayload {
            game_id: "stress-campaign".to_string(),
            game_name: "Recovered Campaign".to_string(),
            seat: 2,
            player_name: "Recovered Empire".to_string(),
        },
        &local_target(),
        "npub1player",
        "npub1gate",
        "2026-03-30T20:10:00Z",
    );

    assert_eq!(cache.games.len(), 1);
    let game: &CachedGame = &cache.games[0];
    assert_eq!(game.id, "stress-campaign");
    assert_eq!(game.name, "Recovered Campaign");
    assert_eq!(game.player_name.as_deref(), Some("Recovered Empire"));
    assert_eq!(game.joined, "2026-03-30T20:10:00Z");
    assert!(game.last_connected.is_none());
}
