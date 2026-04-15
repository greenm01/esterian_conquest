use nc_data::hosted::{RosterStore, get_roster_entry, list_roster, list_roster_events_for_npub};
use std::path::PathBuf;

pub fn run(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || args.iter().any(|arg| matches!(*arg, "--help" | "-h")) {
        print_usage();
        return Ok(());
    }

    let mut games_root = None;
    let mut npub = None;
    let mut subcmd = None;
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
            "--npub" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --npub".into());
                }
                npub = Some(args[i + 1].to_string());
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

    let games_root = games_root.ok_or("missing --root argument")?;
    let roster_db = games_root.join("roster.db");

    if !roster_db.exists() {
        println!("No roster found at {}", roster_db.display());
        println!("(The roster is created automatically when players send invite requests.)");
        return Ok(());
    }

    let store = RosterStore::open(&roster_db)?;

    match subcmd {
        Some("list") => run_list(&store),
        Some("show") => run_show(&store, npub.as_deref()),
        Some(cmd) => Err(format!("unknown subcommand: {}", cmd).into()),
        None => Err("missing subcommand".into()),
    }
}

fn run_list(store: &RosterStore) -> Result<(), Box<dyn std::error::Error>> {
    let entries = list_roster(store.connection())?;

    if entries.is_empty() {
        println!("No players in roster.");
        return Ok(());
    }

    println!(
        "{:<20}  {:>6}  {:>6}  {:>9}  {}",
        "Handle / npub", "Joined", "Abnd'd", "Completed", "Last seen"
    );
    println!("{}", "-".repeat(80));

    for e in &entries {
        let display = if let Some(ref h) = e.handle {
            if h.len() > 18 {
                format!("{}…", &h[..17])
            } else {
                h.clone()
            }
        } else {
            let short = if e.npub.len() >= 12 {
                format!("{}…", &e.npub[..11])
            } else {
                e.npub.clone()
            };
            short
        };

        let last_seen = format_unix_timestamp(e.last_seen_at);
        println!(
            "{:<20}  {:>6}  {:>6}  {:>9}  {}",
            display, e.games_joined, e.games_abandoned, e.games_completed, last_seen
        );
    }

    println!();
    println!("  {} player(s) total", entries.len());

    Ok(())
}

fn run_show(store: &RosterStore, npub: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let npub = npub.ok_or("missing --npub argument")?;

    let entry = get_roster_entry(store.connection(), npub)?
        .ok_or_else(|| format!("no roster entry for {}", npub))?;

    println!("npub:           {}", entry.npub);
    if let Some(ref h) = entry.handle {
        println!("Handle:         {}", h);
    }
    println!(
        "First seen:     {}",
        format_unix_timestamp(entry.first_seen_at)
    );
    println!(
        "Last seen:      {}",
        format_unix_timestamp(entry.last_seen_at)
    );
    println!("Games joined:   {}", entry.games_joined);
    println!("Games completed:{}", entry.games_completed);
    println!("Games abandoned:{}", entry.games_abandoned);

    let events = list_roster_events_for_npub(store.connection(), npub)?;
    if !events.is_empty() {
        println!();
        println!("Event history:");
        for ev in &events {
            let seat_str = ev.seat.map(|s| format!(" seat {}", s)).unwrap_or_default();
            println!(
                "  [{:>12}]  {}  {}{}",
                ev.event_type,
                format_unix_timestamp(ev.created_at),
                ev.game_id,
                seat_str
            );
        }
    }

    Ok(())
}

fn print_usage() {
    println!("Usage:");
    println!("  nc-host roster --root <games-root> list");
    println!("  nc-host roster --root <games-root> show --npub <npub>");
    println!();
    println!("Options:");
    println!("  --root <path>   Games root directory (required)");
    println!("  --npub <npub>   Player public key (required for show)");
}

fn format_unix_timestamp(ts: i64) -> String {
    use chrono::TimeZone;
    chrono::Utc
        .timestamp_opt(ts, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| ts.to_string())
}
