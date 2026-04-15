use nc_data::hosted::{
    CatalogState, HostedStore, get_game_metadata, get_settings, mark_catalog_dirty, update_settings,
};
use std::path::PathBuf;

use crate::config::{host_config::HostConfig, host_identity::HostIdentity, relay::RelayConfig};
use crate::lobby::catalog_publish::publish_game_definition;
use crate::lobby::publish::EventPublisher;
use nc_nostr::game_definition::build_game_definition_tags;
use nostr_sdk::Keys;

pub fn run(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || args.iter().any(|arg| matches!(*arg, "--help" | "-h")) {
        print_usage();
        return Ok(());
    }

    let mut games_root = None;
    let mut subcmd = None;
    let mut game_dir = None;
    let mut config_path = None;
    let mut identity_path = None;
    let mut yes = false;

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
            "--dir" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --dir".into());
                }
                game_dir = Some(PathBuf::from(args[i + 1]));
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
            "--yes" => {
                yes = true;
                i += 1;
            }
            _ => {
                if subcmd.is_none() {
                    subcmd = Some(args[i]);
                } else {
                    return Err(format!("unexpected argument: {}", args[i]).into());
                }
                i += 1;
            }
        }
    }

    match subcmd {
        Some("list") => {
            let root = games_root.ok_or("missing --root argument")?;
            run_list(&root)
        }
        Some("status") => {
            let dir = game_dir.ok_or("missing --dir argument")?;
            run_status(&dir)
        }
        Some("retire") => {
            let dir = game_dir.ok_or("missing --dir argument")?;
            run_set_catalog_state(
                &dir,
                CatalogState::Retired,
                config_path.as_ref(),
                identity_path.as_ref(),
            )
        }
        Some("relist") => {
            let dir = game_dir.ok_or("missing --dir argument")?;
            run_set_catalog_state(
                &dir,
                CatalogState::Listed,
                config_path.as_ref(),
                identity_path.as_ref(),
            )
        }
        Some("delete") => {
            let dir = game_dir.ok_or("missing --dir argument")?;
            run_delete(&dir, yes, config_path.as_ref(), identity_path.as_ref())
        }
        Some(cmd) => Err(format!("unknown games subcommand: {}", cmd).into()),
        None => {
            print_usage();
            Ok(())
        }
    }
}

fn run_list(games_root: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let mut count = 0;

    if let Ok(entries) = std::fs::read_dir(games_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let db_path = path.join("hosted.db");
                if db_path.exists() {
                    if let Some(game_id) = path.file_name().and_then(|n| n.to_str()) {
                        match HostedStore::open(&db_path) {
                            Ok(store) => {
                                if let Ok(meta) = get_game_metadata(store.connection(), game_id) {
                                    let catalog_state = get_settings(store.connection(), game_id)
                                        .ok()
                                        .map(|settings| settings.catalog_state.as_str().to_string())
                                        .unwrap_or_else(|| "listed".to_string());
                                    println!(
                                        "{}  year {} turn {}  status: {}  catalog: {}  players: {}",
                                        game_id,
                                        meta.current_year,
                                        meta.current_turn,
                                        meta.status,
                                        catalog_state,
                                        meta.players
                                    );
                                    count += 1;
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to open game {}: {}", game_id, e);
                            }
                        }
                    }
                }
            }
        }
    }

    println!("\nTotal games: {}", count);
    Ok(())
}

fn run_status(game_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let status = crate::status::collect::collect_game_status(game_dir)?;

    println!("Game: {}", status.game_id);
    println!("  Name: {}", status.name);
    println!("  Status: {}", status.status);
    println!("  Year: {}, Turn: {}", status.year, status.turn);
    println!("  Players: {}", status.players);
    println!("  Claimed seats: {}", status.claimed_seats);
    println!("  Open seats: {}", status.open_seats);
    println!("  Recruiting: {}", status.recruiting);
    println!("  Lobby: {}", status.lobby_visibility);
    println!("  Catalog: {}", status.catalog_state);
    println!(
        "  Maintenance: {}",
        if status.maintenance_due_now {
            "due"
        } else if status.maintenance_enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!("  Pending requests: {}", status.pending_requests);
    println!("  Pending decisions: {}", status.pending_decisions);
    println!("  Pending turns: {}", status.pending_turns);
    println!("  Outbox pending: {}", status.outbox_pending);
    println!("  Outbox failed: {}", status.outbox_failed);

    Ok(())
}

fn run_set_catalog_state(
    game_dir: &PathBuf,
    catalog_state: CatalogState,
    config_path: Option<&PathBuf>,
    identity_path: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let game_id = game_id_from_dir(game_dir)?;
    let db_path = game_dir.join("hosted.db");
    let store = HostedStore::open(&db_path)?;
    let mut settings = get_settings(store.connection(), &game_id)?;
    settings.catalog_state = catalog_state.clone();
    update_settings(store.connection(), &game_id, &settings)?;
    mark_catalog_dirty(store.connection(), &game_id)?;

    if should_publish_catalog_immediately(&settings) {
        publish_catalog_now(
            &store,
            &game_id,
            config_path,
            identity_path,
            &format!("publish {} catalog state", catalog_state.as_str()),
        )?;
    }

    println!(
        "Set catalog state for {} to {}.",
        game_id,
        catalog_state.as_str()
    );
    Ok(())
}

fn run_delete(
    game_dir: &PathBuf,
    yes: bool,
    config_path: Option<&PathBuf>,
    identity_path: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !yes {
        return Err("refusing to delete without --yes".into());
    }

    let game_id = game_id_from_dir(game_dir)?;
    let db_path = game_dir.join("hosted.db");
    let store = HostedStore::open(&db_path)?;
    let mut settings = get_settings(store.connection(), &game_id)?;
    settings.catalog_state = CatalogState::Retired;
    update_settings(store.connection(), &game_id, &settings)?;
    mark_catalog_dirty(store.connection(), &game_id)?;
    publish_catalog_now(
        &store,
        &game_id,
        config_path,
        identity_path,
        "publish retired tombstone before delete",
    )?;

    std::fs::remove_dir_all(game_dir)?;
    println!("Deleted hosted game {}.", game_id);
    Ok(())
}

fn game_id_from_dir(game_dir: &std::path::Path) -> Result<String, Box<dyn std::error::Error>> {
    game_dir
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .ok_or_else(|| "invalid game directory name".into())
}

fn should_publish_catalog_immediately(settings: &nc_data::hosted::GameSettings) -> bool {
    settings.catalog_state == CatalogState::Retired
        || (settings.catalog_state == CatalogState::Listed
            && settings.lobby_visibility == nc_data::hosted::LobbyVisibility::Public
            && settings.recruiting != nc_data::hosted::RecruitingMode::None)
}

fn publish_catalog_now(
    store: &HostedStore,
    game_id: &str,
    config_path: Option<&PathBuf>,
    identity_path: Option<&PathBuf>,
    action: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let context = load_host_context(config_path, identity_path)?;
    let def = context.runtime.block_on(async {
        let def = publish_game_definition(
            store,
            game_id,
            None,
            Some(&context.config.sysop_contact_npub),
            context.config.sysop_contact_label.as_deref(),
            context.config.sysop_contact_nip05.as_deref(),
        )?
        .ok_or_else(|| format!("no catalog event available for {}", game_id))?;
        let tags = build_game_definition_tags(&def);
        context
            .publisher
            .publish_multi(30500, "", tags)
            .await
            .map_err(|err| err.to_string())?;
        Ok::<_, Box<dyn std::error::Error>>(def)
    })?;
    println!(
        "Published {} for {} as {}.",
        action,
        game_id,
        def.catalog_state.as_str()
    );
    Ok(())
}

struct HostPublishContext {
    config: HostConfig,
    publisher: EventPublisher,
    runtime: tokio::runtime::Runtime,
}

fn load_host_context(
    config_path: Option<&PathBuf>,
    identity_path: Option<&PathBuf>,
) -> Result<HostPublishContext, Box<dyn std::error::Error>> {
    let config = if let Some(path) = config_path {
        HostConfig::load(path)?
    } else if let Ok(path) = std::env::var("NC_HOST_CONFIG") {
        HostConfig::load(&PathBuf::from(path))?
    } else {
        HostConfig::load(&HostConfig::default_config_path())?
    };
    let identity_path = identity_path
        .cloned()
        .unwrap_or_else(|| config.identity_path.clone());
    let identity = HostIdentity::load(&identity_path)?;
    let relay_config = RelayConfig::validate(&config.relay_url)?;
    let keys = Keys::parse(&identity.nsec)?;
    let runtime = tokio::runtime::Runtime::new()?;
    let publisher = runtime.block_on(async {
        let client = nostr_sdk::Client::new(keys.clone());
        client
            .add_relay(&relay_config.url)
            .await
            .map_err(|err| err.to_string())?;
        client.connect().await;
        Ok::<_, String>(EventPublisher::new(client, keys))
    })?;

    Ok(HostPublishContext {
        config,
        publisher,
        runtime,
    })
}

fn print_usage() {
    println!("Usage:");
    println!("  nc-host games list --root <path>");
    println!("  nc-host games status --dir <path>");
    println!("  nc-host games retire --dir <path> [--config <path>] [--identity <path>]");
    println!("  nc-host games relist --dir <path> [--config <path>] [--identity <path>]");
    println!("  nc-host games delete --dir <path> --yes [--config <path>] [--identity <path>]");
}
