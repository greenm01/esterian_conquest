use nc_data::hosted::{get_game_metadata, HostedStore};
use std::path::PathBuf;

pub fn run(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || args.iter().any(|arg| matches!(*arg, "--help" | "-h")) {
        print_usage();
        return Ok(());
    }

    let mut games_root = None;
    let mut subcmd = None;
    let mut game_dir = None;

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
                                    println!(
                                        "{}  year {} turn {}  status: {}  players: {}",
                                        game_id,
                                        meta.current_year,
                                        meta.current_turn,
                                        meta.status,
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

fn print_usage() {
    println!("Usage:");
    println!("  nc-host games list --root <path>");
    println!("  nc-host games status --dir <path>");
}
