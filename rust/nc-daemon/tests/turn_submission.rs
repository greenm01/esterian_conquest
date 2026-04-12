mod common;

use common::{create_test_game, hash_invite_code};
use nc_data::hosted::TurnSubmissionStatus;

#[test]
fn test_enqueue_turn() {
    let (_temp, _game_dir, store) = create_test_game("turn-test-1", 4);
    let game_id = "turn-test-1";

    let submit_id = "turn-submit-001";
    let player_pubkey = "player-npub-123";
    let commands = "fleet 1 { order speed=3 }";

    nc_data::hosted::enqueue_turn(
        store.connection(),
        submit_id,
        game_id,
        5,
        player_pubkey,
        commands,
    )
    .expect("should enqueue");

    let pending =
        nc_data::hosted::list_pending_turns(store.connection(), game_id, 5).expect("should list");
    assert_eq!(pending.len(), 1);

    let turn = &pending[0];
    assert_eq!(turn.id, submit_id);
    assert_eq!(turn.turn, 5);
    assert_eq!(turn.player_pubkey, player_pubkey);
    assert_eq!(turn.commands, commands);
    assert_eq!(turn.status, TurnSubmissionStatus::Pending);
}

#[test]
fn test_list_pending_turns_by_turn() {
    let (_temp, _game_dir, store) = create_test_game("turn-test-2", 4);
    let game_id = "turn-test-2";

    nc_data::hosted::enqueue_turn(store.connection(), "t1", game_id, 5, "p1", "cmd1")
        .expect("enqueue");
    nc_data::hosted::enqueue_turn(store.connection(), "t2", game_id, 5, "p2", "cmd2")
        .expect("enqueue");
    nc_data::hosted::enqueue_turn(store.connection(), "t3", game_id, 6, "p1", "cmd3")
        .expect("enqueue");

    let turn5 = nc_data::hosted::list_pending_turns(store.connection(), game_id, 5).expect("list");
    let turn6 = nc_data::hosted::list_pending_turns(store.connection(), game_id, 6).expect("list");

    assert_eq!(turn5.len(), 2);
    assert_eq!(turn6.len(), 1);
}

#[test]
fn test_reject_turn() {
    let (_temp, _game_dir, store) = create_test_game("turn-test-3", 4);
    let game_id = "turn-test-3";

    nc_data::hosted::enqueue_turn(
        store.connection(),
        "reject-turn",
        game_id,
        7,
        "player",
        "orders",
    )
    .expect("enqueue");

    nc_data::hosted::reject_turn(store.connection(), "reject-turn", "Invalid orders")
        .expect("reject");

    let pending =
        nc_data::hosted::list_pending_turns(store.connection(), game_id, 7).expect("list");
    assert_eq!(pending.len(), 0);

    let mut stmt = store
        .connection()
        .prepare("SELECT status, error_message FROM turn_queue WHERE id = 'reject-turn'")
        .expect("prepare");
    let (status, error): (String, String) = stmt
        .query_row([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("get");

    assert_eq!(status, "rejected");
    assert_eq!(error, "Invalid orders");
}

#[test]
fn test_accept_turn() {
    let (_temp, _game_dir, store) = create_test_game("turn-test-4", 4);
    let game_id = "turn-test-4";

    nc_data::hosted::enqueue_turn(
        store.connection(),
        "accept-turn",
        game_id,
        8,
        "player",
        "orders",
    )
    .expect("enqueue");

    nc_data::hosted::accept_turn(store.connection(), "accept-turn").expect("accept");

    let pending =
        nc_data::hosted::list_pending_turns(store.connection(), game_id, 8).expect("list");
    assert_eq!(pending.len(), 0);

    let mut stmt = store
        .connection()
        .prepare("SELECT status FROM turn_queue WHERE id = 'accept-turn'")
        .expect("prepare");
    let status: String = stmt.query_row([], |row| row.get(0)).expect("get");

    assert_eq!(status, "accepted");
}
