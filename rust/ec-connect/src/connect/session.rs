//! High-level session runner shared by the join and direct-mode CLI flows.
//!
//! Both flows reduce to: resolve a target → run handshake → handle result →
//! run bridge.  This module owns that logic so `cli.rs` stays thin.
//!
//! # Multiple-games disambiguation
//!
//! When the gate returns a `multiple_games` 30503 error, the caller passes a
//! `DisambigMode` that controls how the selection is presented to the user:
//!
//! - `DisambigMode::Prompt` — print a numbered list and read a choice from
//!   stdin (used in direct mode and `--join` from the CLI).
//!
//! Future: `DisambigMode::Picker` for the ratatui UI (step 11).

use nostr_sdk::Keys;

use crate::cache::{CachedGame, GameCache, load_cache, save_cache};
use crate::connect::bridge::run_bridge;
use crate::connect::handshake::{GameEntry, HandshakeResult, SessionReadyPayload, run_handshake};
use crate::connect::map_fetch::fetch_map_bundle;
use crate::connect::resolve::ResolvedTarget;
use crate::connect::session_state::fetch_game_metadata;
use crate::connect::ssh_key::EphemeralKeypair;
use crate::map_store::save_map_bundle;
use crate::wallet::io::now_iso8601;

// ── Public types ──────────────────────────────────────────────────────────────

/// How to present multiple-games disambiguation to the user.
pub enum DisambigMode {
    /// Print a numbered list to stdout and read a number from stdin.
    Prompt,
    /// Return `SessionOutcome::NeedsDisambiguation` so the caller (e.g. the
    /// ratatui picker) can present a UI selection screen and retry.
    Picker,
}

/// Outcome returned to the caller after the session ends (or fails to start).
#[derive(Debug)]
pub enum SessionOutcome {
    /// The session completed normally; `exit_code` is the SSH exit status.
    Done {
        exit_code: u32,
        notice: Option<String>,
    },
    /// A non-recoverable error (auth failure, network error, etc.).
    Error(String),
    /// The handshake timed out.
    Timeout,
    /// The gate reported `multiple_games`; the caller should let the user
    /// pick one and retry with that `game_id`.  Only returned when the
    /// `DisambigMode::Picker` mode is active.
    NeedsDisambiguation { games: Vec<GameEntry> },
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Run a full session: handshake → (optional disambig) → bridge.
///
/// `player_keys` — the active identity's Nostr keypair.
/// `target` — resolved server + relay coordinates (may have `game_id` hint
///            pre-populated from the cache by the caller).
/// `username` — SSH username to authenticate as (typically the player's npub).
/// `gate_npub` — the gate's Nostr public key (hex or bech32).
/// `disambig` — how to resolve `multiple_games` errors.
///
/// This is an async function; callers must be inside a tokio runtime.
pub async fn run_session(
    player_keys: &Keys,
    target: ResolvedTarget,
    username: &str,
    gate_npub: &str,
    disambig: DisambigMode,
    maps_root: &std::path::Path,
) -> SessionOutcome {
    let keypair = EphemeralKeypair::generate();
    run_session_with_keypair(
        player_keys,
        target,
        username,
        gate_npub,
        disambig,
        maps_root,
        keypair,
    )
    .await
}

// ── Inner implementation (allows retry with same keypair after disambig) ───────

async fn run_session_with_keypair(
    player_keys: &Keys,
    mut target: ResolvedTarget,
    username: &str,
    gate_npub: &str,
    disambig: DisambigMode,
    maps_root: &std::path::Path,
    keypair: EphemeralKeypair,
) -> SessionOutcome {
    let first_join = target.invite_code.is_some();
    // ── Handshake ─────────────────────────────────────────────────────────────
    let result = match run_handshake(
        player_keys,
        &target,
        &keypair,
        target.game_id.as_deref(),
        gate_npub,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => return SessionOutcome::Error(format!(
            "Could not reach the game server.\n\
             Contact your sysop if this persists.\n\
             \n\
             Technical: handshake failed: {e}"
        )),
    };

    match result {
        HandshakeResult::Timeout => SessionOutcome::Timeout,

        HandshakeResult::Error(err) => {
            if err.error == "multiple_games" && !err.games.is_empty() {
                // Disambiguate and retry once.
                match disambig {
                    DisambigMode::Prompt => {
                        match prompt_game_selection(&err.games) {
                            Ok(selected) => {
                                // Retry with the chosen game_id.
                                target.game_id = Some(selected.game_id.clone());
                                // Generate a fresh keypair for the retry.
                                let retry_keypair = EphemeralKeypair::generate();
                                // Note: DisambigMode doesn't implement Clone; pass Prompt again.
                                Box::pin(run_session_with_keypair(
                                    player_keys,
                                    target,
                                    username,
                                    gate_npub,
                                    DisambigMode::Prompt,
                                    maps_root,
                                    retry_keypair,
                                ))
                                .await
                            }
                            Err(msg) => SessionOutcome::Error(msg),
                        }
                    }
                    DisambigMode::Picker => {
                        // Return control to the picker so it can show a
                        // game-selection screen and retry with the chosen id.
                        SessionOutcome::NeedsDisambiguation { games: err.games }
                    }
                }
            } else {
                SessionOutcome::Error(format!("{}: {}", err.error, err.message))
            }
        }

        HandshakeResult::Ready(payload) => {
            // Update game cache before starting the bridge.
            upsert_cache_entry(&payload, username, gate_npub, &target);
            let map_notice = if first_join {
                auto_fetch_maps(player_keys, &target, gate_npub, &payload, maps_root).await
            } else {
                None
            };

            // Run the SSH bridge.
            match run_bridge(&payload, &keypair, username).await {
                Ok(exit_code) => {
                    refresh_cache_metadata(player_keys, &target, gate_npub, &payload.game_id).await;
                    // Update last-connected timestamp.
                    touch_cache_entry(&payload.game_id);
                    SessionOutcome::Done {
                        exit_code,
                        notice: map_notice,
                    }
                }
                Err(e) => SessionOutcome::Error(format!(
                    "Connection to game server was lost.\n\
                     Contact your sysop if this persists.\n\
                     \n\
                     Technical: bridge error: {e}"
                )),
            }
        }
    }
}

async fn auto_fetch_maps(
    player_keys: &Keys,
    target: &ResolvedTarget,
    gate_npub: &str,
    payload: &SessionReadyPayload,
    maps_root: &std::path::Path,
) -> Option<String> {
    match fetch_map_bundle(player_keys, target, gate_npub, &payload.game_id).await {
        Ok(bundle) => save_map_bundle(&bundle, &target.server_host, target.server_port, maps_root)
            .err()
            .map(|err| format!("Warning: unable to save starmaps: {err}")),
        Err(err) => Some(format!("Warning: unable to download starmaps: {err}")),
    }
}

// ── Disambiguation ────────────────────────────────────────────────────────────

/// Print a numbered list of games and prompt the user to pick one.
/// Returns the selected `GameEntry`, or an error string.
fn prompt_game_selection(games: &[GameEntry]) -> Result<GameEntry, String> {
    use std::io::{self, BufRead, Write};

    // Print the list.
    eprintln!("Multiple games found on this server:");
    for (i, g) in games.iter().enumerate() {
        eprintln!("  {}. {} (Seat {})", i + 1, g.name, g.seat);
    }

    // Read selection.
    eprint!("Select [1-{}]: ", games.len());
    let _ = io::stderr().flush();

    let stdin = io::stdin();
    let line = stdin
        .lock()
        .lines()
        .next()
        .ok_or_else(|| "no input".to_string())?
        .map_err(|e| format!("read error: {e}"))?;

    let n: usize = line
        .trim()
        .parse()
        .map_err(|_| format!("invalid selection: '{}'", line.trim()))?;

    if n < 1 || n > games.len() {
        return Err(format!("selection {} out of range (1–{})", n, games.len()));
    }

    Ok(games[n - 1].clone())
}

// ── Cache helpers ─────────────────────────────────────────────────────────────

/// Upsert (or insert) a `CachedGame` from a successful `SessionReadyPayload`.
/// Silently ignores cache I/O errors so a cache write failure never kills a
/// session.
fn upsert_cache_entry(
    payload: &SessionReadyPayload,
    npub: &str,
    gate_npub: &str,
    target: &ResolvedTarget,
) {
    cache_joined_game(CachedGame {
        id: payload.game_id.clone(),
        name: payload.game_name.clone(),
        player_name: (!payload.player_name.is_empty()).then(|| payload.player_name.clone()),
        server: target.server_host.clone(),
        port: target.server_port,
        relay_url: Some(target.relay_url.clone()),
        seat: payload.seat,
        npub: npub.to_string(),
        gate_npub: gate_npub.to_string(),
        joined: now_iso8601(),
        last_connected: None,
    });
}

pub fn cache_joined_game(entry: CachedGame) {
    let Ok(mut cache) = load_cache() else { return };
    cache.upsert(entry);
    let _ = save_cache(&cache);
}

pub fn build_cached_game(
    game_id: &str,
    game_name: &str,
    player_name: Option<&str>,
    target: &ResolvedTarget,
    npub: &str,
    gate_npub: &str,
    seat: u32,
) -> CachedGame {
    CachedGame {
        id: game_id.to_string(),
        name: game_name.to_string(),
        player_name: player_name
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        server: target.server_host.clone(),
        port: target.server_port,
        relay_url: Some(target.relay_url.clone()),
        seat,
        npub: npub.to_string(),
        gate_npub: gate_npub.to_string(),
        joined: now_iso8601(),
        last_connected: None,
    }
}

/// Update `last-connected` for a game after the bridge session ends.
fn touch_cache_entry(game_id: &str) {
    let Ok(mut cache) = load_cache() else { return };
    cache.touch(game_id, &now_iso8601());
    let _ = save_cache(&cache);
}

async fn refresh_cache_metadata(
    player_keys: &Keys,
    target: &ResolvedTarget,
    gate_npub: &str,
    game_id: &str,
) {
    let Ok(state) = fetch_game_metadata(player_keys, target, gate_npub, game_id).await else {
        return;
    };
    let Ok(mut cache) = load_cache() else { return };
    if !cache.update_metadata(
        &state.game_id,
        &state.game_name,
        Some(state.player_name.as_str()),
        state.seat,
    ) {
        return;
    }
    let _ = save_cache(&cache);
}

// ── Gate npub resolution ──────────────────────────────────────────────────────

/// Resolve the gate's Nostr public key for a server.
///
/// Checks the explicit override first, then falls back to the cache.
/// Returns an error asking the user to supply `--gate <npub>` if neither
/// source has the information.
pub fn resolve_gate_npub(
    server_host: &str,
    cache: &GameCache,
    override_npub: Option<&str>,
) -> Result<String, String> {
    if let Some(npub) = override_npub {
        return Ok(npub.to_string());
    }
    if let Some(npub) = cache.gate_npub_for_server(server_host) {
        return Ok(npub.to_string());
    }
    Err(format!(
        "gate npub not known for {server_host}; supply --gate <npub>"
    ))
}
