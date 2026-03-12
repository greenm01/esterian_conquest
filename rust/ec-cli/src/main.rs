mod commands;
mod support;

use std::env;
use std::fs;
use std::path::Path;

use commands::compare::{compare_all_preserved_scenarios, compare_dirs, compare_preserved_scenario};
use commands::compliance::{print_compliance_batch_report, print_compliance_report};
use commands::fleet_order::{
    init_fleet_order_batch, init_fleet_order_scenario, print_fleet_order_report, set_fleet_order,
};
use commands::guard_starbase::{
    init_guard_starbase_batch, init_guard_starbase_onebase, print_guard_starbase_report,
    set_guard_starbase_onebase,
};
use commands::inspect::{dump_headers, inspect_dir};
use commands::ipbm::{
    init_ipbm_batch, init_ipbm_zero_records, print_ipbm_report, set_ipbm_record_prefix,
    set_ipbm_zero_records, validate_ipbm,
};
use commands::planet_build::{
    init_planet_build_batch, init_planet_build_scenario, print_planet_build_report,
    set_planet_build,
};
use commands::scenario::{
    apply_known_scenario, init_all_known_scenarios, init_known_scenario, print_known_scenario_details,
    print_known_scenarios, validate_all_known_scenarios, validate_all_preserved_scenarios,
    validate_known_scenario, validate_preserved_scenario, KnownScenario,
};
use commands::setup::{
    print_autopilot_after, print_com_irq, print_flow_control, print_local_timeout,
    print_maintenance_days, print_max_key_gap, print_minimum_time, print_port_setup,
    print_purge_after, print_remote_timeout, print_setup_programs, print_snoop,
    set_autopilot_after, set_com_irq, set_flow_control, set_local_timeout,
    set_maintenance_days, set_max_key_gap, set_minimum_time, set_purge_after,
    set_remote_timeout, set_snoop,
};
use support::parse::{
    parse_optional_source_and_target, parse_optional_source_target_and_coord_list,
    parse_optional_source_target_and_count, parse_optional_source_target_and_count_list,
    parse_optional_source_target_and_name, parse_optional_source_target_and_xy,
    parse_target_and_fleet_spec, parse_target_and_fleet_spec_list, parse_target_and_planet_spec,
    parse_target_and_planet_spec_list, parse_u16_arg, parse_u8_arg, parse_usize_1_based,
};
use support::paths::{
    default_fixture_dir, init_fixture_dir, post_maint_fixture_dir, resolve_repo_path,
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
