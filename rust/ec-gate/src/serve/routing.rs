//! Session routing: map a parsed 30501 request to a hosted seat.

use ec_data::HostedSeatStatus;

use crate::serve::catalog::HostedGameEntry;
use crate::serve::request::SessionRequest;

/// A game + seat resolved from a session request.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedSeat {
    pub game_id: String,
    pub game_name: String,
    pub player: usize,
    pub player_npub: String,
    pub first_claim: bool,
}

/// Information about one game the player is enrolled in (for `MultipleGames`).
#[derive(Debug, Clone, PartialEq)]
pub struct GameEntry {
    pub game_id: String,
    pub game_name: String,
    pub player: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RoutingDecision {
    Provisioned(ResolvedSeat),
    Error(RouteError),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RouteError {
    InvalidCode,
    CodeClaimed,
    IdentityAlreadyInGame { player: usize },
    UnknownPlayer,
    MultipleGames(Vec<GameEntry>),
    GameNotFound,
    UnknownPlayerInGame,
}

impl RouteError {
    pub fn error_code(&self) -> &'static str {
        match self {
            RouteError::InvalidCode => "invalid_code",
            RouteError::CodeClaimed => "code_claimed",
            RouteError::IdentityAlreadyInGame { .. } => "identity_already_in_game",
            RouteError::UnknownPlayer => "unknown_player",
            RouteError::MultipleGames(_) => "multiple_games",
            RouteError::GameNotFound => "game_not_found",
            RouteError::UnknownPlayerInGame => "unknown_player",
        }
    }
}

impl std::fmt::Display for RouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCode => write!(f, "invite code not found"),
            Self::CodeClaimed => write!(f, "invite code already claimed"),
            Self::IdentityAlreadyInGame { player } => {
                write!(f, "identity already claimed seat {player} in this game")
            }
            Self::UnknownPlayer => write!(f, "player not found in any hosted seat"),
            Self::MultipleGames(games) => {
                write!(f, "player is in {} games; game-id required", games.len())
            }
            Self::GameNotFound => write!(f, "game not found"),
            Self::UnknownPlayerInGame => write!(f, "player not found in specified game"),
        }
    }
}

pub fn route(request: &SessionRequest, games: &[HostedGameEntry]) -> RoutingDecision {
    if let Some(invite_code) = &request.invite_code {
        route_by_code(request, invite_code, games)
    } else if let Some(game_id) = &request.game_id {
        route_by_game_id(request, game_id, games)
    } else {
        route_by_npub(request, games)
    }
}

pub fn resolve_player_in_game(
    player_pubkey: &str,
    game_id: &str,
    games: &[HostedGameEntry],
) -> Result<ResolvedSeat, RouteError> {
    let Some(game) = games.iter().find(|entry| entry.game.game_id == game_id) else {
        return Err(RouteError::GameNotFound);
    };
    let Some(seat) = game
        .game
        .seats
        .iter()
        .find(|seat| seat.player_npub.as_deref() == Some(player_pubkey))
    else {
        return Err(RouteError::UnknownPlayerInGame);
    };
    Ok(ResolvedSeat {
        game_id: game.game.game_id.clone(),
        game_name: game.game.game_name.clone(),
        player: seat.player_record_index_1_based,
        player_npub: player_pubkey.to_string(),
        first_claim: false,
    })
}

fn route_by_code(
    request: &SessionRequest,
    invite_code: &str,
    games: &[HostedGameEntry],
) -> RoutingDecision {
    let normalized = normalize_code(invite_code);
    let Some((game, seat)) = games.iter().find_map(|entry| {
        entry
            .game
            .seats
            .iter()
            .find(|seat| seat.invite_code == normalized)
            .map(|seat| (entry, seat))
    }) else {
        return RoutingDecision::Error(RouteError::InvalidCode);
    };

    match seat.status {
        HostedSeatStatus::Claimed => {
            if seat.player_npub.as_deref() == Some(request.player_pubkey.as_str()) {
                return RoutingDecision::Provisioned(ResolvedSeat {
                    game_id: game.game.game_id.clone(),
                    game_name: game.game.game_name.clone(),
                    player: seat.player_record_index_1_based,
                    player_npub: request.player_pubkey.clone(),
                    first_claim: false,
                });
            }
            RoutingDecision::Error(RouteError::CodeClaimed)
        }
        HostedSeatStatus::Pending => {
            if let Some(existing) = game.game.seats.iter().find(|other| {
                other.status == HostedSeatStatus::Claimed
                    && other.player_record_index_1_based != seat.player_record_index_1_based
                    && other.player_npub.as_deref() == Some(request.player_pubkey.as_str())
            }) {
                return RoutingDecision::Error(RouteError::IdentityAlreadyInGame {
                    player: existing.player_record_index_1_based,
                });
            }
            RoutingDecision::Provisioned(ResolvedSeat {
                game_id: game.game.game_id.clone(),
                game_name: game.game.game_name.clone(),
                player: seat.player_record_index_1_based,
                player_npub: request.player_pubkey.clone(),
                first_claim: true,
            })
        }
    }
}

fn route_by_game_id(
    request: &SessionRequest,
    game_id: &str,
    games: &[HostedGameEntry],
) -> RoutingDecision {
    match resolve_player_in_game(&request.player_pubkey, game_id, games) {
        Ok(seat) => RoutingDecision::Provisioned(seat),
        Err(err) => RoutingDecision::Error(err),
    }
}

fn route_by_npub(request: &SessionRequest, games: &[HostedGameEntry]) -> RoutingDecision {
    let matches = games
        .iter()
        .flat_map(|entry| {
            entry
                .game
                .seats
                .iter()
                .filter(move |seat| {
                    seat.player_npub.as_deref() == Some(request.player_pubkey.as_str())
                })
                .map(move |seat| (entry, seat))
        })
        .collect::<Vec<_>>();

    match matches.len() {
        0 => RoutingDecision::Error(RouteError::UnknownPlayer),
        1 => {
            let (game, seat) = matches[0];
            RoutingDecision::Provisioned(ResolvedSeat {
                game_id: game.game.game_id.clone(),
                game_name: game.game.game_name.clone(),
                player: seat.player_record_index_1_based,
                player_npub: request.player_pubkey.clone(),
                first_claim: false,
            })
        }
        _ => {
            let games = matches
                .into_iter()
                .map(|(game, seat)| GameEntry {
                    game_id: game.game.game_id.clone(),
                    game_name: game.game.game_name.clone(),
                    player: seat.player_record_index_1_based,
                })
                .collect();
            RoutingDecision::Error(RouteError::MultipleGames(games))
        }
    }
}

fn normalize_code(code: &str) -> String {
    let stripped = code.trim();
    let without_relay = stripped.split('@').next().unwrap_or(stripped);
    without_relay.to_ascii_lowercase()
}
