use nc_data::hosted::HostedStore;
use nc_data::CoreGameData;
use std::path::PathBuf;

pub fn run(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || args.iter().any(|arg| matches!(*arg, "--help" | "-h")) {
        print_usage();
        return Ok(());
    }

    let mut game_dir = None;
    let mut turns = 1;

    let mut i = 0;
    while i < args.len() {
        match args[i] {
            _ if args[i].starts_with("--") => {
                return Err(format!("unknown flag: {}", args[i]).into());
            }
            _ => {
                if game_dir.is_none() {
                    game_dir = Some(PathBuf::from(args[i]));
                } else if let Ok(n) = args[i].parse::<u32>() {
                    turns = n;
                } else {
                    return Err(format!("unexpected argument: {}", args[i]).into());
                }
                i += 1;
            }
        }
    }

    let game_dir = game_dir.ok_or("missing game directory argument")?;
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

    run_maintenance(&store, &game_dir, &game_id, turns)
}

fn run_maintenance(
    store: &HostedStore,
    game_dir: &PathBuf,
    game_id: &str,
    turns: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let settings = nc_data::hosted::get_settings(store.connection(), game_id)?;

    println!("Running maintenance for game {} ({} turns)", game_id, turns);
    println!("  Maintenance enabled: {}", settings.maintenance_enabled);
    println!(
        "  Interval: {} minutes",
        settings.maintenance_interval_minutes
    );

    let pending_turns = nc_data::hosted::list_pending_turns(store.connection(), game_id, 0)?;
    println!("  Pending turns: {}", pending_turns.len());

    let mut game_data = CoreGameData::load(game_dir)
        .map_err(|e| format!("Failed to load authoritative game data from {}: {}", game_dir.display(), e))?;

    let current_turn = (game_data.conquest.game_year() - 3000) as u32;

    for turn_num in 0..turns {
        let turn: u32 = current_turn + turn_num + 1;
        println!("  Processing turn {}...", turn);

        let turn_submissions: Vec<_> = pending_turns
            .iter()
            .filter(|t| t.turn == turn as u32)
            .collect();

        if turn_submissions.is_empty() {
            println!("    No orders submitted for turn {}", turn);
        } else {
            println!("    Applying {} turn submissions", turn_submissions.len());
            for submission in &turn_submissions {
                let short_key = if submission.player_pubkey.len() >= 8 {
                    &submission.player_pubkey[..8]
                } else {
                    &submission.player_pubkey
                };
                println!(
                    "      - Player {}: {} commands",
                    short_key,
                    submission.commands.len()
                );
            }
        }

        match nc_engine::run_maintenance_turn(&mut game_data) {
            Ok(_events) => {
                println!("    Turn {} complete", turn);
            }
            Err(e) => {
                println!("    ERROR processing turn {}: {}", turn, e);
            }
        }
    }

    game_data.save(game_dir)?;
    println!("Saved game state to {}", game_dir.display());

    println!("Maintenance complete for {} turns", turns);

    Ok(())
}

fn print_usage() {
    println!("Usage: nc-daemon maint <dir> [turns]");
    println!();
    println!("Arguments:");
    println!("  <dir>     Game directory path");
    println!("  [turns]  Number of turns to process (default: 1)");
}
