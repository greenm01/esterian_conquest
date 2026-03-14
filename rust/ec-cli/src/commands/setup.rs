use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use ec_data::{ConquestDat, DatabaseDat, SetupConfig, SetupDat, build_seeded_new_game};

pub(crate) fn print_maintenance_days(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
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

pub(crate) fn init_canonical_four_player_start(
    target: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    init_new_game(target, 4)
}

pub(crate) fn init_new_game(
    target: &Path,
    player_count: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    init_new_game_with_seed(target, player_count, runtime_seed())
}

pub(crate) fn init_new_game_with_seed(
    target: &Path,
    player_count: u8,
    seed: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = build_seeded_new_game(player_count, 3000, seed)?;

    fs::create_dir_all(target)?;
    data.save(target)?;

    let planet_names: Vec<String> = data
        .planets
        .records
        .iter()
        .map(|planet| planet.planet_name())
        .collect();
    let database = DatabaseDat::generate_from_planets_and_year(
        &planet_names,
        data.conquest.game_year(),
        data.conquest.player_count() as usize,
        None,
    );
    fs::write(target.join("DATABASE.DAT"), database.to_bytes())?;

    for name in ["MESSAGES.DAT", "RESULTS.DAT"] {
        let path = target.join(name);
        if !path.exists() {
            fs::write(path, [])?;
        }
    }

    Ok(())
}

pub(crate) fn init_new_game_from_config(
    target: &Path,
    config_path: &Path,
    player_count_override: Option<u8>,
    seed_override: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = SetupConfig::load_kdl(config_path)?;
    let config = if let Some(player_count) = player_count_override {
        config.with_player_count_override(player_count)?
    } else {
        config
    };
    let data = config.build_game_data(seed_override.unwrap_or_else(runtime_seed))?;

    fs::create_dir_all(target)?;
    data.save(target)?;

    let planet_names: Vec<String> = data
        .planets
        .records
        .iter()
        .map(|planet| planet.planet_name())
        .collect();
    let database = DatabaseDat::generate_from_planets_and_year(
        &planet_names,
        data.conquest.game_year(),
        data.conquest.player_count() as usize,
        None,
    );
    fs::write(target.join("DATABASE.DAT"), database.to_bytes())?;

    for name in ["MESSAGES.DAT", "RESULTS.DAT"] {
        let path = target.join(name);
        if !path.exists() {
            fs::write(path, [])?;
        }
    }

    Ok(())
}

fn runtime_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0xEC15_1515_0000_0001)
}

pub(crate) fn set_maintenance_days(
    dir: &Path,
    day_names: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut enabled = [false; 7];
    for day_name in day_names {
        let idx = weekday_index(day_name).ok_or_else(|| format!("unknown weekday: {day_name}"))?;
        enabled[idx] = true;
    }

    let conquest_path = dir.join("CONQUEST.DAT");
    let mut conquest = ConquestDat::parse(&fs::read(&conquest_path)?)?;
    conquest.set_maintenance_schedule_enabled(enabled);
    fs::write(&conquest_path, conquest.to_bytes())?;

    print_maintenance_days(dir)?;
    Ok(())
}

pub(crate) fn print_port_setup(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    println!("Directory: {}", dir.display());
    println!("ECUTIL F5 Modem / Com Port Setup");
    for com_index in 0..4 {
        println!(
            "  COM {} IRQ: {}",
            com_index + 1,
            setup.com_irq_raw(com_index).unwrap_or_default()
        );
    }
    for com_index in 0..4 {
        println!(
            "  COM {} Hardware Flow Control: {}",
            com_index + 1,
            yes_no(
                setup
                    .com_hardware_flow_control_enabled(com_index)
                    .unwrap_or(false)
            )
        );
    }
    Ok(())
}

pub(crate) fn print_snoop(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    println!("Directory: {}", dir.display());
    println!(
        "Snoop enabled: {}",
        if setup.snoop_enabled() { "yes" } else { "no" }
    );
    Ok(())
}

pub(crate) fn set_snoop(dir: &Path, enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
    let setup_path = dir.join("SETUP.DAT");
    let mut setup = SetupDat::parse(&fs::read(&setup_path)?)?;
    setup.set_snoop_enabled(enabled);
    fs::write(&setup_path, setup.to_bytes())?;
    print_snoop(dir)?;
    Ok(())
}

pub(crate) fn print_flow_control(
    dir: &Path,
    port_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    let com_index = com_index(port_name).ok_or_else(|| format!("unknown COM port: {port_name}"))?;
    println!("Directory: {}", dir.display());
    println!(
        "COM {} Hardware Flow Control: {}",
        com_index + 1,
        yes_no(
            setup
                .com_hardware_flow_control_enabled(com_index)
                .unwrap_or(false)
        )
    );
    Ok(())
}

pub(crate) fn set_flow_control(
    dir: &Path,
    port_name: &str,
    enabled: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let com_index = com_index(port_name).ok_or_else(|| format!("unknown COM port: {port_name}"))?;
    let setup_path = dir.join("SETUP.DAT");
    let mut setup = SetupDat::parse(&fs::read(&setup_path)?)?;
    setup.set_com_hardware_flow_control_enabled(com_index, enabled);
    fs::write(&setup_path, setup.to_bytes())?;
    print_flow_control(dir, port_name)?;
    Ok(())
}

pub(crate) fn print_com_irq(dir: &Path, port_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    let com_index = com_index(port_name).ok_or_else(|| format!("unknown COM port: {port_name}"))?;
    println!("Directory: {}", dir.display());
    println!(
        "COM {} IRQ: {}",
        com_index + 1,
        setup.com_irq_raw(com_index).unwrap_or_default()
    );
    Ok(())
}

pub(crate) fn set_com_irq(
    dir: &Path,
    port_name: &str,
    irq: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    if irq > 7 {
        return Err(format!("IRQ must be in 0..=7, got {irq}").into());
    }
    let com_index = com_index(port_name).ok_or_else(|| format!("unknown COM port: {port_name}"))?;
    let setup_path = dir.join("SETUP.DAT");
    let mut setup = SetupDat::parse(&fs::read(&setup_path)?)?;
    setup.set_com_irq_raw(com_index, irq);
    fs::write(&setup_path, setup.to_bytes())?;
    print_com_irq(dir, port_name)?;
    Ok(())
}

pub(crate) fn print_local_timeout(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    println!("Directory: {}", dir.display());
    println!(
        "Local timeout enabled: {}",
        if setup.local_timeout_enabled() {
            "yes"
        } else {
            "no"
        }
    );
    Ok(())
}

pub(crate) fn set_local_timeout(
    dir: &Path,
    enabled: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let setup_path = dir.join("SETUP.DAT");
    let mut setup = SetupDat::parse(&fs::read(&setup_path)?)?;
    setup.set_local_timeout_enabled(enabled);
    fs::write(&setup_path, setup.to_bytes())?;
    print_local_timeout(dir)?;
    Ok(())
}

pub(crate) fn print_remote_timeout(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    println!("Directory: {}", dir.display());
    println!(
        "Remote timeout enabled: {}",
        if setup.remote_timeout_enabled() {
            "yes"
        } else {
            "no"
        }
    );
    Ok(())
}

pub(crate) fn set_remote_timeout(
    dir: &Path,
    enabled: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let setup_path = dir.join("SETUP.DAT");
    let mut setup = SetupDat::parse(&fs::read(&setup_path)?)?;
    setup.set_remote_timeout_enabled(enabled);
    fs::write(&setup_path, setup.to_bytes())?;
    print_remote_timeout(dir)?;
    Ok(())
}

pub(crate) fn print_max_key_gap(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    println!("Directory: {}", dir.display());
    println!(
        "Maximum time between key strokes (minutes): {}",
        setup.max_time_between_keys_minutes_raw()
    );
    Ok(())
}

pub(crate) fn set_max_key_gap(dir: &Path, minutes: u8) -> Result<(), Box<dyn std::error::Error>> {
    let setup_path = dir.join("SETUP.DAT");
    let mut setup = SetupDat::parse(&fs::read(&setup_path)?)?;
    setup.set_max_time_between_keys_minutes_raw(minutes);
    fs::write(&setup_path, setup.to_bytes())?;
    print_max_key_gap(dir)?;
    Ok(())
}

pub(crate) fn print_minimum_time(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    println!("Directory: {}", dir.display());
    println!(
        "Minimum time granted (minutes): {}",
        setup.minimum_time_granted_minutes_raw()
    );
    Ok(())
}

pub(crate) fn set_minimum_time(dir: &Path, minutes: u8) -> Result<(), Box<dyn std::error::Error>> {
    let setup_path = dir.join("SETUP.DAT");
    let mut setup = SetupDat::parse(&fs::read(&setup_path)?)?;
    setup.set_minimum_time_granted_minutes_raw(minutes);
    fs::write(&setup_path, setup.to_bytes())?;
    print_minimum_time(dir)?;
    Ok(())
}

pub(crate) fn print_purge_after(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    println!("Directory: {}", dir.display());
    println!("Purge after turns (raw): {}", setup.purge_after_turns_raw());
    Ok(())
}

pub(crate) fn print_setup_programs(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
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
    println!("  C Snoop Enabled: {}", yes_no(setup.snoop_enabled()));
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

pub(crate) fn set_purge_after(dir: &Path, turns: u8) -> Result<(), Box<dyn std::error::Error>> {
    let setup_path = dir.join("SETUP.DAT");
    let mut setup = SetupDat::parse(&fs::read(&setup_path)?)?;
    setup.set_purge_after_turns_raw(turns);
    fs::write(&setup_path, setup.to_bytes())?;
    print_purge_after(dir)?;
    Ok(())
}

pub(crate) fn print_autopilot_after(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    println!("Directory: {}", dir.display());
    println!(
        "Autopilot inactive turns (raw): {}",
        setup.autopilot_inactive_turns_raw()
    );
    Ok(())
}

pub(crate) fn set_autopilot_after(dir: &Path, turns: u8) -> Result<(), Box<dyn std::error::Error>> {
    let setup_path = dir.join("SETUP.DAT");
    let mut setup = SetupDat::parse(&fs::read(&setup_path)?)?;
    setup.set_autopilot_inactive_turns_raw(turns);
    fs::write(&setup_path, setup.to_bytes())?;
    print_autopilot_after(dir)?;
    Ok(())
}

fn weekday_labels() -> [&'static str; 7] {
    ["sun", "mon", "tue", "wed", "thu", "fri", "sat"]
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

fn com_index(port_name: &str) -> Option<usize> {
    match port_name.to_ascii_lowercase().as_str() {
        "com1" | "1" => Some(0),
        "com2" | "2" => Some(1),
        "com3" | "3" => Some(2),
        "com4" | "4" => Some(3),
        _ => None,
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "Yes" } else { "No" }
}
