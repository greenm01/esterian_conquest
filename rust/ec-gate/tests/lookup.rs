//! Tests for roster lookup functions.

use ec_gate::roster::{find_seat_by_code, find_seats_by_npub, Roster, Seat, SeatStatus};

fn make_roster(id: &str, seats: Vec<Seat>) -> Roster {
    Roster {
        id: id.to_string(),
        name: format!("Game {id}"),
        seats,
    }
}

fn pending(player: usize, code: &str) -> Seat {
    Seat {
        player,
        code: code.to_string(),
        status: SeatStatus::Pending,
        npub: None,
    }
}

fn claimed(player: usize, code: &str, npub: &str) -> Seat {
    Seat {
        player,
        code: code.to_string(),
        status: SeatStatus::Claimed,
        npub: Some(npub.to_string()),
    }
}

// --- find_seat_by_code ---

#[test]
fn find_seat_by_code_exact_match() {
    let rosters = vec![make_roster(
        "game1",
        vec![pending(1, "velvet-mountain"), pending(2, "copper-sunrise")],
    )];
    let result = find_seat_by_code(&rosters, "velvet-mountain");
    assert!(result.is_some());
    let (roster, seat) = result.unwrap();
    assert_eq!(roster.id, "game1");
    assert_eq!(seat.player, 1);
}

#[test]
fn find_seat_by_code_case_insensitive() {
    let rosters = vec![make_roster("game1", vec![pending(1, "velvet-mountain")])];
    assert!(find_seat_by_code(&rosters, "Velvet-Mountain").is_some());
    assert!(find_seat_by_code(&rosters, "VELVET-MOUNTAIN").is_some());
}

#[test]
fn find_seat_by_code_strips_relay_suffix() {
    let rosters = vec![make_roster("game1", vec![pending(1, "velvet-mountain")])];
    let result = find_seat_by_code(&rosters, "velvet-mountain@play.example.com");
    assert!(result.is_some());
}

#[test]
fn find_seat_by_code_strips_relay_with_port() {
    let rosters = vec![make_roster("game1", vec![pending(1, "velvet-mountain")])];
    let result = find_seat_by_code(&rosters, "velvet-mountain@play.example.com:4848");
    assert!(result.is_some());
}

#[test]
fn find_seat_by_code_trims_whitespace() {
    let rosters = vec![make_roster("game1", vec![pending(1, "velvet-mountain")])];
    assert!(find_seat_by_code(&rosters, "  velvet-mountain  ").is_some());
}

#[test]
fn find_seat_by_code_no_match_returns_none() {
    let rosters = vec![make_roster("game1", vec![pending(1, "velvet-mountain")])];
    assert!(find_seat_by_code(&rosters, "unknown-code").is_none());
}

#[test]
fn find_seat_by_code_searches_across_multiple_rosters() {
    let rosters = vec![
        make_roster("game1", vec![pending(1, "velvet-mountain")]),
        make_roster("game2", vec![pending(1, "copper-sunrise")]),
    ];
    let result = find_seat_by_code(&rosters, "copper-sunrise");
    assert!(result.is_some());
    assert_eq!(result.unwrap().0.id, "game2");
}

#[test]
fn find_seat_by_code_returns_first_match_on_collision() {
    // Collision should never happen in practice; this test documents behavior.
    let rosters = vec![
        make_roster("game1", vec![pending(1, "abbey-zoom")]),
        make_roster("game2", vec![pending(1, "abbey-zoom")]),
    ];
    let result = find_seat_by_code(&rosters, "abbey-zoom");
    assert_eq!(result.unwrap().0.id, "game1");
}

// --- find_seats_by_npub ---

#[test]
fn find_seats_by_npub_single_game() {
    let rosters = vec![make_roster(
        "game1",
        vec![
            claimed(1, "velvet-mountain", "npub1alice"),
            claimed(2, "copper-sunrise", "npub1bob"),
        ],
    )];
    let results = find_seats_by_npub(&rosters, "npub1alice");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].1.player, 1);
}

#[test]
fn find_seats_by_npub_multi_game() {
    // Player is in two games.
    let rosters = vec![
        make_roster("game1", vec![claimed(2, "velvet-mountain", "npub1shared")]),
        make_roster("game2", vec![claimed(3, "copper-sunrise", "npub1shared")]),
    ];
    let results = find_seats_by_npub(&rosters, "npub1shared");
    assert_eq!(results.len(), 2);
    let ids: Vec<&str> = results.iter().map(|(r, _)| r.id.as_str()).collect();
    assert!(ids.contains(&"game1"));
    assert!(ids.contains(&"game2"));
}

#[test]
fn find_seats_by_npub_unknown_returns_empty() {
    let rosters = vec![make_roster(
        "game1",
        vec![claimed(1, "velvet-mountain", "npub1alice")],
    )];
    let results = find_seats_by_npub(&rosters, "npub1nobody");
    assert!(results.is_empty());
}

#[test]
fn find_seats_by_npub_pending_seats_not_included() {
    let rosters = vec![make_roster("game1", vec![pending(1, "velvet-mountain")])];
    // Pending seats have no npub, so nothing should match.
    let results = find_seats_by_npub(&rosters, "npub1alice");
    assert!(results.is_empty());
}

#[test]
fn find_seats_by_npub_empty_rosters() {
    let rosters: Vec<Roster> = vec![];
    assert!(find_seats_by_npub(&rosters, "npub1anyone").is_empty());
}
