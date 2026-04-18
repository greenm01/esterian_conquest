use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use nc_client::cache::{CachedGame, ClientCache};
use nc_client::hosted::session::{CatalogGame, HostedClientSession, PlayerEventBatch};
use nc_client::keychain::now_iso8601;
use nc_nostr::game_definition::{GameStatus, RecruitingMode};
use nc_nostr::invite_request::{InviteDecision, InviteRequestReceiptStatus};

const PLAYER_EVENT_LOOKBACK_SECS: u64 = 30 * 24 * 60 * 60;

#[derive(Debug, Clone)]
pub struct LobbySnapshot {
    pub cache: ClientCache,
    pub my_games: Vec<crate::app::MyGameRow>,
    pub open_games: Vec<crate::app::OpenGameRow>,
    pub notices: Vec<String>,
}

#[derive(Debug)]
pub enum TransportCommand {
    Connect {
        relay_url: String,
        nsec: String,
        cache: ClientCache,
        reply_to: Sender<Result<LobbySnapshot, String>>,
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
        open_games: build_open_games(&catalog),
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
                seat: game.seat.and_then(|seat| u8::try_from(seat).ok()),
                turn_summary,
                last_turn: game.last_turn,
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

fn build_open_games(catalog: &[CatalogGame]) -> Vec<crate::app::OpenGameRow> {
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
