mod commands;
mod support;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use ec_data::{BaseDat, ConquestDat, FleetDat, IpbmDat, PlanetDat, PlayerDat, SetupDat};
use commands::fleet_order::{
    apply_fleet_order_scenario, fleet_order_errors, init_fleet_order_batch,
    init_fleet_order_scenario, print_fleet_order_report, set_fleet_order,
    validate_fleet_order_scenario,
};
use commands::guard_starbase::{
    apply_guard_starbase_scenario, guard_starbase_errors, init_guard_starbase_batch,
    init_guard_starbase_onebase, print_guard_starbase_report, set_guard_starbase_onebase,
    validate_guard_starbase_scenario,
};
use commands::ipbm::{
    init_ipbm_batch, init_ipbm_zero_records, ipbm_errors, print_ipbm_report,
    set_ipbm_record_prefix, set_ipbm_zero_records, validate_ipbm,
};
use commands::planet_build::{
    apply_planet_build_scenario, init_planet_build_batch, init_planet_build_scenario,
    planet_build_errors, print_planet_build_report, set_planet_build,
    validate_planet_build_scenario,
};
use support::parse::{
    parse_optional_source_and_target, parse_optional_source_target_and_coord_list,
    parse_optional_source_target_and_count, parse_optional_source_target_and_count_list,
    parse_optional_source_target_and_name, parse_optional_source_target_and_xy,
    parse_target_and_fleet_spec, parse_target_and_fleet_spec_list, parse_target_and_planet_spec,
    parse_target_and_planet_spec_list, parse_u16_arg, parse_u8_arg, parse_usize_1_based,
};
use support::paths::{
    default_fixture_dir, init_fixture_dir, post_maint_fixture_dir, repo_root, resolve_repo_path,
};

pub(crate) const INIT_FILES: &[&str] = &[
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KnownScenario {
    FleetOrder,
    PlanetBuild,
    GuardStarbase,
}

impl KnownScenario {
    fn all() -> [Self; 3] {
        [Self::FleetOrder, Self::PlanetBuild, Self::GuardStarbase]
    }

    fn name(self) -> &'static str {
        match self {
            Self::FleetOrder => "fleet-order",
            Self::PlanetBuild => "planet-build",
            Self::GuardStarbase => "guard-starbase",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "fleet-order" => Some(Self::FleetOrder),
            "planet-build" => Some(Self::PlanetBuild),
            "guard-starbase" => Some(Self::GuardStarbase),
            _ => None,
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::FleetOrder => "accepted fleet movement/order fixture rooted in FLEETS.DAT",
            Self::PlanetBuild => "accepted planet build-queue fixture rooted in PLANETS.DAT",
            Self::GuardStarbase => "accepted one-base guard-starbase fixture spanning PLAYER/FLEETS/BASES",
        }
    }

    fn preserved_fixture_dir(self) -> PathBuf {
        let root = repo_root().join("fixtures");
        match self {
            Self::FleetOrder => root.join("ecmaint-fleet-pre/v1.5"),
            Self::PlanetBuild => root.join("ecmaint-build-pre/v1.5"),
            Self::GuardStarbase => root.join("ecmaint-starbase-pre/v1.5"),
        }
    }

    fn exact_match_files(self) -> &'static [&'static str] {
        match self {
            Self::FleetOrder => &["FLEETS.DAT"],
            Self::PlanetBuild => &["PLANETS.DAT"],
            Self::GuardStarbase => &["PLAYER.DAT", "FLEETS.DAT", "BASES.DAT"],
        }
    }
}

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
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target)) = parse_optional_source_and_target(
                remaining,
                default_fixture_dir(),
            ) else {
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
        "port-setup" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            print_port_setup(&dir)?;
        }
        "flow-control" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            let Some(port_name) = args.next() else {
                print_usage();
                return Ok(());
            };
            match args.next().as_deref() {
                None => print_flow_control(&dir, &port_name)?,
                Some("on") => set_flow_control(&dir, &port_name, true)?,
                Some("off") => set_flow_control(&dir, &port_name, false)?,
                _ => print_usage(),
            }
        }
        "com-irq" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            let Some(port_name) = args.next() else {
                print_usage();
                return Ok(());
            };
            match args.next() {
                None => print_com_irq(&dir, &port_name)?,
                Some(irq) => {
                    let irq = irq.parse::<u8>()?;
                    set_com_irq(&dir, &port_name, irq)?;
                }
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
        "local-timeout" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            match args.next().as_deref() {
                None => print_local_timeout(&dir)?,
                Some("on") => set_local_timeout(&dir, true)?,
                Some("off") => set_local_timeout(&dir, false)?,
                _ => print_usage(),
            }
        }
        "remote-timeout" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            match args.next().as_deref() {
                None => print_remote_timeout(&dir)?,
                Some("on") => set_remote_timeout(&dir, true)?,
                Some("off") => set_remote_timeout(&dir, false)?,
                _ => print_usage(),
            }
        }
        "max-key-gap" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            match args.next() {
                None => print_max_key_gap(&dir)?,
                Some(minutes) => {
                    let minutes = minutes.parse::<u8>()?;
                    set_max_key_gap(&dir, minutes)?;
                }
            }
        }
        "minimum-time" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            match args.next() {
                None => print_minimum_time(&dir)?,
                Some(minutes) => {
                    let minutes = minutes.parse::<u8>()?;
                    set_minimum_time(&dir, minutes)?;
                }
            }
        }
        "autopilot-after" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            match args.next() {
                None => print_autopilot_after(&dir)?,
                Some(turns) => {
                    let turns = turns.parse::<u8>()?;
                    set_autopilot_after(&dir, turns)?;
                }
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
        "fleet-order" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            let Some(record_index) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(speed) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(order_code) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(target_x) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(target_y) = args.next() else {
                print_usage();
                return Ok(());
            };
            let aux0 = args.next();
            let aux1 = args.next();
            set_fleet_order(
                &dir,
                parse_usize_1_based(&record_index, "fleet record index")?,
                parse_u8_arg(&speed, "speed")?,
                parse_u8_arg(&order_code, "order code")?,
                parse_u8_arg(&target_x, "target_x")?,
                parse_u8_arg(&target_y, "target_y")?,
                aux0.as_deref().map(|value| parse_u8_arg(value, "aux0")).transpose()?,
                aux1.as_deref().map(|value| parse_u8_arg(value, "aux1")).transpose()?,
            )?;
        }
        "fleet-order-report" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            let record_index_arg = args.next();
            let record_index = record_index_arg.as_deref().unwrap_or("1");
            print_fleet_order_report(
                &dir,
                parse_usize_1_based(record_index, "fleet record index")?,
            )?;
        }
        "fleet-order-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((target, record_index, speed, order_code, target_x, target_y, aux0, aux1)) =
                parse_target_and_fleet_spec(remaining)
            else {
                print_usage();
                return Ok(());
            };
            init_fleet_order_scenario(
                &post_maint_fixture_dir(),
                &target,
                record_index,
                speed,
                order_code,
                target_x,
                target_y,
                aux0,
                aux1,
            )?;
        }
        "fleet-order-batch-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((target_root, specs)) = parse_target_and_fleet_spec_list(remaining) else {
                print_usage();
                return Ok(());
            };
            init_fleet_order_batch(&post_maint_fixture_dir(), &target_root, &specs)?;
        }
        "planet-build" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            let Some(record_index) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(slot_raw) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(kind_raw) = args.next() else {
                print_usage();
                return Ok(());
            };
            set_planet_build(
                &dir,
                parse_usize_1_based(&record_index, "planet record index")?,
                parse_u8_arg(&slot_raw, "build slot")?,
                parse_u8_arg(&kind_raw, "build kind")?,
            )?;
        }
        "planet-build-report" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            let record_index_arg = args.next();
            let record_index = record_index_arg.as_deref().unwrap_or("15");
            print_planet_build_report(
                &dir,
                parse_usize_1_based(record_index, "planet record index")?,
            )?;
        }
        "planet-build-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((target, record_index, slot_raw, kind_raw)) =
                parse_target_and_planet_spec(remaining)
            else {
                print_usage();
                return Ok(());
            };
            init_planet_build_scenario(
                &post_maint_fixture_dir(),
                &target,
                record_index,
                slot_raw,
                kind_raw,
            )?;
        }
        "planet-build-batch-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((target_root, specs)) = parse_target_and_planet_spec_list(remaining) else {
                print_usage();
                return Ok(());
            };
            init_planet_build_batch(&post_maint_fixture_dir(), &target_root, &specs)?;
        }
        "guard-starbase-onebase" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            let Some(target_x) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(target_y) = args.next() else {
                print_usage();
                return Ok(());
            };
            set_guard_starbase_onebase(
                &dir,
                parse_u8_arg(&target_x, "target_x")?,
                parse_u8_arg(&target_y, "target_y")?,
            )?;
        }
        "guard-starbase-report" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            print_guard_starbase_report(&dir)?;
        }
        "guard-starbase-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target, target_x, target_y)) = parse_optional_source_target_and_xy(
                remaining,
                post_maint_fixture_dir(),
            ) else {
                print_usage();
                return Ok(());
            };
            init_guard_starbase_onebase(&source, &target, target_x, target_y)?;
        }
        "ipbm-report" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            print_ipbm_report(&dir)?;
        }
        "ipbm-zero" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            let Some(count) = args.next() else {
                print_usage();
                return Ok(());
            };
            set_ipbm_zero_records(&dir, count.parse::<u16>()?)?;
        }
        "ipbm-record-set" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            let Some(record_index) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(primary) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(owner) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(gate) = args.next() else {
                print_usage();
                return Ok(());
            };
            let Some(follow_on) = args.next() else {
                print_usage();
                return Ok(());
            };
            set_ipbm_record_prefix(
                &dir,
                parse_usize_1_based(&record_index, "ipbm record index")?,
                parse_u16_arg(&primary, "primary")?,
                parse_u8_arg(&owner, "owner")?,
                parse_u16_arg(&gate, "gate")?,
                parse_u16_arg(&follow_on, "follow_on")?,
            )?;
        }
        "ipbm-validate" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            validate_ipbm(dir.as_path())?;
        }
        "ipbm-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target, count)) = parse_optional_source_target_and_count(
                remaining,
                post_maint_fixture_dir(),
            ) else {
                print_usage();
                return Ok(());
            };
            init_ipbm_zero_records(&source, &target, count)?;
        }
        "compliance-report" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            print_compliance_report(&dir)?;
        }
        "guard-starbase-batch-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target_root, coords)) = parse_optional_source_target_and_coord_list(
                remaining,
                post_maint_fixture_dir(),
            ) else {
                print_usage();
                return Ok(());
            };
            init_guard_starbase_batch(&source, &target_root, &coords)?;
        }
        "ipbm-batch-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target_root, counts)) = parse_optional_source_target_and_count_list(
                remaining,
                post_maint_fixture_dir(),
            ) else {
                print_usage();
                return Ok(());
            };
            init_ipbm_batch(&source, &target_root, &counts)?;
        }
        "scenario" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            let selector = args.next();
            if selector.as_deref() == Some("list") {
                print_known_scenarios();
            } else if selector.as_deref() == Some("show") {
                match args.next().as_deref().and_then(KnownScenario::parse) {
                    Some(scenario) => print_known_scenario_details(scenario),
                    None => print_usage(),
                }
            } else {
                match selector.as_deref().and_then(KnownScenario::parse) {
                    Some(scenario) => apply_known_scenario(&dir, scenario)?,
                    None => print_usage(),
                }
            }
        }
        "scenario-init-all" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target_root)) = parse_optional_source_and_target(
                remaining,
                post_maint_fixture_dir(),
            ) else {
                print_usage();
                return Ok(());
            };
            init_all_known_scenarios(&source, &target_root)?;
        }
        "scenario-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target, scenario_name)) = parse_optional_source_target_and_name(
                remaining,
                post_maint_fixture_dir(),
            ) else {
                print_usage();
                return Ok(());
            };
            match KnownScenario::parse(&scenario_name) {
                Some(scenario) => init_known_scenario(&source, &target, scenario)?,
                None => print_usage(),
            }
        }
        "validate" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            match args.next().as_deref() {
                Some("all") => validate_all_known_scenarios(&dir)?,
                Some(name) => match KnownScenario::parse(name) {
                    Some(scenario) => validate_known_scenario(&dir, scenario)?,
                    None => print_usage(),
                },
                _ => print_usage(),
            }
        }
        "validate-preserved" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            match args.next().as_deref() {
                Some("all") => validate_all_preserved_scenarios(&dir)?,
                Some(name) => match KnownScenario::parse(name) {
                    Some(scenario) => validate_preserved_scenario(&dir, scenario)?,
                    None => print_usage(),
                },
                _ => print_usage(),
            }
        }
        "compare-preserved" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            match args.next().as_deref() {
                Some("all") => compare_all_preserved_scenarios(&dir)?,
                Some(name) => match KnownScenario::parse(name) {
                    Some(scenario) => compare_preserved_scenario(&dir, scenario)?,
                    None => print_usage(),
                },
                _ => print_usage(),
            }
        }
        "compliance-batch-report" => {
            let root = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(post_maint_fixture_dir);
            print_compliance_batch_report(&root)?;
        }
        _ => print_usage(),
    }

    Ok(())
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
            "  slot {}: owner_mode={} assigned_player_flag={} tax={} last_run_year={} summary={}",
            idx + 1,
            record.owner_mode_raw(),
            record.assigned_player_flag_raw(),
            record.tax_rate(),
            record.last_run_year(),
            record.ownership_summary()
        );
        println!("    starbase_count_raw={}", record.starbase_count_raw());
    }
    println!();

    println!("Planets:");
    for (idx, record) in planets.records.iter().enumerate().take(5) {
        println!(
            "  planet {:02}: coords={:02x?} hdr={:02x?} len={} text='{}' tail58={:02x?} summary='{}'",
            idx + 1,
            record.coords_raw(),
            record.header_bytes(),
            record.string_len(),
            ascii_trim(record.status_or_name_bytes()),
            &record.raw[0x58..=0x60],
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
                let fleet_display_count = fleets.records.len().min(16);
                for (idx, record) in fleets.records.iter().enumerate().take(fleet_display_count) {
                    println!(
                        "  fleet {:02}: id={} slot={} prev={} next={} cur_spd={} max_spd={} roe={} ships={} loc_raw={:02x?} order={}({}) target_raw={:02x?} summary='{}'",
                        idx + 1,
                        record.fleet_id(),
                        record.local_slot(),
                        record.previous_fleet_id(),
                        record.next_fleet_id(),
                        record.current_speed(),
                        record.max_speed(),
                        record.rules_of_engagement(),
                        record.ship_composition_summary(),
                        record.current_location_coords_raw(),
                        record.standing_order_kind().as_str(),
                        record.standing_order_code_raw(),
                        record.standing_order_target_coords_raw(),
                        record.standing_order_summary()
                    );
                    println!("    mission_aux={:02x?}", record.mission_aux_bytes());
                }
                if fleets.records.len() > fleet_display_count {
                    println!("  ... {} total fleet records", fleets.records.len());
                }

                let looks_like_initialized_blocks = !fleets.records.is_empty()
                    && fleets.records.len() % 4 == 0
                    && fleets
                        .records
                        .chunks_exact(4)
                        .all(|group| group.iter().map(|r| r.local_slot()).eq([1, 2, 3, 4]));

                if looks_like_initialized_blocks {
                    println!();
                    println!("Fleet Groups:");
                    for (group_idx, group) in fleets.records.chunks_exact(4).enumerate() {
                        let home = group[0].current_location_coords_raw();
                        println!(
                            "  empire block {}: loc_raw={:02x?} target_raw={:02x?}",
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
            }
            Err(err) => {
                println!();
                println!("Fleets:");
                println!("  FLEETS.DAT does not match initialized 16x54 layout: {err}");
            }
        },
        Err(_) => {}
    }

    match fs::read(dir.join("BASES.DAT")) {
        Ok(bytes) => match BaseDat::parse(&bytes) {
            Ok(bases) => {
                println!();
                println!("Bases:");
                for (idx, record) in bases.records.iter().enumerate() {
                    println!(
                        "  base {:02}: slot={} active={} id={} link={} owner={} coords={:02x?}",
                        idx + 1,
                        record.local_slot_raw(),
                        record.active_flag_raw(),
                        record.base_id_raw(),
                        record.link_word_raw(),
                        record.owner_empire_raw(),
                        record.coords_raw()
                    );
                }
            }
            Err(err) => {
                println!();
                println!("Bases:");
                println!("  BASES.DAT does not match 35-byte layout: {err}");
            }
        },
        Err(_) => {}
    }

    match fs::read(dir.join("IPBM.DAT")) {
        Ok(bytes) => match IpbmDat::parse(&bytes) {
            Ok(ipbm) => {
                println!();
                println!("IPBM:");
                for (idx, record) in ipbm.records.iter().enumerate() {
                    println!(
                        "  record {:02}: primary={} owner={} gate={} follow_on={}",
                        idx + 1,
                        record.primary_word_raw(),
                        record.owner_empire_raw(),
                        record.gate_word_raw(),
                        record.follow_on_word_raw()
                    );
                }
            }
            Err(err) => {
                println!();
                println!("IPBM:");
                println!("  IPBM.DAT does not match 32-byte layout: {err}");
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
    println!(
        "SETUP.com_irqs=[{}, {}, {}, {}]",
        setup.com_irq_raw(0).unwrap_or_default(),
        setup.com_irq_raw(1).unwrap_or_default(),
        setup.com_irq_raw(2).unwrap_or_default(),
        setup.com_irq_raw(3).unwrap_or_default()
    );
    println!(
        "SETUP.com_flow_control=[{}, {}, {}, {}]",
        setup.com_hardware_flow_control_enabled(0).unwrap_or(false),
        setup.com_hardware_flow_control_enabled(1).unwrap_or(false),
        setup.com_hardware_flow_control_enabled(2).unwrap_or(false),
        setup.com_hardware_flow_control_enabled(3).unwrap_or(false)
    );
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

fn print_port_setup(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
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

fn print_flow_control(dir: &Path, port_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    let com_index = com_index(port_name)
        .ok_or_else(|| format!("unknown COM port: {port_name}"))?;
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

fn set_flow_control(
    dir: &Path,
    port_name: &str,
    enabled: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let com_index = com_index(port_name)
        .ok_or_else(|| format!("unknown COM port: {port_name}"))?;
    let setup_path = dir.join("SETUP.DAT");
    let mut setup = SetupDat::parse(&fs::read(&setup_path)?)?;
    setup.set_com_hardware_flow_control_enabled(com_index, enabled);
    fs::write(&setup_path, setup.to_bytes())?;
    print_flow_control(dir, port_name)?;
    Ok(())
}

fn print_com_irq(dir: &Path, port_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    let com_index = com_index(port_name)
        .ok_or_else(|| format!("unknown COM port: {port_name}"))?;
    println!("Directory: {}", dir.display());
    println!(
        "COM {} IRQ: {}",
        com_index + 1,
        setup.com_irq_raw(com_index).unwrap_or_default()
    );
    Ok(())
}

fn set_com_irq(dir: &Path, port_name: &str, irq: u8) -> Result<(), Box<dyn std::error::Error>> {
    if irq > 7 {
        return Err(format!("IRQ must be in 0..=7, got {irq}").into());
    }
    let com_index = com_index(port_name)
        .ok_or_else(|| format!("unknown COM port: {port_name}"))?;
    let setup_path = dir.join("SETUP.DAT");
    let mut setup = SetupDat::parse(&fs::read(&setup_path)?)?;
    setup.set_com_irq_raw(com_index, irq);
    fs::write(&setup_path, setup.to_bytes())?;
    print_com_irq(dir, port_name)?;
    Ok(())
}

fn print_local_timeout(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    println!("Directory: {}", dir.display());
    println!(
        "Local timeout enabled: {}",
        if setup.local_timeout_enabled() { "yes" } else { "no" }
    );
    Ok(())
}

fn set_local_timeout(dir: &Path, enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
    let setup_path = dir.join("SETUP.DAT");
    let mut setup = SetupDat::parse(&fs::read(&setup_path)?)?;
    setup.set_local_timeout_enabled(enabled);
    fs::write(&setup_path, setup.to_bytes())?;
    print_local_timeout(dir)?;
    Ok(())
}

fn print_remote_timeout(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    println!("Directory: {}", dir.display());
    println!(
        "Remote timeout enabled: {}",
        if setup.remote_timeout_enabled() { "yes" } else { "no" }
    );
    Ok(())
}

fn set_remote_timeout(dir: &Path, enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
    let setup_path = dir.join("SETUP.DAT");
    let mut setup = SetupDat::parse(&fs::read(&setup_path)?)?;
    setup.set_remote_timeout_enabled(enabled);
    fs::write(&setup_path, setup.to_bytes())?;
    print_remote_timeout(dir)?;
    Ok(())
}

fn print_max_key_gap(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    println!("Directory: {}", dir.display());
    println!(
        "Maximum time between key strokes (minutes): {}",
        setup.max_time_between_keys_minutes_raw()
    );
    Ok(())
}

fn set_max_key_gap(dir: &Path, minutes: u8) -> Result<(), Box<dyn std::error::Error>> {
    let setup_path = dir.join("SETUP.DAT");
    let mut setup = SetupDat::parse(&fs::read(&setup_path)?)?;
    setup.set_max_time_between_keys_minutes_raw(minutes);
    fs::write(&setup_path, setup.to_bytes())?;
    print_max_key_gap(dir)?;
    Ok(())
}

fn print_minimum_time(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    println!("Directory: {}", dir.display());
    println!(
        "Minimum time granted (minutes): {}",
        setup.minimum_time_granted_minutes_raw()
    );
    Ok(())
}

fn set_minimum_time(dir: &Path, minutes: u8) -> Result<(), Box<dyn std::error::Error>> {
    let setup_path = dir.join("SETUP.DAT");
    let mut setup = SetupDat::parse(&fs::read(&setup_path)?)?;
    setup.set_minimum_time_granted_minutes_raw(minutes);
    fs::write(&setup_path, setup.to_bytes())?;
    print_minimum_time(dir)?;
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

fn print_autopilot_after(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    println!("Directory: {}", dir.display());
    println!(
        "Autopilot inactive turns (raw): {}",
        setup.autopilot_inactive_turns_raw()
    );
    Ok(())
}

fn set_autopilot_after(dir: &Path, turns: u8) -> Result<(), Box<dyn std::error::Error>> {
    let setup_path = dir.join("SETUP.DAT");
    let mut setup = SetupDat::parse(&fs::read(&setup_path)?)?;
    setup.set_autopilot_inactive_turns_raw(turns);
    fs::write(&setup_path, setup.to_bytes())?;
    print_autopilot_after(dir)?;
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

fn apply_known_scenario(
    dir: &Path,
    scenario: KnownScenario,
) -> Result<(), Box<dyn std::error::Error>> {
    match scenario {
        KnownScenario::FleetOrder => apply_fleet_order_scenario(dir),
        KnownScenario::PlanetBuild => apply_planet_build_scenario(dir),
        KnownScenario::GuardStarbase => apply_guard_starbase_scenario(dir),
    }
}

fn print_compliance_report(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Compliance Report");
    println!("  dir={}", dir.display());
    println!();

    match validate_guard_starbase_scenario(dir) {
        Ok(()) => println!("OK   guard-starbase-linkage"),
        Err(err) => println!("FAIL guard-starbase-linkage: {err}"),
    }

    match validate_ipbm(dir) {
        Ok(()) => println!("OK   ipbm-count-length"),
        Err(err) => println!("FAIL ipbm-count-length: {err}"),
    }

    let player = PlayerDat::parse(&fs::read(dir.join("PLAYER.DAT"))?)?;
    let fleets = FleetDat::parse(&fs::read(dir.join("FLEETS.DAT"))?)?;
    let bases = BaseDat::parse(&fs::read(dir.join("BASES.DAT"))?)?;
    let ipbm = IpbmDat::parse(&fs::read(dir.join("IPBM.DAT"))?)?;

    let player1 = &player.records[0];
    println!();
    println!(
        "Key words: player.starbase_count={} player.ipbm_count={}",
        player1.starbase_count_raw(),
        player1.ipbm_count_raw()
    );
    if let Some(fleet1) = fleets.records.first() {
        println!(
            "  fleet1.local_slot={} fleet1.id={} fleet1.guard={}/{} target={:?}",
            fleet1.local_slot_word_raw(),
            fleet1.fleet_id_word_raw(),
            fleet1.guard_starbase_index_raw(),
            fleet1.guard_starbase_enable_raw(),
            fleet1.standing_order_target_coords_raw()
        );
    }
    if let Some(base1) = bases.records.first() {
        println!(
            "  base1.summary={} base1.id={} base1.chain={} coords={:?}",
            base1.summary_word_raw(),
            base1.base_id_raw(),
            base1.chain_word_raw(),
            base1.coords_raw()
        );
    } else {
        println!("  base1=<none>");
    }
    println!("  ipbm.record_count={}", ipbm.records.len());
    Ok(())
}

fn print_compliance_batch_report(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Compliance Batch Report");
    println!("  root={}", root.display());
    let mut dirs = fs::read_dir(root)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            entry.file_type().ok().filter(|ty| ty.is_dir())?;
            Some(entry.path())
        })
        .collect::<Vec<_>>();
    dirs.sort();

    for dir in dirs {
        print!("{}: ", dir.file_name().unwrap_or_default().to_string_lossy());
        let fleet_ok = match FleetDat::parse(&fs::read(dir.join("FLEETS.DAT"))?) {
            Ok(fleets) => fleet_order_errors(&fleets, 1, 0x03, 0x0C, [0x0F, 0x0D], None, None)
                .is_empty(),
            Err(_) => false,
        };
        let build_ok = match PlanetDat::parse(&fs::read(dir.join("PLANETS.DAT"))?) {
            Ok(planets) => planet_build_errors(&planets, 15, 0x03, 0x01).is_empty(),
            Err(_) => false,
        };
        let guard_ok = match (
            PlayerDat::parse(&fs::read(dir.join("PLAYER.DAT"))?),
            FleetDat::parse(&fs::read(dir.join("FLEETS.DAT"))?),
            BaseDat::parse(&fs::read(dir.join("BASES.DAT"))?),
        ) {
            (Ok(player), Ok(fleets), Ok(bases)) => {
                guard_starbase_errors(&player, &fleets, &bases).is_empty()
            }
            _ => false,
        };
        let ipbm_ok = match (
            PlayerDat::parse(&fs::read(dir.join("PLAYER.DAT"))?),
            fs::read(dir.join("IPBM.DAT")),
        ) {
            (Ok(player), Ok(ipbm_bytes)) => match IpbmDat::parse(&ipbm_bytes) {
                Ok(ipbm) => ipbm_errors(&player, &ipbm, ipbm_bytes.len()).is_empty(),
                Err(_) => false,
            },
            _ => false,
        };
        println!(
            "fleet-order={} planet-build={} guard-starbase={} ipbm={}",
            if fleet_ok { "ok" } else { "fail" },
            if build_ok { "ok" } else { "fail" },
            if guard_ok { "ok" } else { "fail" },
            if ipbm_ok { "ok" } else { "fail" }
        );
    }

    Ok(())
}

fn validate_known_scenario(
    dir: &Path,
    scenario: KnownScenario,
) -> Result<(), Box<dyn std::error::Error>> {
    match scenario {
        KnownScenario::FleetOrder => {
            validate_fleet_order_scenario(dir, 1, 0x03, 0x0C, 0x0F, 0x0D, None, None)
        }
        KnownScenario::PlanetBuild => validate_planet_build_scenario(dir, 15, 0x03, 0x01),
        KnownScenario::GuardStarbase => validate_guard_starbase_scenario(dir),
    }
}

fn validate_all_known_scenarios(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut matched = 0usize;
    for scenario in KnownScenario::all() {
        let name = scenario.name();
        let result = validate_known_scenario(dir, scenario);
        match result {
            Ok(()) => {
                println!("OK   {name}");
                matched += 1;
            }
            Err(err) => {
                println!("FAIL {name}: {err}");
            }
        }
    }

    if matched == 0 {
        Err("directory does not match any known accepted scenario".into())
    } else {
        Ok(())
    }
}

fn validate_preserved_scenario(
    dir: &Path,
    scenario: KnownScenario,
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture_dir = scenario.preserved_fixture_dir();
    let mut errors = Vec::new();

    for name in scenario.exact_match_files() {
        let actual = fs::read(dir.join(name))?;
        let expected = fs::read(fixture_dir.join(name))?;
        if actual != expected {
            errors.push(format!("{name} differs from preserved fixture"));
        }
    }

    if errors.is_empty() {
        println!("Exact preserved match: {}", scenario.name());
        println!("  fixture: {}", fixture_dir.display());
        for name in scenario.exact_match_files() {
            println!("  {name}");
        }
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}

fn validate_all_preserved_scenarios(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut matched = 0usize;
    for scenario in KnownScenario::all() {
        let name = scenario.name();
        match validate_preserved_scenario(dir, scenario) {
            Ok(()) => {
                println!("OK   {name}");
                matched += 1;
            }
            Err(err) => {
                println!("FAIL {name}: {err}");
            }
        }
    }

    if matched == 0 {
        Err("directory does not exactly match any preserved accepted scenario".into())
    } else {
        Ok(())
    }
}

fn compare_preserved_scenario(
    dir: &Path,
    scenario: KnownScenario,
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture_dir = scenario.preserved_fixture_dir();
    println!("Scenario: {}", scenario.name());
    println!("Actual:   {}", dir.display());
    println!("Fixture:  {}", fixture_dir.display());
    println!();

    for name in scenario.exact_match_files() {
        compare_raw_file(dir, &fixture_dir, name)?;
    }

    Ok(())
}

fn compare_all_preserved_scenarios(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for scenario in KnownScenario::all() {
        compare_preserved_scenario(dir, scenario)?;
        println!();
    }
    Ok(())
}

fn init_known_scenario(
    source: &Path,
    target: &Path,
    scenario: KnownScenario,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(target)?;
    for name in INIT_FILES {
        fs::copy(source.join(name), target.join(name))?;
    }
    apply_known_scenario(target, scenario)?;
    println!("Scenario directory initialized at {}", target.display());
    Ok(())
}

fn init_all_known_scenarios(
    source: &Path,
    target_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(target_root)?;
    let mut manifest = String::new();
    manifest.push_str("Known scenarios\n");
    manifest.push_str(&format!("source={}\n", source.display()));
    manifest.push_str(&format!("target_root={}\n", target_root.display()));
    manifest.push('\n');
    for scenario in KnownScenario::all() {
        let scenario_dir = target_root.join(scenario.name());
        init_known_scenario(source, &scenario_dir, scenario)?;
        manifest.push_str(&format!("{}\n", scenario.name()));
        manifest.push_str(&format!("  description={}\n", scenario.description()));
        manifest.push_str(&format!("  dir={}\n", scenario_dir.display()));
        manifest.push_str(&format!(
            "  validate=ec-cli validate {} {}\n\n",
            scenario_dir.display(),
            scenario.name()
        ));
    }
    fs::write(target_root.join("SCENARIOS.txt"), manifest)?;
    println!("Initialized all known scenarios under {}", target_root.display());
    Ok(())
}

fn weekday_labels() -> [&'static str; 7] {
    ["sun", "mon", "tue", "wed", "thu", "fri", "sat"]
}

fn print_known_scenarios() {
    println!("Known scenarios:");
    for scenario in KnownScenario::all() {
        println!("  {}: {}", scenario.name(), scenario.description());
    }
}

fn print_known_scenario_details(scenario: KnownScenario) {
    println!("Scenario: {}", scenario.name());
    println!("Description: {}", scenario.description());
    println!(
        "Preserved fixture: {}",
        scenario.preserved_fixture_dir().display()
    );
    println!("Exact-match files:");
    for name in scenario.exact_match_files() {
        println!("  {name}");
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
    println!(
        "SETUP COM IRQs: [{}, {}, {}, {}]",
        setup.com_irq_raw(0).unwrap_or_default(),
        setup.com_irq_raw(1).unwrap_or_default(),
        setup.com_irq_raw(2).unwrap_or_default(),
        setup.com_irq_raw(3).unwrap_or_default()
    );
    println!(
        "SETUP COM flow control: [{}, {}, {}, {}]",
        yes_no(setup.com_hardware_flow_control_enabled(0).unwrap_or(false)),
        yes_no(setup.com_hardware_flow_control_enabled(1).unwrap_or(false)),
        yes_no(setup.com_hardware_flow_control_enabled(2).unwrap_or(false)),
        yes_no(setup.com_hardware_flow_control_enabled(3).unwrap_or(false))
    );
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
                "  record {:02}: {} differing bytes, current speed {} -> {}, params {:02x?} -> {:02x?}",
                idx + 1,
                count,
                a.current_speed(),
                b.current_speed(),
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
    println!("  ec-cli fleet-order <dir> <fleet_record> <speed> <order_code> <target_x> <target_y> [aux0] [aux1]");
    println!("  ec-cli fleet-order-report [dir] [fleet_record]");
    println!("  ec-cli fleet-order-init <target_dir> <fleet_record> <speed> <order_code> <target_x> <target_y> [aux0] [aux1]");
    println!("  ec-cli fleet-order-batch-init <target_root> <fleet_record:speed:order:target_x:target_y[:aux0[:aux1]]>...");
    println!("  ec-cli planet-build <dir> <planet_record> <build_slot_raw> <build_kind_raw>");
    println!("  ec-cli planet-build-report [dir] [planet_record]");
    println!("  ec-cli planet-build-init <target_dir> <planet_record> <build_slot_raw> <build_kind_raw>");
    println!("  ec-cli planet-build-batch-init <target_root> <planet_record:build_slot_raw:build_kind_raw>...");
    println!("  ec-cli guard-starbase-onebase <dir> <target_x> <target_y>");
    println!("  ec-cli guard-starbase-report <dir>");
    println!("  ec-cli guard-starbase-init [source_dir] <target_dir> <target_x> <target_y>");
    println!("  ec-cli guard-starbase-batch-init [source_dir] <target_root> <x:y> <x:y>...");
    println!("  ec-cli ipbm-report <dir>");
    println!("  ec-cli ipbm-zero <dir> <count>");
    println!("  ec-cli ipbm-record-set <dir> <record_index> <primary> <owner> <gate> <follow_on>");
    println!("  ec-cli ipbm-validate <dir>");
    println!("  ec-cli ipbm-init [source_dir] <target_dir> <count>");
    println!("  ec-cli ipbm-batch-init [source_dir] <target_root> <count> <count>...");
    println!("  ec-cli compliance-report <dir>");
    println!("  ec-cli compliance-batch-report <root>");
    println!("  ec-cli scenario <dir> <fleet-order|planet-build|guard-starbase>");
    println!("  ec-cli scenario <dir> show <fleet-order|planet-build|guard-starbase>");
    println!("  ec-cli scenario <dir> list");
    println!("  ec-cli scenario-init-all [source_dir] <target_root>");
    println!("  ec-cli scenario-init [source_dir] <target_dir> <fleet-order|planet-build|guard-starbase>");
    println!("  ec-cli validate <dir> <all|fleet-order|planet-build|guard-starbase>");
    println!("  ec-cli validate-preserved <dir> <all|fleet-order|planet-build|guard-starbase>");
    println!("  ec-cli compare-preserved <dir> <all|fleet-order|planet-build|guard-starbase>");
    println!("  ec-cli match [dir]");
    println!("  ec-cli compare <left_dir> <right_dir>");
    println!("  ec-cli init [source_dir] <target_dir>");
}
