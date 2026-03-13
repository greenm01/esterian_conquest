use std::path::PathBuf;

use crate::commands::setup::{
    init_canonical_four_player_start, init_new_game, print_autopilot_after, print_com_irq, print_flow_control,
    print_local_timeout, print_maintenance_days, print_max_key_gap, print_minimum_time,
    print_port_setup, print_purge_after, print_remote_timeout, print_setup_programs, print_snoop,
    set_autopilot_after, set_com_irq, set_flow_control, set_local_timeout, set_maintenance_days,
    set_max_key_gap, set_minimum_time, set_purge_after, set_remote_timeout, set_snoop,
};
use crate::support::paths::resolve_repo_path;
use crate::usage::print_usage;
use ec_data::{CoreGameData, GameStateBuilder};

fn next_sysop_dir(args: &mut impl Iterator<Item = String>) -> PathBuf {
    args.next()
        .map(|arg| resolve_repo_path(&arg))
        .unwrap_or_else(|| resolve_repo_path("original/v1.5"))
}

fn generate_gamestate_from_args(
    mut args: impl Iterator<Item = String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(target_dir) = args.next().map(|arg| resolve_repo_path(&arg)) else {
        print_usage();
        return Ok(());
    };
    let player_count: u8 = match args.next().and_then(|s| s.parse().ok()) {
        Some(count) if (1..=4).contains(&count) => count,
        _ => {
            eprintln!("Error: player_count must be 1-4");
            print_usage();
            return Ok(());
        }
    };
    let year: u16 = match args.next().and_then(|s| s.parse().ok()) {
        Some(y) => y,
        None => {
            eprintln!("Error: year must be a valid number");
            print_usage();
            return Ok(());
        }
    };

    let mut homeworld_coords = Vec::new();
    for coord_str in args {
        let parts: Vec<&str> = coord_str.split(':').collect();
        if parts.len() != 2 {
            eprintln!("Error: homeworld coords must be in format x:y");
            print_usage();
            return Ok(());
        }
        let x: u8 = match parts[0].parse() {
            Ok(v) => v,
            Err(_) => {
                eprintln!("Error: invalid x coordinate: {}", parts[0]);
                return Ok(());
            }
        };
        let y: u8 = match parts[1].parse() {
            Ok(v) => v,
            Err(_) => {
                eprintln!("Error: invalid y coordinate: {}", parts[1]);
                return Ok(());
            }
        };
        homeworld_coords.push([x, y]);
    }

    while homeworld_coords.len() < player_count as usize {
        homeworld_coords.push([0, 0]);
    }
    homeworld_coords.truncate(player_count as usize);

    let builder = GameStateBuilder::new()
        .with_player_count(player_count)
        .with_year(year)
        .with_homeworld_coords(homeworld_coords);

    match builder.build_and_save(&target_dir) {
        Ok(()) => {
            println!("Generated gamestate at: {}", target_dir.display());

            let data = CoreGameData::load(&target_dir)?;
            let errors = data.ecmaint_preflight_errors();
            if errors.is_empty() {
                println!("Preflight validation: OK");
            } else {
                println!("Preflight validation errors:");
                for error in errors {
                    println!("  - {}", error);
                }
            }
        }
        Err(e) => eprintln!("Error generating gamestate: {}", e),
    }

    Ok(())
}

fn parse_new_game_player_count(args: &[String]) -> Result<u8, String> {
    if args.is_empty() {
        return Ok(4);
    }

    if args.len() == 2 && (args[0] == "--players" || args[0] == "-p") {
        let player_count: u8 = args[1]
            .parse()
            .map_err(|_| format!("invalid player count: {}", args[1]))?;
        if (1..=4).contains(&player_count) {
            Ok(player_count)
        } else {
            Err(format!("player_count must be 1-4, got {player_count}"))
        }
    } else {
        Err("usage: sysop new-game <target_dir> [--players N]".to_string())
    }
}

pub(crate) fn run_sysop_args(
    mut args: impl Iterator<Item = String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(cmd) = args.next() else {
        print_usage();
        return Ok(());
    };

    match cmd.as_str() {
        "generate-gamestate" => generate_gamestate_from_args(args)?,
        "new-game" | "init-canonical-four-player-start" => {
            let Some(target_dir) = args.next().map(|arg| resolve_repo_path(&arg)) else {
                print_usage();
                return Ok(());
            };
            let remaining = args.collect::<Vec<_>>();
            let player_count = match parse_new_game_player_count(&remaining) {
                Ok(count) => count,
                Err(message) => {
                    eprintln!("Error: {message}");
                    print_usage();
                    return Ok(());
                }
            };
            if player_count == 4 && cmd == "init-canonical-four-player-start" {
                init_canonical_four_player_start(&target_dir)?;
                println!(
                    "Initialized canonical four-player start at: {}",
                    target_dir.display()
                );
            } else {
                init_new_game(&target_dir, player_count)?;
                println!(
                    "Initialized new game at: {} (players={})",
                    target_dir.display(),
                    player_count
                );
            }
        }
        "maintenance-days" => {
            let Some(dir) = args.next().map(|arg| resolve_repo_path(&arg)) else {
                print_usage();
                return Ok(());
            };
            match args.next().as_deref() {
                None => print_maintenance_days(&dir)?,
                Some("set") => {
                    let days = args.collect::<Vec<_>>();
                    set_maintenance_days(&dir, &days)?;
                }
                Some(_) => print_usage(),
            }
        }
        "snoop" => {
            let Some(dir) = args.next().map(|arg| resolve_repo_path(&arg)) else {
                print_usage();
                return Ok(());
            };
            match args.next().as_deref() {
                None => print_snoop(&dir)?,
                Some("on") => set_snoop(&dir, true)?,
                Some("off") => set_snoop(&dir, false)?,
                Some(_) => print_usage(),
            }
        }
        "purge-after" => {
            let Some(dir) = args.next().map(|arg| resolve_repo_path(&arg)) else {
                print_usage();
                return Ok(());
            };
            match args.next() {
                None => print_purge_after(&dir)?,
                Some(value) => set_purge_after(&dir, value.parse()?)?,
            }
        }
        "setup-programs" => {
            let dir = next_sysop_dir(&mut args);
            print_setup_programs(&dir)?;
        }
        "port-setup" => {
            let dir = next_sysop_dir(&mut args);
            print_port_setup(&dir)?;
        }
        "flow-control" => {
            let Some(dir) = args.next().map(|arg| resolve_repo_path(&arg)) else {
                print_usage();
                return Ok(());
            };
            let Some(port_name) = args.next() else {
                print_usage();
                return Ok(());
            };
            match args.next().as_deref() {
                None => print_flow_control(&dir, &port_name)?,
                Some("on") => set_flow_control(&dir, &port_name, true)?,
                Some("off") => set_flow_control(&dir, &port_name, false)?,
                Some(_) => print_usage(),
            }
        }
        "com-irq" => {
            let Some(dir) = args.next().map(|arg| resolve_repo_path(&arg)) else {
                print_usage();
                return Ok(());
            };
            let Some(port_name) = args.next() else {
                print_usage();
                return Ok(());
            };
            match args.next() {
                None => print_com_irq(&dir, &port_name)?,
                Some(value) => set_com_irq(&dir, &port_name, value.parse()?)?,
            }
        }
        "local-timeout" => {
            let Some(dir) = args.next().map(|arg| resolve_repo_path(&arg)) else {
                print_usage();
                return Ok(());
            };
            match args.next().as_deref() {
                None => print_local_timeout(&dir)?,
                Some("on") => set_local_timeout(&dir, true)?,
                Some("off") => set_local_timeout(&dir, false)?,
                Some(_) => print_usage(),
            }
        }
        "remote-timeout" => {
            let Some(dir) = args.next().map(|arg| resolve_repo_path(&arg)) else {
                print_usage();
                return Ok(());
            };
            match args.next().as_deref() {
                None => print_remote_timeout(&dir)?,
                Some("on") => set_remote_timeout(&dir, true)?,
                Some("off") => set_remote_timeout(&dir, false)?,
                Some(_) => print_usage(),
            }
        }
        "max-key-gap" => {
            let Some(dir) = args.next().map(|arg| resolve_repo_path(&arg)) else {
                print_usage();
                return Ok(());
            };
            match args.next() {
                None => print_max_key_gap(&dir)?,
                Some(value) => set_max_key_gap(&dir, value.parse()?)?,
            }
        }
        "minimum-time" => {
            let Some(dir) = args.next().map(|arg| resolve_repo_path(&arg)) else {
                print_usage();
                return Ok(());
            };
            match args.next() {
                None => print_minimum_time(&dir)?,
                Some(value) => set_minimum_time(&dir, value.parse()?)?,
            }
        }
        "autopilot-after" => {
            let Some(dir) = args.next().map(|arg| resolve_repo_path(&arg)) else {
                print_usage();
                return Ok(());
            };
            match args.next() {
                None => print_autopilot_after(&dir)?,
                Some(value) => set_autopilot_after(&dir, value.parse()?)?,
            }
        }
        _ => print_usage(),
    }

    Ok(())
}
