//! Session routing: map a parsed 30501 request to a seat, and claim it if needed.
//!
//! This module is pure logic over the in-memory roster slice plus disk I/O for
//! the atomic claim write.  The Nostr publish step (steps 7-8) is handled by
//! the caller after a successful route.

use std::path::Path;

use tracing::warn;

use crate::roster::io::save_roster;
use crate::roster::lookup::find_seats_by_npub;
use crate::roster::{Roster, SeatStatus};
use crate::serve::request::SessionRequest;

/// A game + seat resolved from a session request.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedSeat {
    /// Game roster ID (e.g. `friday-night`).
    pub game_id: String,
    /// Human-readable game name.
    pub game_name: String,
    /// 1-based seat / player-record index.
    pub player: usize,
    /// Player's Nostr public key (npub bech32 or hex — whatever is stored in the roster).
    pub player_npub: String,
}

/// Information about one game the player is enrolled in (for `MultipleGames`).
#[derive(Debug, Clone, PartialEq)]
pub struct GameEntry {
    pub game_id: String,
    pub game_name: String,
    pub player: usize,
}

/// Outcome of `route()`.
#[derive(Debug, Clone, PartialEq)]
pub enum RoutingDecision {
    /// Seat resolved; caller should provision an SSH key and send 30502.
    Provisioned(ResolvedSeat),
    /// Routing failed for one of the reasons below.
    Error(RouteError),
}

/// Routing failure reasons (map directly to 30503 error codes in the spec).
#[derive(Debug, Clone, PartialEq)]
pub enum RouteError {
    /// The invite code does not match any seat.
    InvalidCode,
    /// The invite code has already been claimed by a different npub.
    CodeClaimed,
    /// The player's npub is not in any roster (no invite code provided).
    UnknownPlayer,
    /// The player is in multiple games and no `game-id` tag was provided.
    MultipleGames(Vec<GameEntry>),
    /// A `game-id` tag was provided but does not match any loaded game.
    GameNotFound,
    /// The player's npub is not in the specified game's roster.
    UnknownPlayerInGame,
}

impl RouteError {
    /// Wire-format error code sent in 30503 encrypted payload.
    pub fn error_code(&self) -> &'static str {
        match self {
            RouteError::InvalidCode => "invalid_code",
            RouteError::CodeClaimed => "code_claimed",
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
            RouteError::InvalidCode => write!(f, "invite code not found"),
            RouteError::CodeClaimed => write!(f, "invite code already claimed"),
            RouteError::UnknownPlayer => write!(f, "player not found in any roster"),
            RouteError::MultipleGames(games) => {
                write!(f, "player is in {} games; game-id required", games.len())
            }
            RouteError::GameNotFound => write!(f, "game not found"),
            RouteError::UnknownPlayerInGame => write!(f, "player not found in specified game"),
        }
    }
}

/// Route a session request to a seat, mutating the roster on a first-time claim.
///
/// `game_dirs[i]` must correspond to `rosters[i]` — it is the directory path
/// used to derive the `roster.kdl` save path on a claim.
///
/// The three routing paths follow the spec exactly:
///
/// 1. **With invite code** — look up by code; validate pending; claim the seat.
/// 2. **No invite code + game-id** — look up the player's npub in that specific game.
/// 3. **No invite code + no game-id** — search all rosters for the npub;
///    disambiguate if multiple matches.
pub fn route(
    request: &SessionRequest,
    rosters: &mut Vec<Roster>,
    game_dirs: &[&Path],
) -> RoutingDecision {
    if let Some(invite_code) = &request.invite_code {
        route_by_code(request, invite_code, rosters, game_dirs)
    } else if let Some(game_id) = &request.game_id {
        route_by_game_id(request, game_id, rosters)
    } else {
        route_by_npub(request, rosters)
    }
}

pub fn resolve_player_in_game(
    player_pubkey: &str,
    game_id: &str,
    rosters: &[Roster],
) -> Result<ResolvedSeat, RouteError> {
    let Some(roster) = rosters.iter().find(|r| r.id == game_id) else {
        return Err(RouteError::GameNotFound);
    };

    let Some(seat) = roster
        .seats
        .iter()
        .find(|s| s.npub.as_deref() == Some(player_pubkey))
    else {
        return Err(RouteError::UnknownPlayerInGame);
    };

    Ok(ResolvedSeat {
        game_id: roster.id.clone(),
        game_name: roster.name.clone(),
        player: seat.player,
        player_npub: player_pubkey.to_string(),
    })
}

// --- private routing paths ---

fn route_by_code(
    request: &SessionRequest,
    invite_code: &str,
    rosters: &mut Vec<Roster>,
    game_dirs: &[&Path],
) -> RoutingDecision {
    // Find the roster index (need to mutate later).
    let roster_idx = rosters.iter().enumerate().find_map(|(i, roster)| {
        roster
            .seats
            .iter()
            .any(|s| codes_match(&s.code, invite_code))
            .then_some(i)
    });

    let Some(ri) = roster_idx else {
        return RoutingDecision::Error(RouteError::InvalidCode);
    };

    // Find the seat index within that roster.
    let seat_idx = rosters[ri]
        .seats
        .iter()
        .position(|s| codes_match(&s.code, invite_code))
        .unwrap(); // safe: we just found it above

    match rosters[ri].seats[seat_idx].status {
        SeatStatus::Claimed => {
            // Already claimed — allow if same npub (reconnect), deny otherwise.
            let existing_npub = rosters[ri].seats[seat_idx].npub.as_deref().unwrap_or("");
            if existing_npub == request.player_pubkey {
                // Same player re-presenting their code — treat as reconnect.
                return RoutingDecision::Provisioned(ResolvedSeat {
                    game_id: rosters[ri].id.clone(),
                    game_name: rosters[ri].name.clone(),
                    player: rosters[ri].seats[seat_idx].player,
                    player_npub: request.player_pubkey.clone(),
                });
            }
            return RoutingDecision::Error(RouteError::CodeClaimed);
        }
        SeatStatus::Pending => {}
    }

    // Claim the seat.
    rosters[ri].seats[seat_idx].status = SeatStatus::Claimed;
    rosters[ri].seats[seat_idx].npub = Some(request.player_pubkey.clone());

    if let Some(dir) = game_dirs.get(ri) {
        let roster_path = dir.join("roster.kdl");
        if let Err(e) = save_roster(&roster_path, &rosters[ri]) {
            // Warn and continue — provisioning should still succeed even if
            // the save fails; the operator will see the error in logs.
            warn!(path = %roster_path.display(), error = %e, "failed to save roster after claim");
        }
    }

    RoutingDecision::Provisioned(ResolvedSeat {
        game_id: rosters[ri].id.clone(),
        game_name: rosters[ri].name.clone(),
        player: rosters[ri].seats[seat_idx].player,
        player_npub: request.player_pubkey.clone(),
    })
}

fn route_by_game_id(
    request: &SessionRequest,
    game_id: &str,
    rosters: &[Roster],
) -> RoutingDecision {
    match resolve_player_in_game(&request.player_pubkey, game_id, rosters) {
        Ok(seat) => RoutingDecision::Provisioned(seat),
        Err(err) => RoutingDecision::Error(err),
    }
}

fn route_by_npub(request: &SessionRequest, rosters: &[Roster]) -> RoutingDecision {
    let matches = find_seats_by_npub(rosters, &request.player_pubkey);

    match matches.len() {
        0 => RoutingDecision::Error(RouteError::UnknownPlayer),
        1 => {
            let (roster, seat) = matches[0];
            RoutingDecision::Provisioned(ResolvedSeat {
                game_id: roster.id.clone(),
                game_name: roster.name.clone(),
                player: seat.player,
                player_npub: request.player_pubkey.clone(),
            })
        }
        _ => {
            let games = matches
                .into_iter()
                .map(|(roster, seat)| GameEntry {
                    game_id: roster.id.clone(),
                    game_name: roster.name.clone(),
                    player: seat.player,
                })
                .collect();
            RoutingDecision::Error(RouteError::MultipleGames(games))
        }
    }
}

// --- helpers ---

fn codes_match(a: &str, b: &str) -> bool {
    normalize_code(a) == normalize_code(b)
}

fn normalize_code(code: &str) -> String {
    let stripped = code.trim();
    let without_relay = stripped.split('@').next().unwrap_or(stripped);
    without_relay.to_lowercase()
}
