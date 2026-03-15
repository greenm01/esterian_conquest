use std::path::PathBuf;

use crate::commands::setup::{
    init_canonical_four_player_start, init_new_game, init_new_game_from_config,
    init_new_game_with_seed, print_autopilot_after, print_com_irq, print_flow_control,
    print_local_timeout, print_maintenance_days, print_max_key_gap, print_minimum_time,
    print_port_setup, print_purge_after, print_remote_timeout, print_setup_programs, print_snoop,
    set_autopilot_after, set_com_irq, set_flow_control, set_local_timeout, set_maintenance_days,
    set_max_key_gap, set_minimum_time, set_purge_after, set_remote_timeout, set_snoop,
};
use crate::support::paths::resolve_repo_path;
use crate::usage::print_usage;
use ec_data::{CampaignStore, CoreGameData, GameStateBuilder};

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
        Some(count) if (1..=25).contains(&count) => count,
        _ => {
            eprintln!("Error: player_count must be 1-25");
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
            CampaignStore::open_default_in_dir(&target_dir)?.import_directory_snapshot(&target_dir)?;
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

fn parse_player_count_value(value: &str) -> Result<u8, String> {
    let player_count: u8 = value
        .parse()
        .map_err(|_| format!("invalid player count: {value}"))?;
    if (1..=25).contains(&player_count) {
        Ok(player_count)
    } else {
        Err(format!("player_count must be 1-25, got {player_count}"))
    }
}

fn parse_seed_value(value: &str) -> Result<u64, String> {
    value
        .parse()
        .map_err(|_| format!("seed must be a valid unsigned integer, got {value}"))
}

fn parse_new_game_options(
    args: &[String],
) -> Result<(Option<u8>, Option<PathBuf>, Option<u64>), String> {
    let mut player_count = None;
    let mut config_path = None;
    let mut seed = None;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--players" | "-p" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --players".to_string());
                };
                player_count = Some(parse_player_count_value(value)?);
                idx += 2;
            }
            "--config" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --config".to_string());
                };
                config_path = Some(resolve_repo_path(value));
                idx += 2;
            }
            "--seed" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --seed".to_string());
                };
                seed = Some(parse_seed_value(value)?);
                idx += 2;
            }
            _ => {
                return Err(
                    "usage: sysop new-game <target_dir> [--players N] [--config path] [--seed N]"
                        .to_string(),
                );
            }
        }
    }

    Ok((Some(player_count.unwrap_or(4)), config_path, seed))
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
            let (player_count, config_path, seed) = match parse_new_game_options(&remaining) {
                Ok(options) => options,
                Err(message) => {
                    eprintln!("Error: {message}");
                    print_usage();
                    return Ok(());
                }
            };
            if let Some(config_path) = config_path {
                init_new_game_from_config(&target_dir, &config_path, player_count, seed)?;
                println!(
                    "Initialized new game at: {} (config={}{}{}{}{})",
                    target_dir.display(),
                    config_path.display(),
                    if player_count.is_some() {
                        ", players="
                    } else {
                        ""
                    },
                    player_count
                        .map(|count| count.to_string())
                        .unwrap_or_default(),
                    if seed.is_some() { ", seed=" } else { "" },
                    seed.map(|value| value.to_string()).unwrap_or_default()
                );
            } else if player_count == Some(4) && cmd == "init-canonical-four-player-start" {
                init_canonical_four_player_start(&target_dir)?;
                println!(
                    "Initialized canonical four-player start at: {}",
                    target_dir.display()
                );
            } else {
                let player_count = player_count.expect("player count should be present");
                if let Some(seed) = seed {
                    init_new_game_with_seed(&target_dir, player_count, seed)?;
                } else {
                    init_new_game(&target_dir, player_count)?;
                }
                println!(
                    "Initialized new game at: {} (players={}{}{})",
                    target_dir.display(),
                    player_count,
                    if seed.is_some() { ", seed=" } else { "" },
                    seed.map(|value| value.to_string()).unwrap_or_default()
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
