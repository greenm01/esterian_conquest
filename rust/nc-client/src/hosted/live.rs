use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

use nc_nostr::claim::SeatClaimResultPayload;
use nc_nostr::contact_message::{ContactMessage, decrypt_contact_message};
use nc_nostr::game_definition::parse_game_definition;
use nc_nostr::invite_request::{InviteDecisionPayload, InviteRequestReceipt};
use nc_nostr::lobby_notice::{LobbyNotice, parse_lobby_notice};
use nc_nostr::player_message::{PlayerMessage, decrypt_player_message};
use nc_nostr::private_payload::decrypt_private_json_from_event;
use nc_nostr::state_sync::{GameState, StateDelta};
use nc_nostr::thread_message::{SysopThreadMessage, decrypt_thread_message};
use nc_nostr::turn_commands::TurnReceipt;
use nostr_sdk::{
    Alphabet, Client, Filter, Keys, Kind, RelayPoolNotification, SingleLetterTag, Timestamp,
};
use tokio::sync::mpsc::{self as tokio_mpsc, UnboundedReceiver, UnboundedSender};

use super::session::{CatalogGame, PlayerEventBatch};

const LOOKBACK_SECS: u64 = 30 * 24 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostedLiveOptions {
    pub include_public_stream: bool,
    pub include_private_stream: bool,
    pub enable_backfill: bool,
}

impl Default for HostedLiveOptions {
    fn default() -> Self {
        Self {
            include_public_stream: true,
            include_private_stream: true,
            enable_backfill: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostedSessionStatus {
    Connected,
    Synced,
    Error,
}

#[derive(Debug, Clone, Default)]
pub struct HostedSessionUpdate {
    pub catalog: Vec<CatalogGame>,
    pub notices: Vec<LobbyNotice>,
    pub threads: Vec<SysopThreadMessage>,
    pub contact_messages: Vec<ContactMessage>,
    pub player_messages: Vec<PlayerMessage>,
    pub player_events: PlayerEventBatch,
    pub status: Option<HostedSessionStatus>,
}

enum LiveCommand {
    RefreshBackfill,
    Stop,
}

pub struct HostedLiveSession {
    command_tx: UnboundedSender<LiveCommand>,
    update_rx: Receiver<HostedSessionUpdate>,
}

impl Drop for HostedLiveSession {
    fn drop(&mut self) {
        let _ = self.command_tx.send(LiveCommand::Stop);
    }
}

impl HostedLiveSession {
    pub fn start(keys: Keys, relay_url: impl Into<String>) -> Self {
        Self::start_with_options(keys, relay_url, HostedLiveOptions::default())
    }

    pub fn start_with_options(
        keys: Keys,
        relay_url: impl Into<String>,
        options: HostedLiveOptions,
    ) -> Self {
        let relay_url = relay_url.into();
        let (update_tx, update_rx) = mpsc::channel::<HostedSessionUpdate>();
        let (command_tx, command_rx) = tokio_mpsc::unbounded_channel::<LiveCommand>();

        thread::spawn(move || {
            let runtime = match tokio::runtime::Runtime::new() {
                Ok(runtime) => runtime,
                Err(err) => {
                    eprintln!("error: failed to start hosted live runtime: {}", err);
                    return;
                }
            };
            runtime.block_on(async move {
                if let Err(err) =
                    run_live_session(keys, relay_url, options, update_tx, command_rx).await
                {
                    eprintln!("error: hosted live session stopped: {}", err);
                }
            });
        });

        Self {
            command_tx,
            update_rx,
        }
    }

    pub fn refresh_backfill(&self) {
        let _ = self.command_tx.send(LiveCommand::RefreshBackfill);
    }

    pub fn drain_updates(&self) -> Vec<HostedSessionUpdate> {
        let mut out = Vec::new();
        loop {
            match self.update_rx.try_recv() {
                Ok(update) => out.push(update),
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }
        out
    }

    pub fn stop(&self) {
        let _ = self.command_tx.send(LiveCommand::Stop);
    }
}

async fn run_live_session(
    keys: Keys,
    relay_url: String,
    options: HostedLiveOptions,
    update_tx: mpsc::Sender<HostedSessionUpdate>,
    mut command_rx: UnboundedReceiver<LiveCommand>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new(keys.clone());
    if let Err(err) = client.add_relay(&relay_url).await {
        send_status_update(&update_tx, HostedSessionStatus::Error);
        return Err(err.into());
    }
    client.connect().await;

    let public_key_hex = keys.public_key().to_hex();
    let public_filter = options.include_public_stream.then(|| {
        Filter::new()
            .kinds([Kind::Custom(30500), Kind::Custom(30516)])
            .since(Timestamp::now() - Duration::from_secs(LOOKBACK_SECS))
    });
    let private_filter = options.include_private_stream.then(|| {
        Filter::new()
            .kinds([
                Kind::Custom(30518),
                Kind::Custom(30511),
                Kind::Custom(30514),
                Kind::Custom(30515),
                Kind::Custom(30517),
                Kind::Custom(30520),
                Kind::Custom(30521),
                Kind::Custom(30523),
                Kind::Custom(30524),
            ])
            .custom_tag(
                SingleLetterTag::lowercase(Alphabet::P),
                public_key_hex.as_str(),
            )
            .since(Timestamp::now() - Duration::from_secs(LOOKBACK_SECS))
    });

    if let Some(filter) = public_filter.as_ref() {
        if let Err(err) = client.subscribe(filter.clone(), None).await {
            send_status_update(&update_tx, HostedSessionStatus::Error);
            return Err(err.into());
        }
    }
    if let Some(filter) = private_filter.as_ref() {
        if let Err(err) = client.subscribe(filter.clone(), None).await {
            send_status_update(&update_tx, HostedSessionStatus::Error);
            return Err(err.into());
        }
    }
    send_status_update(&update_tx, HostedSessionStatus::Connected);

    if options.enable_backfill
        && let Err(err) =
            backfill(&client, &keys, public_filter.as_ref(), private_filter.as_ref(), &update_tx)
                .await
    {
        send_status_update(&update_tx, HostedSessionStatus::Error);
        eprintln!("warning: hosted live initial backfill failed: {err}");
    }

    let mut notifications = client.notifications();
    loop {
        tokio::select! {
            maybe_command = command_rx.recv() => {
                match maybe_command {
                    Some(LiveCommand::RefreshBackfill) => {
                        if options.enable_backfill {
                            let refresh_public = options.include_public_stream.then(|| {
                                Filter::new().kinds([Kind::Custom(30500), Kind::Custom(30516)])
                            });
                            let refresh_private = options.include_private_stream.then(|| {
                                Filter::new()
                                    .kinds([
                                        Kind::Custom(30518),
                                        Kind::Custom(30511),
                                        Kind::Custom(30514),
                                        Kind::Custom(30515),
                                        Kind::Custom(30517),
                                        Kind::Custom(30520),
                                        Kind::Custom(30521),
                                        Kind::Custom(30523),
                                        Kind::Custom(30524),
                                    ])
                                    .custom_tag(
                                        SingleLetterTag::lowercase(Alphabet::P),
                                        public_key_hex.as_str(),
                                    )
                            });
                            if let Err(err) = backfill(
                                &client,
                                &keys,
                                refresh_public.as_ref(),
                                refresh_private.as_ref(),
                                &update_tx,
                            ).await {
                                send_status_update(&update_tx, HostedSessionStatus::Error);
                                eprintln!("warning: hosted live refresh backfill failed: {err}");
                            }
                        }
                    }
                    Some(LiveCommand::Stop) | None => {
                        client.disconnect().await;
                        break;
                    }
                }
            }
            notification = notifications.recv() => {
                match notification {
                    Ok(RelayPoolNotification::Event { event, .. }) => {
                        if let Some(update) = parse_event(&keys, &event) {
                            let _ = update_tx.send(update);
                        }
                    }
                    Ok(_) => {}
                    Err(err) => {
                        send_status_update(&update_tx, HostedSessionStatus::Error);
                        eprintln!("warning: hosted live notification error: {}", err);
                        tokio::time::sleep(Duration::from_millis(250)).await;
                    }
                }
            }
        }
    }

    Ok(())
}

async fn backfill(
    client: &Client,
    keys: &Keys,
    public_filter: Option<&Filter>,
    private_filter: Option<&Filter>,
    update_tx: &mpsc::Sender<HostedSessionUpdate>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(filter) = public_filter {
        let events = client.fetch_events(filter.clone(), Duration::from_secs(8)).await?;
        for event in events.iter() {
            if let Some(update) = parse_event(keys, event) {
                let _ = update_tx.send(update);
            }
        }
    }
    if let Some(filter) = private_filter {
        let events = client.fetch_events(filter.clone(), Duration::from_secs(8)).await?;
        for event in events.iter() {
            if let Some(update) = parse_event(keys, event) {
                let _ = update_tx.send(update);
            }
        }
    }
    send_status_update(update_tx, HostedSessionStatus::Synced);

    Ok(())
}

fn send_status_update(update_tx: &mpsc::Sender<HostedSessionUpdate>, status: HostedSessionStatus) {
    let _ = update_tx.send(HostedSessionUpdate {
        status: Some(status),
        ..HostedSessionUpdate::default()
    });
}

fn parse_event(keys: &Keys, event: &nostr_sdk::Event) -> Option<HostedSessionUpdate> {
    let mut update = HostedSessionUpdate::default();
    match event.kind.as_u16() {
        30500 => {
            let definition = parse_game_definition(event)?;
            update.catalog.push(CatalogGame {
                daemon_pubkey: event.pubkey.to_hex(),
                definition,
                published_at: event.created_at.as_secs(),
            });
        }
        30516 => {
            update.notices.push(parse_lobby_notice(event)?);
        }
        30517 => {
            update
                .threads
                .push(decrypt_thread_message(keys.secret_key(), event)?);
        }
        30518 => {
            update
                .contact_messages
                .push(decrypt_contact_message(keys.secret_key(), event)?);
        }
        30511 => {
            update
                .player_events
                .claim_results
                .push(decrypt_json::<SeatClaimResultPayload>(keys, event)?);
        }
        30514 => {
            update
                .player_events
                .receipts
                .push(decrypt_json::<InviteRequestReceipt>(keys, event)?);
        }
        30515 => {
            update
                .player_events
                .decisions
                .push(decrypt_json::<InviteDecisionPayload>(keys, event)?);
        }
        30520 => {
            update
                .player_events
                .states
                .push(decrypt_json::<GameState>(keys, event)?);
        }
        30523 => {
            update
                .player_messages
                .push(decrypt_player_message(keys.secret_key(), event)?);
        }
        30521 => {
            update
                .player_events
                .deltas
                .push(decrypt_json::<StateDelta>(keys, event)?);
        }
        30524 => {
            update
                .player_events
                .turn_receipts
                .push(decrypt_json::<TurnReceipt>(keys, event)?);
        }
        _ => return None,
    }
    Some(update)
}

fn decrypt_json<T: serde::de::DeserializeOwned>(
    keys: &Keys,
    event: &nostr_sdk::Event,
) -> Option<T> {
    decrypt_private_json_from_event(keys.secret_key(), event).ok()
}
