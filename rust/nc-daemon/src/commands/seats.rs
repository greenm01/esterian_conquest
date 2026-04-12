use crate::invite::generate_invite_code;
use nc_data::hosted::{self, HostedStore};
use std::collections::HashSet;
use std::path::PathBuf;

pub fn run(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || args.iter().any(|arg| matches!(*arg, "--help" | "-h")) {
        print_usage();
        return Ok(());
    }

    let mut game_dir = None;
    let mut subcmd = None;
    let mut seat_number: Option<u32> = None;

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
                seat_number = Some(args[i + 1].parse()?);
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
    let store = HostedStore::open(&db_path)?;

    let game_id = game_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("invalid game directory name")?;

    match subcmd {
        Some("list") => run_list(&store, game_id),
        Some("reissue") => run_reissue(&store, game_id, seat_number),
        Some("reset") => run_reset(&store, game_id, seat_number),
        Some("open") => run_open(&store, game_id, seat_number),
        Some("close") => run_close(&store, game_id, seat_number),
        Some(cmd) => Err(format!("unknown seats subcommand: {}", cmd).into()),
        None => {
            print_usage();
            Ok(())
        }
    }
}

fn run_list(store: &HostedStore, game_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let seats = hosted::list_seats(store.connection(), game_id)?;

    println!("Seats for game '{}':", game_id);
    println!();
    for seat in seats {
        let status = match seat.status {
            hosted::SeatStatus::Pending => "pending",
            hosted::SeatStatus::Claimed => "claimed",
        };
        let player = seat.player_pubkey.as_deref().unwrap_or("-");
        println!("  {:2}  {:8}  {}", seat.seat_number, status, player);
    }

    Ok(())
}

fn run_reissue(
    store: &HostedStore,
    game_id: &str,
    seat_number: Option<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    let seat_number = seat_number.ok_or("missing --player argument")?;

    let existing: HashSet<String> = hosted::list_seats(store.connection(), game_id)?
        .iter()
        .map(|s| s.invite_code.clone())
        .collect();
    let new_code = generate_invite_code(&existing);

    hosted::reissue_seat(store.connection(), game_id, seat_number, &new_code)?;
    hosted::mark_catalog_dirty(store.connection(), game_id)?;

    println!(
        "Reissued seat {} with new invite code: {}",
        seat_number, new_code
    );
    Ok(())
}

fn run_reset(
    store: &HostedStore,
    game_id: &str,
    seat_number: Option<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    let seat_number = seat_number.ok_or("missing --player argument")?;

    hosted::reset_seat(store.connection(), game_id, seat_number)?;
    hosted::mark_catalog_dirty(store.connection(), game_id)?;

    println!("Reset seat {}", seat_number);
    Ok(())
}

fn run_open(
    store: &HostedStore,
    game_id: &str,
    seat_number: Option<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    let seat_number = seat_number.ok_or("missing --player argument")?;

    let existing: HashSet<String> = hosted::list_seats(store.connection(), game_id)?
        .iter()
        .map(|s| s.invite_code.clone())
        .collect();
    let new_code = generate_invite_code(&existing);

    hosted::open_seat(store.connection(), game_id, seat_number, &new_code)?;
    hosted::mark_catalog_dirty(store.connection(), game_id)?;

    println!("Opened seat {} with invite code: {}", seat_number, new_code);
    Ok(())
}

fn run_close(
    store: &HostedStore,
    game_id: &str,
    seat_number: Option<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    let seat_number = seat_number.ok_or("missing --player argument")?;

    hosted::close_seat(store.connection(), game_id, seat_number)?;
    hosted::mark_catalog_dirty(store.connection(), game_id)?;

    println!("Closed seat {}", seat_number);
    Ok(())
}

fn print_usage() {
    println!("Usage:");
    println!("  nc-daemon seats list --dir <path>");
    println!("  nc-daemon seats reissue --dir <path> --player N");
    println!("  nc-daemon seats reset --dir <path> --player N");
    println!("  nc-daemon seats open --dir <path> --player N");
    println!("  nc-daemon seats close --dir <path> --player N");
}
