//! Pure query functions over a slice of loaded rosters.

use super::{Roster, Seat};

/// Find the first seat whose invite code matches `code` (case-insensitive,
/// ignoring any `@relay` suffix).
///
/// Returns `(roster, seat)` or `None` if no match is found.
pub fn find_seat_by_code<'a>(rosters: &'a [Roster], code: &str) -> Option<(&'a Roster, &'a Seat)> {
    let normalized = normalize_code(code);
    for roster in rosters {
        for seat in &roster.seats {
            if normalize_code(&seat.code) == normalized {
                return Some((roster, seat));
            }
        }
    }
    None
}

/// Find all seats across all rosters where `seat.npub == Some(npub)`.
///
/// Returns a vec of `(roster, seat)` pairs. Empty if the npub is unknown.
/// More than one result means the player is in multiple games.
pub fn find_seats_by_npub<'a>(rosters: &'a [Roster], npub: &str) -> Vec<(&'a Roster, &'a Seat)> {
    let mut results = Vec::new();
    for roster in rosters {
        for seat in &roster.seats {
            if seat.npub.as_deref() == Some(npub) {
                results.push((roster, seat));
            }
        }
    }
    results
}

/// Normalize an invite code for comparison: lowercase, trim whitespace,
/// strip an `@...` relay suffix if present.
fn normalize_code(code: &str) -> String {
    let stripped = code.trim();
    let without_relay = stripped.split('@').next().unwrap_or(stripped);
    without_relay.to_lowercase()
}
