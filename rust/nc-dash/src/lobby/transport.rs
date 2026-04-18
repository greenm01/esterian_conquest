use nc_client::cache::{
    CachedGame, ClientCache, ContactEntry, ContactMessageEntry, GameInboxMessageEntry,
    GameRosterEntry, NoticeEntry, cache_path, load_cache_from, save_cache, save_cache_to,
};
use nc_client::config::{ClientConfig, load_config};
use nc_client::contacts::{resolve_contact_input, short_contact_label};
use nc_client::hosted::live::{
    HostedLiveOptions, HostedLiveSession, HostedSessionStatus, HostedSessionUpdate,
};
use nc_client::hosted::session::{
    CatalogGame, HostedClientSession, HostedStateRequestError, PlayerEventBatch,
    SandboxJoinOutcome, compare_catalog_versions,
};
use nc_client::hosted::store::{CachedHostedDraft, HostedDraftStatus, HostedStateStore};
use nc_client::keychain::{
    Keychain, active_keys, load_keychain, now_iso8601, push_new_identity, save_keychain,
    set_active_handle,
};
use nc_client::password::validate_new_password;
use nc_client::relay::validate_relay_url;
use nc_data::TurnSubmission;
use nc_nostr::game_definition::{CatalogState, GameStatus, RecruitingMode};
use nc_nostr::handle_check::HandleCheckStatus;
use nc_nostr::invite_request::{InviteDecision, InviteDecisionPayload};
use nc_nostr::lobby_notice::LobbyNotice as NoticePayload;
use nc_nostr::pubkeys::hex_to_npub;
use nc_nostr::state_sync::{GameState, StateErrorCode, apply_state_delta};
use nc_nostr::turn_commands::{TurnReceipt, TurnReceiptStatus};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use super::models::{
    DirectContactRow, GameInboxMessage, GameInboxRow, JoinedGameRow, LobbyNotice, OpenGameRow,
    ThreadMessage,
};
use super::state::{LobbyNetworkStatus, LobbyStatusTone};
use crate::startup::NativeLaunchOptions;

const CACHE_RESET_MESSAGE: &str =
    "Local cache was reset. Relay data will repopulate after refresh.";
const CACHE_RESET_SAVE_FAILED_MESSAGE: &str = "Local cache was reset for this session, but it could not be saved. Relay data will repopulate after refresh.";
const HANDLE_SAVED_LOCAL_MESSAGE: &str =
    "Handle saved locally. It will be verified when you next contact nc-host.";
const HANDLE_UPDATED_LOCAL_MESSAGE: &str =
    "Handle updated locally. It will be verified when you next contact nc-host.";
const HANDLE_VERIFIED_MESSAGE: &str = "Handle verified with nc-host.";
const HANDLE_UPDATED_VERIFIED_MESSAGE: &str = "Handle updated and verified with nc-host.";
const REQUEST_BOOTSTRAP_LOOKBACK_SECS: u64 = 30 * 24 * 60 * 60;
const PASSIVE_POLL_INTERVAL: Duration = Duration::from_millis(100);

fn format_catalog_created_date(created_at: Option<i64>) -> String {
    created_at
        .and_then(|secs| chrono::DateTime::from_timestamp(secs, 0))
        .map(|dt| dt.date_naive().format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn format_host_label(
    host_alias: Option<&str>,
    host_contact_label: Option<&str>,
    host_contact_nip05: Option<&str>,
    host_contact_npub: Option<&str>,
) -> String {
    host_contact_label
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| host_contact_nip05.and_then(host_nip05_label))
        .or_else(|| host_contact_npub.map(short_contact_label))
        .or_else(|| {
            host_alias
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "daemon".to_string())
}

fn host_nip05_label(nip05: &str) -> Option<String> {
    let trimmed = nip05.trim();
    trimmed
        .split_once('@')
        .map(|(local, _)| local.trim())
        .filter(|local| !local.is_empty())
        .map(str::to_string)
}

fn open_game_status(game: &CatalogGame) -> (&'static str, u8) {
    if game.definition.status == GameStatus::Finished {
        ("Final", 2)
    } else if game.definition.recruiting != RecruitingMode::None {
        ("Open", 0)
    } else {
        ("Live", 1)
    }
}

fn game_tier_label(raw: Option<&str>) -> &'static str {
    match raw.unwrap_or("league") {
        "sandbox" => "Sandbox",
        _ => "League",
    }
}

fn joined_game_status_sort_key(status: &str) -> u8 {
    match status {
        "joined" => 0,
        "requested" | "approved" => 1,
        _ => 2,
    }
}

#[derive(Debug, Clone)]
pub struct LobbyLoadedState {
    pub relay_label: Option<String>,
    pub player_handle: Option<String>,
    pub joined_games: Vec<JoinedGameRow>,
    pub open_games: Vec<OpenGameRow>,
    pub game_inbox: Vec<GameInboxRow>,
    pub notices: Vec<LobbyNotice>,
    pub direct_contacts: Vec<DirectContactRow>,
    pub thread_messages: Vec<ThreadMessage>,
    pub game_inbox_messages: Vec<GameInboxMessage>,
    pub network_status: LobbyNetworkStatus,
    pub status_message: Option<String>,
    pub status_tone: LobbyStatusTone,
}

pub struct SandboxJoinSuccess {
    pub loaded: LobbyLoadedState,
    pub snapshot: GameState,
}

#[derive(Debug, Clone, Default)]
pub struct ActiveHostedGamePollUpdate {
    pub game_id: Option<String>,
    pub promoted_state_hash: Option<String>,
    pub turn_receipt: Option<TurnReceipt>,
}

#[derive(Debug, Clone)]
pub struct LobbyPollUpdate {
    pub loaded: LobbyLoadedState,
    pub active_game: ActiveHostedGamePollUpdate,
}

#[derive(Debug, Clone)]
pub struct SubmitTurnOutcome {
    pub loaded: LobbyLoadedState,
    pub receipt: TurnReceipt,
}

pub enum LobbySandboxJoinResult {
    Joined(SandboxJoinSuccess),
    Full(String),
}

#[derive(Debug)]
pub struct LobbyOpenGameError {
    pub code: Option<StateErrorCode>,
    pub message: String,
    pub loaded: Option<LobbyLoadedState>,
}

struct UnlockedClient {
    password: String,
    keychain: Keychain,
    cache: ClientCache,
    catalog: Vec<CatalogGame>,
    session: Option<HostedClientSession>,
    live_session: Option<HostedLiveSession>,
    bootstrap_rx: Option<Receiver<BootstrapResult>>,
    relay_url: Option<String>,
    network_status: LobbyNetworkStatus,
    cache_dirty: bool,
}

pub struct LobbyTransport {
    relay_override: Option<String>,
    disable_hosted_sessions: bool,
    disable_live_session: bool,
    disable_live_private_stream: bool,
    diagnostic_mode: bool,
    unlocked: Option<UnlockedClient>,
}

struct CacheLoadResult {
    cache: ClientCache,
    status_message: Option<String>,
    status_tone: LobbyStatusTone,
}

struct OpenGameContext {
    session: HostedClientSession,
    player_pubkey: String,
    handle: Option<String>,
    hosted_store: HostedStateStore,
    baseline: Option<GameState>,
    draft: Option<CachedHostedDraft>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenGamePersistOutcome {
    pub had_baseline: bool,
    pub had_draft: bool,
    pub cleared_stale_draft: bool,
}

enum HandleSaveMode {
    Verified,
    LocalOnly,
}

struct BootstrapResult {
    catalog: Vec<CatalogGame>,
    notices: Vec<NoticePayload>,
    network_status: LobbyNetworkStatus,
}

enum BootstrapPoll {
    Pending,
    Disconnected,
    Ready(BootstrapResult),
}

impl LobbyTransport {
    pub fn new(relay_override: Option<String>, native: NativeLaunchOptions) -> Self {
        Self {
            relay_override,
            disable_hosted_sessions: native.disable_hosted_sessions,
            disable_live_session: native.disable_live_session,
            disable_live_private_stream: native.disable_live_private_stream,
            diagnostic_mode: native.diagnostic_mode,
            unlocked: None,
        }
    }

    pub fn has_session(&self) -> bool {
        self.unlocked
            .as_ref()
            .and_then(|unlocked| unlocked.session.as_ref())
            .is_some()
    }

    pub fn is_unlocked(&self) -> bool {
        self.unlocked.is_some()
    }

    pub fn next_poll_deadline(&self, now: Instant) -> Option<Instant> {
        let unlocked = self.unlocked.as_ref()?;
        if unlocked.live_session.is_some() || unlocked.bootstrap_rx.is_some() {
            Some(now + PASSIVE_POLL_INTERVAL)
        } else {
            None
        }
    }

    pub fn flush_cache(&mut self) -> Result<(), String> {
        let Some(unlocked) = self.unlocked.as_mut() else {
            return Ok(());
        };
        if unlocked.cache_dirty {
            save_cache(&unlocked.cache, &unlocked.password).map_err(|err| err.to_string())?;
            unlocked.cache_dirty = false;
        }
        Ok(())
    }

    pub fn lock(&mut self) {
        self.unlocked = None;
    }

    pub fn create_identity(
        &mut self,
        handle: &str,
        password: &str,
        confirm: &str,
    ) -> Result<LobbyLoadedState, String> {
        validate_new_password(password, confirm)?;
        let mut keychain = Keychain::empty();
        push_new_identity(
            &mut keychain,
            now_iso8601(),
            Some(handle.trim().to_string()),
        )
        .map_err(|err| err.to_string())?;
        let cache = ClientCache::empty();
        let config = load_config().map_err(|err| err.to_string())?;
        let relay_url = effective_relay(self.relay_override.as_deref(), &config)?;
        let (session, live_session) = build_sessions(
            &keychain,
            relay_url.as_deref(),
            self.disable_hosted_sessions,
            self.disable_live_session,
            self.disable_live_private_stream,
        )
        .map_err(|err| err.to_string())?;
        let mut catalog = Vec::new();
        let handle_mode =
            validate_local_handle_before_save(session.as_ref(), &mut catalog, &cache, &keychain)?;
        save_keychain(&keychain, password).map_err(|err| err.to_string())?;
        save_cache(&cache, password).map_err(|err| err.to_string())?;
        let has_session = session.is_some();
        self.unlocked = Some(UnlockedClient {
            password: password.to_string(),
            keychain,
            cache,
            catalog,
            session,
            live_session,
            bootstrap_rx: None,
            relay_url,
            network_status: initial_network_status_from_session(has_session),
            cache_dirty: false,
        });
        if let Some(unlocked) = self.unlocked.as_mut() {
            start_bootstrap_worker(unlocked, self.diagnostic_mode);
            Ok(build_loaded_state(
                unlocked,
                Some(match handle_mode {
                    HandleSaveMode::Verified => HANDLE_VERIFIED_MESSAGE.to_string(),
                    HandleSaveMode::LocalOnly => HANDLE_SAVED_LOCAL_MESSAGE.to_string(),
                }),
                LobbyStatusTone::Success,
                Some(unlocked.network_status),
            ))
        } else {
            Err("keychain setup failed".to_string())
        }
    }

    pub fn unlock(&mut self, password: &str) -> Result<LobbyLoadedState, String> {
        let keychain = load_keychain(password)
            .map_err(|err| err.to_string())?
            .ok_or_else(|| "no keychain found".to_string())?;
        let cache_result = load_cache_with_recovery(password, &cache_path());
        let config = load_config().map_err(|err| err.to_string())?;
        let relay_url = effective_relay(self.relay_override.as_deref(), &config)?;
        let (session, live_session) = build_sessions(
            &keychain,
            relay_url.as_deref(),
            self.disable_hosted_sessions,
            self.disable_live_session,
            self.disable_live_private_stream,
        )
        .map_err(|err| err.to_string())?;
        let has_session = session.is_some();
        self.unlocked = Some(UnlockedClient {
            password: password.to_string(),
            keychain,
            cache: cache_result.cache,
            catalog: Vec::new(),
            session,
            live_session,
            bootstrap_rx: None,
            relay_url,
            network_status: initial_network_status_from_session(has_session),
            cache_dirty: false,
        });
        if let Some(unlocked) = self.unlocked.as_mut() {
            start_bootstrap_worker(unlocked, self.diagnostic_mode);
            Ok(build_loaded_state(
                unlocked,
                cache_result.status_message,
                cache_result.status_tone,
                Some(unlocked.network_status),
            ))
        } else {
            Err("keychain unlock failed".to_string())
        }
    }

    pub fn refresh(&mut self) -> Result<LobbyLoadedState, String> {
        let unlocked = self
            .unlocked
            .as_mut()
            .ok_or_else(|| "keychain is locked".to_string())?;
        if unlocked.live_session.is_none() {
            start_bootstrap_worker(unlocked, self.diagnostic_mode);
        }
        let bootstrap_changed = poll_bootstrap_updates(unlocked);
        let live_changed = apply_live_updates(unlocked, None);
        if bootstrap_changed || live_changed.is_some() {
            save_cache(&unlocked.cache, &unlocked.password).map_err(|err| err.to_string())?;
            return Ok(build_loaded_state(
                unlocked,
                None,
                LobbyStatusTone::Info,
                Some(unlocked.network_status),
            ));
        }
        if let Some(live_session) = unlocked.live_session.as_ref() {
            unlocked.network_status = LobbyNetworkStatus::Refreshing;
            live_session.refresh_backfill();
            return Ok(build_loaded_state(
                unlocked,
                None,
                LobbyStatusTone::Info,
                Some(unlocked.network_status),
            ));
        }
        Ok(build_loaded_state(
            unlocked,
            None,
            LobbyStatusTone::Info,
            None,
        ))
    }

    pub fn poll_updates(
        &mut self,
        active_game_id: Option<&str>,
    ) -> Result<Option<LobbyPollUpdate>, String> {
        let Some(unlocked) = self.unlocked.as_mut() else {
            return Ok(None);
        };
        let bootstrap_changed = poll_bootstrap_updates(unlocked);
        let active_game = apply_live_updates(unlocked, active_game_id);
        if !bootstrap_changed && active_game.is_none() {
            return Ok(None);
        }
        unlocked.cache_dirty = true;
        Ok(Some(LobbyPollUpdate {
            loaded: build_loaded_state(
                unlocked,
                None,
                LobbyStatusTone::Info,
                Some(unlocked.network_status),
            ),
            active_game: active_game.unwrap_or_default(),
        }))
    }

    pub fn save_handle(&mut self, handle: &str) -> Result<LobbyLoadedState, String> {
        let unlocked = self
            .unlocked
            .as_mut()
            .ok_or_else(|| "keychain is locked".to_string())?;
        let handle_mode = validate_unlocked_handle_before_save(unlocked, handle)?;
        set_active_handle(&mut unlocked.keychain, Some(handle.trim().to_string()))?;
        save_keychain(&unlocked.keychain, &unlocked.password).map_err(|err| err.to_string())?;
        Ok(build_loaded_state(
            unlocked,
            Some(match handle_mode {
                HandleSaveMode::Verified => HANDLE_UPDATED_VERIFIED_MESSAGE.to_string(),
                HandleSaveMode::LocalOnly => HANDLE_UPDATED_LOCAL_MESSAGE.to_string(),
            }),
            LobbyStatusTone::Success,
            None,
        ))
    }

    pub fn send_invite_request(
        &mut self,
        row: &OpenGameRow,
        message: &str,
    ) -> Result<LobbyLoadedState, String> {
        let unlocked = self
            .unlocked
            .as_mut()
            .ok_or_else(|| "keychain is locked".to_string())?;
        let session = unlocked
            .session
            .as_ref()
            .ok_or_else(|| "no relay configured for the hosted lobby".to_string())?;
        preflight_handle_check(
            session,
            &row.daemon_pubkey,
            current_handle(&unlocked.keychain).as_deref(),
        )?;
        let handle = current_handle(&unlocked.keychain);
        session
            .send_invite_request(&row.game_id, &row.daemon_pubkey, message, handle.as_deref())
            .map_err(|err| err.to_string())?;
        let updated_at = now_iso8601();
        unlocked.cache.upsert_game(CachedGame {
            id: row.game_id.clone(),
            name: row.game.clone(),
            game_tier: Some(row.game_tier.to_ascii_lowercase()),
            host_alias: Some(row.host.clone()),
            host_contact_npub: row.host_contact_npub.clone(),
            host_contact_label: Some(row.host.clone()),
            host_contact_nip05: None,
            relay_url: row.relay_url.clone(),
            daemon_pubkey: row.daemon_pubkey.clone(),
            seat: None,
            status: "requested".to_string(),
            invite_address: None,
            last_turn: None,
            last_hash: None,
            updated_at,
        });
        save_cache(&unlocked.cache, &unlocked.password).map_err(|err| err.to_string())?;
        Ok(build_loaded_state(
            unlocked,
            Some("Join request sent. Waiting for nc-host receipt.".to_string()),
            LobbyStatusTone::Success,
            None,
        ))
    }

    pub fn join_sandbox_game(
        &mut self,
        row: &OpenGameRow,
    ) -> Result<LobbySandboxJoinResult, String> {
        let unlocked = self
            .unlocked
            .as_mut()
            .ok_or_else(|| "keychain is locked".to_string())?;
        let session = unlocked
            .session
            .as_ref()
            .ok_or_else(|| "no relay configured for the hosted lobby".to_string())?;
        let handle = current_handle(&unlocked.keychain);
        let player_pubkey =
            current_player_pubkey(&unlocked.keychain).map_err(|err| err.to_string())?;
        match session
            .join_sandbox_game(&row.game_id, &row.daemon_pubkey, handle.as_deref())
            .map_err(|err| err.to_string())?
        {
            SandboxJoinOutcome::Full(message) => Ok(LobbySandboxJoinResult::Full(message)),
            SandboxJoinOutcome::Joined(snapshot) => {
                unlocked.cache.upsert_game(CachedGame {
                    id: row.game_id.clone(),
                    name: row.game.clone(),
                    game_tier: Some(row.game_tier.to_ascii_lowercase()),
                    host_alias: Some(row.host.clone()),
                    host_contact_npub: row.host_contact_npub.clone(),
                    host_contact_label: Some(row.host.clone()),
                    host_contact_nip05: None,
                    relay_url: row.relay_url.clone(),
                    daemon_pubkey: row.daemon_pubkey.clone(),
                    seat: Some(u32::from(snapshot.player_seat)),
                    status: "joined".to_string(),
                    invite_address: None,
                    last_turn: Some(snapshot.turn),
                    last_hash: Some(snapshot.state_hash.clone()),
                    updated_at: now_iso8601(),
                });
                unlocked.cache.replace_roster(
                    &snapshot.game_id,
                    snapshot
                        .state
                        .roster
                        .iter()
                        .map(|entry| GameRosterEntry {
                            game_id: snapshot.game_id.clone(),
                            empire_id: entry.empire_id,
                            empire_name: entry.empire_name.clone(),
                            is_self: entry.is_self,
                        })
                        .collect(),
                );
                open_hosted_state_store()
                    .and_then(|store| {
                        store.save_snapshot(&unlocked.password, &player_pubkey, &snapshot)
                    })
                    .map_err(|err| err.to_string())?;
                save_cache(&unlocked.cache, &unlocked.password).map_err(|err| err.to_string())?;
                let loaded = build_loaded_state(
                    unlocked,
                    Some("Sandbox joined. Opening hosted dashboard.".to_string()),
                    LobbyStatusTone::Success,
                    None,
                );
                Ok(LobbySandboxJoinResult::Joined(SandboxJoinSuccess {
                    loaded,
                    snapshot,
                }))
            }
        }
    }

    pub fn open_game(&mut self, row: &JoinedGameRow) -> Result<GameState, LobbyOpenGameError> {
        let unlocked = self.unlocked.as_mut().ok_or_else(|| LobbyOpenGameError {
            code: None,
            message: "keychain is locked".to_string(),
            loaded: None,
        })?;
        let context = prepare_open_game(unlocked, row)?;
        let state = fetch_open_game_state(&context, row)
            .map_err(|err| lobby_open_game_error(unlocked, row, err))?;
        persist_open_game_state(&context, &unlocked.password, row, &state).map_err(|err| {
            LobbyOpenGameError {
                code: None,
                message: err.to_string(),
                loaded: None,
            }
        })?;
        update_open_game_cache(unlocked, row, &state).map_err(|err| LobbyOpenGameError {
            code: None,
            message: err.to_string(),
            loaded: None,
        })?;
        Ok(state)
    }

    #[doc(hidden)]
    pub fn open_game_fetch_only(&mut self, row: &JoinedGameRow) -> Result<GameState, String> {
        let unlocked = self
            .unlocked
            .as_mut()
            .ok_or_else(|| "keychain is locked".to_string())?;
        let context = prepare_open_game(unlocked, row).map_err(|err| err.message)?;
        fetch_open_game_state(&context, row).map_err(|err| err.message)
    }

    #[doc(hidden)]
    pub fn persist_open_game_state_for_repro(
        &mut self,
        row: &JoinedGameRow,
        state: &GameState,
    ) -> Result<OpenGamePersistOutcome, String> {
        let unlocked = self
            .unlocked
            .as_mut()
            .ok_or_else(|| "keychain is locked".to_string())?;
        let context = prepare_open_game(unlocked, row).map_err(|err| err.message)?;
        persist_open_game_state(&context, &unlocked.password, row, state)
            .map_err(|err| err.to_string())
    }

    #[doc(hidden)]
    pub fn update_open_game_cache_for_repro(
        &mut self,
        row: &JoinedGameRow,
        state: &GameState,
    ) -> Result<(), String> {
        let unlocked = self
            .unlocked
            .as_mut()
            .ok_or_else(|| "keychain is locked".to_string())?;
        update_open_game_cache(unlocked, row, state).map_err(|err| err.to_string())
    }

    pub fn complete_first_join_setup(
        &mut self,
        row: &JoinedGameRow,
        empire_name: &str,
        homeworld_name: &str,
    ) -> Result<GameState, String> {
        let unlocked = self
            .unlocked
            .as_mut()
            .ok_or_else(|| "keychain is locked".to_string())?;
        let session = unlocked
            .session
            .as_ref()
            .ok_or_else(|| "no relay configured for the hosted lobby".to_string())?;
        let player_pubkey =
            current_player_pubkey(&unlocked.keychain).map_err(|err| err.to_string())?;
        let state = session
            .complete_first_join_setup(
                &row.game_id,
                &row.daemon_pubkey,
                empire_name,
                homeworld_name,
            )
            .map_err(|err| err.to_string())?;
        open_hosted_state_store()
            .and_then(|store| store.save_snapshot(&unlocked.password, &player_pubkey, &state))
            .map_err(|err| err.to_string())?;
        let mut cached = cache_game_from_row(row);
        cached.status = "joined".to_string();
        cached.last_turn = Some(state.turn);
        cached.last_hash = Some(state.state_hash.clone());
        cached.updated_at = now_iso8601();
        unlocked.cache.upsert_game(cached);
        unlocked.cache.replace_roster(
            &state.game_id,
            state
                .state
                .roster
                .iter()
                .map(|entry| GameRosterEntry {
                    game_id: state.game_id.clone(),
                    empire_id: entry.empire_id,
                    empire_name: entry.empire_name.clone(),
                    is_self: entry.is_self,
                })
                .collect(),
        );
        save_cache(&unlocked.cache, &unlocked.password).map_err(|err| err.to_string())?;
        Ok(state)
    }

    pub fn submit_turn(
        &mut self,
        row: &JoinedGameRow,
        turn: u32,
        commands: &str,
    ) -> Result<SubmitTurnOutcome, String> {
        let unlocked = self
            .unlocked
            .as_mut()
            .ok_or_else(|| "keychain is locked".to_string())?;
        let session = unlocked
            .session
            .as_ref()
            .ok_or_else(|| "no relay configured for the hosted lobby".to_string())?;
        let handle = current_handle(&unlocked.keychain);
        let player_pubkey =
            current_player_pubkey(&unlocked.keychain).map_err(|err| err.to_string())?;
        let receipt = session
            .submit_turn(
                &row.game_id,
                &row.daemon_pubkey,
                turn,
                commands,
                handle.as_deref(),
            )
            .map_err(|err| err.to_string())?;
        let hosted_store = open_hosted_state_store().map_err(|err| err.to_string())?;
        let parsed_commands = TurnSubmission::parse_kdl_str(commands).ok();
        let draft_status = match receipt.status {
            TurnReceiptStatus::Accepted | TurnReceiptStatus::Superseded => {
                HostedDraftStatus::SubmittedPending
            }
            TurnReceiptStatus::Rejected
            | TurnReceiptStatus::NotClaimed
            | TurnReceiptStatus::WrongTurn => HostedDraftStatus::Local,
        };
        let draft_submit_id = matches!(
            receipt.status,
            TurnReceiptStatus::Accepted | TurnReceiptStatus::Superseded
        )
        .then_some(receipt.submit_id.as_str());
        if let Some(draft) = parsed_commands {
            hosted_store
                .save_draft(
                    &unlocked.password,
                    &player_pubkey,
                    &row.game_id,
                    row.last_hash.as_deref().unwrap_or(""),
                    &draft,
                    draft_status,
                    draft_submit_id,
                )
                .map_err(|err| err.to_string())?;
        } else if let Some(draft) = hosted_store
            .load_draft(&unlocked.password, &player_pubkey, &row.game_id)
            .map_err(|err| err.to_string())?
        {
            hosted_store
                .save_draft(
                    &unlocked.password,
                    &player_pubkey,
                    &row.game_id,
                    &draft.base_hash,
                    &draft.draft,
                    draft_status,
                    draft_submit_id.or(draft.submit_id.as_deref()),
                )
                .map_err(|err| err.to_string())?;
        }
        let (status_tone, status_message) = match receipt.status {
            TurnReceiptStatus::Accepted => (
                LobbyStatusTone::Success,
                "Turn receipt accepted. Waiting for refreshed state.".to_string(),
            ),
            TurnReceiptStatus::Superseded => (
                LobbyStatusTone::Info,
                "Turn receipt superseded. Waiting for refreshed state.".to_string(),
            ),
            TurnReceiptStatus::Rejected => (
                LobbyStatusTone::Error,
                receipt
                    .message
                    .clone()
                    .unwrap_or_else(|| "Turn submission was rejected by nc-host.".to_string()),
            ),
            TurnReceiptStatus::NotClaimed => (
                LobbyStatusTone::Error,
                receipt.message.clone().unwrap_or_else(|| {
                    "Turn submission was rejected because the seat is no longer claimed."
                        .to_string()
                }),
            ),
            TurnReceiptStatus::WrongTurn => (
                LobbyStatusTone::Error,
                receipt.message.clone().unwrap_or_else(|| {
                    "Turn submission was rejected because the turn is no longer current."
                        .to_string()
                }),
            ),
        };
        Ok(SubmitTurnOutcome {
            loaded: build_loaded_state(unlocked, Some(status_message), status_tone, None),
            receipt,
        })
    }

    pub fn load_cached_hosted_snapshot(&self, game_id: &str) -> Result<Option<GameState>, String> {
        let unlocked = self
            .unlocked
            .as_ref()
            .ok_or_else(|| "keychain is locked".to_string())?;
        let player_pubkey =
            current_player_pubkey(&unlocked.keychain).map_err(|err| err.to_string())?;
        open_hosted_state_store()
            .and_then(|store| store.load_snapshot(&unlocked.password, &player_pubkey, game_id))
            .map(|snapshot| snapshot.map(|cached| cached.snapshot))
            .map_err(|err| err.to_string())
    }

    pub fn load_hosted_draft(&self, game_id: &str) -> Result<Option<CachedHostedDraft>, String> {
        let unlocked = self
            .unlocked
            .as_ref()
            .ok_or_else(|| "keychain is locked".to_string())?;
        let player_pubkey =
            current_player_pubkey(&unlocked.keychain).map_err(|err| err.to_string())?;
        open_hosted_state_store()
            .and_then(|store| store.load_draft(&unlocked.password, &player_pubkey, game_id))
            .map_err(|err| err.to_string())
    }

    pub fn save_hosted_draft(
        &mut self,
        game_id: &str,
        base_hash: &str,
        draft: &TurnSubmission,
        status: HostedDraftStatus,
    ) -> Result<(), String> {
        let unlocked = self
            .unlocked
            .as_ref()
            .ok_or_else(|| "keychain is locked".to_string())?;
        let player_pubkey =
            current_player_pubkey(&unlocked.keychain).map_err(|err| err.to_string())?;
        open_hosted_state_store()
            .and_then(|store| {
                store.save_draft(
                    &unlocked.password,
                    &player_pubkey,
                    game_id,
                    base_hash,
                    draft,
                    status,
                    None,
                )
            })
            .map_err(|err| err.to_string())
    }

    pub fn clear_hosted_draft(&mut self, game_id: &str) -> Result<(), String> {
        let unlocked = self
            .unlocked
            .as_ref()
            .ok_or_else(|| "keychain is locked".to_string())?;
        let player_pubkey =
            current_player_pubkey(&unlocked.keychain).map_err(|err| err.to_string())?;
        open_hosted_state_store()
            .and_then(|store| store.clear_draft(&player_pubkey, game_id))
            .map_err(|err| err.to_string())
    }

    pub fn add_direct_contact(
        &mut self,
        raw_input: &str,
    ) -> Result<(LobbyLoadedState, String), String> {
        let unlocked = self
            .unlocked
            .as_mut()
            .ok_or_else(|| "keychain is locked".to_string())?;
        let resolved = resolve_contact_input(raw_input)?;
        unlocked.cache.upsert_contact(ContactEntry {
            npub: resolved.npub.clone(),
            label: resolved.label.clone(),
            nip05: resolved.nip05.clone(),
            source: "manual".to_string(),
            blocked: false,
            hidden: false,
            unread_count: 0,
            last_activity_at: None,
        });
        save_cache(&unlocked.cache, &unlocked.password).map_err(|err| err.to_string())?;
        Ok((
            build_loaded_state(
                unlocked,
                Some(format!("Added direct contact {}.", resolved.label)),
                LobbyStatusTone::Success,
                None,
            ),
            resolved.npub,
        ))
    }

    pub fn send_direct_message(
        &mut self,
        contact_npub: &str,
        body: &str,
    ) -> Result<LobbyLoadedState, String> {
        let unlocked = self
            .unlocked
            .as_mut()
            .ok_or_else(|| "keychain is locked".to_string())?;
        let session = unlocked
            .session
            .as_ref()
            .ok_or_else(|| "no relay configured for the hosted lobby".to_string())?;
        let sender_label = current_handle(&unlocked.keychain);
        let payload = session
            .send_contact_message(contact_npub, body, sender_label.as_deref())
            .map_err(|err| err.to_string())?;
        unlocked.cache.upsert_contact_message(ContactMessageEntry {
            message_id: payload.message_id,
            contact_npub: contact_npub.to_string(),
            sender_npub: payload.sender_npub,
            sender_label: payload.sender_label,
            body: payload.body,
            outgoing: true,
            created_at: iso8601_from_secs(payload.created_at),
        });
        let created_at = iso8601_from_secs(payload.created_at);
        unlocked
            .cache
            .note_contact_activity(contact_npub, &created_at, 0);
        unlocked.cache.mark_contact_read(contact_npub);
        save_cache(&unlocked.cache, &unlocked.password).map_err(|err| err.to_string())?;
        Ok(build_loaded_state(
            unlocked,
            None,
            LobbyStatusTone::Info,
            None,
        ))
    }

    pub fn mark_direct_contact_read(
        &mut self,
        contact_npub: &str,
    ) -> Result<LobbyLoadedState, String> {
        let unlocked = self
            .unlocked
            .as_mut()
            .ok_or_else(|| "keychain is locked".to_string())?;
        unlocked.cache.mark_contact_read(contact_npub);
        save_cache(&unlocked.cache, &unlocked.password).map_err(|err| err.to_string())?;
        Ok(build_loaded_state(
            unlocked,
            None,
            LobbyStatusTone::Info,
            None,
        ))
    }

    pub fn set_direct_contact_blocked(
        &mut self,
        contact_npub: &str,
        blocked: bool,
    ) -> Result<LobbyLoadedState, String> {
        let unlocked = self
            .unlocked
            .as_mut()
            .ok_or_else(|| "keychain is locked".to_string())?;
        unlocked.cache.set_contact_blocked(contact_npub, blocked);
        save_cache(&unlocked.cache, &unlocked.password).map_err(|err| err.to_string())?;
        Ok(build_loaded_state(
            unlocked,
            Some(if blocked {
                "Direct contact blocked locally.".to_string()
            } else {
                "Direct contact restored locally.".to_string()
            }),
            LobbyStatusTone::Success,
            None,
        ))
    }

    pub fn set_direct_contact_hidden(
        &mut self,
        contact_npub: &str,
        hidden: bool,
    ) -> Result<LobbyLoadedState, String> {
        let unlocked = self
            .unlocked
            .as_mut()
            .ok_or_else(|| "keychain is locked".to_string())?;
        unlocked.cache.set_contact_hidden(contact_npub, hidden);
        save_cache(&unlocked.cache, &unlocked.password).map_err(|err| err.to_string())?;
        Ok(build_loaded_state(
            unlocked,
            Some(if hidden {
                "Conversation hidden locally.".to_string()
            } else {
                "Conversation restored locally.".to_string()
            }),
            LobbyStatusTone::Success,
            None,
        ))
    }

    pub fn send_game_inbox_message(
        &mut self,
        row: &GameInboxRow,
        daemon_pubkey: &str,
        body: &str,
    ) -> Result<LobbyLoadedState, String> {
        let unlocked = self
            .unlocked
            .as_mut()
            .ok_or_else(|| "keychain is locked".to_string())?;
        let session = unlocked
            .session
            .as_ref()
            .ok_or_else(|| "no relay configured for the hosted lobby".to_string())?;
        let payload = session
            .send_player_message(&row.game_id, daemon_pubkey, row.other_empire_id, body)
            .map_err(|err| err.to_string())?;
        unlocked
            .cache
            .upsert_game_inbox_message(GameInboxMessageEntry {
                message_id: payload.message_id,
                game_id: row.game_id.clone(),
                other_empire_id: row.other_empire_id,
                other_empire_name: row.other_empire_name.clone(),
                sender_empire_id: 0,
                sender_empire_name: unlocked
                    .cache
                    .rosters
                    .iter()
                    .find(|entry| entry.game_id == row.game_id && entry.is_self)
                    .map(|entry| entry.empire_name.clone())
                    .unwrap_or_else(|| "You".to_string()),
                body: body.trim().to_string(),
                outgoing: true,
                created_at: iso8601_from_secs(payload.created_at),
            });
        save_cache(&unlocked.cache, &unlocked.password).map_err(|err| err.to_string())?;
        Ok(build_loaded_state(
            unlocked,
            None,
            LobbyStatusTone::Info,
            None,
        ))
    }
}

fn prepare_open_game(
    unlocked: &UnlockedClient,
    row: &JoinedGameRow,
) -> Result<OpenGameContext, LobbyOpenGameError> {
    let session = unlocked
        .session
        .as_ref()
        .cloned()
        .ok_or_else(|| LobbyOpenGameError {
            code: None,
            message: "no relay configured for the hosted lobby".to_string(),
            loaded: None,
        })?;
    let handle = current_handle(&unlocked.keychain);
    let player_pubkey =
        current_player_pubkey(&unlocked.keychain).map_err(|err| LobbyOpenGameError {
            code: None,
            message: err.to_string(),
            loaded: None,
        })?;
    let hosted_store = open_hosted_state_store().map_err(|err| LobbyOpenGameError {
        code: None,
        message: err.to_string(),
        loaded: None,
    })?;
    let baseline = hosted_store
        .load_snapshot(&unlocked.password, &player_pubkey, &row.game_id)
        .map_err(|err| LobbyOpenGameError {
            code: None,
            message: err.to_string(),
            loaded: None,
        })?
        .map(|cached| cached.snapshot);
    let draft = hosted_store
        .load_draft(&unlocked.password, &player_pubkey, &row.game_id)
        .map_err(|err| LobbyOpenGameError {
            code: None,
            message: err.to_string(),
            loaded: None,
        })?;
    Ok(OpenGameContext {
        session,
        player_pubkey,
        handle,
        hosted_store,
        baseline,
        draft,
    })
}

fn fetch_open_game_state(
    context: &OpenGameContext,
    row: &JoinedGameRow,
) -> Result<GameState, HostedStateRequestError> {
    context.session.request_state(
        &row.game_id,
        &row.daemon_pubkey,
        context.baseline.as_ref().map(|state| state.turn),
        context
            .baseline
            .as_ref()
            .map(|state| state.state_hash.as_str()),
        context.handle.as_deref(),
        context.baseline.as_ref(),
    )
}

fn persist_open_game_state(
    context: &OpenGameContext,
    password: &str,
    row: &JoinedGameRow,
    state: &GameState,
) -> Result<OpenGamePersistOutcome, Box<dyn std::error::Error>> {
    context
        .hosted_store
        .save_snapshot(password, &context.player_pubkey, state)?;
    let cleared_stale_draft = context
        .draft
        .as_ref()
        .is_some_and(|draft| state.turn > draft.turn);
    if cleared_stale_draft {
        context
            .hosted_store
            .clear_draft(&context.player_pubkey, &row.game_id)?;
    }
    Ok(OpenGamePersistOutcome {
        had_baseline: context.baseline.is_some(),
        had_draft: context.draft.is_some(),
        cleared_stale_draft,
    })
}

fn update_open_game_cache(
    unlocked: &mut UnlockedClient,
    row: &JoinedGameRow,
    state: &GameState,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cached = cache_game_from_row(row);
    cached.status = "joined".to_string();
    cached.last_turn = Some(state.turn);
    cached.last_hash = Some(state.state_hash.clone());
    cached.updated_at = now_iso8601();
    unlocked.cache.upsert_game(cached);
    apply_state_snapshot_to_cache(unlocked, state);
    save_cache(&unlocked.cache, &unlocked.password)?;
    Ok(())
}

fn load_cache_with_recovery(password: &str, path: &std::path::Path) -> CacheLoadResult {
    match load_cache_from(password, path) {
        Ok(Some(cache)) => CacheLoadResult {
            cache,
            status_message: None,
            status_tone: LobbyStatusTone::Info,
        },
        Ok(None) => CacheLoadResult {
            cache: ClientCache::empty(),
            status_message: None,
            status_tone: LobbyStatusTone::Info,
        },
        Err(_) => {
            let cache = ClientCache::empty();
            let status_message = match save_cache_to(&cache, password, path) {
                Ok(()) => CACHE_RESET_MESSAGE.to_string(),
                Err(_) => CACHE_RESET_SAVE_FAILED_MESSAGE.to_string(),
            };
            CacheLoadResult {
                cache,
                status_message: Some(status_message),
                status_tone: LobbyStatusTone::Error,
            }
        }
    }
}

fn validate_local_handle_before_save(
    session: Option<&HostedClientSession>,
    catalog: &mut Vec<CatalogGame>,
    cache: &ClientCache,
    keychain: &Keychain,
) -> Result<HandleSaveMode, String> {
    let Some(handle) = current_handle(keychain) else {
        return Ok(HandleSaveMode::LocalOnly);
    };
    let Some(session) = session else {
        return Ok(HandleSaveMode::LocalOnly);
    };
    if catalog.is_empty() {
        if let Ok(fetched) = session.fetch_catalog() {
            *catalog = fetched;
        }
    }
    let Some(daemon_pubkey) = resolve_single_host_daemon_pubkey(catalog, cache) else {
        return Ok(HandleSaveMode::LocalOnly);
    };
    validate_handle_with_host(session, &daemon_pubkey, &handle)
}

fn validate_unlocked_handle_before_save(
    unlocked: &mut UnlockedClient,
    handle: &str,
) -> Result<HandleSaveMode, String> {
    let Some(session) = unlocked.session.as_ref() else {
        return Ok(HandleSaveMode::LocalOnly);
    };
    if unlocked.catalog.is_empty() {
        if let Ok(fetched) = session.fetch_catalog() {
            unlocked.catalog = fetched;
        }
    }
    let Some(daemon_pubkey) = resolve_single_host_daemon_pubkey(&unlocked.catalog, &unlocked.cache)
    else {
        return Ok(HandleSaveMode::LocalOnly);
    };
    validate_handle_with_host(session, &daemon_pubkey, handle)
}

fn validate_handle_with_host(
    session: &HostedClientSession,
    daemon_pubkey: &str,
    handle: &str,
) -> Result<HandleSaveMode, String> {
    match session.check_handle(daemon_pubkey, handle) {
        Ok(result) => match result.status {
            HandleCheckStatus::Available | HandleCheckStatus::OwnedBySelf => {
                Ok(HandleSaveMode::Verified)
            }
            HandleCheckStatus::Taken => Err(result.message),
        },
        Err(_) => Ok(HandleSaveMode::LocalOnly),
    }
}

fn preflight_handle_check(
    session: &HostedClientSession,
    daemon_pubkey: &str,
    handle: Option<&str>,
) -> Result<(), String> {
    let Some(handle) = handle.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    match session.check_handle(daemon_pubkey, handle) {
        Ok(result) if result.status == HandleCheckStatus::Taken => Err(result.message),
        Ok(_) | Err(_) => Ok(()),
    }
}

fn resolve_single_host_daemon_pubkey(
    catalog: &[CatalogGame],
    cache: &ClientCache,
) -> Option<String> {
    let mut daemons = std::collections::BTreeSet::new();
    for game in catalog {
        if !game.daemon_pubkey.trim().is_empty() {
            daemons.insert(game.daemon_pubkey.clone());
        }
    }
    for game in &cache.games {
        if !game.daemon_pubkey.trim().is_empty() {
            daemons.insert(game.daemon_pubkey.clone());
        }
    }
    if daemons.len() == 1 {
        daemons.into_iter().next()
    } else {
        None
    }
}

fn effective_relay(
    relay_override: Option<&str>,
    config: &ClientConfig,
) -> Result<Option<String>, String> {
    if let Some(relay) = relay_override {
        return validate_relay_url(relay);
    }
    match config.default_relay_url() {
        Some(relay) => validate_relay_url(relay),
        None => Ok(None),
    }
}

fn current_handle(keychain: &Keychain) -> Option<String> {
    keychain
        .active_identity()
        .and_then(|identity| identity.handle.clone())
}

fn current_player_pubkey(keychain: &Keychain) -> Result<String, Box<dyn std::error::Error>> {
    Ok(active_keys(keychain)?.public_key().to_hex())
}

fn open_hosted_state_store() -> Result<HostedStateStore, Box<dyn std::error::Error>> {
    HostedStateStore::open_default()
}

fn build_sessions(
    keychain: &Keychain,
    relay_url: Option<&str>,
    disable_hosted_sessions: bool,
    disable_live_session: bool,
    disable_live_private_stream: bool,
) -> Result<(Option<HostedClientSession>, Option<HostedLiveSession>), Box<dyn std::error::Error>> {
    let Some(relay_url) = relay_url else {
        return Ok((None, None));
    };
    if disable_hosted_sessions {
        return Ok((None, None));
    }
    let keys = active_keys(keychain)?;
    let session = HostedClientSession::new(keys.clone(), relay_url.to_string());
    let live_session = if disable_live_session {
        None
    } else {
        Some(HostedLiveSession::start_with_options(
            keys,
            relay_url.to_string(),
            HostedLiveOptions {
                include_public_stream: true,
                include_private_stream: !disable_live_private_stream,
                enable_backfill: true,
            },
        ))
    };
    Ok((Some(session), live_session))
}

fn apply_live_updates(
    unlocked: &mut UnlockedClient,
    active_game_id: Option<&str>,
) -> Option<ActiveHostedGamePollUpdate> {
    let Some(live_session) = unlocked.live_session.as_ref() else {
        return None;
    };
    let updates = live_session.drain_updates();
    if updates.is_empty() {
        return None;
    }
    let mut active_game = ActiveHostedGamePollUpdate::default();
    for update in updates {
        apply_live_update(unlocked, update, active_game_id, &mut active_game);
    }
    Some(active_game)
}

fn start_bootstrap_worker(unlocked: &mut UnlockedClient, diagnostic_mode: bool) {
    if unlocked.bootstrap_rx.is_some() {
        return;
    }
    let Some(session) = unlocked.session.clone() else {
        return;
    };
    let (tx, rx) = mpsc::channel();
    unlocked.bootstrap_rx = Some(rx);
    thread::spawn(move || {
        let started = Instant::now();
        if diagnostic_mode {
            tracing::info!("lobby bootstrap worker started");
        }
        let result = fetch_bootstrap_result(session);
        if diagnostic_mode {
            tracing::info!(
                elapsed_ms = started.elapsed().as_millis() as u64,
                status = result.network_status.label(),
                "lobby bootstrap worker finished"
            );
        }
        let _ = tx.send(result);
    });
}

fn poll_bootstrap_updates(unlocked: &mut UnlockedClient) -> bool {
    match take_bootstrap_result(unlocked) {
        BootstrapPoll::Pending => false,
        BootstrapPoll::Disconnected => {
            let changed = unlocked.network_status != LobbyNetworkStatus::Error;
            unlocked.network_status = LobbyNetworkStatus::Error;
            changed
        }
        BootstrapPoll::Ready(result) => {
            let mut changed = unlocked.network_status != result.network_status;
            unlocked.network_status = result.network_status;
            if !result.catalog.is_empty() {
                apply_catalog(unlocked, &result.catalog);
                changed = true;
            }
            if !result.notices.is_empty() {
                apply_notices(unlocked, result.notices);
                changed = true;
            }
            changed
        }
    }
}

fn take_bootstrap_result(unlocked: &mut UnlockedClient) -> BootstrapPoll {
    let outcome = match unlocked.bootstrap_rx.as_ref().map(Receiver::try_recv) {
        Some(Ok(result)) => BootstrapPoll::Ready(result),
        Some(Err(TryRecvError::Disconnected)) => BootstrapPoll::Disconnected,
        Some(Err(TryRecvError::Empty)) | None => return BootstrapPoll::Pending,
    };
    unlocked.bootstrap_rx = None;
    outcome
}

fn fetch_bootstrap_result(session: HostedClientSession) -> BootstrapResult {
    let mut any_success = false;
    let mut saw_error = false;
    let mut catalog = Vec::new();
    let mut notices = Vec::new();

    match session.fetch_catalog() {
        Ok(fetched) => {
            catalog = fetched;
            any_success = true;
        }
        Err(err) => {
            eprintln!("warning: hosted request-session catalog bootstrap failed: {err}");
            saw_error = true;
        }
    }

    match session.fetch_lobby_notices(REQUEST_BOOTSTRAP_LOOKBACK_SECS) {
        Ok(fetched) => {
            notices = fetched;
            any_success = true;
        }
        Err(err) => {
            eprintln!("warning: hosted request-session notice bootstrap failed: {err}");
            saw_error = true;
        }
    }

    BootstrapResult {
        catalog,
        notices,
        network_status: if any_success {
            LobbyNetworkStatus::Synced
        } else if saw_error {
            LobbyNetworkStatus::Error
        } else {
            LobbyNetworkStatus::Connected
        },
    }
}

fn apply_live_update(
    unlocked: &mut UnlockedClient,
    update: HostedSessionUpdate,
    active_game_id: Option<&str>,
    active_game: &mut ActiveHostedGamePollUpdate,
) {
    if let Some(status) = update.status {
        unlocked.network_status = map_hosted_status(status);
    }
    if !update.catalog.is_empty() {
        apply_catalog(unlocked, &update.catalog);
    }
    let catalog = unlocked.catalog.clone();
    apply_player_events(
        unlocked,
        update.player_events,
        &catalog,
        active_game_id,
        active_game,
    );
    apply_notices(unlocked, update.notices);
    apply_direct_messages(unlocked, update.contact_messages);
    apply_game_inbox_messages(unlocked, update.player_messages, &catalog);
}

fn apply_catalog(unlocked: &mut UnlockedClient, catalog: &[CatalogGame]) {
    for catalog_game in catalog {
        let existing_index = unlocked
            .catalog
            .iter()
            .position(|existing| catalog_game_matches(existing, catalog_game));
        let should_apply = existing_index
            .and_then(|index| unlocked.catalog.get(index))
            .map(|existing| compare_catalog_versions(existing, catalog_game).is_lt())
            .unwrap_or(true);
        if !should_apply {
            continue;
        }

        if let Some(index) = existing_index {
            if catalog_game.definition.catalog_state == CatalogState::Retired {
                unlocked.catalog.remove(index);
            } else {
                unlocked.catalog[index] = catalog_game.clone();
            }
        } else if catalog_game.definition.catalog_state != CatalogState::Retired {
            unlocked.catalog.push(catalog_game.clone());
        }

        if let Some(cached) = unlocked
            .cache
            .games
            .iter_mut()
            .find(|cached| cached_game_matches_catalog(cached, catalog_game))
        {
            cached.name = catalog_game.definition.game_name.clone();
            cached.game_tier = Some(
                catalog_game
                    .definition
                    .game_tier
                    .as_ref()
                    .map(|tier| tier.as_str())
                    .unwrap_or("league")
                    .to_string(),
            );
            cached.host_alias = catalog_game.definition.host_alias.clone();
            cached.host_contact_npub = catalog_game.definition.host_contact_npub.clone();
            cached.host_contact_label = catalog_game.definition.host_contact_label.clone();
            cached.host_contact_nip05 = catalog_game.definition.host_contact_nip05.clone();
            cached.daemon_pubkey = catalog_game.daemon_pubkey.clone();
            cached.relay_url = unlocked.relay_url.clone().unwrap_or_default();
            if catalog_game.definition.status == GameStatus::Finished && cached.status == "joined" {
                cached.status = "final".to_string();
            }
        }

        maybe_cache_host_contact(
            &mut unlocked.cache,
            catalog_game.definition.host_contact_npub.as_deref(),
            catalog_game.definition.host_contact_label.as_deref(),
            catalog_game.definition.host_contact_nip05.as_deref(),
        );
    }
}

fn catalog_game_matches(left: &CatalogGame, right: &CatalogGame) -> bool {
    left.daemon_pubkey == right.daemon_pubkey && left.definition.game_id == right.definition.game_id
}

fn cached_game_matches_catalog(cached: &CachedGame, catalog_game: &CatalogGame) -> bool {
    cached.id == catalog_game.definition.game_id
        && (cached.daemon_pubkey.trim().is_empty()
            || cached.daemon_pubkey == catalog_game.daemon_pubkey)
}

fn apply_player_events(
    unlocked: &mut UnlockedClient,
    batch: PlayerEventBatch,
    catalog: &[CatalogGame],
    active_game_id: Option<&str>,
    active_game: &mut ActiveHostedGamePollUpdate,
) {
    let player_pubkey = current_player_pubkey(&unlocked.keychain).ok();
    let hosted_store = open_hosted_state_store().ok();
    for receipt in batch.receipts {
        if let Some(game) = unlocked
            .cache
            .games
            .iter_mut()
            .find(|game| game.id == receipt.game_id)
        {
            if !matches!(game.status.as_str(), "joined" | "final") {
                game.status = match receipt.status {
                    nc_nostr::invite_request::InviteRequestReceiptStatus::Received => {
                        "requested".to_string()
                    }
                    _ => receipt.status.as_str().to_string(),
                };
            }
            game.updated_at = now_iso8601();
        }
    }
    for decision in batch.decisions {
        apply_decision(unlocked, decision, catalog);
    }
    for result in batch.claim_results {
        if let Some(game_id) = result.game_id.as_deref() {
            if let Some(game) = unlocked
                .cache
                .games
                .iter_mut()
                .find(|game| game.id == game_id)
            {
                if result.status.as_str() == "claimed" {
                    game.status = "joined".to_string();
                    game.seat = result.seat;
                }
                game.updated_at = now_iso8601();
            }
        }
    }
    for state in batch.states {
        if let (Some(player_pubkey), Some(store)) =
            (player_pubkey.as_deref(), hosted_store.as_ref())
        {
            let _ = store.save_snapshot(&unlocked.password, player_pubkey, &state);
            clear_advanced_draft_if_needed(
                store,
                &unlocked.password,
                player_pubkey,
                &state.game_id,
                state.turn,
            );
        }
        apply_state_snapshot_to_cache(unlocked, &state);
        note_active_game_state(active_game, active_game_id, &state);
    }
    for delta in batch.deltas {
        let Some(player_pubkey) = player_pubkey.as_deref() else {
            continue;
        };
        let Some(store) = hosted_store.as_ref() else {
            continue;
        };
        let updated = store
            .load_snapshot(&unlocked.password, player_pubkey, &delta.game_id)
            .ok()
            .flatten()
            .map(|cached| cached.snapshot)
            .and_then(|snapshot| apply_state_delta(&snapshot, &delta).ok())
            .or_else(|| recover_full_hosted_state(unlocked, &delta.game_id, catalog).ok());
        let Some(state) = updated else {
            continue;
        };
        let _ = store.save_snapshot(&unlocked.password, player_pubkey, &state);
        clear_advanced_draft_if_needed(
            store,
            &unlocked.password,
            player_pubkey,
            &state.game_id,
            state.turn,
        );
        apply_state_snapshot_to_cache(unlocked, &state);
        note_active_game_state(active_game, active_game_id, &state);
    }
    for message in batch.contact_messages {
        apply_direct_message(unlocked, message);
    }
    for message in batch.player_messages {
        apply_game_inbox_message(unlocked, message, catalog);
    }
    for receipt in batch.turn_receipts {
        if matches!(
            receipt.status,
            TurnReceiptStatus::Accepted | TurnReceiptStatus::Superseded
        ) {
            if let Some(game) = unlocked
                .cache
                .games
                .iter_mut()
                .find(|game| game.id == receipt.game_id)
            {
                game.last_turn = Some(receipt.turn);
                game.updated_at = now_iso8601();
            }
        }
        if active_game_id == Some(receipt.game_id.as_str()) {
            active_game.game_id = Some(receipt.game_id.clone());
            active_game.turn_receipt = Some(receipt);
        }
    }
}

fn note_active_game_state(
    active_game: &mut ActiveHostedGamePollUpdate,
    active_game_id: Option<&str>,
    state: &GameState,
) {
    if active_game_id == Some(state.game_id.as_str()) {
        active_game.game_id = Some(state.game_id.clone());
        active_game.promoted_state_hash = Some(state.state_hash.clone());
    }
}

fn apply_state_snapshot_to_cache(unlocked: &mut UnlockedClient, state: &GameState) {
    if let Some(game) = unlocked
        .cache
        .games
        .iter_mut()
        .find(|game| game.id == state.game_id)
    {
        game.last_turn = Some(state.turn);
        game.last_hash = Some(state.state_hash.clone());
        game.seat = Some(state.player_seat);
        game.status = "joined".to_string();
        game.updated_at = now_iso8601();
    }
    unlocked.cache.replace_roster(
        &state.game_id,
        state
            .state
            .roster
            .iter()
            .map(|entry| GameRosterEntry {
                game_id: state.game_id.clone(),
                empire_id: entry.empire_id,
                empire_name: entry.empire_name.clone(),
                is_self: entry.is_self,
            })
            .collect(),
    );
}

fn clear_advanced_draft_if_needed(
    store: &HostedStateStore,
    password: &str,
    player_pubkey: &str,
    game_id: &str,
    authoritative_turn: u32,
) {
    let Ok(Some(draft)) = store.load_draft(password, player_pubkey, game_id) else {
        return;
    };
    if authoritative_turn > draft.turn {
        let _ = store.clear_draft(player_pubkey, game_id);
    }
}

fn recover_full_hosted_state(
    unlocked: &UnlockedClient,
    game_id: &str,
    catalog: &[CatalogGame],
) -> Result<GameState, HostedStateRequestError> {
    let Some(session) = unlocked.session.as_ref() else {
        return Err(HostedStateRequestError {
            code: None,
            message: "no relay configured for the hosted lobby".to_string(),
        });
    };
    let daemon_pubkey = catalog
        .iter()
        .find(|entry| entry.definition.game_id == game_id)
        .map(|entry| entry.daemon_pubkey.as_str())
        .or_else(|| {
            unlocked
                .cache
                .games
                .iter()
                .find(|entry| entry.id == game_id)
                .map(|entry| entry.daemon_pubkey.as_str())
        })
        .ok_or_else(|| HostedStateRequestError {
            code: None,
            message: "missing daemon pubkey for hosted state recovery".to_string(),
        })?;
    let handle = current_handle(&unlocked.keychain);
    session.request_state(game_id, daemon_pubkey, None, None, handle.as_deref(), None)
}

fn apply_notices(unlocked: &mut UnlockedClient, notices: Vec<NoticePayload>) {
    for notice in notices {
        unlocked.cache.upsert_notice(NoticeEntry {
            notice_id: notice.notice_id,
            sender_npub: notice.sender_npub,
            sender_handle: notice.sender_handle,
            body: notice.body,
            created_at: iso8601_from_secs(notice.created_at),
        });
    }
}

fn apply_direct_messages(
    unlocked: &mut UnlockedClient,
    messages: Vec<nc_nostr::contact_message::ContactMessage>,
) {
    for message in messages {
        apply_direct_message(unlocked, message);
    }
}

fn apply_direct_message(
    unlocked: &mut UnlockedClient,
    message: nc_nostr::contact_message::ContactMessage,
) {
    let sender_npub = message.sender_npub.clone();
    let current_npub: String = active_keys(&unlocked.keychain)
        .ok()
        .and_then(|keys| hex_to_npub(&keys.public_key().to_hex()))
        .unwrap_or_default();
    let contact_npub = if message.sender_npub == current_npub {
        current_npub.clone()
    } else {
        message.sender_npub.clone()
    };
    if sender_npub != current_npub {
        maybe_cache_host_contact(
            &mut unlocked.cache,
            Some(&sender_npub),
            message.sender_label.as_deref(),
            None,
        );
    }
    unlocked.cache.upsert_contact_message(ContactMessageEntry {
        message_id: message.message_id,
        contact_npub: if sender_npub == current_npub {
            contact_npub
        } else {
            sender_npub.clone()
        },
        sender_npub: sender_npub.clone(),
        sender_label: message.sender_label,
        body: message.body,
        outgoing: false,
        created_at: iso8601_from_secs(message.created_at),
    });
    let created_at = iso8601_from_secs(message.created_at);
    unlocked
        .cache
        .note_contact_activity(&sender_npub, &created_at, 1);
}

fn apply_game_inbox_messages(
    unlocked: &mut UnlockedClient,
    messages: Vec<nc_nostr::player_message::PlayerMessage>,
    catalog: &[CatalogGame],
) {
    for message in messages {
        apply_game_inbox_message(unlocked, message, catalog);
    }
}

fn apply_game_inbox_message(
    unlocked: &mut UnlockedClient,
    message: nc_nostr::player_message::PlayerMessage,
    catalog: &[CatalogGame],
) {
    let game_name = lookup_game_name(&message.game_id, &unlocked.cache, catalog)
        .unwrap_or_else(|| message.game_id.clone());
    let self_empire_id = unlocked
        .cache
        .rosters
        .iter()
        .find(|entry| entry.game_id == message.game_id && entry.is_self)
        .map(|entry| entry.empire_id)
        .unwrap_or(0);
    let outgoing = self_empire_id != 0 && message.sender_empire_id == self_empire_id;
    let (other_empire_id, other_empire_name) = if outgoing {
        (
            message.recipient_empire_id,
            message.recipient_empire_name.clone(),
        )
    } else {
        (message.sender_empire_id, message.sender_empire_name.clone())
    };
    unlocked
        .cache
        .upsert_game_inbox_message(GameInboxMessageEntry {
            message_id: message.message_id,
            game_id: message.game_id,
            other_empire_id,
            other_empire_name,
            sender_empire_id: message.sender_empire_id,
            sender_empire_name: message.sender_empire_name,
            body: message.body,
            outgoing,
            created_at: iso8601_from_secs(message.created_at),
        });
    let _ = game_name;
}

fn apply_decision(
    unlocked: &mut UnlockedClient,
    decision: InviteDecisionPayload,
    catalog: &[CatalogGame],
) {
    let updated_at = now_iso8601();
    let game_name = lookup_game_name(&decision.game_id, &unlocked.cache, catalog)
        .unwrap_or_else(|| decision.game_id.clone());
    let catalog_match = catalog
        .iter()
        .find(|game| game.definition.game_id == decision.game_id);
    let daemon_pubkey = catalog_match
        .map(|game| game.daemon_pubkey.clone())
        .or_else(|| {
            unlocked
                .cache
                .games
                .iter()
                .find(|game| game.id == decision.game_id)
                .map(|game| game.daemon_pubkey.clone())
        })
        .unwrap_or_default();
    let mut cached = CachedGame {
        id: decision.game_id.clone(),
        name: game_name,
        game_tier: Some(
            catalog_match
                .and_then(|game| game.definition.game_tier.as_ref().map(|tier| tier.as_str()))
                .unwrap_or("league")
                .to_string(),
        ),
        host_alias: catalog_match.and_then(|game| game.definition.host_alias.clone()),
        host_contact_npub: catalog_match.and_then(|game| game.definition.host_contact_npub.clone()),
        host_contact_label: catalog_match
            .and_then(|game| game.definition.host_contact_label.clone()),
        host_contact_nip05: catalog_match
            .and_then(|game| game.definition.host_contact_nip05.clone()),
        relay_url: unlocked.relay_url.clone().unwrap_or_default(),
        daemon_pubkey,
        seat: None,
        status: "rejected".to_string(),
        invite_address: None,
        last_turn: None,
        last_hash: None,
        updated_at,
    };
    if let Some(existing) = unlocked
        .cache
        .games
        .iter()
        .find(|game| game.id == decision.game_id)
    {
        cached.last_turn = existing.last_turn;
        cached.last_hash = existing.last_hash.clone();
    }
    match &decision.decision {
        InviteDecision::Approved { seat } => {
            cached.status = "joined".to_string();
            cached.seat = Some(*seat);
        }
        InviteDecision::Rejected => {}
    }
    unlocked.cache.upsert_game(cached);
}

fn build_loaded_state(
    unlocked: &UnlockedClient,
    status_message: Option<String>,
    status_tone: LobbyStatusTone,
    network_status: Option<LobbyNetworkStatus>,
) -> LobbyLoadedState {
    let mut open_games = unlocked
        .catalog
        .iter()
        .map(|game| {
            let (status, sort_key) = open_game_status(game);
            (
                sort_key,
                OpenGameRow {
                    game_id: game.definition.game_id.clone(),
                    status: status.to_string(),
                    game_tier: game_tier_label(
                        game.definition.game_tier.as_ref().map(|tier| tier.as_str()),
                    )
                    .to_string(),
                    game: game.definition.game_name.clone(),
                    host: format_host_label(
                        game.definition.host_alias.as_deref(),
                        game.definition.host_contact_label.as_deref(),
                        game.definition.host_contact_nip05.as_deref(),
                        game.definition.host_contact_npub.as_deref(),
                    ),
                    host_contact_npub: game.definition.host_contact_npub.clone(),
                    relay_url: unlocked.relay_url.clone().unwrap_or_default(),
                    daemon_pubkey: game.daemon_pubkey.clone(),
                    recruiting: game.definition.recruiting.as_str().to_string(),
                    open_seats: game.definition.open_seats as u8,
                    total_seats: game.definition.players as u8,
                    created_date: format_catalog_created_date(game.definition.created_at),
                    turn_summary: format!("Y{} T{}", game.definition.year, game.definition.turn),
                    summary: game.definition.summary.clone().unwrap_or_default(),
                },
            )
        })
        .collect::<Vec<_>>();
    open_games.sort_by(|(left_key, left), (right_key, right)| {
        left_key
            .cmp(right_key)
            .then_with(|| left.game.to_lowercase().cmp(&right.game.to_lowercase()))
            .then_with(|| left.game_id.cmp(&right.game_id))
    });
    let open_games = open_games
        .into_iter()
        .map(|(_, row)| row)
        .collect::<Vec<_>>();

    let mut joined_games = unlocked
        .cache
        .games
        .iter()
        .map(|game| JoinedGameRow {
            game_id: game.id.clone(),
            status: match game.status.as_str() {
                "approved" => "requested".to_string(),
                other => other.to_string(),
            },
            game_tier: game_tier_label(game.game_tier.as_deref()).to_string(),
            game: game.name.clone(),
            host: format_host_label(
                game.host_alias.as_deref(),
                game.host_contact_label.as_deref(),
                game.host_contact_nip05.as_deref(),
                game.host_contact_npub.as_deref(),
            ),
            host_contact_npub: game.host_contact_npub.clone(),
            relay_url: game.relay_url.clone(),
            daemon_pubkey: game.daemon_pubkey.clone(),
            seat: game.seat.map(|seat| seat as u8),
            turn_summary: if game.status == "joined" {
                game.last_turn
                    .map(|turn| format!("T{turn}"))
                    .unwrap_or_else(|| "- -".to_string())
            } else {
                "- -".to_string()
            },
            invite_address: game.invite_address.clone(),
            last_turn: game.last_turn,
            last_hash: game.last_hash.clone(),
        })
        .collect::<Vec<_>>();
    joined_games.sort_by(|left, right| {
        joined_game_status_sort_key(&left.status)
            .cmp(&joined_game_status_sort_key(&right.status))
            .then_with(|| right.last_turn.cmp(&left.last_turn))
            .then_with(|| left.game.to_lowercase().cmp(&right.game.to_lowercase()))
            .then_with(|| left.game_id.cmp(&right.game_id))
    });

    let game_inbox = build_game_inbox_rows(&joined_games, &unlocked.cache);

    let notices = unlocked
        .cache
        .notices
        .iter()
        .map(|entry| LobbyNotice {
            notice_id: entry.notice_id.clone(),
            sender: entry
                .sender_handle
                .clone()
                .unwrap_or_else(|| entry.sender_npub.clone()),
            body: entry.body.clone(),
            created_at: entry.created_at.clone(),
        })
        .collect::<Vec<_>>();

    let mut direct_contacts = unlocked
        .cache
        .direct_contacts
        .iter()
        .map(|entry| DirectContactRow {
            npub: entry.npub.clone(),
            label: entry.label.clone(),
            nip05: entry.nip05.clone(),
            source: entry.source.clone(),
            blocked: entry.blocked,
            hidden: entry.hidden,
            unread_count: entry.unread_count,
            last_activity_at: entry.last_activity_at.clone(),
        })
        .collect::<Vec<_>>();
    direct_contacts.sort_by(|left, right| {
        right
            .unread_count
            .cmp(&left.unread_count)
            .then_with(|| right.last_activity_at.cmp(&left.last_activity_at))
            .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
            .then_with(|| left.npub.cmp(&right.npub))
    });

    let thread_messages = unlocked
        .cache
        .direct_messages
        .iter()
        .map(|entry| ThreadMessage {
            message_id: entry.message_id.clone(),
            contact_npub: entry.contact_npub.clone(),
            sender: entry
                .sender_label
                .clone()
                .or_else(|| {
                    unlocked
                        .cache
                        .direct_contacts
                        .iter()
                        .find(|contact| contact.npub == entry.sender_npub)
                        .map(|contact| contact.label.clone())
                })
                .unwrap_or_else(|| short_contact_label(&entry.sender_npub)),
            body: entry.body.clone(),
            outgoing: entry.outgoing,
            created_at: entry.created_at.clone(),
        })
        .collect::<Vec<_>>();

    let game_inbox_messages = unlocked
        .cache
        .game_inbox_messages
        .iter()
        .map(|entry| GameInboxMessage {
            message_id: entry.message_id.clone(),
            game_id: entry.game_id.clone(),
            game: lookup_game_name(&entry.game_id, &unlocked.cache, &unlocked.catalog)
                .unwrap_or_else(|| entry.game_id.clone()),
            other_empire_id: entry.other_empire_id,
            other_empire_name: entry.other_empire_name.clone(),
            sender: entry.sender_empire_name.clone(),
            body: entry.body.clone(),
            outgoing: entry.outgoing,
            created_at: entry.created_at.clone(),
        })
        .collect::<Vec<_>>();

    LobbyLoadedState {
        relay_label: unlocked
            .relay_url
            .as_ref()
            .map(|relay| format!("relay: {relay}")),
        player_handle: current_handle(&unlocked.keychain),
        joined_games,
        open_games,
        game_inbox,
        notices,
        direct_contacts,
        thread_messages,
        game_inbox_messages,
        network_status: network_status.unwrap_or_else(|| unlocked.network_status),
        status_message,
        status_tone,
    }
}

fn build_game_inbox_rows(joined_games: &[JoinedGameRow], cache: &ClientCache) -> Vec<GameInboxRow> {
    let mut rows = Vec::new();
    for game in joined_games {
        let mut roster_entries = cache
            .rosters
            .iter()
            .filter(|entry| entry.game_id == game.game_id && !entry.is_self)
            .collect::<Vec<_>>();
        roster_entries.sort_by(|left, right| left.empire_id.cmp(&right.empire_id));

        for roster in roster_entries {
            let latest = cache.game_inbox_messages.iter().rev().find(|entry| {
                entry.game_id == game.game_id && entry.other_empire_id == roster.empire_id
            });
            rows.push(GameInboxRow {
                game_id: game.game_id.clone(),
                game: game.game.clone(),
                other_empire_id: roster.empire_id,
                other_empire_name: roster.empire_name.clone(),
                preview: latest
                    .map(|entry| entry.body.clone())
                    .unwrap_or_else(|| "<no messages yet>".to_string()),
                updated_at: latest
                    .map(|entry| entry.created_at.clone())
                    .unwrap_or_default(),
            });
        }
    }
    rows.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| left.game.to_lowercase().cmp(&right.game.to_lowercase()))
            .then_with(|| left.other_empire_name.cmp(&right.other_empire_name))
    });
    rows
}

fn lookup_game_name(game_id: &str, cache: &ClientCache, catalog: &[CatalogGame]) -> Option<String> {
    cache
        .games
        .iter()
        .find(|game| game.id == game_id)
        .map(|game| game.name.clone())
        .or_else(|| {
            catalog
                .iter()
                .find(|game| game.definition.game_id == game_id)
                .map(|game| game.definition.game_name.clone())
        })
}

fn maybe_cache_host_contact(
    cache: &mut ClientCache,
    npub: Option<&str>,
    label: Option<&str>,
    nip05: Option<&str>,
) {
    let Some(npub) = npub.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    cache.upsert_contact(ContactEntry {
        npub: npub.to_string(),
        label: label
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .or_else(|| nip05.and_then(host_nip05_label))
            .unwrap_or_else(|| short_contact_label(npub)),
        nip05: nip05
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        source: "host".to_string(),
        blocked: false,
        hidden: false,
        unread_count: 0,
        last_activity_at: None,
    });
}

fn cache_game_from_row(row: &JoinedGameRow) -> CachedGame {
    CachedGame {
        id: row.game_id.clone(),
        name: row.game.clone(),
        game_tier: Some(row.game_tier.to_ascii_lowercase()),
        host_alias: Some(row.host.clone()),
        host_contact_npub: row.host_contact_npub.clone(),
        host_contact_label: Some(row.host.clone()),
        host_contact_nip05: None,
        relay_url: row.relay_url.clone(),
        daemon_pubkey: row.daemon_pubkey.clone(),
        seat: row.seat.map(u32::from),
        status: row.status.clone(),
        invite_address: row.invite_address.clone(),
        last_turn: row.last_turn,
        last_hash: row.last_hash.clone(),
        updated_at: now_iso8601(),
    }
}

fn initial_network_status_from_session(has_session: bool) -> LobbyNetworkStatus {
    if has_session {
        LobbyNetworkStatus::Connecting
    } else {
        LobbyNetworkStatus::NoRelay
    }
}

fn map_hosted_status(status: HostedSessionStatus) -> LobbyNetworkStatus {
    match status {
        HostedSessionStatus::Connected => LobbyNetworkStatus::Connected,
        HostedSessionStatus::Synced => LobbyNetworkStatus::Synced,
        HostedSessionStatus::Error => LobbyNetworkStatus::Error,
    }
}

fn iso8601_from_secs(secs: i64) -> String {
    chrono::DateTime::from_timestamp(secs, 0)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(now_iso8601)
}

fn lobby_open_game_error(
    unlocked: &mut UnlockedClient,
    row: &JoinedGameRow,
    err: HostedStateRequestError,
) -> LobbyOpenGameError {
    let expire_sandbox = row.game_tier.eq_ignore_ascii_case("sandbox")
        && err.code == Some(StateErrorCode::NotAPlayer);
    if expire_sandbox {
        if let Some(cached) = unlocked
            .cache
            .games
            .iter_mut()
            .find(|game| game.id == row.game_id)
        {
            cached.status = "expired".to_string();
            cached.seat = None;
            cached.updated_at = now_iso8601();
        }
        let loaded = save_cache(&unlocked.cache, &unlocked.password)
            .ok()
            .map(|_| {
                build_loaded_state(
                    unlocked,
                    None,
                    LobbyStatusTone::Info,
                    Some(unlocked.network_status),
                )
            });
        return LobbyOpenGameError {
            code: err.code,
            message: "Your sandbox seat is no longer active. Rejoin from Open Games.".to_string(),
            loaded,
        };
    }

    LobbyOpenGameError {
        code: err.code,
        message: err.message,
        loaded: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use nc_client::keychain::Keychain;
    use nc_nostr::game_definition::{
        CatalogState, GameDefinition, GameStatus, RecruitingMode, SeatSlot,
    };
    use nc_nostr::invite_request::{InviteRequestReceipt, InviteRequestReceiptStatus};
    use nc_nostr::state_sync::{
        GameState, HostedPlayerRosterEntry, HostedPlayerState, HostedStarmapState,
        HostedStatePayload, StateErrorCode,
    };
    use nc_nostr::turn_commands::{TurnReceipt, TurnReceiptStatus};

    fn temp_test_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("nc-dash-{name}-{unique}"))
    }

    fn keychain_with_active_identity() -> Keychain {
        let mut keychain = Keychain::empty();
        push_new_identity(&mut keychain, now_iso8601(), Some("tester".to_string()))
            .expect("active identity");
        keychain
    }

    fn unlocked_with_game(status: &str) -> UnlockedClient {
        UnlockedClient {
            password: String::new(),
            keychain: Keychain::empty(),
            cache: ClientCache {
                games: vec![CachedGame {
                    id: "sandbox-smoke".to_string(),
                    name: "Sandbox Smoke".to_string(),
                    game_tier: Some("sandbox".to_string()),
                    host_alias: Some("nc-host".to_string()),
                    host_contact_npub: None,
                    host_contact_label: None,
                    host_contact_nip05: None,
                    relay_url: "ws://127.0.0.1:8080".to_string(),
                    daemon_pubkey: "daemon".to_string(),
                    seat: Some(1),
                    status: status.to_string(),
                    invite_address: None,
                    last_turn: Some(4),
                    last_hash: Some("hash".to_string()),
                    updated_at: now_iso8601(),
                }],
                ..ClientCache::empty()
            },
            catalog: Vec::new(),
            session: None,
            live_session: None,
            bootstrap_rx: None,
            relay_url: Some("ws://127.0.0.1:8080".to_string()),
            network_status: LobbyNetworkStatus::Synced,
            cache_dirty: false,
        }
    }

    fn catalog_game(
        game_id: &str,
        daemon: &str,
        state: CatalogState,
        published_at: u64,
    ) -> CatalogGame {
        CatalogGame {
            daemon_pubkey: daemon.to_string(),
            definition: GameDefinition {
                game_id: game_id.to_string(),
                game_name: format!("Game {game_id}"),
                status: GameStatus::Active,
                catalog_state: state,
                created_at: Some(1_700_000_000),
                players: 4,
                recruiting: RecruitingMode::NewPlayers,
                open_seats: 1,
                year: 3000,
                turn: 4,
                summary: None,
                host_alias: Some("nc-host".to_string()),
                host_contact_npub: None,
                host_contact_label: None,
                host_contact_nip05: None,
                slots: Vec::<SeatSlot>::new(),
                game_tier: None,
            },
            published_at,
        }
    }

    fn sample_state(game_id: &str, turn: u32, hash: &str) -> GameState {
        let full_year = 3000 + turn;
        let year = u16::try_from(full_year).expect("year fits in u16");
        GameState {
            game_id: game_id.to_string(),
            turn,
            year: full_year,
            player_seat: 1,
            player_name: "Terran Union".to_string(),
            state_hash: hash.to_string(),
            state: HostedStatePayload {
                player: HostedPlayerState {
                    seat: 1,
                    empire_name: "Terran Union".to_string(),
                    handle: Some("tester".to_string()),
                    mode: "active".to_string(),
                    tax_rate: 50,
                    planet_count: 1,
                    starbase_count: 0,
                    homeworld_planet_index: 1,
                    last_run_year: year,
                    diplomacy: Vec::new(),
                },
                roster: vec![HostedPlayerRosterEntry {
                    empire_id: 1,
                    empire_name: "Terran Union".to_string(),
                    is_self: true,
                }],
                starmap: HostedStarmapState {
                    map_width: 18,
                    map_height: 18,
                    viewer_empire_id: 1,
                    year,
                    worlds: Vec::new(),
                },
                owned_planets: Vec::new(),
                owned_fleets: Vec::new(),
            },
            queued_mail: Vec::new(),
            report_blocks: Vec::new(),
        }
    }

    #[test]
    fn build_sessions_can_be_disabled_for_diagnostics() {
        let sessions = build_sessions(
            &Keychain::empty(),
            Some("ws://127.0.0.1:8080"),
            true,
            false,
            false,
        )
        .expect("disabled sessions");

        assert!(sessions.0.is_none());
        assert!(sessions.1.is_none());
    }

    #[test]
    fn build_sessions_can_disable_only_live_session() {
        let sessions = build_sessions(
            &keychain_with_active_identity(),
            Some("ws://127.0.0.1:8080"),
            false,
            true,
            false,
        )
        .expect("request session without live session");

        assert!(sessions.0.is_some());
        assert!(sessions.1.is_none());
    }

    #[test]
    fn build_sessions_can_disable_only_private_live_stream() {
        let sessions = build_sessions(
            &keychain_with_active_identity(),
            Some("ws://127.0.0.1:8080"),
            false,
            false,
            true,
        )
        .expect("request session with public-only live stream");

        assert!(sessions.0.is_some());
        assert!(sessions.1.is_some());
    }

    #[test]
    fn next_poll_deadline_requires_pending_transport_work() {
        let mut transport = LobbyTransport::new(None, NativeLaunchOptions::default());
        let now = Instant::now();

        assert!(transport.next_poll_deadline(now).is_none());

        transport.unlocked = Some(unlocked_with_game("joined"));
        assert!(transport.next_poll_deadline(now).is_none());

        let (tx, rx) = mpsc::channel();
        let unlocked = transport.unlocked.as_mut().expect("unlocked");
        unlocked.bootstrap_rx = Some(rx);
        assert_eq!(
            transport.next_poll_deadline(now),
            Some(now + PASSIVE_POLL_INTERVAL)
        );
        drop(tx);
    }

    #[test]
    fn poll_updates_applies_bootstrap_results() {
        let mut transport = LobbyTransport::new(None, NativeLaunchOptions::default());
        let (tx, rx) = mpsc::channel();
        transport.unlocked = Some(unlocked_with_game("joined"));
        let unlocked = transport.unlocked.as_mut().expect("unlocked");
        unlocked.network_status = LobbyNetworkStatus::Connecting;
        unlocked.bootstrap_rx = Some(rx);

        tx.send(BootstrapResult {
            catalog: vec![catalog_game(
                "bootstrap-game",
                "daemon",
                CatalogState::Listed,
                1,
            )],
            notices: vec![NoticePayload {
                notice_id: "notice-001".to_string(),
                sender_npub: "npub1bootstrap".to_string(),
                sender_handle: Some("host".to_string()),
                body: "Bootstrapped.".to_string(),
                created_at: 1_700_000_000,
            }],
            network_status: LobbyNetworkStatus::Synced,
        })
        .expect("bootstrap result");

        let update = transport
            .poll_updates(None)
            .expect("poll updates")
            .expect("bootstrap update");

        assert_eq!(update.loaded.network_status, LobbyNetworkStatus::Synced);
        assert_eq!(update.loaded.open_games.len(), 1);
        assert_eq!(update.loaded.open_games[0].game_id, "bootstrap-game");
        assert_eq!(update.loaded.notices.len(), 1);
    }

    #[test]
    fn invite_receipt_does_not_downgrade_joined_game() {
        let mut unlocked = unlocked_with_game("joined");
        apply_player_events(
            &mut unlocked,
            PlayerEventBatch {
                receipts: vec![InviteRequestReceipt {
                    request_id: "req-001".to_string(),
                    game_id: "sandbox-smoke".to_string(),
                    status: InviteRequestReceiptStatus::Received,
                    message: "Auto-approved.".to_string(),
                }],
                ..PlayerEventBatch::default()
            },
            &[],
            None,
            &mut ActiveHostedGamePollUpdate::default(),
        );

        assert_eq!(unlocked.cache.games[0].status, "joined");
    }

    #[test]
    fn received_receipt_maps_to_requested_for_unjoined_game() {
        let mut unlocked = unlocked_with_game("requested");
        apply_player_events(
            &mut unlocked,
            PlayerEventBatch {
                receipts: vec![InviteRequestReceipt {
                    request_id: "req-001".to_string(),
                    game_id: "sandbox-smoke".to_string(),
                    status: InviteRequestReceiptStatus::Received,
                    message: "Auto-approved.".to_string(),
                }],
                ..PlayerEventBatch::default()
            },
            &[],
            None,
            &mut ActiveHostedGamePollUpdate::default(),
        );

        assert_eq!(unlocked.cache.games[0].status, "requested");
    }

    #[test]
    fn sandbox_not_a_player_open_error_marks_game_expired() {
        let mut unlocked = unlocked_with_game("joined");
        let mut row = JoinedGameRow::new(
            "sandbox-smoke",
            "joined",
            "Sandbox Smoke",
            "nc-host",
            "ws://127.0.0.1:8080",
            "daemon",
            Some(1),
            "T4",
        );
        row.game_tier = "Sandbox".to_string();

        let err = lobby_open_game_error(
            &mut unlocked,
            &row,
            HostedStateRequestError {
                code: Some(StateErrorCode::NotAPlayer),
                message: "You no longer have a claimed seat in this game.".to_string(),
            },
        );

        assert_eq!(unlocked.cache.games[0].status, "expired");
        assert_eq!(unlocked.cache.games[0].seat, None);
        assert_eq!(
            err.message,
            "Your sandbox seat is no longer active. Rejoin from Open Games."
        );
        assert!(err.loaded.is_some());
    }

    #[test]
    fn league_not_a_player_open_error_does_not_expire_game() {
        let mut unlocked = unlocked_with_game("joined");
        let mut row = JoinedGameRow::new(
            "league-night",
            "joined",
            "League Night",
            "nc-host",
            "ws://127.0.0.1:8080",
            "daemon",
            Some(1),
            "T4",
        );
        row.game_tier = "League".to_string();
        unlocked.cache.games[0].id = "league-night".to_string();
        unlocked.cache.games[0].name = "League Night".to_string();
        unlocked.cache.games[0].game_tier = Some("league".to_string());

        let err = lobby_open_game_error(
            &mut unlocked,
            &row,
            HostedStateRequestError {
                code: Some(StateErrorCode::NotAPlayer),
                message: "You no longer have a claimed seat in this game.".to_string(),
            },
        );

        assert_eq!(unlocked.cache.games[0].status, "joined");
        assert_eq!(
            err.message,
            "You no longer have a claimed seat in this game."
        );
        assert!(err.loaded.is_none());
    }

    #[test]
    fn newer_retired_catalog_event_removes_open_game() {
        let mut unlocked = unlocked_with_game("joined");
        unlocked.catalog = vec![catalog_game(
            "sandbox-smoke",
            "daemon",
            CatalogState::Listed,
            10,
        )];

        apply_catalog(
            &mut unlocked,
            &[catalog_game(
                "sandbox-smoke",
                "daemon",
                CatalogState::Retired,
                11,
            )],
        );

        assert!(unlocked.catalog.is_empty());
    }

    #[test]
    fn older_catalog_event_does_not_replace_newer_state() {
        let mut unlocked = unlocked_with_game("joined");
        unlocked.catalog = vec![catalog_game(
            "sandbox-smoke",
            "daemon",
            CatalogState::Listed,
            11,
        )];

        apply_catalog(
            &mut unlocked,
            &[catalog_game(
                "sandbox-smoke",
                "daemon",
                CatalogState::Listed,
                10,
            )],
        );

        assert_eq!(unlocked.catalog.len(), 1);
        assert_eq!(
            unlocked.catalog[0].definition.catalog_state,
            CatalogState::Listed
        );
    }

    #[test]
    fn update_open_game_cache_sets_joined_row_and_roster() {
        let mut unlocked = unlocked_with_game("requested");
        let row = JoinedGameRow::new(
            "sandbox-smoke",
            "requested",
            "Sandbox Smoke",
            "nc-host",
            "ws://127.0.0.1:8080",
            "daemon",
            Some(1),
            "- -",
        );
        let state = sample_state("sandbox-smoke", 4, "abc123");

        update_open_game_cache(&mut unlocked, &row, &state).expect("update open-game cache");

        let cached = unlocked
            .cache
            .games
            .iter()
            .find(|game| game.id == "sandbox-smoke")
            .expect("cached game row");
        assert_eq!(cached.status, "joined");
        assert_eq!(cached.last_turn, Some(4));
        assert_eq!(cached.last_hash.as_deref(), Some("abc123"));
        assert_eq!(cached.seat, Some(1));
        assert_eq!(unlocked.cache.rosters.len(), 1);
        assert_eq!(unlocked.cache.rosters[0].game_id, "sandbox-smoke");
        assert_eq!(unlocked.cache.rosters[0].empire_name, "Terran Union");
        assert!(unlocked.cache.rosters[0].is_self);
    }

    #[test]
    fn active_game_summary_tracks_latest_promoted_state_and_receipt() {
        let mut unlocked = unlocked_with_game("joined");
        let mut active_game = ActiveHostedGamePollUpdate::default();

        apply_player_events(
            &mut unlocked,
            PlayerEventBatch {
                states: vec![
                    sample_state("sandbox-smoke", 5, "hash-5"),
                    sample_state("sandbox-smoke", 6, "hash-6"),
                ],
                turn_receipts: vec![TurnReceipt {
                    submit_id: "submit-1".to_string(),
                    game_id: "sandbox-smoke".to_string(),
                    turn: 6,
                    status: TurnReceiptStatus::Accepted,
                    message: Some("Accepted.".to_string()),
                    errors: Vec::new(),
                }],
                ..PlayerEventBatch::default()
            },
            &[],
            Some("sandbox-smoke"),
            &mut active_game,
        );

        assert_eq!(active_game.game_id.as_deref(), Some("sandbox-smoke"));
        assert_eq!(active_game.promoted_state_hash.as_deref(), Some("hash-6"));
        assert_eq!(
            active_game
                .turn_receipt
                .as_ref()
                .map(|receipt| receipt.status.clone()),
            Some(TurnReceiptStatus::Accepted)
        );
        assert_eq!(unlocked.cache.games[0].last_turn, Some(6));
        assert_eq!(unlocked.cache.games[0].last_hash.as_deref(), Some("hash-6"));
    }

    #[test]
    fn inactive_game_updates_do_not_mark_active_game_summary() {
        let mut unlocked = unlocked_with_game("joined");
        let mut active_game = ActiveHostedGamePollUpdate::default();

        apply_player_events(
            &mut unlocked,
            PlayerEventBatch {
                states: vec![sample_state("other-game", 7, "hash-7")],
                turn_receipts: vec![TurnReceipt {
                    submit_id: "submit-2".to_string(),
                    game_id: "other-game".to_string(),
                    turn: 7,
                    status: TurnReceiptStatus::Accepted,
                    message: None,
                    errors: Vec::new(),
                }],
                ..PlayerEventBatch::default()
            },
            &[],
            Some("sandbox-smoke"),
            &mut active_game,
        );

        assert!(active_game.game_id.is_none());
        assert!(active_game.promoted_state_hash.is_none());
        assert!(active_game.turn_receipt.is_none());
    }

    #[test]
    fn cache_load_recovers_after_corruption_and_rewrites_empty_cache() {
        let root = temp_test_path("cache-recover");
        fs::create_dir_all(&root).expect("create temp root");
        let path = root.join("cache.kdl");
        fs::write(&path, b"corrupt-cache").expect("write corrupt cache");

        let result = load_cache_with_recovery("secret", &path);

        assert!(result.cache.games.is_empty());
        assert_eq!(result.status_message.as_deref(), Some(CACHE_RESET_MESSAGE));
        assert_eq!(result.status_tone, LobbyStatusTone::Error);
        assert!(
            load_cache_from("secret", &path)
                .expect("load rewritten cache")
                .is_some()
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn cache_load_still_unlocks_when_reset_cannot_be_saved() {
        let root = temp_test_path("cache-save-fail");
        fs::create_dir_all(&root).expect("create temp root");
        let blocker = root.join("blocker");
        fs::write(&blocker, b"not-a-directory").expect("write blocker");
        let path = blocker.join("cache.kdl");

        let result = load_cache_with_recovery("secret", &path);

        assert!(result.cache.games.is_empty());
        assert_eq!(
            result.status_message.as_deref(),
            Some(CACHE_RESET_SAVE_FAILED_MESSAGE)
        );
        assert_eq!(result.status_tone, LobbyStatusTone::Error);

        let _ = fs::remove_dir_all(&root);
    }
}
