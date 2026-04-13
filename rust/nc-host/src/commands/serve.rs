use crate::config::{host_config, host_identity, relay};
use crate::lobby::publish::EventPublisher;
use crate::supervisor::routing;
use crate::support::pubkeys::short_pubkey;
use nostr_sdk::{
    Alphabet, Client, Filter, Keys, Kind, RelayPoolNotification, SingleLetterTag, ToBech32,
};
use std::path::PathBuf;
use std::sync::Arc;

const CATALOG_PUBLISH_INTERVAL_SECS: u64 = 300;
const DECISION_PUBLISH_INTERVAL_SECS: u64 = 5;
const OUTBOX_PUBLISH_INTERVAL_SECS: u64 = 1;
const SYSOP_NOTIFICATION_INTERVAL_SECS: u64 = 5;

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
        host_config::HostConfig::load(&path)?
    } else if let Ok(default_path) = std::env::var("NC_HOST_CONFIG") {
        host_config::HostConfig::load(&PathBuf::from(default_path))?
    } else {
        let default = host_config::HostConfig {
            games_root: games_root.clone(),
            relay_url: "wss://relay.example.com".to_string(),
            invite_relay_host: "relay.example.com".to_string(),
            identity_path: PathBuf::from("/etc/nc-host/host.nsec"),
            sysop_contact_npub: String::new(),
            sysop_contact_label: None,
            sysop_contact_nip05: None,
        };
        default
    };

    let identity_path = identity_path.unwrap_or_else(|| config.identity_path.clone());
    let identity = match host_identity::HostIdentity::load(&identity_path) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Warning: failed to load identity: {}", e);
            eprintln!(
                "Run 'nc-host nostr init --path {}' first",
                identity_path.display()
            );
            return Err("identity not configured".into());
        }
    };
    let relay_config = relay::RelayConfig::validate(&config.relay_url)?;

    println!("Starting nc-host...");
    println!("  games root: {}", config.games_root.display());
    println!("  relay: {}", relay_config.url);
    println!("  identity: {}", short_pubkey(&identity.npub));

    run_server(&config, &identity, &relay_config)
}

fn run_server(
    config: &host_config::HostConfig,
    identity: &host_identity::HostIdentity,
    relay_config: &relay::RelayConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Host server starting");
    tracing::info!("Games root: {}", config.games_root.display());
    tracing::info!("Relay: {}", relay_config.url);
    tracing::info!("Identity: {}", short_pubkey(&identity.npub));

    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async { run_async_server(config, identity, relay_config).await })
}

async fn run_async_server(
    config: &host_config::HostConfig,
    identity: &host_identity::HostIdentity,
    relay_config: &relay::RelayConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let keys = Keys::parse(&identity.nsec)?;
    let public_key = keys.public_key();
    let host_hex = public_key.to_hex();
    let npub = public_key.to_bech32()?;

    tracing::info!("Public key: {}", short_pubkey(&npub));

    let client = Client::builder().build();

    client.add_relay(&relay_config.url).await?;

    tracing::info!("Connecting to relay: {}", relay_config.url);
    client.connect().await;

    let publisher = Arc::new(EventPublisher::new(client.clone(), keys.clone()));
    let games_root = Arc::new(config.games_root.clone());

    let worker_registry = Arc::new(crate::supervisor::worker_registry::WorkerRegistry::new(
        config.games_root.clone(),
    ));

    let filter = Filter::new()
        .kind(Kind::Custom(30507))
        .kind(Kind::Custom(30510))
        .kind(Kind::Custom(30513))
        .kind(Kind::Custom(30517))
        .kind(Kind::Custom(30523))
        .kind(Kind::Custom(30522))
        .custom_tag(SingleLetterTag::lowercase(Alphabet::P), host_hex.as_str());

    let _ = client.subscribe(filter, None).await;

    tracing::info!("Subscribed to kinds 30507, 30510, 30513, 30517, 30522, 30523");
    tracing::info!("Event loop started. Press Ctrl+C to stop.");

    let mut notifications = client.notifications();
    let mut catalog_interval = tokio::time::interval(std::time::Duration::from_secs(
        CATALOG_PUBLISH_INTERVAL_SECS,
    ));
    let mut decisions_interval = tokio::time::interval(std::time::Duration::from_secs(
        DECISION_PUBLISH_INTERVAL_SECS,
    ));
    let mut outbox_interval =
        tokio::time::interval(std::time::Duration::from_secs(OUTBOX_PUBLISH_INTERVAL_SECS));
    let mut sysop_notifications_interval = tokio::time::interval(std::time::Duration::from_secs(
        SYSOP_NOTIFICATION_INTERVAL_SECS,
    ));

    loop {
        tokio::select! {
            _ = catalog_interval.tick() => {
                publish_lobby_catalog(
                    &games_root,
                    false,
                    &config.sysop_contact_npub,
                    config.sysop_contact_label.as_deref(),
                    config.sysop_contact_nip05.as_deref(),
                ).await;
            }
            _ = decisions_interval.tick() => {
                publish_pending_decisions(&games_root).await;
            }
            _ = outbox_interval.tick() => {
                process_outbox(&publisher, &games_root).await;
            }
            _ = sysop_notifications_interval.tick() => {
                crate::lobby::notify_sysop::publish_pending_notifications(
                    &publisher,
                    &games_root,
                    &config.sysop_contact_npub,
                ).await;
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

                        match routing::route_event(event.clone(), &games_root, &public_key) {
                            Ok(routed) => {
                                let effects = if event.kind.as_u16() == 30517 {
                                    match nc_nostr::thread_message::decrypt_thread_message(keys.secret_key(), &event) {
                                        Some(message) => vec![crate::game::effects::GameEffects::HandleThreadMessage {
                                            message,
                                            game_id: routed.game_id.clone(),
                                        }],
                                        None => vec![crate::game::effects::GameEffects::InvalidEvent {
                                            reason: "failed to decrypt SysopThreadMessage".to_string(),
                                        }],
                                    }
                                } else {
                                    routing::process_event(&routed, keys.secret_key())
                                };
                                tracing::debug!("Processing {} effects for game {}", effects.len(), routed.game_id);

                                let worker_registry = worker_registry.clone();
                                let game_id = routed.game_id.clone();
                                tokio::spawn(async move {
                                    let worker = worker_registry.get_or_create(game_id).await;

                                    for effect in effects {
                                        let msg = crate::game::msg::GameMsg::HandleEffect(effect);
                                        if let Err(e) = worker.send(msg).await {
                                            tracing::error!("Failed to send effect to worker: {}", e);
                                        }
                                    }
                                });
                            }
                            Err(routing::RoutingError::UnknownGame(game_id)) => {
                                tracing::warn!("Routing error: unknown game {}", game_id);
                                if let Some(request) = nc_nostr::invite_request::parse_invite_request(keys.secret_key(), &event) {
                                    publish_invite_request_receipt_direct(
                                        &publisher,
                                        &request.player_pubkey,
                                        &nc_nostr::invite_request::InviteRequestReceipt {
                                            request_id: request.request_id,
                                            game_id: game_id.clone(),
                                            status: nc_nostr::invite_request::InviteRequestReceiptStatus::UnknownGame,
                                            message: format!("Unknown game: {}", game_id),
                                        },
                                    ).await;
                                }
                            }
                            Err(routing::RoutingError::NotAddressedToHost) => {
                                tracing::debug!("Ignoring event not addressed to this host");
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

    tracing::info!("Host stopped");

    Ok(())
}
fn print_usage() {
    println!("Usage: nc-host serve --root <games-root> [--config <path>]");
    println!();
    println!("Options:");
    println!("  --root <path>     Games root directory (required)");
    println!("  --config <path>  Config file path (default: /etc/nc-host/host.kdl)");
    println!("  --identity <path> Identity nsec file (default: from config)");
}

async fn publish_invite_request_receipt_direct(
    publisher: &EventPublisher,
    player_pubkey: &str,
    receipt: &nc_nostr::invite_request::InviteRequestReceipt,
) {
    let content = match serde_json::to_string(receipt) {
        Ok(content) => content,
        Err(e) => {
            tracing::error!("Failed to serialize invite receipt: {}", e);
            return;
        }
    };

    let tags = nc_nostr::invite_request::build_invite_request_receipt_tags(receipt)
        .into_iter()
        .map(|(key, value)| vec![key.to_string(), value])
        .collect();

    if let Err(e) = publisher
        .publish_encrypted_multi(player_pubkey, 30514, &content, tags)
        .await
    {
        tracing::error!(
            "Failed to publish invite receipt to {}: {}",
            short_pubkey(player_pubkey),
            e
        );
    }
}

async fn publish_lobby_catalog(
    games_root: &std::sync::Arc<std::path::PathBuf>,
    force: bool,
    host_contact_npub: &str,
    host_contact_label: Option<&str>,
    host_contact_nip05: Option<&str>,
) {
    use crate::lobby::catalog_publish::publish_game_definition;
    use nc_data::hosted::{
        HostedStore, LobbyVisibility, RecruitingMode, clear_catalog_dirty, get_catalog_dirty_since,
        get_settings,
    };
    use nc_nostr::game_definition::build_game_definition_tags;

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

            if !force {
                if let Ok(None) = get_catalog_dirty_since(store.connection(), game_id) {
                    tracing::debug!("Skipping {} (catalog clean)", game_id);
                    continue;
                }
            }

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

            match publish_game_definition(
                &store,
                game_id,
                settings.host_alias.as_deref(),
                Some(host_contact_npub),
                host_contact_label,
                host_contact_nip05,
            ) {
                Ok(Some(def)) => {
                    let tags = build_game_definition_tags(&def);
                    if let Err(e) = crate::game::outbox::enqueue_public_event(
                        store.connection(),
                        game_id,
                        30500,
                        "",
                        tags,
                    ) {
                        tracing::error!("Failed to queue 30500 for {}: {}", game_id, e);
                    } else {
                        tracing::info!("Queued 30500 for game {}", game_id);
                        let _ = clear_catalog_dirty(store.connection(), game_id);
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

async fn publish_pending_decisions(games_root: &std::sync::Arc<std::path::PathBuf>) {
    use crate::lobby::invite_decisions::enqueue_invite_decision;
    use nc_data::hosted::{HostedStore, list_pending_decisions, mark_decision_published};
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
                            let decision = if request.status
                                == nc_data::hosted::InviteRequestStatus::Approved
                            {
                                let invite = request.issued_invite_code.clone().unwrap_or_default();
                                InviteDecision::Approved { invite }
                            } else {
                                InviteDecision::Rejected
                            };

                            let message = request.decision_message.as_deref().unwrap_or("");

                            match enqueue_invite_decision(
                                &store,
                                game_id,
                                &request.player_pubkey,
                                &request.id,
                                decision,
                                message,
                            ) {
                                Ok(_) => {
                                    let _ =
                                        mark_decision_published(store.connection(), &request.id);
                                    tracing::info!(
                                        "Published invite decision {} for request {}",
                                        if request.status
                                            == nc_data::hosted::InviteRequestStatus::Approved
                                        {
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

async fn process_outbox(
    publisher: &EventPublisher,
    games_root: &std::sync::Arc<std::path::PathBuf>,
) {
    use nc_data::hosted::{HostedStore, get_pending, mark_failed, mark_published};

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
                Err(_) => continue,
            };

            let pending = match get_pending(store.connection(), game_id, 10) {
                Ok(items) => items,
                Err(_) => continue,
            };

            for item in pending {
                let tags: Vec<Vec<String>> = serde_json::from_str(&item.tags).unwrap_or_default();
                let publish_result = if item.pubkey.is_empty() {
                    publisher
                        .publish_multi(item.kind, &item.content, tags)
                        .await
                } else {
                    publisher
                        .publish_encrypted_multi(&item.pubkey, item.kind, &item.content, tags)
                        .await
                };

                match publish_result {
                    Ok(_) => {
                        let _ = mark_published(store.connection(), &item.id, "relay");
                        tracing::debug!("Published outbox item {} for game {}", item.id, game_id);
                    }
                    Err(e) => {
                        let _ = mark_failed(store.connection(), &item.id, &e.to_string());
                        tracing::warn!("Failed to publish outbox item {}: {}", item.id, e);
                    }
                }
            }
        }
    }
}
