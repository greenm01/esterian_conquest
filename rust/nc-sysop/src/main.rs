mod nostr;
mod nostr_relay;
mod usage;

use std::collections::BTreeSet;
use std::env;
#[cfg(unix)]
use std::fs;
use std::path::{Path, PathBuf};

use nc_data::{
    BbsGameConfig, CampaignSettings, CampaignStore, SeatReservation, generate_campaign_seed,
};
use nc_engine::{build_seeded_new_game, run_maintenance_turn_with_seed};

#[derive(Clone)]
struct ParsedArgs {
    log_file: Option<PathBuf>,
    log_level: nc_log::LogLevel,
    args: Vec<String>,
}

fn main() {
    if let Err(err) = run() {
        tracing::error!(error = %err, "nc-sysop command failed");
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let parsed = parse_args(env::args().skip(1))?;
    let mut args = parsed.args.clone().into_iter();
    let Some(cmd) = args.next() else {
        usage::print_usage();
        return Ok(());
    };
    let rest = args.collect::<Vec<_>>();

    match cmd.as_str() {
        "--help" | "-h" | "help" => {
            usage::print_usage();
            Ok(())
        }
        "new-game" => {
            init_logging(&parsed, false)?;
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_new_game_usage();
                return Ok(());
            }
            tracing::info!("running nc-sysop new-game");
            run_new_game(&rest)
        }
        "maint" => {
            init_logging(&parsed, false)?;
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_maint_usage();
                return Ok(());
            }
            tracing::info!("running nc-sysop maint");
            run_maint(&rest)
        }
        "maint-all" => {
            init_logging(&parsed, false)?;
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_maint_all_usage();
                return Ok(());
            }
            tracing::info!("running nc-sysop maint-all");
            run_maint_all(&rest)
        }
        "settings" => {
            init_logging(&parsed, false)?;
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_settings_usage();
                return Ok(());
            }
            tracing::info!("running nc-sysop settings");
            run_settings(&rest)
        }
        "host" => {
            init_logging(&parsed, false)?;
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_host_usage();
                return Ok(());
            }
            tracing::info!("running nc-sysop host");
            run_host(&rest)
        }
        "nostr" => run_nostr(&parsed, rest),
        _ => {
            usage::print_usage();
            Err(format!("unknown subcommand: {cmd}").into())
        }
    }
}

fn run_new_game(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut bbs_mode = false;
    let mut target_dir = None;
    let mut player_count = 4u8;
    let mut seed = None;
    let mut game_name = None;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--bbs" => {
                bbs_mode = true;
                idx += 1;
            }
            "--players" | "-p" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --players".into());
                };
                player_count = parse_player_count(value)?;
                idx += 2;
            }
            "--seed" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --seed".into());
                };
                seed = Some(value.parse::<u64>()?);
                idx += 2;
            }
            "--name" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --name".into());
                };
                game_name = Some(value.to_string());
                idx += 2;
            }
            value if !value.starts_with('-') => {
                if target_dir.is_some() {
                    return Err(format!("unexpected argument: {value}").into());
                }
                target_dir = Some(resolve_repo_path(value));
                idx += 1;
            }
            other => {
                return Err(format!("unexpected argument: {other}").into());
            }
        }
    }
    let Some(target_dir) = target_dir else {
        usage::print_new_game_usage();
        return Err("missing target_dir for new-game".into());
    };

    let bbs_config_path = target_dir.join("config.kdl");
    if bbs_mode {
        if game_name.is_some() || args.iter().any(|arg| arg == "--players" || arg == "-p") {
            return Err(
                "new-game --bbs reads players from config.kdl; do not pass --name or --players"
                    .into(),
            );
        }
        if !bbs_config_path.exists() {
            return Err(format!(
                "{} requires an existing config.kdl in the target directory",
                target_dir.display()
            )
            .into());
        }
    } else {
        std::fs::create_dir_all(&target_dir)?;
        if bbs_config_path.exists() {
            return Err(format!(
                "{} contains config.kdl; use 'nc-sysop new-game --bbs {}' for BBS campaigns",
                target_dir.display(),
                target_dir.display()
            )
            .into());
        }
    }

    let slug = slug_from_dir(&target_dir)?;
    let game_name = if bbs_mode {
        humanize_slug(&slug)
    } else {
        game_name.unwrap_or_else(|| humanize_slug(&slug))
    };
    let existing_entries = std::fs::read_dir(&target_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    if bbs_mode {
        if existing_entries.iter().any(|path| path != &bbs_config_path) {
            return Err(format!(
                "{} must contain only config.kdl before creating a BBS campaign",
                target_dir.display()
            )
            .into());
        }
    } else if existing_entries
        .iter()
        .any(|path| path.file_name().and_then(|name| name.to_str()) != Some("ncgame.db"))
    {
        return Err(format!(
            "{} must be empty before creating a DB-only campaign",
            target_dir.display()
        )
        .into());
    }

    let store = CampaignStore::open_default_in_dir(&target_dir)?;
    if store.has_snapshots()? {
        return Err(format!("{} already contains a campaign", target_dir.display()).into());
    }

    let (player_count, seed) = if bbs_mode {
        let config = BbsGameConfig::load_kdl(&bbs_config_path)?;
        (config.players, seed.unwrap_or_else(generate_campaign_seed))
    } else {
        (player_count, seed.unwrap_or_else(generate_campaign_seed))
    };
    let game_data = build_seeded_new_game(player_count, 3000, seed)?;
    store.save_runtime_state_structured(&game_data, &BTreeSet::new(), &[], &[])?;
    store.save_campaign_settings(&CampaignSettings::new(&slug, &game_name))?;
    if !bbs_mode {
        store
            .initialize_hosted_seats_if_empty(&nostr::build_pending_seats(player_count as usize))?;
    }

    println!(
        "Initialized new game at: {} (name={}, players={}, year=3000, seed={})",
        target_dir.display(),
        game_name,
        player_count,
        seed
    );
    Ok(())
}

fn run_maint(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let Some(dir_arg) = args.first() else {
        usage::print_maint_usage();
        return Err("missing dir for maint".into());
    };
    let dir = resolve_repo_path(dir_arg);
    let turns = match args.get(1) {
        Some(value) => value.parse::<u16>()?,
        None => 1,
    };
    if args.len() > 2 {
        return Err("usage: nc-sysop maint <dir> [turns]".into());
    }
    run_maintenance_for_dir(&dir, turns, false)
}

fn run_maint_all(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = parse_single_path_flag(args, "--config")?;
    let config_path = config_path.unwrap_or_else(nc_gate::config::config_path);
    let config = nc_gate::config::load_config(&config_path)?;
    let now = unix_now();
    let mut ran = 0usize;
    let mut skipped_due_busy = 0usize;
    let mut skipped_not_due = 0usize;

    for dir in &config.games {
        let store = CampaignStore::open_default_in_dir(dir)?;
        let settings = store.load_campaign_settings()?;
        if !settings.maintenance_due_at(now) {
            skipped_not_due += 1;
            continue;
        }
        if store.has_live_session_leases(now)? {
            skipped_due_busy += 1;
            println!("Skipped busy game: {}", dir.display());
            continue;
        }
        run_maintenance_for_dir(dir, 1, true)?;
        ran += 1;
    }

    println!(
        "maint-all complete: ran={}, skipped_busy={}, skipped_not_due={}",
        ran, skipped_due_busy, skipped_not_due
    );
    Ok(())
}

fn run_settings(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let Some(cmd) = args.first().map(String::as_str) else {
        usage::print_settings_usage();
        return Ok(());
    };
    match cmd {
        "show" => {
            let dir = parse_required_dir_flag(&args[1..])?;
            if is_bbs_campaign_dir(&dir) {
                let config = load_bbs_game_config(&dir)?;
                println!("mode=bbs");
                println!("players={}", config.players);
                for reservation in config.reservations {
                    println!(
                        "reservation seat={} alias={}",
                        reservation.player_record_index_1_based, reservation.alias
                    );
                }
                return Ok(());
            }
            let store = CampaignStore::open_default_in_dir(&dir)?;
            let settings = store.load_campaign_settings()?;
            println!("slug={}", settings.slug);
            println!("game_name={}", settings.game_name);
            println!("default_theme_key={}", settings.default_theme_key);
            println!("snoop={}", settings.snoop_enabled);
            println!(
                "session_max_idle_minutes={}",
                settings.session_max_idle_minutes
            );
            println!(
                "session_minimum_time_minutes={}",
                settings.session_minimum_time_minutes
            );
            println!("session_local_timeout={}", settings.session_local_timeout);
            println!("session_remote_timeout={}", settings.session_remote_timeout);
            println!(
                "inactivity_purge_after_turns={}",
                settings.inactivity_purge_after_turns
            );
            println!(
                "inactivity_autopilot_after_turns={}",
                settings.inactivity_autopilot_after_turns
            );
            println!("maintenance_enabled={}", settings.maintenance_enabled);
            println!(
                "maintenance_interval_minutes={}",
                settings.maintenance_interval_minutes
            );
            println!(
                "maintenance_next_due_unix_seconds={}",
                settings
                    .maintenance_next_due_unix_seconds
                    .map(|value| value.to_string())
                    .unwrap_or_default()
            );
            for reservation in settings.reservations {
                println!(
                    "reservation seat={} alias={}",
                    reservation.player_record_index_1_based, reservation.alias
                );
            }
            Ok(())
        }
        "set" => {
            let dir = parse_required_dir_flag(&args[1..])?;
            if is_bbs_campaign_dir(&dir) {
                return Err(
                    "BBS campaigns do not support 'settings set'; edit config.kdl directly and use settings reserve/unreserve for reservations"
                        .into(),
                );
            }
            let store = CampaignStore::open_default_in_dir(&dir)?;
            let mut settings = store.load_campaign_settings()?;
            let mut idx = 1;
            while idx < args.len() {
                match args[idx].as_str() {
                    "--dir" => idx += 2,
                    "--game-name" => {
                        settings.game_name = args_value(args, &mut idx, "--game-name")?;
                    }
                    "--theme-key" => {
                        settings.default_theme_key = args_value(args, &mut idx, "--theme-key")?;
                    }
                    "--snoop" => {
                        settings.snoop_enabled =
                            parse_on_off(&args_value(args, &mut idx, "--snoop")?)?;
                    }
                    "--session-max-idle" => {
                        settings.session_max_idle_minutes =
                            args_value(args, &mut idx, "--session-max-idle")?.parse::<u8>()?;
                    }
                    "--session-minimum-time" => {
                        settings.session_minimum_time_minutes =
                            args_value(args, &mut idx, "--session-minimum-time")?.parse::<u8>()?;
                    }
                    "--session-local-timeout" => {
                        settings.session_local_timeout =
                            parse_on_off(&args_value(args, &mut idx, "--session-local-timeout")?)?;
                    }
                    "--session-remote-timeout" => {
                        settings.session_remote_timeout =
                            parse_on_off(&args_value(args, &mut idx, "--session-remote-timeout")?)?;
                    }
                    "--inactivity-purge-after" => {
                        settings.inactivity_purge_after_turns =
                            args_value(args, &mut idx, "--inactivity-purge-after")?
                                .parse::<u8>()?;
                    }
                    "--inactivity-autopilot-after" => {
                        settings.inactivity_autopilot_after_turns =
                            args_value(args, &mut idx, "--inactivity-autopilot-after")?
                                .parse::<u8>()?;
                    }
                    "--maintenance-enabled" => {
                        settings.maintenance_enabled =
                            parse_on_off(&args_value(args, &mut idx, "--maintenance-enabled")?)?;
                    }
                    "--maintenance-interval-minutes" => {
                        settings.maintenance_interval_minutes =
                            args_value(args, &mut idx, "--maintenance-interval-minutes")?
                                .parse::<u32>()?;
                    }
                    "--maintenance-next-due" => {
                        let value = args_value(args, &mut idx, "--maintenance-next-due")?;
                        settings.maintenance_next_due_unix_seconds = if value.trim().is_empty() {
                            None
                        } else {
                            Some(value.parse::<u64>()?)
                        };
                    }
                    other => return Err(format!("unexpected argument: {other}").into()),
                }
            }
            store.save_campaign_settings(&settings)?;
            println!("Updated settings for {}", dir.display());
            Ok(())
        }
        "reserve" => {
            let dir = parse_required_dir_flag(&args[1..])?;
            let player = parse_required_usize_flag(&args[1..], "--player")?;
            let alias = parse_required_string_flag(&args[1..], "--alias")?;
            if is_bbs_campaign_dir(&dir) {
                let store = CampaignStore::open_default_in_dir(&dir)?;
                let mut config = load_bbs_game_config(&dir)?;
                config
                    .reservations
                    .retain(|reservation| reservation.player_record_index_1_based != player);
                config.reservations.push(SeatReservation {
                    player_record_index_1_based: player,
                    alias,
                });
                validate_bbs_reservations_against_runtime(&store, &config)?;
                save_bbs_game_config(&dir, &config)?;
                println!("Reserved seat {} in {}", player, dir.display());
                return Ok(());
            }
            let store = CampaignStore::open_default_in_dir(&dir)?;
            let mut settings = store.load_campaign_settings()?;
            settings
                .reservations
                .retain(|reservation| reservation.player_record_index_1_based != player);
            settings.reservations.push(SeatReservation {
                player_record_index_1_based: player,
                alias,
            });
            validate_reservations_against_runtime(&store, &settings)?;
            store.save_campaign_settings(&settings)?;
            println!("Reserved seat {} in {}", player, dir.display());
            Ok(())
        }
        "unreserve" => {
            let dir = parse_required_dir_flag(&args[1..])?;
            let player = parse_required_usize_flag(&args[1..], "--player")?;
            if is_bbs_campaign_dir(&dir) {
                let mut config = load_bbs_game_config(&dir)?;
                config
                    .reservations
                    .retain(|reservation| reservation.player_record_index_1_based != player);
                save_bbs_game_config(&dir, &config)?;
                println!(
                    "Removed reservation for seat {} in {}",
                    player,
                    dir.display()
                );
                return Ok(());
            }
            let store = CampaignStore::open_default_in_dir(&dir)?;
            let mut settings = store.load_campaign_settings()?;
            settings
                .reservations
                .retain(|reservation| reservation.player_record_index_1_based != player);
            store.save_campaign_settings(&settings)?;
            println!(
                "Removed reservation for seat {} in {}",
                player,
                dir.display()
            );
            Ok(())
        }
        other => Err(format!("unknown settings subcommand: {other}").into()),
    }
}

fn run_host(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let Some(cmd) = args.first().map(String::as_str) else {
        usage::print_host_usage();
        return Ok(());
    };
    match cmd {
        "games" => run_host_games(&args[1..]),
        "status" => run_host_status(&args[1..]),
        other => Err(format!("unknown host subcommand: {other}").into()),
    }
}

fn run_host_games(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let Some(cmd) = args.first().map(String::as_str) else {
        usage::print_host_usage();
        return Ok(());
    };
    match cmd {
        "list" => {
            let config_path = parse_single_path_flag(&args[1..], "--config")?
                .unwrap_or_else(nc_gate::config::config_path);
            let config = nc_gate::config::load_config(&config_path)?;
            println!("config={}", config_path.display());
            if config.games.is_empty() {
                println!("games=0");
                return Ok(());
            }
            for dir in &config.games {
                println!("{}", dir.display());
            }
            Ok(())
        }
        "add" => {
            let flags = parse_path_flags(&args[1..], &["--config", "--dir"])?;
            let config_path = flags
                .get("--config")
                .cloned()
                .flatten()
                .unwrap_or_else(nc_gate::config::config_path);
            let dir = flags
                .get("--dir")
                .cloned()
                .flatten()
                .ok_or_else(|| "missing value for --dir".to_string())?;
            let mut config = nc_gate::config::load_config(&config_path)?;
            validate_hosted_game_service_access(&dir, &config.ssh_user)?;
            if config.games.iter().any(|game| game == &dir) {
                println!("Game already registered: {}", dir.display());
                return Ok(());
            }
            config.games.push(dir.clone());
            nc_gate::config::save_config(&config_path, &config)?;
            println!(
                "Registered game {} in {}",
                dir.display(),
                config_path.display()
            );
            Ok(())
        }
        "remove" => {
            let flags = parse_path_flags(&args[1..], &["--config", "--dir"])?;
            let config_path = flags
                .get("--config")
                .cloned()
                .flatten()
                .unwrap_or_else(nc_gate::config::config_path);
            let dir = flags
                .get("--dir")
                .cloned()
                .flatten()
                .ok_or_else(|| "missing value for --dir".to_string())?;
            let mut config = nc_gate::config::load_config(&config_path)?;
            let before = config.games.len();
            config.games.retain(|game| game != &dir);
            if config.games.len() == before {
                println!("Game was not registered: {}", dir.display());
                return Ok(());
            }
            nc_gate::config::save_config(&config_path, &config)?;
            println!(
                "Removed game {} from {}",
                dir.display(),
                config_path.display()
            );
            Ok(())
        }
        other => Err(format!("unknown host games subcommand: {other}").into()),
    }
}

fn run_host_status(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let config_path =
        parse_single_path_flag(args, "--config")?.unwrap_or_else(nc_gate::config::config_path);
    let config = nc_gate::config::load_config(&config_path)?;
    let now = unix_now();

    println!("config={}", config_path.display());
    println!("relay={}", config.relay);
    println!("ssh_host={}", config.ssh_host);
    println!("ssh_user={}", config.ssh_user);
    println!("games={}", config.games.len());

    for dir in &config.games {
        let store = match CampaignStore::open_default_in_dir(dir) {
            Ok(store) => store,
            Err(err) => {
                println!("game={} error={err}", dir.display());
                continue;
            }
        };
        let settings = match store.load_campaign_settings() {
            Ok(settings) => settings,
            Err(err) => {
                println!("game={} error={err}", dir.display());
                continue;
            }
        };
        let seats = match store.hosted_seats() {
            Ok(seats) => seats,
            Err(err) => {
                println!("game={} error={err}", dir.display());
                continue;
            }
        };
        let claimed = seats
            .iter()
            .filter(|seat| seat.player_npub.as_deref().is_some())
            .count();
        let busy = store.has_live_session_leases(now)?;
        let due = settings.maintenance_due_at(now);
        let service_access_issue = hosted_game_service_access_issue(dir, &config.ssh_user)?;
        println!(
            "game={} slug={} name={} seats={} claimed={} busy={} maintenance_due={} service_writable={}",
            dir.display(),
            settings.slug,
            settings.game_name,
            seats.len(),
            claimed,
            busy,
            due,
            service_access_issue.is_none()
        );
        if let Some(issue) = service_access_issue {
            println!("warning={issue}");
        }
    }

    Ok(())
}

fn run_maintenance_for_dir(
    dir: &Path,
    turns: u16,
    update_schedule: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "Running Rust maintenance on: {} ({} turn{})",
        dir.display(),
        turns,
        if turns == 1 { "" } else { "s" }
    );
    let store = CampaignStore::open_default_in_dir(dir)?;
    let runtime_state = store
        .load_latest_runtime_state()?
        .ok_or("campaign store has no snapshots; initialize the campaign with nc-sysop first")?;
    let mut game_data = runtime_state.game_data;
    let campaign_seed = runtime_state.campaign_seed;
    let start_year = game_data.conquest.game_year();

    for turn in 1..=turns {
        run_maintenance_turn_with_seed(&mut game_data, campaign_seed)?;
        println!("  Turn {}: year {}", turn, game_data.conquest.game_year());
    }

    store.save_runtime_state_structured(
        &game_data,
        &runtime_state.planet_scorch_orders,
        &[],
        &runtime_state.queued_mail,
    )?;

    if update_schedule {
        let mut settings = store.load_campaign_settings()?;
        if settings.maintenance_enabled {
            settings.maintenance_next_due_unix_seconds = Some(unix_now().saturating_add(
                u64::from(settings.maintenance_interval_minutes).saturating_mul(60),
            ));
            store.save_campaign_settings(&settings)?;
        }
    }

    println!(
        "  Year advanced: {} -> {}",
        start_year,
        game_data.conquest.game_year()
    );
    println!("Rust maintenance complete.");
    Ok(())
}

fn validate_reservations_against_runtime(
    store: &CampaignStore,
    settings: &CampaignSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    let runtime_state = store
        .load_latest_runtime_state()?
        .ok_or("campaign store has no snapshots; initialize the campaign with nc-sysop first")?;
    settings
        .validate_reservations_for_player_count(runtime_state.game_data.player.records.len())?;
    Ok(())
}

fn validate_bbs_reservations_against_runtime(
    store: &CampaignStore,
    config: &BbsGameConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let runtime_state = store
        .load_latest_runtime_state()?
        .ok_or("campaign store has no snapshots; initialize the campaign with nc-sysop first")?;
    config.validate_reservations_for_player_count(runtime_state.game_data.player.records.len())?;
    Ok(())
}

fn is_bbs_campaign_dir(dir: &Path) -> bool {
    dir.join("config.kdl").exists()
}

fn load_bbs_game_config(dir: &Path) -> Result<BbsGameConfig, Box<dyn std::error::Error>> {
    Ok(BbsGameConfig::load_kdl(&dir.join("config.kdl"))?)
}

fn save_bbs_game_config(
    dir: &Path,
    config: &BbsGameConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    config.save_kdl(&dir.join("config.kdl"))?;
    Ok(())
}

fn validate_hosted_game_service_access(
    dir: &Path,
    ssh_user: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(issue) = hosted_game_service_access_issue(dir, ssh_user)? {
        return Err(issue.into());
    }
    Ok(())
}

fn hosted_game_service_access_issue(
    dir: &Path,
    ssh_user: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    #[cfg(unix)]
    {
        let account = unix_account_for_user(ssh_user)?;
        let db_path = dir.join("ncgame.db");
        let dir_meta =
            fs::metadata(dir).map_err(|err| format!("cannot stat {}: {err}", dir.display()))?;
        if !unix_account_can_read_write_execute(&dir_meta, &account) {
            return Ok(Some(format!(
                "{} is not writable by service user '{}'; create hosted games with 'sudo -u {} nc-sysop new-game {} --name \"<Game Name>\" --players 4' or repair ownership with 'sudo chown -R {}:{} {}'",
                dir.display(),
                ssh_user,
                ssh_user,
                dir.display(),
                ssh_user,
                ssh_user,
                dir.display()
            )));
        }
        let db_meta = fs::metadata(&db_path)
            .map_err(|err| format!("cannot stat {}: {err}", db_path.display()))?;
        if !unix_account_can_read_write(&db_meta, &account) {
            return Ok(Some(format!(
                "{} is not writable by service user '{}'; create hosted games with 'sudo -u {} nc-sysop new-game {} --name \"<Game Name>\" --players 4' or repair ownership with 'sudo chown -R {}:{} {}'",
                db_path.display(),
                ssh_user,
                ssh_user,
                dir.display(),
                ssh_user,
                ssh_user,
                dir.display()
            )));
        }
        Ok(None)
    }
    #[cfg(not(unix))]
    {
        let _ = (dir, ssh_user);
        Ok(None)
    }
}

#[cfg(unix)]
#[derive(Debug)]
struct UnixAccount {
    uid: u32,
    gids: BTreeSet<u32>,
}

#[cfg(unix)]
fn unix_account_for_user(username: &str) -> Result<UnixAccount, Box<dyn std::error::Error>> {
    let passwd = fs::read_to_string("/etc/passwd")
        .map_err(|err| format!("cannot read /etc/passwd: {err}"))?;
    let mut uid = None;
    let mut primary_gid = None;
    for line in passwd.lines() {
        let mut fields = line.split(':');
        let Some(name) = fields.next() else {
            continue;
        };
        if name != username {
            continue;
        }
        let _password = fields.next();
        uid = Some(
            fields
                .next()
                .ok_or_else(|| format!("invalid /etc/passwd entry for {username}"))?
                .parse::<u32>()?,
        );
        primary_gid = Some(
            fields
                .next()
                .ok_or_else(|| format!("invalid /etc/passwd entry for {username}"))?
                .parse::<u32>()?,
        );
        break;
    }
    let uid = uid.ok_or_else(|| format!("service user '{}' not found in /etc/passwd", username))?;
    let primary_gid =
        primary_gid.ok_or_else(|| format!("service user '{}' has no primary group", username))?;
    let mut gids = BTreeSet::from([primary_gid]);
    if let Ok(group_text) = fs::read_to_string("/etc/group") {
        for line in group_text.lines() {
            let mut fields = line.split(':');
            let _group_name = fields.next();
            let _password = fields.next();
            let Some(gid_raw) = fields.next() else {
                continue;
            };
            let Some(members_raw) = fields.next() else {
                continue;
            };
            if !members_raw
                .split(',')
                .filter(|member| !member.is_empty())
                .any(|member| member == username)
            {
                continue;
            }
            gids.insert(gid_raw.parse::<u32>()?);
        }
    }
    Ok(UnixAccount { uid, gids })
}

#[cfg(unix)]
fn unix_account_can_read_write(meta: &fs::Metadata, account: &UnixAccount) -> bool {
    use std::os::unix::fs::MetadataExt;

    let mode = meta.mode();
    if meta.uid() == account.uid {
        return (mode & 0o600) == 0o600;
    }
    if account.gids.contains(&meta.gid()) {
        return (mode & 0o060) == 0o060;
    }
    (mode & 0o006) == 0o006
}

#[cfg(unix)]
fn unix_account_can_read_write_execute(meta: &fs::Metadata, account: &UnixAccount) -> bool {
    use std::os::unix::fs::MetadataExt;

    let mode = meta.mode();
    if meta.uid() == account.uid {
        return (mode & 0o700) == 0o700;
    }
    if account.gids.contains(&meta.gid()) {
        return (mode & 0o070) == 0o070;
    }
    (mode & 0o007) == 0o007
}

fn slug_from_dir(dir: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let slug = dir
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| format!("cannot derive slug from {}", dir.display()))?
        .to_string();
    if !slug
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(format!(
            "game slug '{}' must use only lowercase ascii letters, digits, and dashes",
            slug
        )
        .into());
    }
    Ok(slug)
}

fn humanize_slug(slug: &str) -> String {
    slug.split('-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut word = String::new();
                    word.extend(first.to_uppercase());
                    word.push_str(chars.as_str());
                    word
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_player_count(value: &str) -> Result<u8, Box<dyn std::error::Error>> {
    let player_count = value.parse::<u8>()?;
    if !(1..=25).contains(&player_count) {
        return Err(format!("player_count must be 1-25, got {player_count}").into());
    }
    Ok(player_count)
}

fn parse_on_off(value: &str) -> Result<bool, Box<dyn std::error::Error>> {
    match value.trim().to_ascii_lowercase().as_str() {
        "on" | "true" | "1" => Ok(true),
        "off" | "false" | "0" => Ok(false),
        other => Err(format!("expected on/off, got {other}").into()),
    }
}

fn parse_required_dir_flag(args: &[String]) -> Result<PathBuf, Box<dyn std::error::Error>> {
    parse_optional_path_flag_anywhere(args, "--dir")?
        .ok_or_else(|| "missing value for --dir".into())
}

fn parse_required_string_flag(
    args: &[String],
    flag: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == flag {
            let Some(next) = args.get(i + 1) else {
                return Err(format!("missing value for {flag}").into());
            };
            return Ok(next.to_string());
        }
        if let Some(value) = arg.strip_prefix(&format!("{flag}=")) {
            return Ok(value.to_string());
        }
        i += 1;
    }
    Err(format!("missing value for {flag}").into())
}

fn parse_required_usize_flag(
    args: &[String],
    flag: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    Ok(parse_required_string_flag(args, flag)?.parse::<usize>()?)
}

fn args_value(
    args: &[String],
    idx: &mut usize,
    flag: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let Some(value) = args.get(*idx + 1) else {
        return Err(format!("missing value for {flag}").into());
    };
    *idx += 2;
    Ok(value.to_string())
}

fn parse_optional_path_flag_anywhere(
    args: &[String],
    flag: &str,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == flag {
            let Some(next) = args.get(i + 1) else {
                return Err(format!("missing value for {flag}").into());
            };
            return Ok(Some(PathBuf::from(next)));
        }
        if let Some(value) = arg.strip_prefix(&format!("{flag}=")) {
            return Ok(Some(PathBuf::from(value)));
        }
        i += 1;
    }
    Ok(None)
}

fn resolve_repo_path(arg: &str) -> PathBuf {
    let path = PathBuf::from(arg);
    if path.is_absolute() {
        path
    } else if path.exists() {
        path
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
    }
}

fn init_logging(
    parsed: &ParsedArgs,
    default_to_stderr: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(log_file) = &parsed.log_file {
        nc_log::init_file_logging(log_file, parsed.log_level)?;
        tracing::info!(
            log_file = %log_file.display(),
            level = ?parsed.log_level,
            "nc-sysop logging initialized"
        );
    } else if default_to_stderr {
        nc_log::init_stderr_logging(parsed.log_level)?;
        tracing::info!(level = ?parsed.log_level, "nc-sysop stderr logging initialized");
    }
    Ok(())
}

fn run_nostr(parsed: &ParsedArgs, rest: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = rest.into_iter();
    let Some(cmd) = args.next() else {
        usage::print_nostr_usage();
        return Ok(());
    };
    let rest = args.collect::<Vec<_>>();

    match cmd.as_str() {
        "--help" | "-h" | "help" => {
            usage::print_nostr_usage();
            Ok(())
        }
        "init" => {
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_nostr_init_usage();
                return Ok(());
            }
            init_logging(parsed, false)?;
            let identity_path = parse_single_path_flag(&rest, "--identity")?;
            tracing::info!(
                identity_path = ?identity_path.as_ref().map(|path| path.display().to_string()),
                "running nc-sysop nostr init"
            );
            let initialized = nc_gate::init_identity_at(identity_path)?;
            if initialized.already_exists {
                println!(
                    "Daemon identity already exists at: {}",
                    initialized.path.display()
                );
                println!("Public key (npub): {}", initialized.npub);
                println!("Created: {}", initialized.created);
            } else {
                println!("Daemon identity created at: {}", initialized.path.display());
                println!("Public key (npub): {}", initialized.npub);
            }
            Ok(())
        }
        "serve" => {
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_nostr_serve_usage();
                return Ok(());
            }
            init_logging(parsed, true)?;
            let parsed_flags = parse_path_flags(&rest, &["--config", "--identity"])?;
            let config_path = parsed_flags.get("--config").cloned().flatten();
            let identity_path = parsed_flags.get("--identity").cloned().flatten();
            tracing::info!(
                config_path = ?config_path.as_ref().map(|path| path.display().to_string()),
                identity_path = ?identity_path.as_ref().map(|path| path.display().to_string()),
                "running nc-sysop nostr serve"
            );
            nc_gate::serve_from_paths(config_path, identity_path)
        }
        "migrate-roster" => {
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_nostr_migrate_roster_usage();
                return Ok(());
            }
            init_logging(parsed, false)?;
            let dir = nostr::parse_required_dir_flag(&rest)?;
            tracing::info!(dir = %dir.display(), "running nc-sysop nostr migrate-roster");
            println!("{}", nostr::migrate_roster(&dir)?);
            Ok(())
        }
        "seats" => {
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_nostr_seats_usage();
                return Ok(());
            }
            init_logging(parsed, false)?;
            let dir = nostr::parse_required_dir_flag(&rest)?;
            tracing::info!(dir = %dir.display(), "running nc-sysop nostr seats");
            print!("{}", nostr::render_hosted_seats(&dir)?);
            Ok(())
        }
        "reissue" => {
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_nostr_reissue_usage();
                return Ok(());
            }
            init_logging(parsed, false)?;
            let flags = parse_path_flags(&rest, &["--dir", "--player", "--config", "--identity"])?;
            let dir = flags
                .get("--dir")
                .cloned()
                .flatten()
                .ok_or_else(|| "missing value for --dir".to_string())?;
            let player = flags
                .get("--player")
                .and_then(|value| value.as_ref())
                .ok_or_else(|| "missing value for --player".to_string())?
                .to_string_lossy()
                .parse::<usize>()?;
            let config_path = flags.get("--config").cloned().flatten();
            let identity_path = flags.get("--identity").cloned().flatten();
            tracing::info!(dir = %dir.display(), player, "running nc-sysop nostr reissue");
            println!(
                "{}",
                nostr_relay::reissue_hosted_seat_with_publish(
                    &dir,
                    player,
                    config_path,
                    identity_path,
                )?
            );
            Ok(())
        }
        "claim" => {
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_nostr_claim_usage();
                return Ok(());
            }
            init_logging(parsed, false)?;
            let mut dir = None;
            let mut player = None;
            let mut npub = None;
            let mut config_path = None;
            let mut identity_path = None;
            let mut i = 0;
            while i < rest.len() {
                match rest[i].as_str() {
                    "--dir" => {
                        i += 1;
                        let Some(next) = rest.get(i) else {
                            return Err("missing value for --dir".into());
                        };
                        dir = Some(PathBuf::from(next));
                    }
                    arg if arg.starts_with("--dir=") => {
                        dir = Some(PathBuf::from(arg.trim_start_matches("--dir=")));
                    }
                    "--player" => {
                        i += 1;
                        let Some(next) = rest.get(i) else {
                            return Err("missing value for --player".into());
                        };
                        player = Some(next.parse::<usize>()?);
                    }
                    arg if arg.starts_with("--player=") => {
                        player = Some(arg.trim_start_matches("--player=").parse::<usize>()?);
                    }
                    "--npub" => {
                        i += 1;
                        let Some(next) = rest.get(i) else {
                            return Err("missing value for --npub".into());
                        };
                        npub = Some(next.to_string());
                    }
                    arg if arg.starts_with("--npub=") => {
                        npub = Some(arg.trim_start_matches("--npub=").to_string());
                    }
                    "--config" => {
                        i += 1;
                        let Some(next) = rest.get(i) else {
                            return Err("missing value for --config".into());
                        };
                        config_path = Some(PathBuf::from(next));
                    }
                    arg if arg.starts_with("--config=") => {
                        config_path = Some(PathBuf::from(arg.trim_start_matches("--config=")));
                    }
                    "--identity" => {
                        i += 1;
                        let Some(next) = rest.get(i) else {
                            return Err("missing value for --identity".into());
                        };
                        identity_path = Some(PathBuf::from(next));
                    }
                    arg if arg.starts_with("--identity=") => {
                        identity_path = Some(PathBuf::from(arg.trim_start_matches("--identity=")));
                    }
                    arg => return Err(format!("unexpected argument: {arg}").into()),
                }
                i += 1;
            }
            let dir = dir.ok_or_else(|| "missing value for --dir".to_string())?;
            let player = player.ok_or_else(|| "missing value for --player".to_string())?;
            let npub = npub.ok_or_else(|| "missing value for --npub".to_string())?;
            tracing::info!(
                dir = %dir.display(),
                player,
                npub = %npub,
                "running nc-sysop nostr claim"
            );
            println!(
                "{}",
                nostr_relay::claim_hosted_seat_with_publish(
                    &dir,
                    player,
                    &npub,
                    config_path,
                    identity_path,
                )?
            );
            Ok(())
        }
        "publish" => {
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_nostr_publish_usage();
                return Ok(());
            }
            init_logging(parsed, false)?;
            let flags = parse_path_flags(&rest, &["--dir", "--config", "--identity"])?;
            let dir = flags
                .get("--dir")
                .cloned()
                .flatten()
                .ok_or_else(|| "missing value for --dir".to_string())?;
            let config_path = flags.get("--config").cloned().flatten();
            let identity_path = flags.get("--identity").cloned().flatten();
            tracing::info!(dir = %dir.display(), "running nc-sysop nostr publish");
            println!(
                "{}",
                nostr_relay::publish_hosted_game(&dir, config_path, identity_path)?
            );
            Ok(())
        }
        "verify" => {
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_nostr_verify_usage();
                return Ok(());
            }
            init_logging(parsed, false)?;
            let flags = parse_path_flags(&rest, &["--dir", "--config", "--identity"])?;
            let dir = flags
                .get("--dir")
                .cloned()
                .flatten()
                .ok_or_else(|| "missing value for --dir".to_string())?;
            let config_path = flags.get("--config").cloned().flatten();
            let identity_path = flags.get("--identity").cloned().flatten();
            tracing::info!(dir = %dir.display(), "running nc-sysop nostr verify");
            println!(
                "{}",
                nostr_relay::verify_hosted_game(&dir, config_path, identity_path)?
            );
            Ok(())
        }
        _ => {
            usage::print_nostr_usage();
            Err(format!("unknown nostr subcommand: {cmd}").into())
        }
    }
}

fn parse_single_path_flag(
    args: &[String],
    flag: &str,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    let mut value = None;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == flag {
            i += 1;
            let Some(next) = args.get(i) else {
                return Err(format!("missing value for {flag}").into());
            };
            value = Some(PathBuf::from(next));
        } else if let Some(next) = arg.strip_prefix(&format!("{flag}=")) {
            value = Some(PathBuf::from(next));
        } else {
            return Err(format!("unexpected argument: {arg}").into());
        }
        i += 1;
    }
    Ok(value)
}

fn parse_path_flags(
    args: &[String],
    allowed_flags: &[&str],
) -> Result<std::collections::BTreeMap<String, Option<PathBuf>>, Box<dyn std::error::Error>> {
    let allowed = allowed_flags
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let mut values = allowed_flags
        .iter()
        .map(|flag| ((*flag).to_string(), None))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if let Some((flag, value)) = allowed.iter().find_map(|flag| {
            arg.strip_prefix(&format!("{flag}="))
                .map(|value| (*flag, value))
        }) {
            values.insert(flag.to_string(), Some(PathBuf::from(value)));
            i += 1;
            continue;
        }
        if allowed.contains(arg.as_str()) {
            i += 1;
            let Some(next) = args.get(i) else {
                return Err(format!("missing value for {arg}").into());
            };
            values.insert(arg.clone(), Some(PathBuf::from(next)));
            i += 1;
            continue;
        }
        return Err(format!("unexpected argument: {arg}").into());
    }
    Ok(values)
}

fn parse_args(
    args: impl IntoIterator<Item = String>,
) -> Result<ParsedArgs, Box<dyn std::error::Error>> {
    let mut log_file = None;
    let mut log_level = nc_log::LogLevel::Info;
    let mut remaining = Vec::new();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--log-file" => {
                let Some(value) = iter.next() else {
                    return Err("missing value for --log-file".into());
                };
                log_file = Some(PathBuf::from(value));
            }
            "--log-level" => {
                let Some(value) = iter.next() else {
                    return Err("missing value for --log-level".into());
                };
                log_level = nc_log::LogLevel::parse(&value)?;
            }
            _ => {
                remaining.push(arg);
                remaining.extend(iter);
                break;
            }
        }
    }
    Ok(ParsedArgs {
        log_file,
        log_level,
        args: remaining,
    })
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
