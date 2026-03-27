//! Regression tests for session routing (step 6).

use std::path::Path;

use ec_gate::roster::{Roster, Seat, SeatStatus};
use ec_gate::serve::request::SessionRequest;
use ec_gate::serve::routing::{GameEntry, ResolvedSeat, RouteError, RoutingDecision, route};

// --- test fixtures ---

const PLAYER_A: &str = "npub1aaa0000000000000000000000000000000000000000000000000000000000";
const PLAYER_B: &str = "npub1bbb0000000000000000000000000000000000000000000000000000000000";
const SSH_KEY: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIBGk6test";

fn make_request(
    invite_code: Option<&str>,
    game_id: Option<&str>,
    player_pubkey: &str,
) -> SessionRequest {
    SessionRequest {
        nonce: "test-nonce".to_string(),
        player_pubkey: player_pubkey.to_string(),
        ssh_pubkey: SSH_KEY.to_string(),
        invite_code: invite_code.map(str::to_string),
        game_id: game_id.map(str::to_string),
    }
}

fn pending_seat(player: usize, code: &str) -> Seat {
    Seat {
        player,
        code: code.to_string(),
        status: SeatStatus::Pending,
        npub: None,
    }
}

fn claimed_seat(player: usize, code: &str, npub: &str) -> Seat {
    Seat {
        player,
        code: code.to_string(),
        status: SeatStatus::Claimed,
        npub: Some(npub.to_string()),
    }
}

fn make_roster(id: &str, name: &str, seats: Vec<Seat>) -> Roster {
    Roster {
        id: id.to_string(),
        name: name.to_string(),
        seats,
    }
}

// --- invite code path ---

#[test]
fn route_by_code_claims_pending_seat() {
    let mut rosters = vec![make_roster(
        "friday-night",
        "Friday Night EC",
        vec![
            pending_seat(1, "velvet-azure"),
            pending_seat(2, "abbey-zoom"),
        ],
    )];
    let req = make_request(Some("velvet-azure"), None, PLAYER_A);

    match route(&req, &mut rosters, &[Path::new("/tmp")]) {
        RoutingDecision::Provisioned(seat) => {
            assert_eq!(seat.game_id, "friday-night");
            assert_eq!(seat.player, 1);
            assert_eq!(seat.player_npub, PLAYER_A);
        }
        other => panic!("expected Provisioned, got {other:?}"),
    }

    // Seat should now be claimed in memory.
    assert_eq!(rosters[0].seats[0].status, SeatStatus::Claimed);
    assert_eq!(rosters[0].seats[0].npub.as_deref(), Some(PLAYER_A));
}

#[test]
fn route_by_code_case_insensitive() {
    let mut rosters = vec![make_roster(
        "game1",
        "Game 1",
        vec![pending_seat(1, "velvet-azure")],
    )];
    let req = make_request(Some("VELVET-AZURE"), None, PLAYER_A);

    assert!(matches!(
        route(&req, &mut rosters, &[Path::new("/tmp")]),
        RoutingDecision::Provisioned(_)
    ));
}

#[test]
fn route_by_code_strips_relay_suffix() {
    let mut rosters = vec![make_roster(
        "game1",
        "Game 1",
        vec![pending_seat(1, "velvet-azure")],
    )];
    let req = make_request(Some("velvet-azure@relay.example.com"), None, PLAYER_A);

    assert!(matches!(
        route(&req, &mut rosters, &[Path::new("/tmp")]),
        RoutingDecision::Provisioned(_)
    ));
}

#[test]
fn route_by_code_invalid_code_returns_error() {
    let mut rosters = vec![make_roster(
        "game1",
        "Game 1",
        vec![pending_seat(1, "velvet-azure")],
    )];
    let req = make_request(Some("wrong-code"), None, PLAYER_A);

    assert_eq!(
        route(&req, &mut rosters, &[Path::new("/tmp")]),
        RoutingDecision::Error(RouteError::InvalidCode)
    );
}

#[test]
fn route_by_code_already_claimed_by_other_returns_error() {
    let mut rosters = vec![make_roster(
        "game1",
        "Game 1",
        vec![claimed_seat(1, "velvet-azure", PLAYER_B)],
    )];
    let req = make_request(Some("velvet-azure"), None, PLAYER_A);

    assert_eq!(
        route(&req, &mut rosters, &[Path::new("/tmp")]),
        RoutingDecision::Error(RouteError::CodeClaimed)
    );
}

#[test]
fn route_by_code_same_player_reconnect_succeeds() {
    // Same player re-presenting their code after claiming — should succeed.
    let mut rosters = vec![make_roster(
        "game1",
        "Game 1",
        vec![claimed_seat(1, "velvet-azure", PLAYER_A)],
    )];
    let req = make_request(Some("velvet-azure"), None, PLAYER_A);

    assert!(matches!(
        route(&req, &mut rosters, &[Path::new("/tmp")]),
        RoutingDecision::Provisioned(_)
    ));
}

// --- game-id path ---

#[test]
fn route_by_game_id_finds_known_player() {
    let mut rosters = vec![make_roster(
        "friday-night",
        "Friday Night EC",
        vec![claimed_seat(2, "abbey-zoom", PLAYER_A)],
    )];
    let req = make_request(None, Some("friday-night"), PLAYER_A);

    match route(&req, &mut rosters, &[Path::new("/tmp")]) {
        RoutingDecision::Provisioned(seat) => {
            assert_eq!(seat.game_id, "friday-night");
            assert_eq!(seat.player, 2);
        }
        other => panic!("expected Provisioned, got {other:?}"),
    }
}

#[test]
fn route_by_game_id_unknown_game_returns_error() {
    let mut rosters = vec![make_roster(
        "friday-night",
        "Friday Night EC",
        vec![claimed_seat(1, "velvet-azure", PLAYER_A)],
    )];
    let req = make_request(None, Some("no-such-game"), PLAYER_A);

    assert_eq!(
        route(&req, &mut rosters, &[Path::new("/tmp")]),
        RoutingDecision::Error(RouteError::GameNotFound)
    );
}

#[test]
fn route_by_game_id_player_not_in_game_returns_error() {
    let mut rosters = vec![make_roster(
        "friday-night",
        "Friday Night EC",
        vec![claimed_seat(1, "velvet-azure", PLAYER_B)],
    )];
    let req = make_request(None, Some("friday-night"), PLAYER_A);

    assert_eq!(
        route(&req, &mut rosters, &[Path::new("/tmp")]),
        RoutingDecision::Error(RouteError::UnknownPlayerInGame)
    );
}

// --- npub-only path ---

#[test]
fn route_by_npub_single_match_succeeds() {
    let mut rosters = vec![make_roster(
        "friday-night",
        "Friday Night EC",
        vec![claimed_seat(3, "abbey-zoom", PLAYER_A)],
    )];
    let req = make_request(None, None, PLAYER_A);

    match route(&req, &mut rosters, &[Path::new("/tmp")]) {
        RoutingDecision::Provisioned(seat) => {
            assert_eq!(seat.player, 3);
            assert_eq!(seat.game_id, "friday-night");
        }
        other => panic!("expected Provisioned, got {other:?}"),
    }
}

#[test]
fn route_by_npub_unknown_player_returns_error() {
    let mut rosters = vec![make_roster(
        "friday-night",
        "Friday Night EC",
        vec![claimed_seat(1, "velvet-azure", PLAYER_B)],
    )];
    let req = make_request(None, None, PLAYER_A);

    assert_eq!(
        route(&req, &mut rosters, &[Path::new("/tmp")]),
        RoutingDecision::Error(RouteError::UnknownPlayer)
    );
}

#[test]
fn route_by_npub_multiple_games_returns_disambiguation() {
    let mut rosters = vec![
        make_roster(
            "friday-night",
            "Friday Night EC",
            vec![claimed_seat(1, "velvet-azure", PLAYER_A)],
        ),
        make_roster(
            "saturday-showdown",
            "Saturday Showdown",
            vec![claimed_seat(3, "abbey-zoom", PLAYER_A)],
        ),
    ];
    let req = make_request(None, None, PLAYER_A);

    match route(&req, &mut rosters, &[Path::new("/tmp"), Path::new("/tmp")]) {
        RoutingDecision::Error(RouteError::MultipleGames(games)) => {
            assert_eq!(games.len(), 2);
            assert!(
                games
                    .iter()
                    .any(|g| g.game_id == "friday-night" && g.player == 1)
            );
            assert!(
                games
                    .iter()
                    .any(|g| g.game_id == "saturday-showdown" && g.player == 3)
            );
        }
        other => panic!("expected MultipleGames, got {other:?}"),
    }
}

// --- error code strings ---

#[test]
fn route_error_codes_match_spec() {
    assert_eq!(RouteError::InvalidCode.error_code(), "invalid_code");
    assert_eq!(RouteError::CodeClaimed.error_code(), "code_claimed");
    assert_eq!(RouteError::UnknownPlayer.error_code(), "unknown_player");
    assert_eq!(RouteError::GameNotFound.error_code(), "game_not_found");
    assert_eq!(
        RouteError::UnknownPlayerInGame.error_code(),
        "unknown_player"
    );
    assert_eq!(
        RouteError::MultipleGames(vec![]).error_code(),
        "multiple_games"
    );
}

// --- multi-game roster, code uniqueness across games ---

#[test]
fn route_by_code_finds_seat_in_second_roster() {
    let mut rosters = vec![
        make_roster("game-a", "Game A", vec![pending_seat(1, "velvet-azure")]),
        make_roster("game-b", "Game B", vec![pending_seat(1, "abbey-zoom")]),
    ];
    let req = make_request(Some("abbey-zoom"), None, PLAYER_A);

    match route(&req, &mut rosters, &[Path::new("/tmp"), Path::new("/tmp")]) {
        RoutingDecision::Provisioned(seat) => {
            assert_eq!(seat.game_id, "game-b");
            assert_eq!(seat.player, 1);
        }
        other => panic!("expected Provisioned in game-b, got {other:?}"),
    }
}

// --- ResolvedSeat and GameEntry derive coverage ---

#[test]
fn resolved_seat_fields_accessible() {
    let s = ResolvedSeat {
        game_id: "g".to_string(),
        game_name: "G".to_string(),
        player: 1,
        player_npub: "npub1test".to_string(),
    };
    assert_eq!(s.game_id, "g");
    assert_eq!(s.player, 1);
}

#[test]
fn game_entry_fields_accessible() {
    let e = GameEntry {
        game_id: "g".to_string(),
        game_name: "G".to_string(),
        player: 2,
    };
    assert_eq!(e.game_id, "g");
    assert_eq!(e.player, 2);
}
