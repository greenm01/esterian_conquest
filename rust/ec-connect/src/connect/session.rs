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

use std::path::PathBuf;

use nostr_sdk::Keys;

use crate::cache::{CachedGame, CachedGameStatus, GameCache, load_cache, save_cache};
use crate::connect::bridge::run_bridge;
use crate::connect::handshake::{GameEntry, HandshakeResult, SessionReadyPayload, run_handshake};
use crate::connect::map_fetch::fetch_map_bundle;
use crate::connect::resolve::ResolvedTarget;
use crate::connect::session_state::{SessionStatePayload, fetch_game_metadata};
use crate::connect::ssh_key::EphemeralKeypair;
use crate::map_store::save_map_bundle;
use crate::wallet::io::now_iso8601;

const HOSTED_ONBOARDING_INVARIANT_EXIT_CODE: u32 = 72;

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
        maps_saved_to: Option<PathBuf>,
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

/// Prepared session state ready to enter the SSH bridge.
pub struct PreparedSession {
    payload: SessionReadyPayload,
    keypair: EphemeralKeypair,
    post_bridge: PostBridgeAction,
}

pub struct PreparedLiveSession {
    pub payload: SessionReadyPayload,
    pub keypair: EphemeralKeypair,
}

pub struct PreparedSessionFinalizer {
    payload: SessionReadyPayload,
    post_bridge: PostBridgeAction,
}

/// Result of the pre-bridge session phase.
pub enum SessionPreparation {
    Ready(PreparedSession),
    Outcome(SessionOutcome),
}

enum PostBridgeAction {
    TouchCache,
    FinalizeFirstJoin {
        player_keys: Keys,
        target: ResolvedTarget,
        gate_npub: String,
        maps_root: PathBuf,
        npub: String,
    },
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
    match prepare_session_with_keypair(
        player_keys,
        target,
        username,
        gate_npub,
        disambig,
        maps_root,
        keypair,
    )
    .await
    {
        SessionPreparation::Ready(prepared) => finish_prepared_session(prepared, username).await,
        SessionPreparation::Outcome(outcome) => outcome,
    }
}

pub async fn prepare_session(
    player_keys: &Keys,
    target: ResolvedTarget,
    username: &str,
    gate_npub: &str,
    disambig: DisambigMode,
    maps_root: &std::path::Path,
) -> SessionPreparation {
    let keypair = EphemeralKeypair::generate();
    prepare_session_with_keypair(
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

async fn prepare_session_with_keypair(
    player_keys: &Keys,
    mut target: ResolvedTarget,
    username: &str,
    gate_npub: &str,
    disambig: DisambigMode,
    maps_root: &std::path::Path,
    keypair: EphemeralKeypair,
) -> SessionPreparation {
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
        Err(e) => {
            return SessionPreparation::Outcome(SessionOutcome::Error(format!(
                "Could not reach the game server. Contact your sysop if this persists.\n\nTechnical: handshake failed: {e}"
            )));
        }
    };

    match result {
        HandshakeResult::Timeout => SessionPreparation::Outcome(SessionOutcome::Timeout),

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
                                Box::pin(prepare_session_with_keypair(
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
                            Err(msg) => SessionPreparation::Outcome(SessionOutcome::Error(msg)),
                        }
                    }
                    DisambigMode::Picker => {
                        // Return control to the picker so it can show a
                        // game-selection screen and retry with the chosen id.
                        SessionPreparation::Outcome(SessionOutcome::NeedsDisambiguation {
                            games: err.games,
                        })
                    }
                }
            } else if err.error == "unknown_player"
                && target.invite_code.is_none()
                && target.game_id.is_some()
            {
                SessionPreparation::Outcome(SessionOutcome::Error(
                    unfinished_first_join_error_message().to_string(),
                ))
            } else if err.error == "identity_already_in_game" {
                SessionPreparation::Outcome(SessionOutcome::Error(
                    duplicate_identity_seat_message().to_string(),
                ))
            } else {
                SessionPreparation::Outcome(SessionOutcome::Error(format!(
                    "{}: {}",
                    err.error, err.message
                )))
            }
        }

        HandshakeResult::Ready(payload) => {
            let post_bridge = if first_join {
                upsert_pending_cache_entry(&payload, username, gate_npub, &target);
                PostBridgeAction::FinalizeFirstJoin {
                    player_keys: player_keys.clone(),
                    target: target.clone(),
                    gate_npub: gate_npub.to_string(),
                    maps_root: maps_root.to_path_buf(),
                    npub: username.to_string(),
                }
            } else {
                upsert_cache_entry(&payload, username, gate_npub, &target);
                PostBridgeAction::TouchCache
            };
            SessionPreparation::Ready(PreparedSession {
                payload,
                keypair,
                post_bridge,
            })
        }
    }
}

pub async fn finish_prepared_session(prepared: PreparedSession, username: &str) -> SessionOutcome {
    let (live, finalizer) = prepared.split();
    let outcome = run_bridge(&live.payload, &live.keypair, username).await;
    finalizer.finish(outcome).await
}

impl PreparedSession {
    pub fn split(self) -> (PreparedLiveSession, PreparedSessionFinalizer) {
        (
            PreparedLiveSession {
                payload: self.payload.clone(),
                keypair: self.keypair,
            },
            PreparedSessionFinalizer {
                payload: self.payload,
                post_bridge: self.post_bridge,
            },
        )
    }
}

impl PreparedSessionFinalizer {
    pub async fn finish(
        self,
        bridge_result: Result<u32, crate::connect::bridge::BridgeError>,
    ) -> SessionOutcome {
        let PreparedSessionFinalizer {
            payload,
            post_bridge,
        } = self;
        match bridge_result {
            Ok(exit_code) => {
                if exit_code == HOSTED_ONBOARDING_INVARIANT_EXIT_CODE {
                    return SessionOutcome::Error(
                        hosted_onboarding_invariant_message().to_string(),
                    );
                }
                let completion = match post_bridge {
                    PostBridgeAction::TouchCache => {
                        touch_cache_entry(&payload.game_id);
                        FirstJoinCompletion::default()
                    }
                    PostBridgeAction::FinalizeFirstJoin {
                        player_keys,
                        target,
                        gate_npub,
                        maps_root,
                        npub,
                    } => {
                        finalize_first_join_after_session(
                            &payload,
                            &player_keys,
                            &target,
                            &gate_npub,
                            &maps_root,
                            &npub,
                        )
                        .await
                    }
                };
                SessionOutcome::Done {
                    exit_code,
                    notice: completion.notice,
                    maps_saved_to: completion.maps_saved_to,
                }
            }
            Err(err) => SessionOutcome::Error(format_bridge_error_message(
                &payload.ssh_host,
                &err.to_string(),
            )),
        }
    }
}

pub fn format_bridge_error_message(ssh_host: &str, err: &str) -> String {
    if is_local_ssh_target(ssh_host) && err.contains("SSH public-key authentication failed") {
        return format!(
            "Could not authenticate to the local game server over SSH. For localhost testing, your local hosted helper may be using the wrong SSH user or auth-keys path.\n\nTechnical: bridge error: {err}"
        );
    }

    format!(
        "Connection to game server was lost. Contact your sysop if this persists.\n\nTechnical: bridge error: {err}"
    )
}

pub fn unfinished_first_join_error_message() -> &'static str {
    "This identity is not enrolled in that game yet. If this was a first-time join, you left before naming your empire. Use the invite code again to finish joining."
}

pub fn hosted_onboarding_invariant_message() -> &'static str {
    "Hosted join failed before empire naming. The game server sent this player to the wrong first-time screen. Retry the invite. If this keeps happening, contact your sysop."
}

pub fn duplicate_identity_seat_message() -> &'static str {
    "This identity already claimed another seat in that game. Reconnect with the claimed seat instead of using a second invite."
}

pub fn is_unfinished_first_join_error(message: &str) -> bool {
    message.trim() == unfinished_first_join_error_message()
}

fn is_local_ssh_target(ssh_host: &str) -> bool {
    matches!(ssh_host.trim(), "localhost" | "127.0.0.1" | "::1" | "[::1]")
}

#[derive(Debug, Default)]
struct FirstJoinCompletion {
    notice: Option<String>,
    maps_saved_to: Option<PathBuf>,
}

async fn finalize_first_join_after_session(
    payload: &SessionReadyPayload,
    player_keys: &Keys,
    target: &ResolvedTarget,
    gate_npub: &str,
    maps_root: &std::path::Path,
    npub: &str,
) -> FirstJoinCompletion {
    let refreshed_state = fetch_game_metadata(player_keys, target, gate_npub, &payload.game_id)
        .await
        .ok();
    let Some(state) = refreshed_state.as_ref() else {
        return FirstJoinCompletion::default();
    };

    merge_session_state_into_cache(state, target, npub, gate_npub);
    touch_cache_entry(&state.game_id);
    match fetch_map_bundle(player_keys, target, gate_npub, &state.game_id).await {
        Ok(bundle) => match save_map_bundle(&bundle, &target.relay_url, maps_root) {
            Ok(path) => FirstJoinCompletion {
                notice: None,
                maps_saved_to: Some(path),
            },
            Err(err) => FirstJoinCompletion {
                notice: Some(format!("Warning: unable to save starmaps: {err}")),
                maps_saved_to: None,
            },
        },
        Err(err) => FirstJoinCompletion {
            notice: Some(format!("Warning: unable to download starmaps: {err}")),
            maps_saved_to: None,
        },
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
    cache_game(build_cached_game_from_ready_payload(
        payload,
        target,
        npub,
        gate_npub,
        &now_iso8601(),
    ));
}

fn upsert_pending_cache_entry(
    payload: &SessionReadyPayload,
    npub: &str,
    gate_npub: &str,
    target: &ResolvedTarget,
) {
    cache_game(build_pending_cached_game_from_ready_payload(
        payload,
        target,
        npub,
        gate_npub,
        &now_iso8601(),
    ));
}

pub fn cache_game(entry: CachedGame) {
    let Ok(mut cache) = load_cache() else { return };
    cache.upsert(entry);
    let _ = save_cache(&cache);
}

fn merge_session_state_into_cache(
    state: &SessionStatePayload,
    target: &ResolvedTarget,
    npub: &str,
    gate_npub: &str,
) {
    let Ok(mut cache) = load_cache() else { return };
    merge_session_state(&mut cache, state, target, npub, gate_npub, &now_iso8601());
    let _ = save_cache(&cache);
}

#[doc(hidden)]
pub fn merge_session_state(
    cache: &mut GameCache,
    state: &SessionStatePayload,
    target: &ResolvedTarget,
    npub: &str,
    gate_npub: &str,
    joined_if_missing: &str,
) {
    if cache.update_metadata(
        &state.game_id,
        &state.game_name,
        Some(&state.player_name),
        state.seat,
    ) {
        return;
    }

    cache.upsert(build_cached_game_with_joined_status(
        &state.game_id,
        &state.game_name,
        Some(&state.player_name),
        target,
        npub,
        gate_npub,
        state.seat,
        CachedGameStatus::Joined,
        None,
        joined_if_missing,
    ));
}

#[doc(hidden)]
pub fn build_cached_game_from_ready_payload(
    payload: &SessionReadyPayload,
    target: &ResolvedTarget,
    npub: &str,
    gate_npub: &str,
    joined: &str,
) -> CachedGame {
    build_cached_game_with_joined_status(
        &payload.game_id,
        &payload.game_name,
        Some(&payload.player_name),
        target,
        npub,
        gate_npub,
        payload.seat,
        CachedGameStatus::Joined,
        None,
        joined,
    )
}

pub fn build_pending_cached_game_from_ready_payload(
    payload: &SessionReadyPayload,
    target: &ResolvedTarget,
    npub: &str,
    gate_npub: &str,
    joined: &str,
) -> CachedGame {
    build_cached_game_with_joined_status(
        &payload.game_id,
        &payload.game_name,
        Some(&payload.player_name),
        target,
        npub,
        gate_npub,
        payload.seat,
        CachedGameStatus::Pending,
        target.invite_code.as_deref(),
        joined,
    )
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
    build_cached_game_with_joined_status(
        game_id,
        game_name,
        player_name,
        target,
        npub,
        gate_npub,
        seat,
        CachedGameStatus::Joined,
        None,
        &now_iso8601(),
    )
}

fn build_cached_game_with_joined_status(
    game_id: &str,
    game_name: &str,
    player_name: Option<&str>,
    target: &ResolvedTarget,
    npub: &str,
    gate_npub: &str,
    seat: u32,
    status: CachedGameStatus,
    invite_code: Option<&str>,
    joined: &str,
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
        status,
        invite_code: invite_code
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        joined: joined.to_string(),
        last_connected: None,
    }
}

/// Update `last-connected` for a game after the bridge session ends.
fn touch_cache_entry(game_id: &str) {
    let Ok(mut cache) = load_cache() else { return };
    cache.touch(game_id, &now_iso8601());
    let _ = save_cache(&cache);
}

// ── Gate npub resolution ──────────────────────────────────────────────────────

/// Resolve the gate's Nostr public key for a server.
///
/// Checks the explicit override first, then falls back to the cache.
/// Returns an error telling the user to reconnect from the picker or join with
/// an invite code if neither source has the information.
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
        "This server is not in your joined game list yet: {server_host}. Join it with an invite code first, or reconnect from the picker."
    ))
}
