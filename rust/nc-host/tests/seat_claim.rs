mod common;

use common::{create_test_game, hash_invite_code};
use nc_data::hosted::SeatStatus;

#[test]
fn test_create_seats() {
    let (_temp, _game_dir, store) = create_test_game("seat-claim-test-1", 4);
    let game_id = "seat-claim-test-1";

    let seats = nc_data::hosted::list_seats(store.connection(), game_id).expect("should list");
    assert_eq!(seats.len(), 4);

    for i in 1..=4 {
        assert!(seats.iter().any(|s| s.seat_number == i));
        assert!(
            seats
                .iter()
                .any(|s| s.seat_number == i && s.status == SeatStatus::Pending)
        );
        let seat = seats
            .iter()
            .find(|s| s.seat_number == i)
            .expect("seat should exist");
        assert!(seat.invite_code.contains('-'));
        assert!(!seat.invite_code.contains('@'));
    }
}

#[test]
fn test_find_seat_by_invite_hash() {
    let (_temp, _game_dir, store) = create_test_game("seat-claim-test-2", 4);
    let game_id = "seat-claim-test-2";

    let invite_code = "test-invite-code-123";
    let hash = hash_invite_code(invite_code);

    nc_data::hosted::open_seat(store.connection(), game_id, 1, invite_code)
        .expect("should open seat");

    let seat = nc_data::hosted::find_seat_by_invite_hash(store.connection(), game_id, &hash)
        .expect("should find")
        .expect("seat should exist");

    assert_eq!(seat.seat_number, 1);
    assert_eq!(seat.status, SeatStatus::Pending);
}

#[test]
fn test_claim_seat() {
    let (_temp, _game_dir, store) = create_test_game("seat-claim-test-3", 4);
    let game_id = "seat-claim-test-3";

    let invite_code = "claim-test-code";
    let player_pubkey = "test-player-npub-123";

    nc_data::hosted::open_seat(store.connection(), game_id, 2, invite_code)
        .expect("should open seat");

    let hash = hash_invite_code(invite_code);
    let seat_before = nc_data::hosted::find_seat_by_invite_hash(store.connection(), game_id, &hash)
        .expect("should find")
        .expect("seat should exist");
    assert_eq!(seat_before.status, SeatStatus::Pending);

    nc_data::hosted::claim_seat(store.connection(), game_id, 2, player_pubkey, 3000)
        .expect("should claim");

    let seat_after = nc_data::hosted::get_seat_by_number(store.connection(), game_id, 2)
        .expect("should get")
        .expect("seat should exist");

    assert_eq!(seat_after.status, SeatStatus::Claimed);
    assert_eq!(seat_after.player_pubkey, Some(player_pubkey.to_string()));
    assert!(seat_after.claimed_at.is_some());
    assert_eq!(seat_after.claimed_year, Some(3000));
}

#[test]
fn test_claim_already_claimed_seat_no_op() {
    let (_temp, _game_dir, store) = create_test_game("seat-claim-test-4", 4);
    let game_id = "seat-claim-test-4";

    let invite_code = "already-claimed-code";
    let player1 = "player-1";
    let player2 = "player-2";

    nc_data::hosted::open_seat(store.connection(), game_id, 3, invite_code).expect("should open");
    nc_data::hosted::claim_seat(store.connection(), game_id, 3, player1, 3000).expect("claim 1");

    nc_data::hosted::claim_seat(store.connection(), game_id, 3, player2, 3000)
        .expect("claim 2 should succeed (no error)");

    let seat = nc_data::hosted::get_seat_by_number(store.connection(), game_id, 3)
        .expect("get")
        .expect("seat should exist");

    assert_eq!(
        seat.player_pubkey,
        Some(player1.to_string()),
        "original player still owns seat"
    );
}

#[test]
fn test_reset_seat() {
    let (_temp, _game_dir, store) = create_test_game("seat-claim-test-5", 4);
    let game_id = "seat-claim-test-5";

    let invite_code = "reset-test-code";

    nc_data::hosted::open_seat(store.connection(), game_id, 4, invite_code).expect("should open");
    nc_data::hosted::claim_seat(store.connection(), game_id, 4, "player-1", 3000).expect("claim");

    nc_data::hosted::reset_seat(store.connection(), game_id, 4).expect("should reset");

    let seat = nc_data::hosted::get_seat_by_number(store.connection(), game_id, 4)
        .expect("should get")
        .expect("seat should exist");

    assert_eq!(seat.status, SeatStatus::Pending);
    assert_eq!(seat.player_pubkey, None);
    assert_eq!(seat.claimed_at, None);
    assert_eq!(seat.claimed_year, None);
}
