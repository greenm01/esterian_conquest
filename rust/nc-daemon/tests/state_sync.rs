mod common;

use common::create_test_game;

#[test]
fn test_state_sync_structs() {
    use nc_nostr::state_sync::{GameState, StateDelta, StateDeltas};

    let state = GameState {
        game_id: "test".to_string(),
        turn: 5,
        year: 3005,
        player_seat: 1,
        player_name: "Test Empire".to_string(),
        state_hash: "abc123".to_string(),
        state: serde_json::json!({"planets": []}),
        queued_mail: vec![],
        report_blocks: vec![],
    };

    assert_eq!(state.turn, 5);
    assert_eq!(state.year, 3005);

    let deltas = StateDeltas {
        planets: vec![],
        fleets: vec![],
        events: vec![],
    };

    let delta = StateDelta {
        game_id: "test".to_string(),
        turn: 6,
        base_hash: "abc123".to_string(),
        state_hash: "def456".to_string(),
        deltas,
    };

    assert_eq!(delta.turn, 6);
    assert_eq!(delta.base_hash, "abc123");
}

#[test]
fn test_game_settings_for_state() {
    let (_temp, _game_dir, store) = create_test_game("state-sync-test", 4);
    let game_id = "state-sync-test";

    let settings = nc_data::hosted::get_settings(store.connection(), game_id).expect("should get");

    assert!(settings.maintenance_enabled);
    assert_eq!(settings.maintenance_interval_minutes, 1440);
    assert_eq!(
        settings.lobby_visibility,
        nc_data::hosted::LobbyVisibility::Public
    );
}

#[test]
fn test_seat_lookup_for_player() {
    let (_temp, _game_dir, store) = create_test_game("state-sync-seat", 4);
    let game_id = "state-sync-seat";

    let player_pubkey = "test-player-npub-xyz";

    nc_data::hosted::open_seat(store.connection(), game_id, 1, "invite-123").expect("open");
    nc_data::hosted::claim_seat(store.connection(), game_id, 1, player_pubkey).expect("claim");

    let seat = nc_data::hosted::get_seat_by_pubkey(store.connection(), game_id, player_pubkey)
        .expect("should get")
        .expect("seat should exist");

    assert_eq!(seat.seat_number, 1);
    assert_eq!(seat.player_pubkey, Some(player_pubkey.to_string()));
}
