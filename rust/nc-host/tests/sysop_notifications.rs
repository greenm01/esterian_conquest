mod common;

use common::create_test_game;
use nc_host::lobby::notify_sysop::{
    enqueue_invite_request_summary, enqueue_thread_message_summary,
};

#[test]
fn invite_request_summary_is_queued_once() {
    let (_temp, _game_dir, store) = create_test_game("notify-test-1", 4);
    let player_pubkey = "8a937a446e7061f24f6b4b037c56c671146f50c8754472601527805a35cd4dc4";

    enqueue_invite_request_summary(
        &store,
        "notify-test-1",
        "req-001",
        player_pubkey,
        Some("StarRider"),
    )
    .expect("queue summary");
    enqueue_invite_request_summary(
        &store,
        "notify-test-1",
        "req-001",
        player_pubkey,
        Some("StarRider"),
    )
    .expect("queue duplicate summary");

    let pending =
        nc_data::hosted::get_pending_sysop_notifications(store.connection(), "notify-test-1", 10)
            .expect("list pending notifications");

    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].category, "invite_request");
    assert!(pending[0].summary.contains("StarRider"));
    assert!(!pending[0].summary.contains("please let me join"));
}

#[test]
fn thread_message_summary_uses_game_name_not_body() {
    let (_temp, _game_dir, store) = create_test_game("notify-test-2", 4);
    let player_pubkey = "8a937a446e7061f24f6b4b037c56c671146f50c8754472601527805a35cd4dc4";

    enqueue_thread_message_summary(
        &store,
        "notify-test-2",
        "thread-001",
        player_pubkey,
        Some("StarRider"),
    )
    .expect("queue thread summary");

    let pending =
        nc_data::hosted::get_pending_sysop_notifications(store.connection(), "notify-test-2", 10)
            .expect("list pending notifications");

    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].category, "thread_message");
    assert!(pending[0].summary.contains("StarRider"));
    assert!(pending[0].summary.contains("notify-test-2"));
    assert!(!pending[0].summary.contains("Seat 2 is open"));
}
