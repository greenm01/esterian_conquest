//! 30502 SessionReady and 30503 SessionError event construction and publishing.
//!
//! Both events are NIP-44 encrypted to the player's public key.  The caller
//! (the serve loop) calls `publish_session_ready` or `publish_session_error`
//! after routing and provisioning are complete.

use nostr_sdk::nips::nip44;
use nostr_sdk::{Client, EventBuilder, Keys, Kind, PublicKey, Tag};
use nostr_sdk::nips::nip44::Version;

use crate::config::GateConfig;
use crate::serve::provision::ProvisionedKey;
use crate::serve::routing::{GameEntry, ResolvedSeat, RouteError};

// ---------------------------------------------------------------------------
// Session ready
// ---------------------------------------------------------------------------

/// JSON payload encrypted inside a 30502 SessionReady event.
#[derive(Debug)]
pub struct SessionReadyPayload<'a> {
    pub game_id: &'a str,
    pub ssh_host: &'a str,
    pub ssh_port: u16,
    pub game_name: &'a str,
    pub seat: usize,
}

impl SessionReadyPayload<'_> {
    /// Serialize to a compact JSON string.
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"game_id":"{game_id}","ssh_host":"{ssh_host}","ssh_port":{ssh_port},"game_name":"{game_name}","seat":{seat}}}"#,
            game_id = self.game_id,
            ssh_host = self.ssh_host,
            ssh_port = self.ssh_port,
            game_name = self.game_name,
            seat = self.seat,
        )
    }
}

/// Build and publish a 30502 SessionReady event.
///
/// The event is NIP-44 encrypted to `player_pubkey` and published on `client`.
/// Returns the event ID as a hex string on success.
pub async fn publish_session_ready(
    client: &Client,
    gate_keys: &Keys,
    player_pubkey: &PublicKey,
    session_nonce: &str,
    config: &GateConfig,
    seat: &ResolvedSeat,
    _provisioned: &ProvisionedKey,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let payload = SessionReadyPayload {
        game_id: &seat.game_id,
        ssh_host: &config.ssh_host,
        ssh_port: config.ssh_port,
        game_name: &seat.game_name,
        seat: seat.player,
    };
    let plaintext = payload.to_json();

    let encrypted = nip44::encrypt(
        gate_keys.secret_key(),
        player_pubkey,
        &plaintext,
        Version::V2,
    )?;

    let tags = vec![
        Tag::parse(["d", session_nonce])?,
        Tag::parse(["p", &player_pubkey.to_hex()])?,
    ];

    let event = EventBuilder::new(Kind::Custom(30502), encrypted)
        .tags(tags)
        .sign_with_keys(gate_keys)?;

    client.send_event(&event).await?;

    Ok(event.id.to_hex())
}

// ---------------------------------------------------------------------------
// Session error
// ---------------------------------------------------------------------------

/// Build the JSON error payload for a 30503 SessionError event.
pub fn session_error_payload(error: &RouteError) -> String {
    match error {
        RouteError::MultipleGames(games) => {
            let entries = games
                .iter()
                .map(|g| game_entry_json(g))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                r#"{{"error":"multiple_games","message":"Your identity is in multiple games on this server.","games":[{entries}]}}"#
            )
        }
        _ => {
            let code = error.error_code();
            let message = escape_json_string(&error.to_string());
            format!(r#"{{"error":"{code}","message":"{message}"}}"#)
        }
    }
}

fn game_entry_json(g: &GameEntry) -> String {
    let game_id = escape_json_string(&g.game_id);
    let name = escape_json_string(&g.game_name);
    format!(
        r#"{{"game_id":"{game_id}","name":"{name}","seat":{seat}}}"#,
        seat = g.player
    )
}

/// Build and publish a 30503 SessionError event.
pub async fn publish_session_error(
    client: &Client,
    gate_keys: &Keys,
    player_pubkey: &PublicKey,
    session_nonce: &str,
    error: &RouteError,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let plaintext = session_error_payload(error);

    let encrypted = nip44::encrypt(
        gate_keys.secret_key(),
        player_pubkey,
        &plaintext,
        Version::V2,
    )?;

    let tags = vec![
        Tag::parse(["d", session_nonce])?,
        Tag::parse(["p", &player_pubkey.to_hex()])?,
    ];

    let event = EventBuilder::new(Kind::Custom(30503), encrypted)
        .tags(tags)
        .sign_with_keys(gate_keys)?;

    client.send_event(&event).await?;

    Ok(event.id.to_hex())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Minimal JSON string escaping for values embedded in format strings.
fn escape_json_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
