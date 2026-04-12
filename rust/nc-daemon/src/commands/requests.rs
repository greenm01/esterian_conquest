use crate::invite::generate_invite_code;
use crate::config::daemon_config::DaemonConfig;
use nc_data::hosted::{list_seats, HostedStore};
use std::collections::HashSet;
use std::path::PathBuf;

pub fn run(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || args.iter().any(|arg| matches!(*arg, "--help" | "-h")) {
        print_usage();
        return Ok(());
    }

    let mut game_dir = None;
    let mut subcmd = None;
    let mut request_id = None;
    let mut player = None;
    let mut message: Option<String> = None;
    let mut config_path = None;

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
            "--request" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --request".into());
                }
                request_id = Some(args[i + 1].to_string());
                i += 2;
            }
            "--player" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --player".into());
                }
                player = Some(args[i + 1].parse()?);
                i += 2;
            }
            "--message" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --message".into());
                }
                message = Some(args[i + 1].to_string());
                i += 2;
            }
            "--config" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --config".into());
                }
                config_path = Some(PathBuf::from(args[i + 1]));
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
        .and_then(|n| n.to_str())
        .unwrap_or("game")
        .to_string();

    match subcmd {
        Some("list") => run_list(&store, &game_id).map_err(|e| e.into()),
        Some("show") => run_show(&store, request_id.as_ref()).map_err(|e| e.into()),
        Some("approve") => {
            let config = load_config(config_path.as_ref())?;
            run_approve(
                &store,
                &game_id,
                &config.invite_relay_host,
                request_id.as_ref(),
                player,
                message.as_ref(),
            )
        }
        Some("reject") => run_reject(&store, &game_id, request_id.as_ref(), message.as_ref()),
        Some(cmd) => Err(format!("unknown subcommand: {}", cmd).into()),
        None => Err("missing subcommand".into()),
    }
}

fn run_list(store: &HostedStore, game_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let requests = nc_data::hosted::list_requests(store.connection(), game_id)?;

    println!("Invite requests for game {}:", game_id);
    println!();

    if requests.is_empty() {
        println!("  No pending requests");
    } else {
        for req in requests {
            println!("  ID: {}", req.id);
            println!("  Player: {}", req.player_pubkey);
            println!("  Message: {}", req.message);
            println!("  Status: {:?}", req.status);
            println!("  Created: {}", req.created_at);
            println!();
        }
    }

    Ok(())
}

fn run_show(
    store: &HostedStore,
    request_id: Option<&String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let request_id = request_id.ok_or("missing --request argument")?;

    let req = nc_data::hosted::get_request(store.connection(), request_id)?
        .ok_or_else(|| format!("request {} not found", request_id))?;

    println!("Request ID: {}", req.id);
    println!("Game ID: {}", req.game_id);
    println!("Player: {}", req.player_pubkey);
    println!("Message: {}", req.message);
    println!("Status: {:?}", req.status);
    println!("Created: {}", req.created_at);

    if let Some(processed) = req.processed_at {
        println!("Processed: {}", processed);
    }
    if let Some(msg) = req.decision_message {
        println!("Decision: {}", msg);
    }
    if let Some(code) = req.issued_invite_code {
        println!("Invite code: {}", code);
    }

    Ok(())
}

fn run_approve(
    store: &HostedStore,
    game_id: &str,
    invite_relay_host: &str,
    request_id: Option<&String>,
    player: Option<u32>,
    message: Option<&String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let request_id = request_id.ok_or("missing --request argument")?;
    let player = player.ok_or("missing --player argument")?;
    let message = message.map(|s| s.as_str()).unwrap_or("Approved");

    let existing: HashSet<String> = list_seats(store.connection(), game_id)?
        .iter()
        .map(|s| s.invite_code.clone())
        .collect();
    let invite_token = generate_invite_code(&existing);
    let issued_invite = format!("{}@{}", invite_token, invite_relay_host);

    nc_data::hosted::approve_request_for_seat(
        store.connection(),
        request_id,
        game_id,
        player,
        &invite_token,
        &issued_invite,
        message,
    )?;

    println!("Approved request {} for player seat {}", request_id, player);
    println!("Invite code: {}", issued_invite);

    Ok(())
}

fn run_reject(
    store: &HostedStore,
    _game_id: &str,
    request_id: Option<&String>,
    message: Option<&String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let request_id = request_id.ok_or("missing --request argument")?;
    let message = message.map(|s| s.as_str()).unwrap_or("Rejected");

    nc_data::hosted::reject_request(store.connection(), request_id, message)?;

    println!("Rejected request {}", request_id);

    Ok(())
}

fn print_usage() {
    println!("Usage:");
    println!("  nc-daemon requests list --dir <path> [--config <path>]");
    println!("  nc-daemon requests show --dir <path> --request <id> [--config <path>]");
    println!(
        "  nc-daemon requests approve --dir <path> --request <id> --player N [--message \"...\"] [--config <path>]"
    );
    println!("  nc-daemon requests reject --dir <path> --request <id> [--message \"...\"] [--config <path>]");
}

fn load_config(path: Option<&PathBuf>) -> Result<DaemonConfig, Box<dyn std::error::Error>> {
    if let Some(path) = path {
        return Ok(DaemonConfig::load(path)?);
    }

    if let Ok(path) = std::env::var("NC_DAEMON_CONFIG") {
        return Ok(DaemonConfig::load(&PathBuf::from(path))?);
    }

    Ok(DaemonConfig::load(&DaemonConfig::default_config_path())?)
}
