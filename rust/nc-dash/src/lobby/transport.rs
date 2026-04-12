use nc_client::cache::{
    CachedGame, ClientCache, InboxEntry, NoticeEntry, ThreadEntry, load_cache, save_cache,
};
use nc_client::config::{ClientConfig, load_config};
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
use nc_nostr::state_sync::GameState;
use nc_nostr::thread_message::SysopThreadMessage;
use nc_nostr::turn_commands::TurnReceiptStatus;

use super::models::{InboxItem, JoinedGameRow, LobbyNotice, OpenGameRow, ThreadMessage};

#[derive(Debug, Clone)]
pub struct LobbyLoadedState {
    pub relay_label: Option<String>,
    pub player_handle: Option<String>,
    pub joined_games: Vec<JoinedGameRow>,
    pub open_games: Vec<OpenGameRow>,
    pub inbox: Vec<InboxItem>,
    pub notices: Vec<LobbyNotice>,
    pub thread_messages: Vec<ThreadMessage>,
    pub status_message: Option<String>,
}

#[derive(Debug, Clone)]
struct UnlockedClient {
    password: String,
    keychain: Keychain,
    cache: ClientCache,
    session: Option<HostedClientSession>,
    relay_url: Option<String>,
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
        let session = relay_url
            .as_deref()
            .map(|relay| {
                active_keys(&keychain)
                    .map(|keys| HostedClientSession::new(keys, relay.to_string()))
                    .map_err(|err| err.to_string())
            })
            .transpose()?;
        self.unlocked = Some(UnlockedClient {
            password: password.to_string(),
            keychain,
            cache,
            session,
            relay_url,
        });
        self.refresh()
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
        let session = relay_url
            .as_deref()
            .map(|relay| {
                active_keys(&keychain)
                    .map(|keys| HostedClientSession::new(keys, relay.to_string()))
                    .map_err(|err| err.to_string())
            })
            .transpose()?;
        self.unlocked = Some(UnlockedClient {
            password: password.to_string(),
            keychain,
            cache,
            session,
            relay_url,
        });
        self.refresh()
    }

    pub fn refresh(&mut self) -> Result<LobbyLoadedState, String> {
        let unlocked = self
            .unlocked
            .as_mut()
            .ok_or_else(|| "keychain is locked".to_string())?;
        let catalog = if let Some(session) = unlocked.session.as_ref() {
            session.fetch_catalog().map_err(|err| err.to_string())?
        } else {
            Vec::new()
        };
        apply_catalog(unlocked, &catalog);
        let session = unlocked.session.clone();
        if let Some(session) = session.as_ref() {
            let batch = session
                .refresh_player_events(7 * 24 * 60 * 60)
                .map_err(|err| err.to_string())?;
            apply_player_events(unlocked, batch, &catalog);
            let notices = session
                .fetch_lobby_notices(30 * 24 * 60 * 60)
                .map_err(|err| err.to_string())?;
            apply_notices(unlocked, notices);
            let threads = session
                .fetch_thread_messages(30 * 24 * 60 * 60)
                .map_err(|err| err.to_string())?;
            apply_threads(unlocked, threads);
        }
        save_cache(&unlocked.cache, &unlocked.password).map_err(|err| err.to_string())?;
        Ok(build_loaded_state(unlocked, catalog, None))
    }

    pub fn save_handle(&mut self, handle: &str) -> Result<LobbyLoadedState, String> {
        let unlocked = self
            .unlocked
            .as_mut()
            .ok_or_else(|| "keychain is locked".to_string())?;
        set_active_handle(&mut unlocked.keychain, Some(handle.trim().to_string()))?;
        save_keychain(&unlocked.keychain, &unlocked.password).map_err(|err| err.to_string())?;
        self.refresh()
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
        let request_id = session
            .send_invite_request(&row.game_id, &row.daemon_pubkey, message, handle.as_deref())
            .map_err(|err| err.to_string())?;
        let updated_at = now_iso8601();
        unlocked.cache.upsert_game(CachedGame {
            id: row.game_id.clone(),
            name: row.game.clone(),
            host_alias: Some(row.host.clone()),
            relay_url: row.relay_url.clone(),
            daemon_pubkey: row.daemon_pubkey.clone(),
            seat: None,
            status: "requested".to_string(),
            invite_address: None,
            last_turn: None,
            last_hash: None,
            updated_at: updated_at.clone(),
        });
        unlocked.cache.upsert_inbox(InboxEntry {
            kind: "request".to_string(),
            request_id: Some(request_id),
            game_id: row.game_id.clone(),
            game_name: Some(row.game.clone()),
            status: "sent".to_string(),
            message: message.trim().to_string(),
            invite_address: None,
            turn: None,
            updated_at,
        });
        save_cache(&unlocked.cache, &unlocked.password).map_err(|err| err.to_string())?;
        self.refresh()
    }

    pub fn claim_invite(
        &mut self,
        row: &JoinedGameRow,
    ) -> Result<LobbyLoadedState, String> {
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
        unlocked.cache.upsert_inbox(InboxEntry {
            kind: "claim".to_string(),
            request_id: Some(result.nonce.clone()),
            game_id: row.game_id.clone(),
            game_name: Some(row.game.clone()),
            status: result.status.as_str().to_string(),
            message: result.message.clone(),
            invite_address: row.invite_address.clone(),
            turn: None,
            updated_at: now_iso8601(),
        });
        if result.status.as_str() == "claimed" {
            let mut updated = cache_game_from_row(row);
            updated.status = "joined".to_string();
            updated.seat = result.seat;
            updated.invite_address = row.invite_address.clone();
            updated.updated_at = now_iso8601();
            unlocked.cache.upsert_game(updated);
        }
        save_cache(&unlocked.cache, &unlocked.password).map_err(|err| err.to_string())?;
        self.refresh()
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
        let receipt = session
            .submit_turn(
                &row.game_id,
                &row.daemon_pubkey,
                turn,
                commands,
                handle.as_deref(),
            )
            .map_err(|err| err.to_string())?;
        unlocked.cache.upsert_inbox(InboxEntry {
            kind: "turn".to_string(),
            request_id: Some(receipt.submit_id.clone()),
            game_id: row.game_id.clone(),
            game_name: Some(row.game.clone()),
            status: receipt.status.as_str().to_string(),
            message: receipt
                .message
                .clone()
                .unwrap_or_else(|| "turn receipt received".to_string()),
            invite_address: None,
            turn: Some(receipt.turn),
            updated_at: now_iso8601(),
        });
        save_cache(&unlocked.cache, &unlocked.password).map_err(|err| err.to_string())?;
        self.refresh()
    }

    pub fn send_thread_message(
        &mut self,
        game_id: &str,
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
        let handle = current_handle(&unlocked.keychain);
        let payload = session
            .send_thread_message(game_id, daemon_pubkey, body, handle.as_deref())
            .map_err(|err| err.to_string())?;
        unlocked.cache.upsert_thread(ThreadEntry {
            message_id: payload.message_id,
            game_id: payload.game_id,
            sender_role: payload.sender_role.as_str().to_string(),
            sender_npub: payload.sender_npub,
            sender_handle: payload.sender_handle,
            body: payload.body,
            outgoing: true,
            created_at: payload.created_at.to_string(),
        });
        save_cache(&unlocked.cache, &unlocked.password).map_err(|err| err.to_string())?;
        self.refresh()
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

fn apply_catalog(unlocked: &mut UnlockedClient, catalog: &[CatalogGame]) {
    for catalog_game in catalog {
        if let Some(cached) = unlocked
            .cache
            .games
            .iter_mut()
            .find(|cached| cached.id == catalog_game.definition.game_id)
        {
            cached.name = catalog_game.definition.game_name.clone();
            cached.host_alias = catalog_game.definition.host_alias.clone();
            cached.daemon_pubkey = catalog_game.daemon_pubkey.clone();
            cached.relay_url = unlocked.relay_url.clone().unwrap_or_default();
        }
    }
}

fn apply_player_events(
    unlocked: &mut UnlockedClient,
    batch: PlayerEventBatch,
    catalog: &[CatalogGame],
) {
    for receipt in batch.receipts {
        unlocked.cache.upsert_inbox(InboxEntry {
            kind: "request".to_string(),
            request_id: Some(receipt.request_id.clone()),
            game_id: receipt.game_id.clone(),
            game_name: lookup_game_name(&receipt.game_id, &unlocked.cache, catalog),
            status: receipt.status.as_str().to_string(),
            message: receipt.message.clone(),
            invite_address: None,
            turn: None,
            updated_at: now_iso8601(),
        });
    }
    for decision in batch.decisions {
        apply_decision(unlocked, decision, catalog);
    }
    for result in batch.claim_results {
        unlocked.cache.upsert_inbox(InboxEntry {
            kind: "claim".to_string(),
            request_id: Some(result.nonce.clone()),
            game_id: result.game_id.clone().unwrap_or_default(),
            game_name: result
                .game_id
                .as_deref()
                .and_then(|game_id| lookup_game_name(game_id, &unlocked.cache, catalog)),
            status: result.status.as_str().to_string(),
            message: result.message.clone(),
            invite_address: None,
            turn: None,
            updated_at: now_iso8601(),
        });
        if let Some(game_id) = result.game_id.as_deref() {
            if let Some(game) = unlocked.cache.games.iter_mut().find(|game| game.id == game_id) {
                if result.status.as_str() == "claimed" {
                    game.status = "joined".to_string();
                    game.seat = result.seat;
                    game.updated_at = now_iso8601();
                }
            }
        }
    }
    for state in batch.states {
        if let Some(game) = unlocked.cache.games.iter_mut().find(|game| game.id == state.game_id) {
            game.last_turn = Some(state.turn);
            game.last_hash = Some(state.state_hash.clone());
            game.seat = Some(state.player_seat);
            game.status = "joined".to_string();
            game.updated_at = now_iso8601();
        }
    }
    for receipt in batch.turn_receipts {
        unlocked.cache.upsert_inbox(InboxEntry {
            kind: "turn".to_string(),
            request_id: Some(receipt.submit_id.clone()),
            game_id: receipt.game_id.clone(),
            game_name: lookup_game_name(&receipt.game_id, &unlocked.cache, catalog),
            status: receipt.status.as_str().to_string(),
            message: receipt
                .message
                .clone()
                .unwrap_or_else(|| default_turn_message(receipt.status)),
            invite_address: None,
            turn: Some(receipt.turn),
            updated_at: now_iso8601(),
        });
    }
}

fn apply_notices(unlocked: &mut UnlockedClient, notices: Vec<NoticePayload>) {
    for notice in notices {
        unlocked.cache.upsert_notice(NoticeEntry {
            notice_id: notice.notice_id,
            sender_npub: notice.sender_npub,
            sender_handle: notice.sender_handle,
            body: notice.body,
            created_at: notice.created_at.to_string(),
        });
    }
}

fn apply_threads(unlocked: &mut UnlockedClient, messages: Vec<SysopThreadMessage>) {
    for message in messages {
        unlocked.cache.upsert_thread(ThreadEntry {
            message_id: message.message_id,
            game_id: message.game_id,
            sender_role: message.sender_role.as_str().to_string(),
            sender_npub: message.sender_npub,
            sender_handle: message.sender_handle,
            body: message.body,
            outgoing: false,
            created_at: message.created_at.to_string(),
        });
    }
}

fn apply_decision(
    unlocked: &mut UnlockedClient,
    decision: InviteDecisionPayload,
    catalog: &[CatalogGame],
) {
    let updated_at = now_iso8601();
    let game_name = lookup_game_name(&decision.game_id, &unlocked.cache, catalog);
    let catalog_match = catalog
        .iter()
        .find(|game| game.definition.game_id == decision.game_id);
    match &decision.decision {
        InviteDecision::Approved { invite } => {
            unlocked.cache.upsert_game(CachedGame {
                id: decision.game_id.clone(),
                name: game_name.clone().unwrap_or_else(|| decision.game_id.clone()),
                host_alias: catalog_match.and_then(|game| game.definition.host_alias.clone()),
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
                updated_at: updated_at.clone(),
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
                game.updated_at = updated_at.clone();
            }
        }
    }
    unlocked.cache.upsert_inbox(InboxEntry {
        kind: "decision".to_string(),
        request_id: Some(decision.request_id.clone()),
        game_id: decision.game_id.clone(),
        game_name,
        status: match decision.decision {
            InviteDecision::Approved { .. } => "approved".to_string(),
            InviteDecision::Rejected => "rejected".to_string(),
        },
        message: decision.message,
        invite_address: match decision.decision {
            InviteDecision::Approved { invite } => Some(invite),
            InviteDecision::Rejected => None,
        },
        turn: None,
        updated_at,
    });
}

fn build_loaded_state(
    unlocked: &UnlockedClient,
    catalog: Vec<CatalogGame>,
    status_message: Option<String>,
) -> LobbyLoadedState {
    let open_games = catalog
        .iter()
        .filter(|game| game.definition.recruiting != RecruitingMode::None)
        .filter(|game| game.definition.status != GameStatus::Finished)
        .map(|game| OpenGameRow {
            game_id: game.definition.game_id.clone(),
            game: game.definition.game_name.clone(),
            host: game
                .definition
                .host_alias
                .clone()
                .unwrap_or_else(|| "daemon".to_string()),
            relay_url: unlocked.relay_url.clone().unwrap_or_default(),
            daemon_pubkey: game.daemon_pubkey.clone(),
            recruiting: game.definition.recruiting.as_str().to_string(),
            open_seats: game.definition.open_seats as u8,
            turn_summary: format!("Y{} T{}", game.definition.year, game.definition.turn),
            summary: game.definition.summary.clone().unwrap_or_default(),
        })
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
            host: game
                .host_alias
                .clone()
                .unwrap_or_else(|| "daemon".to_string()),
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

    let inbox = unlocked
        .cache
        .inbox
        .iter()
        .map(|entry| InboxItem {
            kind: entry.kind.clone(),
            request_id: entry.request_id.clone(),
            game_id: entry.game_id.clone(),
            game: entry
                .game_name
                .clone()
                .unwrap_or_else(|| entry.game_id.clone()),
            status: entry.status.clone(),
            message: entry.message.clone(),
            invite_address: entry.invite_address.clone(),
        })
        .collect::<Vec<_>>();

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

    let thread_messages = unlocked
        .cache
        .threads
        .iter()
        .map(|entry| ThreadMessage {
            message_id: entry.message_id.clone(),
            game_id: entry.game_id.clone(),
            sender: entry
                .sender_handle
                .clone()
                .unwrap_or_else(|| entry.sender_role.clone()),
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
        inbox,
        notices,
        thread_messages,
        status_message: status_message.or_else(|| {
            if unlocked.session.is_some() {
                Some("Hosted lobby connected.".to_string())
            } else {
                Some("No relay configured. Pass --relay or add one to config.kdl.".to_string())
            }
        }),
    }
}

fn lookup_game_name(
    game_id: &str,
    cache: &ClientCache,
    catalog: &[CatalogGame],
) -> Option<String> {
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

fn default_turn_message(status: TurnReceiptStatus) -> String {
    match status {
        TurnReceiptStatus::Accepted => "turn accepted".to_string(),
        TurnReceiptStatus::Rejected => "turn rejected".to_string(),
        TurnReceiptStatus::Superseded => "turn superseded".to_string(),
        TurnReceiptStatus::NotClaimed => "turn rejected: unclaimed seat".to_string(),
        TurnReceiptStatus::WrongTurn => "turn rejected: wrong turn".to_string(),
    }
}

fn cache_game_from_row(row: &JoinedGameRow) -> CachedGame {
    CachedGame {
        id: row.game_id.clone(),
        name: row.game.clone(),
        host_alias: Some(row.host.clone()),
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
