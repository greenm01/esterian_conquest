mod common;

use common::{create_test_game, hash_invite_code};
use nc_data::hosted::{InviteRequest, InviteRequestStatus};

#[test]
fn test_create_invite_request() {
    let (_temp, game_dir, store) = create_test_game("invite-test-1", 4);
    let game_id = "invite-test-1";

    let request_id = "req-001";
    let player_pubkey = "test-player-npub";

    nc_data::hosted::create_request(
        store.connection(),
        request_id,
        game_id,
        player_pubkey,
        "Please let me join!",
    )
    .expect("request should be created");

    let requests =
        nc_data::hosted::list_requests(store.connection(), game_id).expect("should list");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].id, "req-001");
    assert_eq!(requests[0].player_pubkey, "test-player-npub");
    assert_eq!(requests[0].message, "Please let me join!");
    assert_eq!(requests[0].status, InviteRequestStatus::Pending);
}

#[test]
fn test_approve_request() {
    let (_temp, game_dir, store) = create_test_game("invite-test-2", 4);
    let game_id = "invite-test-2";

    nc_data::hosted::create_request(
        store.connection(),
        "req-002",
        game_id,
        "player-npub",
        "Want to join",
    )
    .expect("request should be created");

    nc_data::hosted::approve_request(
        store.connection(),
        "req-002",
        "Approved for seat 1",
        "invite-code-abc",
    )
    .expect("should approve");

    let req = nc_data::hosted::get_request(store.connection(), "req-002")
        .expect("should get")
        .expect("request should exist");

    assert_eq!(req.status, InviteRequestStatus::Approved);
    assert_eq!(
        req.decision_message,
        Some("Approved for seat 1".to_string())
    );
    assert_eq!(req.issued_invite_code, Some("invite-code-abc".to_string()));
    assert!(req.processed_at.is_some());
}

#[test]
fn test_reject_request() {
    let (_temp, game_dir, store) = create_test_game("invite-test-3", 4);
    let game_id = "invite-test-3";

    nc_data::hosted::create_request(
        store.connection(),
        "req-003",
        game_id,
        "player-npub",
        "Want to join",
    )
    .expect("request should be created");

    nc_data::hosted::reject_request(store.connection(), "req-003", "Game is full")
        .expect("should reject");

    let req = nc_data::hosted::get_request(store.connection(), "req-003")
        .expect("should get")
        .expect("request should exist");

    assert_eq!(req.status, InviteRequestStatus::Rejected);
    assert_eq!(req.decision_message, Some("Game is full".to_string()));
}

#[test]
fn test_list_pending_decisions() {
    let (_temp, game_dir, store) = create_test_game("invite-test-4", 4);
    let game_id = "invite-test-4";

    nc_data::hosted::create_request(store.connection(), "req-004", game_id, "player1", "Hi")
        .expect("create");
    nc_data::hosted::create_request(store.connection(), "req-005", game_id, "player2", "Hi")
        .expect("create");
    nc_data::hosted::create_request(store.connection(), "req-006", game_id, "player3", "Hi")
        .expect("create");

    nc_data::hosted::approve_request(store.connection(), "req-004", "Approved", "code1")
        .expect("approve");
    nc_data::hosted::reject_request(store.connection(), "req-005", "Rejected").expect("reject");

    let pending =
        nc_data::hosted::list_pending_decisions(store.connection(), game_id).expect("should list");
    assert_eq!(pending.len(), 2);

    assert!(pending.iter().any(|r| r.id == "req-004"));
    assert!(pending.iter().any(|r| r.id == "req-005"));
    assert!(!pending.iter().any(|r| r.id == "req-006"));
}

#[test]
fn test_mark_decision_published() {
    let (_temp, game_dir, store) = create_test_game("invite-test-5", 4);
    let game_id = "invite-test-5";

    nc_data::hosted::create_request(store.connection(), "req-007", game_id, "player", "Hi")
        .expect("create");
    nc_data::hosted::approve_request(store.connection(), "req-007", "Approved", "code")
        .expect("approve");

    let pending_before =
        nc_data::hosted::list_pending_decisions(store.connection(), game_id).expect("should list");
    assert_eq!(pending_before.len(), 1);

    nc_data::hosted::mark_decision_published(store.connection(), "req-007").expect("should mark");

    let pending_after =
        nc_data::hosted::list_pending_decisions(store.connection(), game_id).expect("should list");
    assert_eq!(pending_after.len(), 0);

    let req = nc_data::hosted::get_request(store.connection(), "req-007")
        .expect("should get")
        .expect("should exist");
    assert!(req.decision_published_at.is_some());
}
