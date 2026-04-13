mod common;

use common::create_test_game;
use nc_host::lobby::threads::{enqueue_sysop_message, store_player_message};
use nc_nostr::pubkeys::hex_to_npub;
use nc_nostr::thread_message::{SenderRole, SysopThreadMessage};

#[test]
fn store_player_message_persists_private_thread_row() {
    let (_temp, _game_dir, store) = create_test_game("thread-test-1", 4);
    let player_pubkey = "8a937a446e7061f24f6b4b037c56c671146f50c8754472601527805a35cd4dc4";
    let payload = SysopThreadMessage {
        message_id: "thread-001".to_string(),
        game_id: "thread-test-1".to_string(),
        sender_role: SenderRole::Player,
        sender_pubkey: player_pubkey.to_string(),
        sender_npub: hex_to_npub(player_pubkey).expect("npub"),
        sender_handle: Some("StarRider".to_string()),
        body: "Can I take the replacement seat?".to_string(),
        created_at: 1_770_000_000,
    };

    store_player_message(&store, "thread-test-1", &payload).expect("store player message");

    let messages =
        nc_data::hosted::list_thread_messages(store.connection(), "thread-test-1", player_pubkey)
            .expect("list thread messages");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].sender_role, "player");
    assert_eq!(messages[0].sender_pubkey, player_pubkey);
    assert_eq!(messages[0].sender_handle.as_deref(), Some("StarRider"));
    assert_eq!(messages[0].body, "Can I take the replacement seat?");
}

#[test]
fn enqueue_sysop_message_stores_thread_and_outbox() {
    let (_temp, _game_dir, store) = create_test_game("thread-test-2", 4);
    let player_pubkey = "8a937a446e7061f24f6b4b037c56c671146f50c8754472601527805a35cd4dc4";
    let host_pubkey = "639ac889d29403689b9d92ece5a79efcaee992da1e423c0cc424e20b81ded48e";

    enqueue_sysop_message(
        &store,
        "thread-test-2",
        player_pubkey,
        host_pubkey,
        Some("nc-host"),
        "Seat 2 is open if you still want it.",
        "thread-002",
    )
    .expect("queue sysop thread message");

    let messages =
        nc_data::hosted::list_thread_messages(store.connection(), "thread-test-2", player_pubkey)
            .expect("list thread messages");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].sender_role, "sysop");
    assert_eq!(messages[0].sender_pubkey, host_pubkey);

    let pending =
        nc_data::hosted::get_pending(store.connection(), "thread-test-2", 10).expect("list outbox");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].kind, 30517);
    assert_eq!(pending[0].pubkey, player_pubkey);
}
