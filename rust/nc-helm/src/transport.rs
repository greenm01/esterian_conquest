use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use nc_client::hosted::session::{CatalogGame, HostedClientSession};

#[derive(Debug, Clone)]
pub struct LobbySnapshot {
    pub games: Vec<crate::app::GameRow>,
    pub notices: Vec<String>,
}

#[derive(Debug)]
pub enum TransportCommand {
    Connect {
        relay_url: String,
        nsec: String,
        reply_to: Sender<Result<LobbySnapshot, String>>,
    },
    Disconnect,
}

#[derive(Debug)]
pub struct TransportActor {
    tx: Sender<TransportCommand>,
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
        reply_to: Sender<Result<LobbySnapshot, String>>,
    ) -> Result<(), String> {
        self.tx
            .send(TransportCommand::Connect {
                relay_url,
                nsec,
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
    let mut active: Option<(HostedClientSession, Sender<Result<LobbySnapshot, String>>)> = None;
    let mut next_poll = Instant::now();

    loop {
        if let Some((session, reply_to)) = &active {
            if Instant::now() >= next_poll {
                let result = fetch_snapshot(session);
                let _ = reply_to.send(result);
                next_poll = Instant::now() + Duration::from_secs(15);
            }
        }

        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(TransportCommand::Connect {
                relay_url,
                nsec,
                reply_to,
            }) => match nostr_sdk::Keys::parse(&nsec) {
                Ok(keys) => {
                    let session = HostedClientSession::new(keys, relay_url);
                    let first_result = fetch_snapshot(&session);
                    let _ = reply_to.send(first_result);
                    active = Some((session, reply_to));
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

fn fetch_snapshot(session: &HostedClientSession) -> Result<LobbySnapshot, String> {
    let catalog = session.fetch_catalog().map_err(|err| err.to_string())?;
    let notices = session
        .fetch_lobby_notices(86_400)
        .map_err(|err| err.to_string())?;
    Ok(LobbySnapshot {
        games: catalog.into_iter().map(game_row_from_catalog).collect(),
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
    })
}

fn game_row_from_catalog(game: CatalogGame) -> crate::app::GameRow {
    crate::app::GameRow {
        game_id: game.definition.game_id,
        name: game.definition.game_name,
        host: game
            .definition
            .host_alias
            .unwrap_or_else(|| "daemon".to_string()),
        tier: game
            .definition
            .game_tier
            .map(|tier| tier.as_str().to_string())
            .unwrap_or_else(|| "open".to_string()),
        seats: format!(
            "{}/{}",
            game.definition.players - game.definition.open_seats,
            game.definition.players
        ),
        when: format!("Y{} T{}", game.definition.year, game.definition.turn),
    }
}
