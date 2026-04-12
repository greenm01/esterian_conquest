use crate::config::{host_config::HostConfig, host_identity::HostIdentity};
use crate::lobby::threads;
use crate::support::ids::new_outbox_id;
use nc_data::hosted::HostedStore;
use std::path::PathBuf;

pub fn run(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || args.iter().any(|arg| matches!(*arg, "--help" | "-h")) {
        print_usage();
        return Ok(());
    }

    let mut game_dir = None;
    let mut subcmd = None;
    let mut player = None;
    let mut message = None;
    let mut handle = Some("nc-host".to_string());
    let mut config_path = None;
    let mut identity_path = None;

    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "--dir" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --dir".into());
                }
                game_dir = Some(PathBuf::from(args[i + 1]));
                i += 2;
            }
            "--player" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --player".into());
                }
                player = Some(args[i + 1].to_string());
                i += 2;
            }
            "--message" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --message".into());
                }
                message = Some(args[i + 1].to_string());
                i += 2;
            }
            "--handle" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --handle".into());
                }
                handle = Some(args[i + 1].to_string());
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

    let game_dir = game_dir.ok_or("missing --dir argument")?;
    let db_path = game_dir.join("hosted.db");
    if !db_path.exists() {
        return Err(format!("game not found at {}", game_dir.display()).into());
    }
    let store = HostedStore::open(&db_path)?;
    let game_id = game_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("game")
        .to_string();

    match subcmd {
        Some("list") => run_list(&store, &game_id),
        Some("show") => run_show(&store, &game_id, player.as_deref()),
        Some("send") => run_send(
            &store,
            &game_id,
            player.as_deref(),
            message.as_deref(),
            handle.as_deref(),
            config_path.as_ref(),
            identity_path.as_ref(),
        ),
        Some(cmd) => Err(format!("unknown subcommand: {}", cmd).into()),
        None => Err("missing subcommand".into()),
    }
}

fn run_list(store: &HostedStore, game_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let players = threads::list_players(store, game_id)?;
    println!("Thread participants for {}:", game_id);
    if players.is_empty() {
        println!("  No private threads yet");
    } else {
        for player in players {
            println!("  {}", player);
        }
    }
    Ok(())
}

fn run_show(
    store: &HostedStore,
    game_id: &str,
    player_pubkey: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let player_pubkey = player_pubkey.ok_or("missing --player argument")?;
    let messages = threads::list_messages(store, game_id, player_pubkey)?;
    println!("Thread for {} / {}:", game_id, player_pubkey);
    if messages.is_empty() {
        println!("  No messages");
    } else {
        for message in messages {
            let sender = message
                .sender_handle
                .as_deref()
                .unwrap_or(message.sender_role.as_str());
            println!("  [{}] {}: {}", message.id, sender, message.body);
        }
    }
    Ok(())
}

fn run_send(
    store: &HostedStore,
    game_id: &str,
    player_pubkey: Option<&str>,
    message: Option<&str>,
    handle: Option<&str>,
    config_path: Option<&PathBuf>,
    identity_path: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let player_pubkey = player_pubkey.ok_or("missing --player argument")?;
    let body = message
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .ok_or("missing --message argument")?;
    let config = load_config(config_path)?;
    let identity_path = identity_path
        .cloned()
        .unwrap_or_else(|| config.identity_path.clone());
    let identity = HostIdentity::load(&identity_path)?;
    let message_id = new_outbox_id("thread", game_id);
    threads::enqueue_sysop_message(
        store,
        game_id,
        player_pubkey,
        &identity.npub,
        handle,
        body,
        &message_id,
    )?;
    println!("Queued sysop thread message {}", message_id);
    Ok(())
}

fn load_config(path: Option<&PathBuf>) -> Result<HostConfig, Box<dyn std::error::Error>> {
    if let Some(path) = path {
        return Ok(HostConfig::load(path)?);
    }
    if let Ok(path) = std::env::var("NC_HOST_CONFIG") {
        return Ok(HostConfig::load(&PathBuf::from(path))?);
    }
    Ok(HostConfig::load(&HostConfig::default_config_path())?)
}

fn print_usage() {
    println!("Usage:");
    println!("  nc-host threads list --dir <path>");
    println!("  nc-host threads show --dir <path> --player <npub>");
    println!("  nc-host threads send --dir <path> --player <npub> --message \"...\" [--handle <name>] [--config <path>] [--identity <path>]");
}
