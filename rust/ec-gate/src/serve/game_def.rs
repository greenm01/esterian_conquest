//! 30500 GameDefinition event construction and publishing.
//!
//! A GameDefinition event is a public, unencrypted NIP-33 event that advertises
//! one game: its name, number of seats, and which seats are claimed (with npubs).
//! Invite codes are SHA-256-hashed before publishing so relay observers cannot
//! read unclaimed codes.
//!
//! `ec-gate` publishes an updated 30500 for a game whenever:
//!   - `serve` starts up (announce all loaded games)
//!   - a seat is claimed (roster changed)

use sha2::{Digest, Sha256};

use nostr_sdk::{Client, EventBuilder, Keys, Kind, Tag};

use crate::roster::Roster;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Build and publish a 30500 GameDefinition event for `roster`.
///
/// Returns the event ID as a hex string on success.
pub async fn publish_game_definition(
    client: &Client,
    gate_keys: &Keys,
    roster: &Roster,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let tags = build_game_def_tags(roster)?;

    let event = EventBuilder::new(Kind::Custom(30500), "")
        .tags(tags)
        .sign_with_keys(gate_keys)?;

    client.send_event(&event).await?;

    Ok(event.id.to_hex())
}

// ---------------------------------------------------------------------------
// Tag construction (pub for unit tests)
// ---------------------------------------------------------------------------

/// Build the NIP-33 tag list for a 30500 GameDefinition event.
///
/// Tags produced (in order):
///   `d` = game id slug
///   `name` = human-readable game name
///   `status` = "active"
///   `players` = total number of seats
///   `slot` = [seat-index, invite-code-hash, npub-or-empty, status] per seat
pub fn build_game_def_tags(
    roster: &Roster,
) -> Result<Vec<Tag>, Box<dyn std::error::Error + Send + Sync>> {
    let mut tags = Vec::new();

    tags.push(Tag::parse(["d", &roster.id])?);
    tags.push(Tag::parse(["name", &roster.name])?);
    tags.push(Tag::parse(["status", "active"])?);
    tags.push(Tag::parse(["players", &roster.seats.len().to_string()])?);

    for seat in &roster.seats {
        let code_hash = sha256_hex(&seat.code.to_lowercase());
        let npub = seat.npub.as_deref().unwrap_or("");
        let status = seat.status.as_str();
        let seat_idx = seat.player.to_string();
        tags.push(Tag::parse(["slot", &seat_idx, &code_hash, npub, status])?);
    }

    Ok(tags)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compute SHA-256 of a string and return lowercase hex.
pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    result.iter().map(|b| format!("{b:02x}")).collect()
}
