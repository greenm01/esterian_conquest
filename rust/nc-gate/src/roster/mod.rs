//! Player roster types and operations.
//!
//! A roster maps player seats to Nostr public keys for one game.
//! Each game directory contains a `roster.kdl` file managed by nc-gate.

pub mod io;
pub mod lookup;

/// One game's complete player roster.
#[derive(Debug, Clone, PartialEq)]
pub struct Roster {
    /// Short slug derived from the game directory name (e.g. `friday-night`).
    /// Used as the `d` tag in 30500 events and in `game-id` disambiguation.
    pub id: String,
    /// Human-readable game name shown to players in nc-connect.
    pub name: String,
    /// Ordered list of seats (typically indexed by `seat.player`).
    pub seats: Vec<Seat>,
}

/// One player seat in a roster.
#[derive(Debug, Clone, PartialEq)]
pub struct Seat {
    /// 1-based seat / player-record index.
    pub player: usize,
    /// Current invite code for this seat (two Monero mnemonic words, hyphenated).
    pub code: String,
    /// Whether this seat has been claimed by a player.
    pub status: SeatStatus,
    /// Player's Nostr public key. Present only when `status` is `Claimed`.
    pub npub: Option<String>,
}

/// Claim state of a seat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeatStatus {
    /// Seat is available; invite code has not been redeemed.
    Pending,
    /// Seat is bound to an npub; invite code was already redeemed.
    Claimed,
}

impl SeatStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            SeatStatus::Pending => "pending",
            SeatStatus::Claimed => "claimed",
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "pending" => Ok(SeatStatus::Pending),
            "claimed" => Ok(SeatStatus::Claimed),
            other => Err(format!("unknown seat status: {other}")),
        }
    }
}

pub use io::{load_roster, save_roster};
pub use lookup::{find_seat_by_code, find_seats_by_npub};
