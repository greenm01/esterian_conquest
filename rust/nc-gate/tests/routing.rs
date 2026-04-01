//! Regression tests for session routing (step 6).

use std::path::PathBuf;

use nc_data::{HostedSeat, HostedSeatStatus};
use nc_gate::serve::catalog::{HostedGame, HostedGameEntry};
use nc_gate::serve::request::SessionRequest;
use nc_gate::serve::routing::{GameEntry, ResolvedSeat, RouteError, RoutingDecision, route};

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

fn pending_seat(player: usize, code: &str) -> HostedSeat {
    HostedSeat {
        player_record_index_1_based: player,
        invite_code: code.to_string(),
        status: HostedSeatStatus::Pending,
        player_npub: None,
    }
}

fn claimed_seat(player: usize, code: &str, npub: &str) -> HostedSeat {
    HostedSeat {
        player_record_index_1_based: player,
        invite_code: code.to_string(),
        status: HostedSeatStatus::Claimed,
        player_npub: Some(npub.to_string()),
    }
}

fn make_game(id: &str, name: &str, seats: Vec<HostedSeat>) -> HostedGameEntry {
    HostedGameEntry {
        dir: PathBuf::from(format!("/tmp/{id}")),
        game: HostedGame {
            game_id: id.to_string(),
            game_name: name.to_string(),
            seats,
        },
    }
}

#[test]
fn route_by_code_marks_pending_seat_as_first_claim() {
    let games = vec![make_game(
        "friday-night",
        "Friday Night EC",
        vec![
            pending_seat(1, "velvet-azure"),
            pending_seat(2, "abbey-zoom"),
        ],
    )];
    let req = make_request(Some("velvet-azure"), None, PLAYER_A);

    match route(&req, &games) {
        RoutingDecision::Provisioned(seat) => {
            assert_eq!(seat.game_id, "friday-night");
            assert_eq!(seat.player, 1);
            assert_eq!(seat.player_npub, PLAYER_A);
            assert!(seat.first_claim);
        }
        other => panic!("expected Provisioned, got {other:?}"),
    }
}

#[test]
fn route_by_code_case_insensitive() {
    let games = vec![make_game(
        "game1",
        "Game 1",
        vec![pending_seat(1, "velvet-azure")],
    )];
    let req = make_request(Some("VELVET-AZURE"), None, PLAYER_A);
    assert!(matches!(
        route(&req, &games),
        RoutingDecision::Provisioned(_)
    ));
}

#[test]
fn route_by_code_strips_relay_suffix() {
    let games = vec![make_game(
        "game1",
        "Game 1",
        vec![pending_seat(1, "velvet-azure")],
    )];
    let req = make_request(Some("velvet-azure@relay.example.com"), None, PLAYER_A);
    assert!(matches!(
        route(&req, &games),
        RoutingDecision::Provisioned(_)
    ));
}

#[test]
fn route_by_code_invalid_code_returns_error() {
    let games = vec![make_game(
        "game1",
        "Game 1",
        vec![pending_seat(1, "velvet-azure")],
    )];
    let req = make_request(Some("wrong-code"), None, PLAYER_A);
    assert_eq!(
        route(&req, &games),
        RoutingDecision::Error(RouteError::InvalidCode)
    );
}

#[test]
fn route_by_code_already_claimed_by_other_returns_error() {
    let games = vec![make_game(
        "game1",
        "Game 1",
        vec![claimed_seat(1, "velvet-azure", PLAYER_B)],
    )];
    let req = make_request(Some("velvet-azure"), None, PLAYER_A);
    assert_eq!(
        route(&req, &games),
        RoutingDecision::Error(RouteError::CodeClaimed)
    );
}

#[test]
fn route_by_code_same_player_reconnect_succeeds_without_first_claim() {
    let games = vec![make_game(
        "game1",
        "Game 1",
        vec![claimed_seat(1, "velvet-azure", PLAYER_A)],
    )];
    let req = make_request(Some("velvet-azure"), None, PLAYER_A);
    match route(&req, &games) {
        RoutingDecision::Provisioned(seat) => assert!(!seat.first_claim),
        other => panic!("expected Provisioned, got {other:?}"),
    }
}

#[test]
fn route_by_code_rejects_same_identity_claiming_second_seat_in_game() {
    let games = vec![make_game(
        "game1",
        "Game 1",
        vec![
            claimed_seat(1, "velvet-azure", PLAYER_A),
            pending_seat(2, "abbey-zoom"),
        ],
    )];
    let req = make_request(Some("abbey-zoom"), None, PLAYER_A);
    assert_eq!(
        route(&req, &games),
        RoutingDecision::Error(RouteError::IdentityAlreadyInGame { player: 1 })
    );
}

#[test]
fn route_by_game_id_finds_known_player() {
    let games = vec![make_game(
        "friday-night",
        "Friday Night EC",
        vec![claimed_seat(2, "abbey-zoom", PLAYER_A)],
    )];
    let req = make_request(None, Some("friday-night"), PLAYER_A);
    match route(&req, &games) {
        RoutingDecision::Provisioned(seat) => {
            assert_eq!(seat.game_id, "friday-night");
            assert_eq!(seat.player, 2);
            assert!(!seat.first_claim);
        }
        other => panic!("expected Provisioned, got {other:?}"),
    }
}

#[test]
fn route_by_game_id_unknown_game_returns_error() {
    let games = vec![make_game(
        "friday-night",
        "Friday Night EC",
        vec![claimed_seat(1, "velvet-azure", PLAYER_A)],
    )];
    let req = make_request(None, Some("no-such-game"), PLAYER_A);
    assert_eq!(
        route(&req, &games),
        RoutingDecision::Error(RouteError::GameNotFound)
    );
}

#[test]
fn route_by_game_id_player_not_in_game_returns_error() {
    let games = vec![make_game(
        "friday-night",
        "Friday Night EC",
        vec![claimed_seat(1, "velvet-azure", PLAYER_B)],
    )];
    let req = make_request(None, Some("friday-night"), PLAYER_A);
    assert_eq!(
        route(&req, &games),
        RoutingDecision::Error(RouteError::UnknownPlayerInGame)
    );
}

#[test]
fn route_by_npub_single_match_succeeds() {
    let games = vec![make_game(
        "friday-night",
        "Friday Night EC",
        vec![claimed_seat(3, "abbey-zoom", PLAYER_A)],
    )];
    let req = make_request(None, None, PLAYER_A);
    match route(&req, &games) {
        RoutingDecision::Provisioned(seat) => {
            assert_eq!(seat.player, 3);
            assert_eq!(seat.game_id, "friday-night");
        }
        other => panic!("expected Provisioned, got {other:?}"),
    }
}

#[test]
fn route_by_npub_unknown_player_returns_error() {
    let games = vec![make_game(
        "friday-night",
        "Friday Night EC",
        vec![claimed_seat(1, "velvet-azure", PLAYER_B)],
    )];
    let req = make_request(None, None, PLAYER_A);
    assert_eq!(
        route(&req, &games),
        RoutingDecision::Error(RouteError::UnknownPlayer)
    );
}

#[test]
fn route_by_npub_multiple_games_returns_disambiguation() {
    let games = vec![
        make_game(
            "friday-night",
            "Friday Night EC",
            vec![claimed_seat(1, "velvet-azure", PLAYER_A)],
        ),
        make_game(
            "saturday-showdown",
            "Saturday Showdown",
            vec![claimed_seat(3, "abbey-zoom", PLAYER_A)],
        ),
    ];
    let req = make_request(None, None, PLAYER_A);

    match route(&req, &games) {
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

#[test]
fn route_by_code_finds_seat_in_second_game() {
    let games = vec![
        make_game("game-a", "Game A", vec![pending_seat(1, "velvet-azure")]),
        make_game("game-b", "Game B", vec![pending_seat(1, "abbey-zoom")]),
    ];
    let req = make_request(Some("abbey-zoom"), None, PLAYER_A);
    match route(&req, &games) {
        RoutingDecision::Provisioned(seat) => {
            assert_eq!(seat.game_id, "game-b");
            assert_eq!(seat.player, 1);
        }
        other => panic!("expected Provisioned in game-b, got {other:?}"),
    }
}

#[test]
fn resolved_seat_fields_accessible() {
    let s = ResolvedSeat {
        game_id: "g".to_string(),
        game_name: "G".to_string(),
        player: 1,
        player_npub: "npub1test".to_string(),
        first_claim: false,
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
