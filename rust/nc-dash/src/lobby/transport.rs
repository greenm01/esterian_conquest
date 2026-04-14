use nc_client::cache::{
    CachedGame, ClientCache, ContactEntry, ContactMessageEntry, GameInboxMessageEntry,
    GameRosterEntry, NoticeEntry, load_cache, save_cache,
};
use nc_client::config::{ClientConfig, load_config};
use nc_client::contacts::{resolve_contact_input, short_contact_label};
use nc_client::hosted::live::{HostedLiveSession, HostedSessionStatus, HostedSessionUpdate};
use nc_client::hosted::session::{CatalogGame, HostedClientSession, PlayerEventBatch};
use nc_client::keychain::{
    Keychain, active_keys, load_keychain, now_iso8601, push_new_identity, save_keychain,
    set_active_handle,
};
use nc_client::password::validate_new_password;
use nc_client::relay::validate_relay_url;
use nc_nostr::game_definition::{GameStatus, RecruitingMode};
use nc_nostr::invite_request::{InviteDecision, InviteDecisionPayload};
use nc_nostr::lobby_notice::LobbyNotice as NoticePayload;
use nc_nostr::pubkeys::hex_to_npub;
use nc_nostr::state_sync::GameState;
use nc_nostr::turn_commands::TurnReceiptStatus;

use super::models::{
    DirectContactRow, GameInboxMessage, GameInboxRow, JoinedGameRow, LobbyNotice, OpenGameRow,
    ThreadMessage,
};
use super::state::{LobbyNetworkStatus, LobbyStatusTone};

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

struct UnlockedClient {
    password: String,
    keychain: Keychain,
    cache: ClientCache,
    catalog: Vec<CatalogGame>,
    session: Option<HostedClientSession>,
    live_session: Option<HostedLiveSession>,
    relay_url: Option<String>,
    network_status: LobbyNetworkStatus,
}

pub struct LobbyTransport {
    relay_override: Option<String>,
    unlocked: Option<UnlockedClient>,
}

impl LobbyTransport {
    pub fn new(relay_override: Option<String>) -> Self {
        Self {
            relay_override,
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
        save_keychain(&keychain, password).map_err(|err| err.to_string())?;
        let cache = ClientCache::empty();
        save_cache(&cache, password).map_err(|err| err.to_string())?;
        let config = load_config().map_err(|err| err.to_string())?;
        let relay_url = effective_relay(self.relay_override.as_deref(), &config)?;
        let (session, live_session) =
            build_sessions(&keychain, relay_url.as_deref()).map_err(|err| err.to_string())?;
        let has_session = session.is_some();
        self.unlocked = Some(UnlockedClient {
            password: password.to_string(),
            keychain,
            cache,
            catalog: Vec::new(),
            session,
            live_session,
            relay_url,
            network_status: initial_network_status_from_session(has_session),
        });
        if let Some(unlocked) = self.unlocked.as_mut() {
            Ok(build_loaded_state(
                unlocked,
                None,
                LobbyStatusTone::Info,
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
        let cache = load_cache(password)
            .map_err(|err| err.to_string())?
            .unwrap_or_else(ClientCache::empty);
        let config = load_config().map_err(|err| err.to_string())?;
        let relay_url = effective_relay(self.relay_override.as_deref(), &config)?;
        let (session, live_session) =
            build_sessions(&keychain, relay_url.as_deref()).map_err(|err| err.to_string())?;
        let has_session = session.is_some();
        self.unlocked = Some(UnlockedClient {
            password: password.to_string(),
            keychain,
            cache,
            catalog: Vec::new(),
            session,
            live_session,
            relay_url,
            network_status: initial_network_status_from_session(has_session),
        });
        if let Some(unlocked) = self.unlocked.as_mut() {
            Ok(build_loaded_state(
                unlocked,
                None,
                LobbyStatusTone::Info,
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
        let changed = apply_live_updates(unlocked);
        if changed {
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

    pub fn poll_updates(&mut self) -> Result<Option<LobbyLoadedState>, String> {
        let Some(unlocked) = self.unlocked.as_mut() else {
            return Ok(None);
        };
        if !apply_live_updates(unlocked) {
            return Ok(None);
        }
        save_cache(&unlocked.cache, &unlocked.password).map_err(|err| err.to_string())?;
        Ok(Some(build_loaded_state(
            unlocked,
            None,
            LobbyStatusTone::Info,
            Some(unlocked.network_status),
        )))
    }

    pub fn save_handle(&mut self, handle: &str) -> Result<LobbyLoadedState, String> {
        let unlocked = self
            .unlocked
            .as_mut()
            .ok_or_else(|| "keychain is locked".to_string())?;
        set_active_handle(&mut unlocked.keychain, Some(handle.trim().to_string()))?;
        save_keychain(&unlocked.keychain, &unlocked.password).map_err(|err| err.to_string())?;
        Ok(build_loaded_state(
            unlocked,
            Some("Handle updated locally. It will be sent on your next hosted action.".to_string()),
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
        let handle = current_handle(&unlocked.keychain);
        session
            .send_invite_request(&row.game_id, &row.daemon_pubkey, message, handle.as_deref())
            .map_err(|err| err.to_string())?;
        let updated_at = now_iso8601();
        unlocked.cache.upsert_game(CachedGame {
            id: row.game_id.clone(),
            name: row.game.clone(),
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
            Some("Invite request sent. Waiting for nc-host receipt.".to_string()),
            LobbyStatusTone::Success,
            None,
        ))
    }

    pub fn claim_invite(&mut self, row: &JoinedGameRow) -> Result<LobbyLoadedState, String> {
        let invite = row
            .invite_address
            .as_deref()
            .ok_or_else(|| "selected game has no approved invite".to_string())?;
        let unlocked = self
            .unlocked
            .as_mut()
            .ok_or_else(|| "keychain is locked".to_string())?;
        let session = unlocked
            .session
            .as_ref()
            .ok_or_else(|| "no relay configured for the hosted lobby".to_string())?;
        let handle = current_handle(&unlocked.keychain);
        let result = session
            .claim_invite(&row.game_id, &row.daemon_pubkey, invite, handle.as_deref())
            .map_err(|err| err.to_string())?;
        if result.status.as_str() == "claimed" {
            let mut updated = cache_game_from_row(row);
            updated.status = "joined".to_string();
            updated.seat = result.seat;
            updated.invite_address = row.invite_address.clone();
            updated.updated_at = now_iso8601();
            unlocked.cache.upsert_game(updated);
        }
        save_cache(&unlocked.cache, &unlocked.password).map_err(|err| err.to_string())?;
        Ok(build_loaded_state(
            unlocked,
            Some("Seat claim processed by nc-host.".to_string()),
            LobbyStatusTone::Success,
            None,
        ))
    }

    pub fn open_game(&mut self, row: &JoinedGameRow) -> Result<GameState, String> {
        let unlocked = self
            .unlocked
            .as_mut()
            .ok_or_else(|| "keychain is locked".to_string())?;
        let session = unlocked
            .session
            .as_ref()
            .ok_or_else(|| "no relay configured for the hosted lobby".to_string())?;
        let handle = current_handle(&unlocked.keychain);
        let state = session
            .request_state(
                &row.game_id,
                &row.daemon_pubkey,
                row.last_turn,
                row.last_hash.as_deref(),
                handle.as_deref(),
            )
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
    ) -> Result<LobbyLoadedState, String> {
        let unlocked = self
            .unlocked
            .as_mut()
            .ok_or_else(|| "keychain is locked".to_string())?;
        let session = unlocked
            .session
            .as_ref()
            .ok_or_else(|| "no relay configured for the hosted lobby".to_string())?;
        let handle = current_handle(&unlocked.keychain);
        session
            .submit_turn(
                &row.game_id,
                &row.daemon_pubkey,
                turn,
                commands,
                handle.as_deref(),
            )
            .map_err(|err| err.to_string())?;
        Ok(build_loaded_state(
            unlocked,
            Some("Turn submitted. Waiting for nc-host receipt.".to_string()),
            LobbyStatusTone::Success,
            None,
        ))
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
        unlocked.cache.upsert_game_inbox_message(GameInboxMessageEntry {
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

fn build_sessions(
    keychain: &Keychain,
    relay_url: Option<&str>,
) -> Result<(Option<HostedClientSession>, Option<HostedLiveSession>), Box<dyn std::error::Error>> {
    let Some(relay_url) = relay_url else {
        return Ok((None, None));
    };
    let keys = active_keys(keychain)?;
    let session = HostedClientSession::new(keys.clone(), relay_url.to_string());
    let live_session = HostedLiveSession::start(keys, relay_url.to_string());
    Ok((Some(session), Some(live_session)))
}

fn apply_live_updates(unlocked: &mut UnlockedClient) -> bool {
    let Some(live_session) = unlocked.live_session.as_ref() else {
        return false;
    };
    let updates = live_session.drain_updates();
    if updates.is_empty() {
        return false;
    }
    for update in updates {
        apply_live_update(unlocked, update);
    }
    true
}

fn apply_live_update(unlocked: &mut UnlockedClient, update: HostedSessionUpdate) {
    if let Some(status) = update.status {
        unlocked.network_status = map_hosted_status(status);
    }
    if !update.catalog.is_empty() {
        apply_catalog(unlocked, &update.catalog);
    }
    let catalog = unlocked.catalog.clone();
    apply_player_events(unlocked, update.player_events, &catalog);
    apply_notices(unlocked, update.notices);
    apply_direct_messages(unlocked, update.contact_messages);
    apply_game_inbox_messages(unlocked, update.player_messages, &catalog);
}

fn apply_catalog(unlocked: &mut UnlockedClient, catalog: &[CatalogGame]) {
    for catalog_game in catalog {
        if let Some(existing) = unlocked
            .catalog
            .iter_mut()
            .find(|existing| existing.definition.game_id == catalog_game.definition.game_id)
        {
            *existing = catalog_game.clone();
        } else {
            unlocked.catalog.push(catalog_game.clone());
        }

        if let Some(cached) = unlocked
            .cache
            .games
            .iter_mut()
            .find(|cached| cached.id == catalog_game.definition.game_id)
        {
            cached.name = catalog_game.definition.game_name.clone();
            cached.host_alias = catalog_game.definition.host_alias.clone();
            cached.host_contact_npub = catalog_game.definition.host_contact_npub.clone();
            cached.host_contact_label = catalog_game.definition.host_contact_label.clone();
            cached.host_contact_nip05 = catalog_game.definition.host_contact_nip05.clone();
            cached.daemon_pubkey = catalog_game.daemon_pubkey.clone();
            cached.relay_url = unlocked.relay_url.clone().unwrap_or_default();
        }

        maybe_cache_host_contact(
            &mut unlocked.cache,
            catalog_game.definition.host_contact_npub.as_deref(),
            catalog_game.definition.host_contact_label.as_deref(),
            catalog_game.definition.host_contact_nip05.as_deref(),
        );
    }
}

fn apply_player_events(
    unlocked: &mut UnlockedClient,
    batch: PlayerEventBatch,
    catalog: &[CatalogGame],
) {
    for receipt in batch.receipts {
        if let Some(game) = unlocked
            .cache
            .games
            .iter_mut()
            .find(|game| game.id == receipt.game_id)
        {
            game.status = receipt.status.as_str().to_string();
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
    for message in batch.contact_messages {
        apply_direct_message(unlocked, message);
    }
    for message in batch.player_messages {
        apply_game_inbox_message(unlocked, message, catalog);
    }
    for receipt in batch.turn_receipts {
        if matches!(receipt.status, TurnReceiptStatus::Accepted | TurnReceiptStatus::Superseded) {
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
    }
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
        (message.recipient_empire_id, message.recipient_empire_name.clone())
    } else {
        (message.sender_empire_id, message.sender_empire_name.clone())
    };
    unlocked.cache.upsert_game_inbox_message(GameInboxMessageEntry {
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
    match &decision.decision {
        InviteDecision::Approved { invite } => {
            unlocked.cache.upsert_game(CachedGame {
                id: decision.game_id.clone(),
                name: game_name,
                host_alias: catalog_match.and_then(|game| game.definition.host_alias.clone()),
                host_contact_npub: catalog_match
                    .and_then(|game| game.definition.host_contact_npub.clone()),
                host_contact_label: catalog_match
                    .and_then(|game| game.definition.host_contact_label.clone()),
                host_contact_nip05: catalog_match
                    .and_then(|game| game.definition.host_contact_nip05.clone()),
                relay_url: unlocked.relay_url.clone().unwrap_or_default(),
                daemon_pubkey: catalog_match
                    .map(|game| game.daemon_pubkey.clone())
                    .or_else(|| {
                        unlocked
                            .cache
                            .games
                            .iter()
                            .find(|game| game.id == decision.game_id)
                            .map(|game| game.daemon_pubkey.clone())
                    })
                    .unwrap_or_default(),
                seat: None,
                status: "approved".to_string(),
                invite_address: Some(invite.clone()),
                last_turn: None,
                last_hash: None,
                updated_at,
            });
        }
        InviteDecision::Rejected => {
            if let Some(game) = unlocked
                .cache
                .games
                .iter_mut()
                .find(|game| game.id == decision.game_id)
            {
                game.status = "rejected".to_string();
                game.updated_at = updated_at;
            }
        }
    }
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

    let joined_games = unlocked
        .cache
        .games
        .iter()
        .filter(|game| matches!(game.status.as_str(), "approved" | "joined"))
        .map(|game| JoinedGameRow {
            game_id: game.id.clone(),
            status: game.status.clone(),
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
            turn_summary: game
                .last_turn
                .map(|turn| format!("T{turn}"))
                .unwrap_or_else(|| "awaiting claim".to_string()),
            invite_address: game.invite_address.clone(),
            last_turn: game.last_turn,
            last_hash: game.last_hash.clone(),
        })
        .collect::<Vec<_>>();

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
        network_status: network_status.unwrap_or_else(|| {
            unlocked.network_status
        }),
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
            let latest = cache
                .game_inbox_messages
                .iter()
                .rev()
                .find(|entry| {
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
