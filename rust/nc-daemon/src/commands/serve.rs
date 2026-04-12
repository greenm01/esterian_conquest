use crate::config::{daemon_config, identity, relay};
use crate::supervisor::routing;
use crate::lobby::publish::EventPublisher;
use nostr_sdk::{Client, Filter, Kind, ToBech32, Keys, RelayPoolNotification};
use std::path::PathBuf;
use std::sync::Arc;

pub fn run(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    if args.iter().any(|arg| matches!(*arg, "--help" | "-h")) {
        print_usage();
        return Ok(());
    }

    let mut games_root = None;
    let mut config_path = None;
    let mut identity_path = None;

    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "--root" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --root".into());
                }
                games_root = Some(PathBuf::from(args[i + 1]));
                i += 2;
            }
            "--config" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --config".into());
                }
                config_path = Some(PathBuf::from(args[i + 1]));
                i += 2;
            }
            "--identity" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --identity".into());
                }
                identity_path = Some(PathBuf::from(args[i + 1]));
                i += 2;
            }
            _ => return Err(format!("unknown argument: {}", args[i]).into()),
        }
    }

    let games_root = games_root.ok_or("missing --root argument")?;

    let config = if let Some(path) = config_path {
        daemon_config::DaemonConfig::load(&path)?
    } else if let Ok(default_path) = std::env::var("NC_DAEMON_CONFIG") {
        daemon_config::DaemonConfig::load(&PathBuf::from(default_path))?
    } else {
        let default = daemon_config::DaemonConfig {
            games_root: games_root.clone(),
            relay_url: "wss://relay.example.com".to_string(),
            identity_path: PathBuf::from("/etc/nc-daemon/daemon.nsec"),
            sysop_contact_npub: String::new(),
        };
        default
    };

    let identity_path = identity_path.unwrap_or_else(|| config.identity_path.clone());
    let identity = match identity::DaemonIdentity::load(&identity_path) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Warning: failed to load identity: {}", e);
            eprintln!(
                "Run 'nc-daemon nostr init --path {}' first",
                identity_path.display()
            );
            return Err("identity not configured".into());
        }
    };
    let relay_config = relay::RelayConfig::validate(&config.relay_url)?;

    println!("Starting nc-daemon...");
    println!("  games root: {}", config.games_root.display());
    println!("  relay: {}", relay_config.url);
    println!("  identity: {}", identity.npub);

    run_server(&config, &identity, &relay_config)
}

fn run_server(
    config: &daemon_config::DaemonConfig,
    identity: &identity::DaemonIdentity,
    relay_config: &relay::RelayConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Daemon server starting");
    tracing::info!("Games root: {}", config.games_root.display());
    tracing::info!("Relay: {}", relay_config.url);
    tracing::info!("Identity: {}", identity.npub);

    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        run_async_server(config, identity, relay_config).await
    })
}

async fn run_async_server(
    config: &daemon_config::DaemonConfig,
    identity: &identity::DaemonIdentity,
    relay_config: &relay::RelayConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let keys = Keys::parse(&identity.nsec)?;
    let public_key = keys.public_key();
    let npub = public_key.to_bech32()?;

    tracing::info!("Public key: {}", npub);

    let client = Client::builder()
        .build();

    client.add_relay(&relay_config.url).await?;

    tracing::info!("Connecting to relay: {}", relay_config.url);
    client.connect().await;

    let publisher = Arc::new(EventPublisher::new(client.clone(), keys));
    let games_root = Arc::new(config.games_root.clone());

    let filter = Filter::new()
        .kind(Kind::Custom(30507))
        .kind(Kind::Custom(30513))
        .kind(Kind::Custom(30522));

    let _ = client.subscribe(filter, None).await;

    tracing::info!("Subscribed to kinds 30507, 30513, 30522");
    tracing::info!("Event loop started. Press Ctrl+C to stop.");

    let mut notifications = client.notifications();
    let mut catalog_interval = tokio::time::interval(std::time::Duration::from_secs(300));
    let mut decisions_interval = tokio::time::interval(std::time::Duration::from_secs(60));

    loop {
        tokio::select! {
            _ = catalog_interval.tick() => {
                publish_lobby_catalog(&publisher, &games_root).await;
            }
            _ = decisions_interval.tick() => {
                publish_pending_decisions(&publisher, &games_root).await;
            }
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Shutting down...");
                break;
            }
            notification = notifications.recv() => {
                match notification {
                    Ok(RelayPoolNotification::Event { event, .. }) => {
                        let event = *event;
                        tracing::debug!("Received event: kind={}", u16::from(event.kind));
                        
                        match routing::route_event(event, &games_root, &public_key) {
                            Ok(routed) => {
                                let effects = routing::process_event(&routed);
                                tracing::debug!("Processing {} effects for game {}", effects.len(), routed.game_id);
                                
                                for effect in effects {
                                    handle_effect(effect, &routed, &publisher).await;
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Routing error: {:?}", e);
                            }
                        }
                    }
                    Ok(RelayPoolNotification::Message { relay_url: _, message: _ }) => {}
                    Ok(RelayPoolNotification::Shutdown) => {
                        tracing::info!("Relay pool shutdown");
                        break;
                    }
                    Err(e) => {
                        tracing::error!("Notification error: {}", e);
                    }
                }
            }
        }
    }

    tracing::info!("Daemon stopped");

    Ok(())
}

async fn handle_effect(
    effect: crate::game::effects::GameEffects,
    routed: &routing::RoutedEvent,
    publisher: &EventPublisher,
) {
    match effect {
        crate::game::effects::GameEffects::HandleInviteRequest { request, game_id } => {
            tracing::info!("Handling invite request for game {} from {}", game_id, request.player_pubkey);
            
            if let Err(e) = nc_data::hosted::create_request(
                routed.store.connection(),
                &request.request_id,
                &game_id,
                &request.player_pubkey,
                &request.message,
            ) {
                tracing::error!("Failed to store invite request: {}", e);
                return;
            }

            let receipt = nc_nostr::invite_request::InviteRequestReceipt {
                request_id: request.request_id.clone(),
                game_id: game_id.clone(),
                status: nc_nostr::invite_request::InviteRequestReceiptStatus::Received,
                message: "Your request has been queued for the sysop.".to_string(),
            };

            let content = serde_json::to_string(&receipt).unwrap_or_default();

            let d_tag = request.request_id.clone();
            let gid_tag = game_id.clone();
            let tag_refs: Vec<(&str, &str)> = vec![
                ("d", &d_tag),
                ("game-id", &gid_tag),
                ("status", "received"),
            ];
            
            if let Err(e) = publisher.publish_encrypted(&request.player_pubkey, 30514, &content, tag_refs).await {
                tracing::error!("Failed to publish invite receipt: {}", e);
            } else {
                tracing::info!("Published encrypted invite request receipt to {}", request.player_pubkey);
            }
        }
        crate::game::effects::GameEffects::HandleTurnCommands { commands, game_id } => {
            tracing::info!("Handling turn commands for game {} turn {} from {}", game_id, commands.turn, commands.player_pubkey);
            
            let _seat = match nc_data::hosted::get_seat_by_pubkey(routed.store.connection(), &game_id, &commands.player_pubkey) {
                Ok(Some(s)) => s,
                Ok(None) => {
                    tracing::warn!("Player {} has no claimed seat in game {}", commands.player_pubkey, game_id);
                    return;
                }
                Err(e) => {
                    tracing::error!("Failed to lookup seat: {}", e);
                    return;
                }
            };

            if let Err(e) = nc_data::hosted::enqueue_turn(
                routed.store.connection(),
                &commands.submit_id,
                &game_id,
                commands.turn,
                &commands.player_pubkey,
                &commands.commands,
            ) {
                tracing::error!("Failed to enqueue turn: {}", e);
                return;
            }

            let receipt = nc_nostr::turn_commands::TurnReceipt {
                submit_id: commands.submit_id.clone(),
                game_id: game_id.clone(),
                turn: commands.turn,
                status: nc_nostr::turn_commands::TurnReceiptStatus::Accepted,
                message: Some("Orders staged for the next maintenance run.".to_string()),
                errors: vec![],
            };

            let content = serde_json::to_string(&receipt).unwrap_or_default();

            let d_tag = commands.submit_id.clone();
            let gid_tag = game_id.clone();
            let turn_str = commands.turn.to_string();
            let tag_refs: Vec<(&str, &str)> = vec![
                ("d", &d_tag),
                ("game-id", &gid_tag),
                ("turn", &turn_str),
                ("status", "accepted"),
            ];
            
            if let Err(e) = publisher.publish_encrypted(&commands.player_pubkey, 30524, &content, tag_refs).await {
                tracing::error!("Failed to publish turn receipt: {}", e);
            } else {
                tracing::info!("Published encrypted turn receipt to {}", commands.player_pubkey);
            }
        }
        crate::game::effects::GameEffects::HandleStateRequest { request } => {
            tracing::info!("Handling state request for game {} from {}", request.game_id, request.player_pubkey);
            
            let turn: u32 = 0;
            let year: u32 = 3000;
            
            let state_hash = blake3::hash(format!("{}:{}:{}", request.game_id, turn, "player").as_bytes()).to_hex().to_string();
            
            if let (Some(last_hash), Some(last_turn)) = (&request.last_hash, request.last_turn) {
                if last_hash == &state_hash && last_turn == turn {
                    tracing::debug!("Client already has current state (turn {}, hash {})", turn, state_hash);
                } else {
                    tracing::debug!("Client has stale state (requested turn {}, last_hash matches: {})", 
                        last_turn, last_hash == &state_hash);
                }
            }
            
            let state_payload = serde_json::json!({
                "game_id": request.game_id,
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
            
            let gid_tag = request.game_id.clone();
            let turn_str = turn.to_string();
            let year_str = year.to_string();
            let tag_refs: Vec<(&str, &str)> = vec![
                ("game-id", &gid_tag),
                ("turn", &turn_str),
                ("year", &year_str),
                ("hash", &state_hash),
            ];
            
            if let Err(e) = publisher.publish_encrypted(&request.player_pubkey, 30520, &content, tag_refs).await {
                tracing::error!("Failed to publish state: {}", e);
            } else {
                tracing::info!("Published encrypted state to {}", request.player_pubkey);
            }
        }
        crate::game::effects::GameEffects::InvalidEvent { reason } => {
            tracing::warn!("Invalid event: {}", reason);
        }
        other => {
            tracing::debug!("Unhandled effect: {:?}", other);
        }
    }
}

fn print_usage() {
    println!("Usage: nc-daemon serve --root <games-root> [--config <path>]");
    println!();
    println!("Options:");
    println!("  --root <path>     Games root directory (required)");
    println!("  --config <path>  Config file path (default: /etc/nc-daemon/daemon.kdl)");
    println!("  --identity <path> Identity nsec file (default: from config)");
}

async fn send_claim_error(
    publisher: &EventPublisher,
    player_pubkey: &str,
    nonce: &str,
    error: &str,
    message: &str,
) {
    let payload = serde_json::json!({
        "error": error,
        "message": message,
    });
    let content = serde_json::to_string(&payload).unwrap_or_default();
    let tag_refs: Vec<(&str, &str)> = vec![
        ("d", nonce),
        ("error", error),
    ];
    
    if let Err(e) = publisher.publish_to_pubkey(player_pubkey, 30511, &content, tag_refs).await {
        tracing::error!("Failed to publish claim error: {}", e);
    }
}

async fn publish_lobby_catalog(
    publisher: &EventPublisher,
    games_root: &std::sync::Arc<std::path::PathBuf>,
) {
    use crate::lobby::catalog_publish::publish_game_definition;
    use nc_data::hosted::{get_settings, HostedStore, LobbyVisibility, RecruitingMode};

    if let Ok(entries) = std::fs::read_dir(games_root.as_path()) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            
            let db_path = path.join("hosted.db");
            if !db_path.exists() {
                continue;
            }
            
            let Some(game_id) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            
            let store = match HostedStore::open(&db_path) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("Failed to open game {}: {}", game_id, e);
                    continue;
                }
            };
            
            let settings = match get_settings(store.connection(), game_id) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("Failed to get settings for {}: {}", game_id, e);
                    continue;
                }
            };
            
            if settings.lobby_visibility != LobbyVisibility::Public {
                continue;
            }
            if settings.recruiting == RecruitingMode::None {
                continue;
            }
            
            match publish_game_definition(&store, game_id, settings.host_alias.as_deref()) {
                Ok(Some(def)) => {
                    let content = serde_json::to_string(&def).unwrap_or_default();
                    let tags = vec![("d", game_id)];
                    if let Err(e) = publisher.publish(30500, &content, tags).await {
                        tracing::error!("Failed to publish 30500 for {}: {}", game_id, e);
                    } else {
                        tracing::info!("Published 30500 for game {}", game_id);
                    }
                }
                Ok(None) => {
                    tracing::debug!("Skipped non-public game {}", game_id);
                }
                Err(e) => {
                    tracing::warn!("Failed to build game definition for {}: {}", game_id, e);
                }
            }
        }
    }
}

async fn publish_pending_decisions(
    publisher: &EventPublisher,
    games_root: &std::sync::Arc<std::path::PathBuf>,
) {
    use crate::lobby::invite_decisions::publish_invite_decision;
    use nc_data::hosted::{list_pending_decisions, mark_decision_published, HostedStore};
    use nc_nostr::invite_request::InviteDecision;

    if let Ok(entries) = std::fs::read_dir(games_root.as_path()) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let db_path = path.join("hosted.db");
                if db_path.exists() {
                    if let Some(game_id) = path.file_name().and_then(|n| n.to_str()) {
                        let store = match HostedStore::open(&db_path) {
                            Ok(s) => s,
                            Err(_) => continue,
                        };

                        let pending = match list_pending_decisions(store.connection(), game_id) {
                            Ok(p) => p,
                            Err(_) => continue,
                        };

                        for request in pending {
                            let decision = if request.status == nc_data::hosted::InviteRequestStatus::Approved {
                                let invite = request.issued_invite_code.clone().unwrap_or_default();
                                InviteDecision::Approved { invite }
                            } else {
                                InviteDecision::Rejected
                            };

                            let message = request.decision_message.as_deref().unwrap_or("");

                            match publish_invite_decision(
                                publisher,
                                &request.player_pubkey,
                                &request.id,
                                &request.game_id,
                                decision,
                                message,
                            )
                            .await
                            {
                                Ok(_) => {
                                    let _ = mark_decision_published(store.connection(), &request.id);
                                    tracing::info!(
                                        "Published invite decision {} for request {}",
                                        if request.status == nc_data::hosted::InviteRequestStatus::Approved {
                                            "approved"
                                        } else {
                                            "rejected"
                                        },
                                        request.id
                                    );
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Failed to publish invite decision {}: {}",
                                        request.id,
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
