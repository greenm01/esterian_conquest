use crate::game::effects::GameEffects;
use crate::game::msg::GameMsg;
use crate::lobby::publish::EventPublisher;
use nc_data::hosted::{self, HostedStore};
use nc_nostr::invite_request::{InviteRequest, InviteRequestReceipt, InviteRequestReceiptStatus};
use nc_nostr::state_sync::StateRequest;
use nc_nostr::turn_commands::{TurnCommands, TurnReceipt, TurnReceiptStatus};
use nostr_sdk::Keys;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use std::fmt;

pub struct GameWorker {
    game_id: String,
    db_path: PathBuf,
    publisher: EventPublisher,
    keys: Arc<Keys>,
}

impl GameWorker {
    pub fn new(
        game_id: String,
        db_path: PathBuf,
        publisher: EventPublisher,
        keys: Arc<Keys>,
    ) -> Self {
        Self {
            game_id,
            db_path,
            publisher,
            keys,
        }
    }

    pub async fn handle_effect(&self, effect: GameEffects) {
        let store = match HostedStore::open(&self.db_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to open store for {}: {}", self.game_id, e);
                return;
            }
        };

        match effect {
            GameEffects::HandleStateRequest { request } => {
                self.handle_state_request(request).await;
            }
            GameEffects::HandleInviteRequest { request, .. } => {
                self.handle_invite_request(request).await;
            }
            GameEffects::HandleTurnCommands { commands, .. } => {
                self.handle_turn_commands(commands).await;
            }
            GameEffects::QueueEvent { .. } => {}
            GameEffects::UpdateLobbyCatalog { .. } => {}
            GameEffects::NotifySysop { .. } => {}
            GameEffects::RunMaintenance { .. } => {}
            GameEffects::InvalidEvent { reason } => {
                tracing::warn!("Invalid event for game {}: {}", self.game_id, reason);
            }
        }
    }

    async fn handle_state_request(&self, request: StateRequest) {
        let turn: u32 = 0;
        let year: u32 = 3000;

        let state_hash = blake3::hash(
            format!("{}:{}:{}", self.game_id, turn, "player").as_bytes()
        ).to_hex().to_string();

        let state_payload = serde_json::json!({
            "game_id": self.game_id,
            "turn": turn,
            "year": year,
            "player_seat": 1,
            "player_name": "Player 1",
            "state_hash": state_hash,
            "state": serde_json::Value::Null,
            "queued_mail": Vec::<serde_json::Value>::new(),
            "report_blocks": Vec::<serde_json::Value>::new(),
        });

        let content = serde_json::to_string(&state_payload).unwrap_or_default();
        
        let gid_tag = self.game_id.clone();
        let turn_str = turn.to_string();
        let year_str = year.to_string();
        let tag_refs: Vec<(&str, &str)> = vec![
            ("game-id", &gid_tag),
            ("turn", &turn_str),
            ("year", &year_str),
            ("hash", &state_hash),
        ];

        if let Err(e) = self.publisher.publish_encrypted(&request.player_pubkey, 30520, &content, tag_refs).await {
            tracing::error!("Failed to publish state: {}", e);
        } else {
            tracing::info!("Published encrypted state to {}", request.player_pubkey);
        }
    }

    async fn handle_invite_request(&self, request: InviteRequest) {
        let store = match HostedStore::open(&self.db_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to open store: {}", e);
                return;
            }
        };

        if let Err(e) = hosted::create_request(
            store.connection(),
            &request.request_id,
            &self.game_id,
            &request.player_pubkey,
            &request.message,
        ) {
            tracing::error!("Failed to store invite request: {}", e);
            return;
        }

        let receipt = InviteRequestReceipt {
            request_id: request.request_id.clone(),
            game_id: self.game_id.clone(),
            status: InviteRequestReceiptStatus::Received,
            message: "Your request has been queued for the sysop.".to_string(),
        };

        let content = serde_json::to_string(&receipt).unwrap_or_default();

        let d_tag = request.request_id.clone();
        let gid_tag = self.game_id.clone();
        let tag_refs: Vec<(&str, &str)> = vec![
            ("d", &d_tag),
            ("game-id", &gid_tag),
            ("status", "received"),
        ];
        
        if let Err(e) = self.publisher.publish_encrypted(&request.player_pubkey, 30514, &content, tag_refs).await {
            tracing::error!("Failed to publish invite receipt: {}", e);
        } else {
            tracing::info!("Published encrypted invite request receipt to {}", request.player_pubkey);
        }
    }

    async fn handle_turn_commands(&self, commands: TurnCommands) {
        let store = match HostedStore::open(&self.db_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to open store: {}", e);
                return;
            }
        };

        let _seat = match hosted::get_seat_by_pubkey(store.connection(), &self.game_id, &commands.player_pubkey) {
            Ok(Some(s)) => s,
            Ok(None) => {
                tracing::warn!("Player {} has no claimed seat in game {}", commands.player_pubkey, self.game_id);
                return;
            }
            Err(e) => {
                tracing::error!("Failed to lookup seat: {}", e);
                return;
            }
        };

        if let Err(e) = hosted::enqueue_turn(
            store.connection(),
            &commands.submit_id,
            &self.game_id,
            commands.turn,
            &commands.player_pubkey,
            &commands.commands,
        ) {
            tracing::error!("Failed to enqueue turn: {}", e);
            return;
        }

        let receipt = TurnReceipt {
            submit_id: commands.submit_id.clone(),
            game_id: self.game_id.clone(),
            turn: commands.turn,
            status: TurnReceiptStatus::Accepted,
            message: Some("Orders staged for the next maintenance run.".to_string()),
            errors: vec![],
        };

        let content = serde_json::to_string(&receipt).unwrap_or_default();

        let d_tag = commands.submit_id.clone();
        let gid_tag = self.game_id.clone();
        let turn_str = commands.turn.to_string();
        let tag_refs: Vec<(&str, &str)> = vec![
            ("d", &d_tag),
            ("game-id", &gid_tag),
            ("turn", &turn_str),
            ("status", "accepted"),
        ];
        
        if let Err(e) = self.publisher.publish_encrypted(&commands.player_pubkey, 30524, &content, tag_refs).await {
            tracing::error!("Failed to publish turn receipt: {}", e);
        } else {
            tracing::info!("Published encrypted turn receipt to {}", commands.player_pubkey);
        }
    }
}

#[derive(Clone)]
pub struct GameWorkerHandle {
    pub game_id: String,
    sender: mpsc::Sender<GameMsg>,
}

impl fmt::Debug for GameWorkerHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GameWorkerHandle")
            .field("game_id", &self.game_id)
            .finish()
    }
}

impl GameWorkerHandle {
    pub async fn send(&self, msg: GameMsg) -> Result<(), mpsc::error::SendError<GameMsg>> {
        self.sender.send(msg).await
    }
}

pub fn spawn_worker(
    game_id: String,
    db_path: PathBuf,
    publisher: EventPublisher,
    keys: Arc<Keys>,
) -> GameWorkerHandle {
    let (tx, mut rx) = mpsc::channel::<GameMsg>(100);
    
    let worker = GameWorker::new(game_id.clone(), db_path, publisher, keys);
    let worker = Arc::new(worker);
    
    let game_id_clone = game_id.clone();
    
    tokio::spawn(async move {
        tracing::debug!("Game worker started for {}", game_id_clone);
        
        while let Some(msg) = rx.recv().await {
            match msg {
                GameMsg::Tick => {}
                GameMsg::PublishLobbyCatalog => {}
                GameMsg::ProcessInviteRequest { request_id } => {
                    tracing::debug!("Processing invite request {} for {}", request_id, game_id_clone);
                }
                GameMsg::ProcessTurnSubmission { submit_id } => {
                    tracing::debug!("Processing turn submission {} for {}", submit_id, game_id_clone);
                }
                GameMsg::HandleEffect(effect) => {
                    worker.handle_effect(effect).await;
                }
            }
        }
        
        tracing::debug!("Game worker stopped for {}", game_id_clone);
    });
    
    GameWorkerHandle {
        game_id,
        sender: tx,
    }
}
