mod common;

use common::create_test_game;
use nc_host::lobby::threads::{enqueue_sysop_message, store_player_message};
use nc_nostr::thread_message::{SenderRole, SysopThreadMessage};

#[test]
fn store_player_message_persists_private_thread_row() {
    let (_temp, _game_dir, store) = create_test_game("thread-test-1", 4);
    let payload = SysopThreadMessage {
        message_id: "thread-001".to_string(),
        game_id: "thread-test-1".to_string(),
        sender_role: SenderRole::Player,
        sender_npub: "npub1playerexample".to_string(),
        sender_handle: Some("StarRider".to_string()),
        body: "Can I take the replacement seat?".to_string(),
        created_at: 1_770_000_000,
    };

    store_player_message(&store, "thread-test-1", &payload).expect("store player message");

    let messages = nc_data::hosted::list_thread_messages(
        store.connection(),
        "thread-test-1",
        "npub1playerexample",
    )
    .expect("list thread messages");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].sender_role, "player");
    assert_eq!(messages[0].sender_handle.as_deref(), Some("StarRider"));
    assert_eq!(messages[0].body, "Can I take the replacement seat?");
}

#[test]
fn enqueue_sysop_message_stores_thread_and_outbox() {
    let (_temp, _game_dir, store) = create_test_game("thread-test-2", 4);

    enqueue_sysop_message(
        &store,
        "thread-test-2",
        "npub1playerexample",
        "npub1hostexample",
        Some("nc-host"),
        "Seat 2 is open if you still want it.",
        "thread-002",
    )
    .expect("queue sysop thread message");

    let messages = nc_data::hosted::list_thread_messages(
        store.connection(),
        "thread-test-2",
        "npub1playerexample",
    )
    .expect("list thread messages");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].sender_role, "sysop");

    let pending = nc_data::hosted::get_pending(store.connection(), "thread-test-2", 10)
        .expect("list outbox");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].kind, 30517);
    assert_eq!(pending[0].pubkey, "npub1playerexample");
}
