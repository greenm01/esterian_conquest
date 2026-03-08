use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use ec_data::{ConquestDat, FleetDat, PlanetDat, PlayerDat, SetupDat};

const INIT_FILES: &[&str] = &[
    "BASES.DAT",
    "CONQUEST.DAT",
    "DATABASE.DAT",
    "FLEETS.DAT",
    "IPBM.DAT",
    "MESSAGES.DAT",
    "PLANETS.DAT",
    "PLAYER.DAT",
    "RESULTS.DAT",
    "SETUP.DAT",
];

const ORIGINAL_FILES: &[&str] = &[
    "BASES.DAT",
    "CONQUEST.DAT",
    "DATABASE.DAT",
    "FLEETS.DAT",
    "PLANETS.DAT",
    "PLAYER.DAT",
    "SETUP.DAT",
];

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let Some(cmd) = args.next() else {
        print_usage();
        return Ok(());
    };

    match cmd.as_str() {
        "inspect" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            inspect_dir(&dir)?;
        }
        "headers" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            dump_headers(&dir)?;
        }
        "match" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            match_fixture_set(&dir)?;
        }
        "compare" => {
            let Some(left) = args.next().map(|arg| resolve_repo_path(&arg)) else {
                print_usage();
                return Ok(());
            };
            let Some(right) = args.next().map(|arg| resolve_repo_path(&arg)) else {
                print_usage();
                return Ok(());
            };
            compare_dirs(&left, &right)?;
        }
        "init" => {
            let source = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            let Some(target) = args.next().map(PathBuf::from) else {
                print_usage();
                return Ok(());
            };
            initialize_dir(&source, &target)?;
        }
        "maintenance-days" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            match args.next().as_deref() {
                None => print_maintenance_days(&dir)?,
                Some("set") => {
                    let days = args.collect::<Vec<_>>();
                    set_maintenance_days(&dir, &days)?;
                }
                _ => print_usage(),
            }
        }
        "snoop" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            match args.next().as_deref() {
                None => print_snoop(&dir)?,
                Some("on") => set_snoop(&dir, true)?,
                Some("off") => set_snoop(&dir, false)?,
                _ => print_usage(),
            }
        }
        "purge-after" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            match args.next() {
                None => print_purge_after(&dir)?,
                Some(turns) => {
                    let turns = turns.parse::<u8>()?;
                    set_purge_after(&dir, turns)?;
                }
            }
        }
        "setup-programs" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            print_setup_programs(&dir)?;
        }
        _ => print_usage(),
    }

    Ok(())
}

fn default_fixture_dir() -> PathBuf {
    repo_root().join("original/v1.5")
}

fn init_fixture_dir() -> PathBuf {
    repo_root().join("fixtures/ecutil-init/v1.5")
}

fn post_maint_fixture_dir() -> PathBuf {
    repo_root().join("fixtures/ecmaint-post/v1.5")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn resolve_repo_path(arg: &str) -> PathBuf {
    let path = PathBuf::from(arg);
    if path.is_absolute() {
        path
    } else if path.exists() {
        path
    } else {
        repo_root().join(path)
    }
}

fn inspect_dir(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let player = PlayerDat::parse(&fs::read(dir.join("PLAYER.DAT"))?)?;
    let planets = PlanetDat::parse(&fs::read(dir.join("PLANETS.DAT"))?)?;
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    let conquest = ConquestDat::parse(&fs::read(dir.join("CONQUEST.DAT"))?)?;

    println!("Directory: {}", dir.display());
    print_header_summary(&setup, &conquest);
    println!();

    println!("Players:");
    for (idx, record) in player.records.iter().enumerate() {
        println!(
            "  slot {}: occupied={} tax={} handle='{}' empire='{}'",
            idx + 1,
            record.occupied_flag(),
            record.tax_rate(),
            ascii_trim(record.handle_bytes()),
            ascii_trim(record.empire_name_bytes())
        );
    }
    println!();

    println!("Planets:");
    for (idx, record) in planets.records.iter().enumerate().take(5) {
        println!(
            "  planet {:02}: coords={:02x?} hdr={:02x?} len={} text='{}' summary='{}'",
            idx + 1,
            record.coords_raw(),
            record.header_bytes(),
            record.string_len(),
            ascii_trim(record.status_or_name_bytes()),
            record.derived_summary()
        );
    }
    println!("  ... {} total planet records", planets.records.len());

    let homeworld_like = planets
        .records
        .iter()
        .enumerate()
        .filter(|(_, record)| record.is_named_homeworld_seed())
        .collect::<Vec<_>>();
    if !homeworld_like.is_empty() {
        println!();
        println!("Planet Seeds:");
        for (idx, record) in homeworld_like {
            println!(
                "  planet {:02}: summary='{}' header_value_raw={:02x}",
                idx + 1,
                record.derived_summary(),
                record.header_value_raw()
            );
        }
    }

    match fs::read(dir.join("FLEETS.DAT")) {
        Ok(bytes) => match FleetDat::parse(&bytes) {
            Ok(fleets) => {
                println!();
                println!("Fleets:");
                for (idx, record) in fleets.records.iter().enumerate().take(4) {
                    println!(
                        "  fleet {:02}: id={} slot={} prev={} next={} max_spd={} roe={} ships={} home_raw={:02x?} order={}({}) target_raw={:02x?} mission={} summary='{}'",
                        idx + 1,
                        record.fleet_id(),
                        record.local_slot(),
                        record.previous_fleet_id(),
                        record.next_fleet_id(),
                        record.max_speed(),
                        record.rules_of_engagement(),
                        record.ship_composition_summary(),
                        record.home_system_coords_raw(),
                        record.standing_order_kind().as_str(),
                        record.standing_order_code_raw(),
                        record.standing_order_target_coords_raw(),
                        record.mission_code(),
                        record.standing_order_summary()
                    );
                }
                println!("  ... {} total fleet records", fleets.records.len());

                println!();
                println!("Fleet Groups:");
                for (group_idx, group) in fleets.records.chunks_exact(4).enumerate() {
                    let home = group[0].home_system_coords_raw();
                    println!(
                        "  empire block {}: home_raw={:02x?} target_raw={:02x?}",
                        group_idx + 1,
                        home,
                        group[0].standing_order_target_coords_raw()
                    );
                    for record in group {
                        println!(
                            "    id={} slot={} ships={} max_spd={} order={} summary='{}'",
                            record.fleet_id(),
                            record.local_slot(),
                            record.ship_composition_summary(),
                            record.max_speed(),
                            record.standing_order_kind().as_str(),
                            record.standing_order_summary()
                        );
                    }
                }
            }
            Err(err) => {
                println!();
                println!("Fleets:");
                println!("  FLEETS.DAT does not match initialized 16x54 layout: {err}");
            }
        },
        Err(_) => {}
    }

    Ok(())
}

fn dump_headers(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    let conquest = ConquestDat::parse(&fs::read(dir.join("CONQUEST.DAT"))?)?;

    println!("Directory: {}", dir.display());
    println!("SETUP.version={}", String::from_utf8_lossy(setup.version_tag()));
    println!("SETUP.option_prefix={:02x?}", setup.option_prefix());
    println!("SETUP.snoop_enabled={}", setup.snoop_enabled());
    println!(
        "SETUP.local_timeout_enabled={}",
        setup.local_timeout_enabled()
    );
    println!(
        "SETUP.remote_timeout_enabled={}",
        setup.remote_timeout_enabled()
    );
    println!(
        "SETUP.max_time_between_keys_minutes_raw={}",
        setup.max_time_between_keys_minutes_raw()
    );
    println!(
        "SETUP.minimum_time_granted_minutes_raw={}",
        setup.minimum_time_granted_minutes_raw()
    );
    println!("SETUP.purge_after_turns_raw={}", setup.purge_after_turns_raw());
    println!(
        "SETUP.autopilot_inactive_turns_raw={}",
        setup.autopilot_inactive_turns_raw()
    );
    println!("CONQUEST.game_year={}", conquest.game_year());
    println!("CONQUEST.player_count={}", conquest.player_count());
    println!("CONQUEST.player_config_word={:04x}", conquest.player_config_word());
    println!(
        "CONQUEST.maintenance_schedule={:02x?}",
        conquest.maintenance_schedule_bytes()
    );
    println!("CONQUEST.header_len={}", conquest.control_header().len());
    println!("CONQUEST.header_words={:04x?}", conquest.header_words());

    Ok(())
}

fn print_maintenance_days(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let conquest = ConquestDat::parse(&fs::read(dir.join("CONQUEST.DAT"))?)?;
    let enabled = conquest.maintenance_schedule_enabled();
    println!("Directory: {}", dir.display());
    println!(
        "Maintenance days: {}",
        weekday_labels()
            .into_iter()
            .zip(enabled)
            .map(|(label, enabled)| format!("{label}={}", if enabled { "yes" } else { "no" }))
            .collect::<Vec<_>>()
            .join(" ")
    );
    println!(
        "Maintenance raw: {:02x?}",
        conquest.maintenance_schedule_bytes()
    );
    Ok(())
}

fn print_snoop(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    println!("Directory: {}", dir.display());
    println!("Snoop enabled: {}", if setup.snoop_enabled() { "yes" } else { "no" });
    Ok(())
}

fn set_snoop(dir: &Path, enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
    let setup_path = dir.join("SETUP.DAT");
    let mut setup = SetupDat::parse(&fs::read(&setup_path)?)?;
    setup.set_snoop_enabled(enabled);
    fs::write(&setup_path, setup.to_bytes())?;
    print_snoop(dir)?;
    Ok(())
}

fn print_purge_after(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    println!("Directory: {}", dir.display());
    println!("Purge after turns (raw): {}", setup.purge_after_turns_raw());
    Ok(())
}

fn print_setup_programs(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    println!("Directory: {}", dir.display());
    println!("ECUTIL F4 Modify Program Options");
    println!(
        "  A Purge messages & reports after: {} turn(s)",
        setup.purge_after_turns_raw()
    );
    println!(
        "  B Autopilot any empires inactive for: {} turn(s)",
        setup.autopilot_inactive_turns_raw()
    );
    println!(
        "  C Snoop Enabled: {}",
        yes_no(setup.snoop_enabled())
    );
    println!(
        "  D Enable timeout for local users: {}",
        yes_no(setup.local_timeout_enabled())
    );
    println!(
        "  E Enable timeout for remote users: {}",
        yes_no(setup.remote_timeout_enabled())
    );
    println!(
        "  F Maximum time between key strokes: {} minute(s)",
        setup.max_time_between_keys_minutes_raw()
    );
    println!(
        "  G Minimum time granted: {} minute(s)",
        setup.minimum_time_granted_minutes_raw()
    );
    Ok(())
}

fn set_purge_after(dir: &Path, turns: u8) -> Result<(), Box<dyn std::error::Error>> {
    let setup_path = dir.join("SETUP.DAT");
    let mut setup = SetupDat::parse(&fs::read(&setup_path)?)?;
    setup.set_purge_after_turns_raw(turns);
    fs::write(&setup_path, setup.to_bytes())?;
    print_purge_after(dir)?;
    Ok(())
}

fn set_maintenance_days(dir: &Path, day_names: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut enabled = [false; 7];
    for day_name in day_names {
        let idx = weekday_index(day_name)
            .ok_or_else(|| format!("unknown weekday: {day_name}"))?;
        enabled[idx] = true;
    }

    let conquest_path = dir.join("CONQUEST.DAT");
    let mut conquest = ConquestDat::parse(&fs::read(&conquest_path)?)?;
    conquest.set_maintenance_schedule_enabled(enabled);
    fs::write(&conquest_path, conquest.to_bytes())?;

    print_maintenance_days(dir)?;
    Ok(())
}

fn weekday_labels() -> [&'static str; 7] {
    ["sun", "mon", "tue", "wed", "thu", "fri", "sat"]
}

fn yes_no(value: bool) -> &'static str {
    if value { "Yes" } else { "No" }
}

fn weekday_index(day_name: &str) -> Option<usize> {
    match day_name.to_ascii_lowercase().as_str() {
        "sun" | "sunday" => Some(0),
        "mon" | "monday" => Some(1),
        "tue" | "tues" | "tuesday" => Some(2),
        "wed" | "wednesday" => Some(3),
        "thu" | "thur" | "thurs" | "thursday" => Some(4),
        "fri" | "friday" => Some(5),
        "sat" | "saturday" => Some(6),
        _ => None,
    }
}

fn print_header_summary(setup: &SetupDat, conquest: &ConquestDat) {
    println!("SETUP version: {}", String::from_utf8_lossy(setup.version_tag()));
    println!("SETUP option prefix: {:02x?}", setup.option_prefix());
    println!("SETUP snoop enabled: {}", if setup.snoop_enabled() { "yes" } else { "no" });
    println!(
        "SETUP local timeout enabled: {}",
        if setup.local_timeout_enabled() { "yes" } else { "no" }
    );
    println!(
        "SETUP remote timeout enabled: {}",
        if setup.remote_timeout_enabled() { "yes" } else { "no" }
    );
    println!(
        "SETUP max time between keys (raw minutes): {}",
        setup.max_time_between_keys_minutes_raw()
    );
    println!(
        "SETUP minimum time granted (raw minutes): {}",
        setup.minimum_time_granted_minutes_raw()
    );
    println!("SETUP purge after turns (raw): {}", setup.purge_after_turns_raw());
    println!(
        "SETUP autopilot inactive turns (raw): {}",
        setup.autopilot_inactive_turns_raw()
    );
    println!("CONQUEST game year: {}", conquest.game_year());
    println!("CONQUEST player count: {}", conquest.player_count());
    println!(
        "CONQUEST maintenance schedule: {:02x?}",
        conquest.maintenance_schedule_bytes()
    );
    println!("CONQUEST header bytes: {}", conquest.control_header().len());
    println!(
        "CONQUEST first header words: {:04x?}",
        &conquest.header_words()[..8]
    );
}

fn initialize_dir(source: &Path, target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    copy_top_level_files(source, target)?;

    let init_dir = init_fixture_dir();
    for name in INIT_FILES {
        fs::copy(init_dir.join(name), target.join(name))?;
    }

    println!("Initialized game directory: {}", target.display());
    println!("  source snapshot: {}", source.display());
    println!("  init fixture set: {}", init_dir.display());
    println!("  overlaid files:");
    for name in INIT_FILES {
        println!("    {name}");
    }

    Ok(())
}

fn compare_dirs(left: &Path, right: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Left:  {}", left.display());
    println!("Right: {}", right.display());
    println!();

    compare_raw_file(left, right, "SETUP.DAT")?;
    compare_raw_file(left, right, "CONQUEST.DAT")?;
    compare_raw_file(left, right, "DATABASE.DAT")?;
    compare_player(left, right)?;
    compare_planets(left, right)?;
    compare_fleets(left, right)?;

    Ok(())
}

fn match_fixture_set(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let candidates = [
        ("original/v1.5", default_fixture_dir(), ORIGINAL_FILES),
        ("fixtures/ecutil-init/v1.5", init_fixture_dir(), INIT_FILES),
        ("fixtures/ecmaint-post/v1.5", post_maint_fixture_dir(), INIT_FILES),
    ];

    println!("Directory: {}", dir.display());
    let mut matched_any = false;
    for (label, candidate, files) in candidates {
        if dir_matches(dir, &candidate, files)? {
            println!("MATCH {label}");
            matched_any = true;
        }
    }
    if !matched_any {
        println!("MATCH none");
    }

    Ok(())
}

fn dir_matches(
    dir: &Path,
    candidate: &Path,
    files: &[&str],
) -> Result<bool, Box<dyn std::error::Error>> {
    for name in files {
        if fs::read(dir.join(name))? != fs::read(candidate.join(name))? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn compare_raw_file(
    left_dir: &Path,
    right_dir: &Path,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let left = fs::read(left_dir.join(name))?;
    let right = fs::read(right_dir.join(name))?;
    println!(
        "{name}: size {} vs {}, differing bytes {}",
        left.len(),
        right.len(),
        diff_count(&left, &right)
    );
    Ok(())
}

fn compare_player(left_dir: &Path, right_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let left = PlayerDat::parse(&fs::read(left_dir.join("PLAYER.DAT"))?)?;
    let right = PlayerDat::parse(&fs::read(right_dir.join("PLAYER.DAT"))?)?;
    println!("PLAYER.DAT:");
    for (idx, (a, b)) in left.records.iter().zip(right.records.iter()).enumerate() {
        let count = diff_count(&a.raw, &b.raw);
        if count == 0 {
            continue;
        }
        println!(
            "  record {}: {} differing bytes, tax {} -> {}",
            idx + 1,
            count,
            a.tax_rate(),
            b.tax_rate()
        );
    }
    Ok(())
}

fn compare_planets(left_dir: &Path, right_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let left = PlanetDat::parse(&fs::read(left_dir.join("PLANETS.DAT"))?)?;
    let right = PlanetDat::parse(&fs::read(right_dir.join("PLANETS.DAT"))?)?;
    println!("PLANETS.DAT:");
    for (idx, (a, b)) in left.records.iter().zip(right.records.iter()).enumerate() {
        let count = diff_count(&a.raw, &b.raw);
        if count == 0 {
            continue;
        }
        println!(
            "  record {:02}: {} differing bytes, text '{}' -> '{}'",
            idx + 1,
            count,
            ascii_trim(a.status_or_name_bytes()),
            ascii_trim(b.status_or_name_bytes())
        );
    }
    Ok(())
}

fn compare_fleets(left_dir: &Path, right_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let left_bytes = fs::read(left_dir.join("FLEETS.DAT"))?;
    let right_bytes = fs::read(right_dir.join("FLEETS.DAT"))?;
    println!(
        "FLEETS.DAT: size {} vs {}, differing bytes {}",
        left_bytes.len(),
        right_bytes.len(),
        diff_count(&left_bytes, &right_bytes)
    );

    let left = FleetDat::parse(&left_bytes);
    let right = FleetDat::parse(&right_bytes);
    if let (Ok(left), Ok(right)) = (left, right) {
        for (idx, (a, b)) in left.records.iter().zip(right.records.iter()).enumerate() {
            let count = diff_count(&a.raw, &b.raw);
            if count == 0 {
                continue;
            }
            println!(
                "  record {:02}: {} differing bytes, mission {} -> {}, params {:02x?} -> {:02x?}",
                idx + 1,
                count,
                a.mission_code(),
                b.mission_code(),
                a.mission_param_bytes(),
                b.mission_param_bytes()
            );
        }
    }

    Ok(())
}

fn copy_top_level_files(source: &Path, target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(target)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_file() {
            continue;
        }

        let file_name = entry.file_name();
        fs::copy(&path, target.join(file_name))?;
    }

    Ok(())
}

fn ascii_trim(bytes: &[u8]) -> String {
    let text = bytes
        .iter()
        .map(|b| if (32..127).contains(b) { *b as char } else { ' ' })
        .collect::<String>();
    text.trim().to_string()
}

fn diff_count(left: &[u8], right: &[u8]) -> usize {
    let shared = left.iter().zip(right.iter()).filter(|(a, b)| a != b).count();
    shared + left.len().abs_diff(right.len())
}

fn print_usage() {
    println!("Usage:");
    println!("  ec-cli inspect [dir]");
    println!("  ec-cli headers [dir]");
    println!("  ec-cli maintenance-days [dir]");
    println!("  ec-cli maintenance-days <dir> set <sun|mon|tue|wed|thu|fri|sat>...");
    println!("  ec-cli snoop [dir]");
    println!("  ec-cli snoop <dir> <on|off>");
    println!("  ec-cli purge-after [dir]");
    println!("  ec-cli purge-after <dir> <turns>");
    println!("  ec-cli match [dir]");
    println!("  ec-cli compare <left_dir> <right_dir>");
    println!("  ec-cli init [source_dir] <target_dir>");
}
