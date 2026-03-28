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

use ec_nostr::hash::sha256_hex;
use ec_nostr::invite::{InvitePayload, encode_invite};
use nostr_sdk::{Client, EventBuilder, Keys, Kind, PublicKey, Tag};

use ec_data::HostedSeatStatus;

use crate::serve::catalog::HostedGame;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Build and publish a 30500 GameDefinition event for `game`.
///
/// Returns the event ID as a hex string on success.
pub async fn publish_game_definition(
    client: &Client,
    gate_keys: &Keys,
    game: &HostedGame,
    ssh_host: &str,
    ssh_port: u16,
    relay_url: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let tags = build_game_def_tags(game, ssh_host, ssh_port, relay_url, &gate_keys.public_key())?;

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
///   `ssh-host` = SSH hostname players connect to
///   `ssh-port` = SSH port players connect to
///   `players` = total number of seats
///   `slot` = [seat-index, invite-code-hash, npub-or-empty, status] per seat
///   `invite-bech32` = bech32-encoded invite for each pending seat (relay + words embedded)
pub fn build_game_def_tags(
    game: &HostedGame,
    ssh_host: &str,
    ssh_port: u16,
    relay_url: &str,
    gate_pubkey: &PublicKey,
) -> Result<Vec<Tag>, Box<dyn std::error::Error + Send + Sync>> {
    // Decode gate pubkey hex into raw 32 bytes for the bech32 invite payload.
    let gate_npub_bytes: Option<[u8; 32]> = {
        let hex = gate_pubkey.to_hex();
        let mut bytes = [0u8; 32];
        let mut ok = hex.len() == 64;
        for (i, chunk) in hex.as_bytes().chunks(2).enumerate().take(32) {
            if let Ok(s) = std::str::from_utf8(chunk) {
                if let Ok(b) = u8::from_str_radix(s, 16) {
                    bytes[i] = b;
                } else {
                    ok = false;
                    break;
                }
            }
        }
        if ok { Some(bytes) } else { None }
    };

    let mut tags = Vec::new();

    tags.push(Tag::parse(["d", &game.game_id])?);
    tags.push(Tag::parse(["name", &game.game_name])?);
    tags.push(Tag::parse(["status", "active"])?);
    tags.push(Tag::parse(["ssh-host", ssh_host])?);
    tags.push(Tag::parse(["ssh-port", &ssh_port.to_string()])?);
    tags.push(Tag::parse(["players", &game.seats.len().to_string()])?);

    for seat in &game.seats {
        let code_hash = sha256_hex(seat.invite_code.to_ascii_lowercase().as_bytes());
        let npub = seat.player_npub.as_deref().unwrap_or("");
        let status = match seat.status {
            HostedSeatStatus::Pending => "pending",
            HostedSeatStatus::Claimed => "claimed",
        };
        let seat_idx = seat.player_record_index_1_based.to_string();
        tags.push(Tag::parse(["slot", &seat_idx, &code_hash, npub, status])?);

        // Emit a bech32 invite tag for pending seats so sysops can copy-paste
        // a self-contained invite string that embeds the relay URL.
        if seat.status == HostedSeatStatus::Pending {
            let payload = InvitePayload {
                relay_url: relay_url.to_string(),
                words: seat.invite_code.to_ascii_lowercase(),
                game_id: Some(game.game_id.clone()),
                gate_npub: gate_npub_bytes,
            };
            if let Ok(encoded) = encode_invite(&payload) {
                tags.push(Tag::parse(["invite-bech32", &seat_idx, &encoded])?);
            }
        }
    }

    Ok(tags)
}

