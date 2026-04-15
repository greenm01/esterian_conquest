mod common;

use common::create_test_game;
use nc_data::hosted::{InviteRequestStatus, SandboxApprovalOutcome};

#[test]
fn test_create_invite_request() {
    let (_temp, _game_dir, store) = create_test_game("invite-test-1", 4);
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
    let (_temp, _game_dir, store) = create_test_game("invite-test-2", 4);
    let game_id = "invite-test-2";

    nc_data::hosted::create_request(
        store.connection(),
        "req-002",
        game_id,
        "player-npub",
        "Want to join",
    )
    .expect("request should be created");

    nc_data::hosted::approve_request_for_seat(
        store.connection(),
        "req-002",
        game_id,
        2,
        "player-npub",
        3000,
        "amber-river",
        "Approved for seat 1",
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
    assert_eq!(req.assigned_seat, Some(2));
    assert_eq!(req.issued_invite_code, None);
    assert!(req.processed_at.is_some());

    let seat = nc_data::hosted::get_seat_by_number(store.connection(), game_id, 2)
        .expect("should get seat")
        .expect("seat should exist");
    assert!(!seat.invite_code.is_empty());
    assert_eq!(seat.player_pubkey.as_deref(), Some("player-npub"));
    assert_eq!(seat.claimed_year, Some(3000));
}

#[test]
fn test_approve_request_opens_missing_seat_and_claims_it() {
    let (_temp, _game_dir, store) = create_test_game("invite-test-2b", 4);
    let game_id = "invite-test-2b";

    nc_data::hosted::close_seat(store.connection(), game_id, 4).expect("seat should close");
    nc_data::hosted::create_request(
        store.connection(),
        "req-002b",
        game_id,
        "player-b-npub",
        "Want the reopened seat",
    )
    .expect("request should be created");

    nc_data::hosted::approve_request_for_seat(
        store.connection(),
        "req-002b",
        game_id,
        4,
        "player-b-npub",
        3000,
        "reserve-token",
        "Approved for seat 4",
    )
    .expect("should approve");

    let req = nc_data::hosted::get_request(store.connection(), "req-002b")
        .expect("should get")
        .expect("request should exist");
    assert_eq!(req.assigned_seat, Some(4));
    assert_eq!(req.issued_invite_code, None);

    let seat = nc_data::hosted::get_seat_by_number(store.connection(), game_id, 4)
        .expect("should get seat")
        .expect("seat should exist");
    assert_eq!(seat.invite_code, "reserve-token");
    assert_eq!(seat.player_pubkey.as_deref(), Some("player-b-npub"));
}

#[test]
fn test_reject_request() {
    let (_temp, _game_dir, store) = create_test_game("invite-test-3", 4);
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
    let (_temp, _game_dir, store) = create_test_game("invite-test-4", 4);
    let game_id = "invite-test-4";

    nc_data::hosted::create_request(store.connection(), "req-004", game_id, "player1", "Hi")
        .expect("create");
    nc_data::hosted::create_request(store.connection(), "req-005", game_id, "player2", "Hi")
        .expect("create");
    nc_data::hosted::create_request(store.connection(), "req-006", game_id, "player3", "Hi")
        .expect("create");

    nc_data::hosted::approve_request(store.connection(), "req-004", "Approved", 1, None)
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
    let (_temp, _game_dir, store) = create_test_game("invite-test-5", 4);
    let game_id = "invite-test-5";

    nc_data::hosted::create_request(store.connection(), "req-007", game_id, "player", "Hi")
        .expect("create");
    nc_data::hosted::approve_request(store.connection(), "req-007", "Approved", 1, None)
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

#[test]
fn test_auto_approve_sandbox_request_claims_first_open_seat() {
    let (_temp, _game_dir, store) = create_test_game("sandbox-invite-1", 4);
    let game_id = "sandbox-invite-1";

    nc_data::hosted::create_request(
        store.connection(),
        "req-sandbox-1",
        game_id,
        "sandbox-player",
        "",
    )
    .expect("request should be created");

    let outcome = nc_data::hosted::auto_approve_sandbox_request(
        store.connection(),
        "req-sandbox-1",
        game_id,
        "sandbox-player",
        3000,
        "Auto-approved for sandbox game.",
    )
    .expect("sandbox auto approve should succeed");

    assert_eq!(outcome, SandboxApprovalOutcome::Claimed { seat: 1 });

    let seat = nc_data::hosted::get_seat_by_number(store.connection(), game_id, 1)
        .expect("get seat")
        .expect("seat should exist");
    assert_eq!(seat.player_pubkey.as_deref(), Some("sandbox-player"));
    assert_eq!(seat.claimed_year, Some(3000));
}

#[test]
fn test_auto_approve_sandbox_request_returns_existing_claimed_seat() {
    let (_temp, _game_dir, store) = create_test_game("sandbox-invite-2", 4);
    let game_id = "sandbox-invite-2";

    nc_data::hosted::claim_seat(store.connection(), game_id, 3, "sandbox-player", 3000)
        .expect("claim seat");
    nc_data::hosted::create_request(
        store.connection(),
        "req-sandbox-2",
        game_id,
        "sandbox-player",
        "",
    )
    .expect("request should be created");

    let outcome = nc_data::hosted::auto_approve_sandbox_request(
        store.connection(),
        "req-sandbox-2",
        game_id,
        "sandbox-player",
        3001,
        "Auto-approved for sandbox game.",
    )
    .expect("sandbox auto approve should succeed");

    assert_eq!(outcome, SandboxApprovalOutcome::Claimed { seat: 3 });

    let seat = nc_data::hosted::get_seat_by_number(store.connection(), game_id, 3)
        .expect("get seat")
        .expect("seat should exist");
    assert_eq!(seat.player_pubkey.as_deref(), Some("sandbox-player"));
    assert_eq!(seat.claimed_year, Some(3000));
}

#[test]
fn test_auto_approve_sandbox_request_reports_full_when_no_pending_seats() {
    let (_temp, _game_dir, store) = create_test_game("sandbox-invite-3", 2);
    let game_id = "sandbox-invite-3";

    nc_data::hosted::claim_seat(store.connection(), game_id, 1, "player-1", 3000)
        .expect("claim seat 1");
    nc_data::hosted::claim_seat(store.connection(), game_id, 2, "player-2", 3000)
        .expect("claim seat 2");
    nc_data::hosted::create_request(
        store.connection(),
        "req-sandbox-3",
        game_id,
        "sandbox-player",
        "",
    )
    .expect("request should be created");

    let outcome = nc_data::hosted::auto_approve_sandbox_request(
        store.connection(),
        "req-sandbox-3",
        game_id,
        "sandbox-player",
        3001,
        "Auto-approved for sandbox game.",
    )
    .expect("sandbox auto approve should return full");

    assert_eq!(outcome, SandboxApprovalOutcome::Full);

    let req = nc_data::hosted::get_request(store.connection(), "req-sandbox-3")
        .expect("get request")
        .expect("request should still exist until caller clears it");
    assert_eq!(req.status, InviteRequestStatus::Pending);
}
