use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use nc_client::cache::{CachedGame, ClientCache};
use nc_client::hosted::session::{
    CatalogGame, HostedClientSession, HostedStateRequestError, PlayerEventBatch, SandboxJoinOutcome,
};
use nc_client::hosted::store::HostedStateStore;
use nc_client::keychain::now_iso8601;
use nc_nostr::game_definition::{GameStatus, RecruitingMode};
use nc_nostr::invite_request::{InviteDecision, InviteRequestReceiptStatus};
use nc_nostr::sandbox_release::SandboxReleaseResult as SandboxReleaseProtocolResult;
use nc_nostr::state_sync::StateErrorCode;

const PLAYER_EVENT_LOOKBACK_SECS: u64 = 30 * 24 * 60 * 60;

#[derive(Debug, Clone)]
pub struct LobbySnapshot {
    pub cache: ClientCache,
    pub my_games: Vec<crate::app::MyGameRow>,
    pub open_games: Vec<crate::app::OpenGameRow>,
    pub notices: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct HostedGameOpenSuccess {
    pub snapshot: nc_nostr::state_sync::GameState,
    pub row: crate::app::MyGameRow,
    pub cache: ClientCache,
}

#[derive(Debug, Clone)]
pub enum SandboxJoinResult {
    Joined(HostedGameOpenSuccess),
    Full(String),
}

#[derive(Debug, Clone)]
pub enum HostedGameOpenResult {
    Opened(HostedGameOpenSuccess),
    Expired {
        row: crate::app::MyGameRow,
        cache: ClientCache,
        message: String,
    },
}

#[derive(Debug, Clone)]
pub struct SandboxReleaseSuccess {
    pub game_id: String,
    pub cache: ClientCache,
}

#[derive(Debug)]
pub enum TransportCommand {
    Connect {
        relay_url: String,
        nsec: String,
        cache: ClientCache,
        reply_to: Sender<Result<LobbySnapshot, String>>,
    },
    JoinSandbox {
        row: crate::app::OpenGameRow,
        password: String,
        handle: Option<String>,
        reply_to: Sender<Result<SandboxJoinResult, String>>,
    },
    OpenHostedGame {
        row: crate::app::MyGameRow,
        password: String,
        handle: Option<String>,
        reply_to: Sender<Result<HostedGameOpenResult, String>>,
    },
    CompleteFirstJoinSetup {
        row: crate::app::MyGameRow,
        empire_name: String,
        homeworld_name: String,
        password: String,
        reply_to: Sender<Result<HostedGameOpenSuccess, String>>,
    },
    Refresh {
        reply_to: Sender<Result<LobbySnapshot, String>>,
    },
    ReleaseSandbox {
        row: crate::app::MyGameRow,
        reply_to: Sender<Result<SandboxReleaseSuccess, String>>,
    },
    Disconnect,
}

#[derive(Debug)]
pub struct TransportActor {
    tx: Sender<TransportCommand>,
}

#[derive(Debug)]
struct ActiveTransport {
    session: HostedClientSession,
    reply_to: Sender<Result<LobbySnapshot, String>>,
    cache: ClientCache,
}

impl TransportActor {
    pub fn start() -> Self {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || transport_loop(rx));
        Self { tx }
    }

    pub fn connect(
        &self,
        relay_url: String,
        nsec: String,
        cache: ClientCache,
        reply_to: Sender<Result<LobbySnapshot, String>>,
    ) -> Result<(), String> {
        self.tx
            .send(TransportCommand::Connect {
                relay_url,
                nsec,
                cache,
                reply_to,
            })
            .map_err(|_| "transport actor unavailable".to_string())
    }

    pub fn disconnect(&self) -> Result<(), String> {
        self.tx
            .send(TransportCommand::Disconnect)
            .map_err(|_| "transport actor unavailable".to_string())
    }

    pub fn join_sandbox(
        &self,
        row: crate::app::OpenGameRow,
        password: String,
        handle: Option<String>,
        reply_to: Sender<Result<SandboxJoinResult, String>>,
    ) -> Result<(), String> {
        self.tx
            .send(TransportCommand::JoinSandbox {
                row,
                password,
                handle,
                reply_to,
            })
            .map_err(|_| "transport actor unavailable".to_string())
    }

    pub fn open_hosted_game(
        &self,
        row: crate::app::MyGameRow,
        password: String,
        handle: Option<String>,
        reply_to: Sender<Result<HostedGameOpenResult, String>>,
    ) -> Result<(), String> {
        self.tx
            .send(TransportCommand::OpenHostedGame {
                row,
                password,
                handle,
                reply_to,
            })
            .map_err(|_| "transport actor unavailable".to_string())
    }

    pub fn complete_first_join_setup(
        &self,
        row: crate::app::MyGameRow,
        empire_name: String,
        homeworld_name: String,
        password: String,
        reply_to: Sender<Result<HostedGameOpenSuccess, String>>,
    ) -> Result<(), String> {
        self.tx
            .send(TransportCommand::CompleteFirstJoinSetup {
                row,
                empire_name,
                homeworld_name,
                password,
                reply_to,
            })
            .map_err(|_| "transport actor unavailable".to_string())
    }

    pub fn refresh(&self, reply_to: Sender<Result<LobbySnapshot, String>>) -> Result<(), String> {
        self.tx
            .send(TransportCommand::Refresh { reply_to })
            .map_err(|_| "transport actor unavailable".to_string())
    }

    pub fn release_sandbox(
        &self,
        row: crate::app::MyGameRow,
        reply_to: Sender<Result<SandboxReleaseSuccess, String>>,
    ) -> Result<(), String> {
        self.tx
            .send(TransportCommand::ReleaseSandbox { row, reply_to })
            .map_err(|_| "transport actor unavailable".to_string())
    }
}

fn transport_loop(rx: Receiver<TransportCommand>) {
    let mut active: Option<ActiveTransport> = None;
    let mut next_poll = Instant::now();

    loop {
        if let Some(active_transport) = &mut active {
            if Instant::now() >= next_poll {
                let result = fetch_snapshot(&active_transport.session, &mut active_transport.cache);
                let _ = active_transport.reply_to.send(result);
                next_poll = Instant::now() + Duration::from_secs(15);
            }
        }

        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(TransportCommand::Connect {
                relay_url,
                nsec,
                cache,
                reply_to,
            }) => match nostr_sdk::Keys::parse(&nsec) {
                Ok(keys) => {
                    let session = HostedClientSession::new(keys, relay_url);
                    let mut next_active = ActiveTransport {
                        session,
                        reply_to,
                        cache,
                    };
                    let first_result = fetch_snapshot(&next_active.session, &mut next_active.cache);
                    let _ = next_active.reply_to.send(first_result);
                    active = Some(next_active);
                    next_poll = Instant::now() + Duration::from_secs(15);
                }
                Err(err) => {
                    let _ = reply_to.send(Err(format!("invalid active identity: {err}")));
                }
            },
            Ok(TransportCommand::JoinSandbox {
                row,
                password,
                handle,
                reply_to,
            }) => {
                let result = active
                    .as_mut()
                    .ok_or_else(|| "keychain is locked".to_string())
                    .and_then(|transport| join_sandbox_game(transport, row, &password, handle));
                let _ = reply_to.send(result);
            }
            Ok(TransportCommand::OpenHostedGame {
                row,
                password,
                handle,
                reply_to,
            }) => {
                let result = active
                    .as_mut()
                    .ok_or_else(|| "keychain is locked".to_string())
                    .and_then(|transport| open_hosted_game(transport, row, &password, handle));
                let _ = reply_to.send(result);
            }
            Ok(TransportCommand::CompleteFirstJoinSetup {
                row,
                empire_name,
                homeworld_name,
                password,
                reply_to,
            }) => {
                let result = active
                    .as_mut()
                    .ok_or_else(|| "keychain is locked".to_string())
                    .and_then(|transport| {
                        complete_first_join_setup(
                            transport,
                            row,
                            &empire_name,
                            &homeworld_name,
                            &password,
                        )
                    });
                let _ = reply_to.send(result);
            }
            Ok(TransportCommand::Refresh { reply_to }) => {
                let result = active
                    .as_mut()
                    .ok_or_else(|| "keychain is locked".to_string())
                    .and_then(|transport| fetch_snapshot(&transport.session, &mut transport.cache));
                let _ = reply_to.send(result);
            }
            Ok(TransportCommand::ReleaseSandbox { row, reply_to }) => {
                let result = active
                    .as_mut()
                    .ok_or_else(|| "keychain is locked".to_string())
                    .and_then(|transport| release_sandbox_game(transport, row));
                let _ = reply_to.send(result);
            }
            Ok(TransportCommand::Disconnect) => {
                active = None;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn fetch_snapshot(
    session: &HostedClientSession,
    cache: &mut ClientCache,
) -> Result<LobbySnapshot, String> {
    let catalog = session.fetch_catalog().map_err(|err| err.to_string())?;
    let notices = session
        .fetch_lobby_notices(86_400)
        .map_err(|err| err.to_string())?;
    let events = session
        .refresh_player_events(PLAYER_EVENT_LOOKBACK_SECS)
        .map_err(|err| err.to_string())?;

    apply_catalog(cache, &catalog, session.relay_url());
    apply_player_events(cache, &catalog, &events, session.relay_url());

    Ok(LobbySnapshot {
        my_games: build_my_games(cache, &catalog, &events),
        open_games: build_open_games(&catalog, session.relay_url()),
        notices: notices
            .into_iter()
            .take(8)
            .map(|notice| {
                let sender = notice
                    .sender_handle
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| "sysop".to_string());
                format!("{sender}: {}", notice.body)
            })
            .collect(),
        cache: cache.clone(),
    })
}

fn apply_catalog(cache: &mut ClientCache, catalog: &[CatalogGame], relay_url: &str) {
    for game in catalog {
        let existing = cache
            .games
            .iter()
            .find(|entry| entry.id == game.definition.game_id);
        cache.upsert_game(CachedGame {
            id: game.definition.game_id.clone(),
            name: game.definition.game_name.clone(),
            game_tier: game
                .definition
                .game_tier
                .as_ref()
                .map(|tier| tier.as_str().to_string()),
            host_alias: game.definition.host_alias.clone(),
            host_contact_npub: game.definition.host_contact_npub.clone(),
            host_contact_label: game.definition.host_contact_label.clone(),
            host_contact_nip05: game.definition.host_contact_nip05.clone(),
            relay_url: relay_url.to_string(),
            daemon_pubkey: game.daemon_pubkey.clone(),
            seat: existing.and_then(|entry| entry.seat),
            status: if game.definition.status == GameStatus::Finished {
                "final".to_string()
            } else {
                existing
                    .map(|entry| entry.status.clone())
                    .unwrap_or_else(|| game.definition.status.as_str().to_string())
            },
            invite_address: existing.and_then(|entry| entry.invite_address.clone()),
            last_turn: Some(game.definition.turn),
            last_hash: existing.and_then(|entry| entry.last_hash.clone()),
            updated_at: now_iso8601(),
        });
    }
}

fn apply_player_events(
    cache: &mut ClientCache,
    catalog: &[CatalogGame],
    events: &PlayerEventBatch,
    relay_url: &str,
) {
    let catalog_index = catalog
        .iter()
        .map(|game| (game.definition.game_id.as_str(), game))
        .collect::<HashMap<_, _>>();

    for receipt in &events.receipts {
        if cache.is_game_released(&receipt.game_id) {
            continue;
        }
        let mut game = cached_game_seed(cache, &catalog_index, &receipt.game_id, relay_url);
        game.status = match receipt.status {
            InviteRequestReceiptStatus::Received => "requested",
            InviteRequestReceiptStatus::GameFull
            | InviteRequestReceiptStatus::HandleTaken
            | InviteRequestReceiptStatus::NotRecruiting
            | InviteRequestReceiptStatus::GameClosed
            | InviteRequestReceiptStatus::RateLimited
            | InviteRequestReceiptStatus::UnknownGame => "rejected",
        }
        .to_string();
        game.updated_at = now_iso8601();
        cache.upsert_game(game);
    }

    for decision in &events.decisions {
        if cache.is_game_released(&decision.game_id) {
            continue;
        }
        let mut game = cached_game_seed(cache, &catalog_index, &decision.game_id, relay_url);
        match decision.decision {
            InviteDecision::Approved { seat } => {
                game.status = "approved".to_string();
                game.seat = Some(seat);
            }
            InviteDecision::Rejected => {
                game.status = "rejected".to_string();
            }
        }
        game.updated_at = now_iso8601();
        cache.upsert_game(game);
    }

    for state in &events.states {
        if cache.is_game_released(&state.game_id) {
            continue;
        }
        let mut game = cached_game_seed(cache, &catalog_index, &state.game_id, relay_url);
        game.status = "joined".to_string();
        game.seat = Some(state.player_seat);
        game.last_turn = Some(state.turn);
        game.last_hash = Some(state.state_hash.clone());
        game.updated_at = now_iso8601();
        cache.upsert_game(game);
    }
}

fn cached_game_seed<'a>(
    cache: &ClientCache,
    catalog_index: &HashMap<&'a str, &'a CatalogGame>,
    game_id: &str,
    relay_url: &str,
) -> CachedGame {
    if let Some(existing) = cache.games.iter().find(|entry| entry.id == game_id) {
        return existing.clone();
    }

    if let Some(game) = catalog_index.get(game_id) {
        return CachedGame {
            id: game.definition.game_id.clone(),
            name: game.definition.game_name.clone(),
            game_tier: game
                .definition
                .game_tier
                .as_ref()
                .map(|tier| tier.as_str().to_string()),
            host_alias: game.definition.host_alias.clone(),
            host_contact_npub: game.definition.host_contact_npub.clone(),
            host_contact_label: game.definition.host_contact_label.clone(),
            host_contact_nip05: game.definition.host_contact_nip05.clone(),
            relay_url: relay_url.to_string(),
            daemon_pubkey: game.daemon_pubkey.clone(),
            seat: None,
            status: game.definition.status.as_str().to_string(),
            invite_address: None,
            last_turn: Some(game.definition.turn),
            last_hash: None,
            updated_at: now_iso8601(),
        };
    }

    CachedGame {
        id: game_id.to_string(),
        name: game_id.to_string(),
        game_tier: None,
        host_alias: Some("daemon".to_string()),
        host_contact_npub: None,
        host_contact_label: None,
        host_contact_nip05: None,
        relay_url: relay_url.to_string(),
        daemon_pubkey: String::new(),
        seat: None,
        status: "requested".to_string(),
        invite_address: None,
        last_turn: None,
        last_hash: None,
        updated_at: now_iso8601(),
    }
}

fn build_my_games(
    cache: &ClientCache,
    catalog: &[CatalogGame],
    events: &PlayerEventBatch,
) -> Vec<crate::app::MyGameRow> {
    let catalog_index = catalog
        .iter()
        .map(|game| (game.definition.game_id.as_str(), game))
        .collect::<HashMap<_, _>>();
    let state_index = events
        .states
        .iter()
        .map(|state| (state.game_id.as_str(), state))
        .collect::<HashMap<_, _>>();
    let mut rows = cache
        .games
        .iter()
        .filter_map(|game| {
            let status = normalized_joined_status(
                &game.status,
                game.seat,
                catalog_index
                    .get(game.id.as_str())
                    .map(|entry| entry.definition.status == GameStatus::Finished)
                    .unwrap_or(false),
            )?;
            let turn_summary = state_index
                .get(game.id.as_str())
                .map(|state| format!("Y{} T{}", state.year, state.turn))
                .or_else(|| {
                    catalog_index.get(game.id.as_str()).map(|entry| {
                        format!("Y{} T{}", entry.definition.year, entry.definition.turn)
                    })
                })
                .or_else(|| game.last_turn.map(|turn| format!("Y? T{turn}")))
                .unwrap_or_else(|| "-".to_string());
            Some(crate::app::MyGameRow {
                game_id: game.id.clone(),
                status: status.to_string(),
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
                seat: game.seat.and_then(|seat| u8::try_from(seat).ok()),
                turn_summary,
                last_turn: game.last_turn,
                last_hash: game.last_hash.clone(),
            })
        })
        .collect::<Vec<_>>();

    rows.sort_by(|left, right| {
        joined_game_status_sort_key(&left.status)
            .cmp(&joined_game_status_sort_key(&right.status))
            .then_with(|| right.last_turn.cmp(&left.last_turn))
            .then_with(|| left.game.to_lowercase().cmp(&right.game.to_lowercase()))
            .then_with(|| left.game_id.cmp(&right.game_id))
    });
    rows
}

fn build_open_games(catalog: &[CatalogGame], relay_url: &str) -> Vec<crate::app::OpenGameRow> {
    let mut rows = catalog
        .iter()
        .map(|game| {
            let (status, status_rank) = open_game_status(game);
            (
                status_rank,
                crate::app::OpenGameRow {
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
                    relay_url: relay_url.to_string(),
                    daemon_pubkey: game.daemon_pubkey.clone(),
                    open_seats: u8::try_from(game.definition.open_seats).unwrap_or(u8::MAX),
                    total_seats: u8::try_from(game.definition.players).unwrap_or(u8::MAX),
                    created_date: format_catalog_created_date(game.definition.created_at),
                    turn_summary: format!("Y{} T{}", game.definition.year, game.definition.turn),
                    summary: game.definition.summary.clone().unwrap_or_default(),
                },
            )
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.game.to_lowercase().cmp(&right.1.game.to_lowercase()))
            .then_with(|| left.1.game_id.cmp(&right.1.game_id))
    });
    rows.into_iter().map(|(_, row)| row).collect()
}

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
        .or_else(|| {
            host_contact_nip05
                .and_then(|nip05| nip05.split_once('@').map(|(local, _)| local.trim()))
                .filter(|local| !local.is_empty())
                .map(str::to_string)
        })
        .or_else(|| {
            host_alias
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
        .or_else(|| {
            host_contact_npub.map(|npub| {
                let prefix = npub.chars().take(8).collect::<String>();
                if prefix.is_empty() {
                    "daemon".to_string()
                } else {
                    prefix
                }
            })
        })
        .unwrap_or_else(|| "daemon".to_string())
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

fn normalized_joined_status(
    status: &str,
    seat: Option<u32>,
    final_game: bool,
) -> Option<&'static str> {
    if final_game {
        return Some("final");
    }
    if seat.is_some() {
        return Some("joined");
    }
    match status {
        "requested" | "approved" => Some("requested"),
        "rejected" => Some("rejected"),
        "expired" => Some("expired"),
        "final" => Some("final"),
        "joined" => Some("joined"),
        _ => None,
    }
}

fn join_sandbox_game(
    transport: &mut ActiveTransport,
    row: crate::app::OpenGameRow,
    password: &str,
    handle: Option<String>,
) -> Result<SandboxJoinResult, String> {
    match transport
        .session
        .join_sandbox_game(&row.game_id, &row.daemon_pubkey, handle.as_deref())
        .map_err(|err| err.to_string())?
    {
        SandboxJoinOutcome::Full(message) => Ok(SandboxJoinResult::Full(message)),
        SandboxJoinOutcome::Joined(snapshot) => {
            persist_snapshot(&transport.session, password, &snapshot)?;
            let joined_row = joined_row_from_snapshot(&row, &snapshot);
            transport.cache.clear_game_release(&joined_row.game_id);
            transport
                .cache
                .upsert_game(cached_game_from_joined_row(&joined_row));
            replace_roster(&mut transport.cache, &snapshot);
            Ok(SandboxJoinResult::Joined(HostedGameOpenSuccess {
                snapshot,
                row: joined_row,
                cache: transport.cache.clone(),
            }))
        }
    }
}

fn open_hosted_game(
    transport: &mut ActiveTransport,
    row: crate::app::MyGameRow,
    password: &str,
    handle: Option<String>,
) -> Result<HostedGameOpenResult, String> {
    match transport.session.request_state(
        &row.game_id,
        &row.daemon_pubkey,
        row.last_turn,
        row.last_hash.as_deref(),
        handle.as_deref(),
        None,
    ) {
        Ok(snapshot) => {
            persist_snapshot(&transport.session, password, &snapshot)?;
            let joined_row = joined_row_from_snapshot_and_existing(&row, &snapshot);
            transport.cache.clear_game_release(&joined_row.game_id);
            transport
                .cache
                .upsert_game(cached_game_from_joined_row(&joined_row));
            replace_roster(&mut transport.cache, &snapshot);
            Ok(HostedGameOpenResult::Opened(HostedGameOpenSuccess {
                snapshot,
                row: joined_row,
                cache: transport.cache.clone(),
            }))
        }
        Err(err) => handle_open_game_error(transport, row, err),
    }
}

fn complete_first_join_setup(
    transport: &mut ActiveTransport,
    row: crate::app::MyGameRow,
    empire_name: &str,
    homeworld_name: &str,
    password: &str,
) -> Result<HostedGameOpenSuccess, String> {
    let snapshot = transport
        .session
        .complete_first_join_setup(
            &row.game_id,
            &row.daemon_pubkey,
            empire_name,
            homeworld_name,
        )
        .map_err(|err| err.to_string())?;
    persist_snapshot(&transport.session, password, &snapshot)?;
    let joined_row = joined_row_from_snapshot_and_existing(&row, &snapshot);
    transport.cache.clear_game_release(&joined_row.game_id);
    transport
        .cache
        .upsert_game(cached_game_from_joined_row(&joined_row));
    replace_roster(&mut transport.cache, &snapshot);
    Ok(HostedGameOpenSuccess {
        snapshot,
        row: joined_row,
        cache: transport.cache.clone(),
    })
}

fn release_sandbox_game(
    transport: &mut ActiveTransport,
    row: crate::app::MyGameRow,
) -> Result<SandboxReleaseSuccess, String> {
    match transport
        .session
        .release_sandbox_game(&row.game_id, &row.daemon_pubkey)
    {
        Ok(SandboxReleaseProtocolResult { message: _, .. }) => {
            finalize_sandbox_release(transport, row)
        }
        Err(err) => {
            let err = err.to_string();
            if is_already_released_sandbox_error(&err) {
                finalize_sandbox_release(transport, row)
            } else {
                Err(err)
            }
        }
    }
}

fn finalize_sandbox_release(
    transport: &mut ActiveTransport,
    row: crate::app::MyGameRow,
) -> Result<SandboxReleaseSuccess, String> {
    transport.cache.mark_game_released(&row.game_id);
    transport.cache.remove_game(&row.game_id);
    transport.cache.remove_roster(&row.game_id);
    transport.cache.remove_game_inbox_messages(&row.game_id);
    Ok(SandboxReleaseSuccess {
        game_id: row.game_id,
        cache: transport.cache.clone(),
    })
}

fn is_already_released_sandbox_error(message: &str) -> bool {
    let normalized = message.trim().to_ascii_lowercase();
    normalized.contains("no longer have a claimed sandbox seat")
        || normalized.contains("no longer have a claimed seat")
}

#[cfg(test)]
mod tests {
    use super::is_already_released_sandbox_error;

    #[test]
    fn already_released_sandbox_error_is_treated_as_idempotent_cleanup() {
        assert!(is_already_released_sandbox_error(
            "You no longer have a claimed sandbox seat in this game."
        ));
        assert!(is_already_released_sandbox_error(
            "You no longer have a claimed seat in this game."
        ));
        assert!(!is_already_released_sandbox_error(
            "Only sandbox games can be deleted from My Games."
        ));
    }
}

fn handle_open_game_error(
    transport: &mut ActiveTransport,
    mut row: crate::app::MyGameRow,
    err: HostedStateRequestError,
) -> Result<HostedGameOpenResult, String> {
    let expire_sandbox = row.game_tier.eq_ignore_ascii_case("sandbox")
        && err.code == Some(StateErrorCode::NotAPlayer);
    if expire_sandbox {
        row.status = "expired".to_string();
        row.seat = None;
        row.last_hash = None;
        if let Some(game) = transport
            .cache
            .games
            .iter_mut()
            .find(|game| game.id == row.game_id)
        {
            game.status = "expired".to_string();
            game.seat = None;
            game.last_hash = None;
            game.updated_at = now_iso8601();
        }
        return Ok(HostedGameOpenResult::Expired {
            row,
            cache: transport.cache.clone(),
            message: "Your sandbox seat is no longer active. Rejoin from Open Games.".to_string(),
        });
    }
    Err(err.to_string())
}

fn persist_snapshot(
    session: &HostedClientSession,
    password: &str,
    snapshot: &nc_nostr::state_sync::GameState,
) -> Result<(), String> {
    HostedStateStore::open_default()
        .and_then(|store| store.save_snapshot(password, &session.public_key_hex(), snapshot))
        .map_err(|err| err.to_string())
}

fn replace_roster(cache: &mut ClientCache, snapshot: &nc_nostr::state_sync::GameState) {
    cache.replace_roster(
        &snapshot.game_id,
        snapshot
            .state
            .roster
            .iter()
            .map(|entry| nc_client::cache::GameRosterEntry {
                game_id: snapshot.game_id.clone(),
                empire_id: entry.empire_id,
                empire_name: entry.empire_name.clone(),
                is_self: entry.is_self,
            })
            .collect(),
    );
}

fn joined_row_from_snapshot(
    row: &crate::app::OpenGameRow,
    snapshot: &nc_nostr::state_sync::GameState,
) -> crate::app::MyGameRow {
    crate::app::MyGameRow {
        game_id: row.game_id.clone(),
        status: "joined".to_string(),
        game_tier: row.game_tier.clone(),
        game: row.game.clone(),
        host: row.host.clone(),
        host_contact_npub: row.host_contact_npub.clone(),
        relay_url: row.relay_url.clone(),
        daemon_pubkey: row.daemon_pubkey.clone(),
        seat: Some(snapshot.player_seat as u8),
        turn_summary: format!("Y{} T{}", snapshot.year, snapshot.turn),
        last_turn: Some(snapshot.turn),
        last_hash: Some(snapshot.state_hash.clone()),
    }
}

fn joined_row_from_snapshot_and_existing(
    row: &crate::app::MyGameRow,
    snapshot: &nc_nostr::state_sync::GameState,
) -> crate::app::MyGameRow {
    crate::app::MyGameRow {
        game_id: row.game_id.clone(),
        status: "joined".to_string(),
        game_tier: row.game_tier.clone(),
        game: row.game.clone(),
        host: row.host.clone(),
        host_contact_npub: row.host_contact_npub.clone(),
        relay_url: row.relay_url.clone(),
        daemon_pubkey: row.daemon_pubkey.clone(),
        seat: Some(snapshot.player_seat as u8),
        turn_summary: format!("Y{} T{}", snapshot.year, snapshot.turn),
        last_turn: Some(snapshot.turn),
        last_hash: Some(snapshot.state_hash.clone()),
    }
}

fn cached_game_from_joined_row(row: &crate::app::MyGameRow) -> CachedGame {
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
        invite_address: None,
        last_turn: row.last_turn,
        last_hash: row.last_hash.clone(),
        updated_at: now_iso8601(),
    }
}
