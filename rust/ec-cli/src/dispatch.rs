use std::path::PathBuf;

use crate::commands::compare::{
    compare_all_preserved_scenarios, compare_dirs, compare_preserved_scenario,
};
use crate::commands::compliance::{print_compliance_batch_report, print_compliance_report};
use crate::commands::core::{
    print_core_report, sync_core_baseline, sync_core_counts, validate_core_state,
};
use crate::commands::fleet_order::{
    init_fleet_order_batch, init_fleet_order_scenario, print_fleet_order_report, set_fleet_order,
};
use crate::commands::guard_starbase::{
    init_guard_starbase_batch, init_guard_starbase_onebase, print_guard_starbase_report,
    set_guard_starbase_onebase,
};
use crate::commands::inspect::{dump_headers, inspect_dir};
use crate::commands::ipbm::{
    init_ipbm_batch, init_ipbm_zero_records, print_ipbm_report, set_ipbm_record_prefix,
    set_ipbm_zero_records, validate_ipbm,
};
use crate::commands::planet_build::{
    init_planet_build_batch, init_planet_build_scenario, print_planet_build_report,
    set_planet_build,
};
use crate::commands::scenario::{
    apply_known_scenario, apply_known_scenarios, init_all_known_scenarios, init_known_scenario,
    init_known_scenario_chain,
    print_known_scenario_details, print_known_scenarios, validate_all_known_scenarios,
    validate_all_preserved_scenarios, validate_known_scenario, validate_preserved_scenario,
    KnownScenario,
};
use crate::commands::setup::{
    print_autopilot_after, print_com_irq, print_flow_control, print_local_timeout,
    print_maintenance_days, print_max_key_gap, print_minimum_time, print_port_setup,
    print_purge_after, print_remote_timeout, print_setup_programs, print_snoop,
    set_autopilot_after, set_com_irq, set_flow_control, set_local_timeout,
    set_maintenance_days, set_max_key_gap, set_minimum_time, set_purge_after,
    set_remote_timeout, set_snoop,
};
use crate::support::parse::{
    parse_optional_source_and_target, parse_optional_source_target_and_coord_list,
    parse_optional_source_target_and_count, parse_optional_source_target_and_count_list,
    parse_optional_source_target_and_name, parse_optional_source_target_and_xy,
    parse_target_and_fleet_spec, parse_target_and_fleet_spec_list, parse_target_and_planet_spec,
    parse_target_and_planet_spec_list, parse_u16_arg, parse_u8_arg, parse_usize_1_based,
};
use crate::support::paths::{default_fixture_dir, post_maint_fixture_dir, resolve_repo_path};
use crate::usage::print_usage;
use crate::workspace::{initialize_dir, match_fixture_set};

fn next_dir(args: &mut impl Iterator<Item = String>) -> PathBuf {
    args.next()
        .map(|arg| resolve_repo_path(&arg))
        .unwrap_or_else(default_fixture_dir)
}

pub fn run_args(
    mut args: impl Iterator<Item = String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(cmd) = args.next() else {
        print_usage();
        return Ok(());
    };

    match cmd.as_str() {
        "inspect" => inspect_dir(&next_dir(&mut args))?,
        "core-report" => print_core_report(&next_dir(&mut args))?,
        "core-validate" => validate_core_state(&next_dir(&mut args))?,
        "core-sync-counts" => sync_core_counts(&next_dir(&mut args))?,
        "core-sync-baseline" => sync_core_baseline(&next_dir(&mut args))?,
        "headers" => dump_headers(&next_dir(&mut args))?,
        "match" => match_fixture_set(&next_dir(&mut args))?,
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
            let Some((source, target)) =
                parse_optional_source_and_target(remaining, default_fixture_dir())
            else {
                print_usage();
                return Ok(());
            };
            initialize_dir(&source, &target)?;
        }
        "maintenance-days" => {
            let dir = next_dir(&mut args);
            match args.next().as_deref() {
                None => print_maintenance_days(&dir)?,
                Some("set") => {
                    let days = args.collect::<Vec<_>>();
                    set_maintenance_days(&dir, &days)?;
                }
                _ => print_usage(),
            }
        }
        "port-setup" => print_port_setup(&next_dir(&mut args))?,
        "flow-control" => {
            let dir = next_dir(&mut args);
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
            let dir = next_dir(&mut args);
            let Some(port_name) = args.next() else {
                print_usage();
                return Ok(());
            };
            match args.next() {
                None => print_com_irq(&dir, &port_name)?,
                Some(irq) => set_com_irq(&dir, &port_name, irq.parse::<u8>()?)?,
            }
        }
        "snoop" => {
            let dir = next_dir(&mut args);
            match args.next().as_deref() {
                None => print_snoop(&dir)?,
                Some("on") => set_snoop(&dir, true)?,
                Some("off") => set_snoop(&dir, false)?,
                _ => print_usage(),
            }
        }
        "local-timeout" => {
            let dir = next_dir(&mut args);
            match args.next().as_deref() {
                None => print_local_timeout(&dir)?,
                Some("on") => set_local_timeout(&dir, true)?,
                Some("off") => set_local_timeout(&dir, false)?,
                _ => print_usage(),
            }
        }
        "remote-timeout" => {
            let dir = next_dir(&mut args);
            match args.next().as_deref() {
                None => print_remote_timeout(&dir)?,
                Some("on") => set_remote_timeout(&dir, true)?,
                Some("off") => set_remote_timeout(&dir, false)?,
                _ => print_usage(),
            }
        }
        "max-key-gap" => {
            let dir = next_dir(&mut args);
            match args.next() {
                None => print_max_key_gap(&dir)?,
                Some(minutes) => set_max_key_gap(&dir, minutes.parse::<u8>()?)?,
            }
        }
        "minimum-time" => {
            let dir = next_dir(&mut args);
            match args.next() {
                None => print_minimum_time(&dir)?,
                Some(minutes) => set_minimum_time(&dir, minutes.parse::<u8>()?)?,
            }
        }
        "autopilot-after" => {
            let dir = next_dir(&mut args);
            match args.next() {
                None => print_autopilot_after(&dir)?,
                Some(turns) => set_autopilot_after(&dir, turns.parse::<u8>()?)?,
            }
        }
        "purge-after" => {
            let dir = next_dir(&mut args);
            match args.next() {
                None => print_purge_after(&dir)?,
                Some(turns) => set_purge_after(&dir, turns.parse::<u8>()?)?,
            }
        }
        "setup-programs" => print_setup_programs(&next_dir(&mut args))?,
        "fleet-order" => {
            let dir = next_dir(&mut args);
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
                aux0.as_deref()
                    .map(|value| parse_u8_arg(value, "aux0"))
                    .transpose()?,
                aux1.as_deref()
                    .map(|value| parse_u8_arg(value, "aux1"))
                    .transpose()?,
            )?;
        }
        "fleet-order-report" => {
            let dir = next_dir(&mut args);
            let record_index_arg = args.next();
            let record_index = record_index_arg.as_deref().unwrap_or("1");
            print_fleet_order_report(&dir, parse_usize_1_based(record_index, "fleet record index")?)?;
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
            let dir = next_dir(&mut args);
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
            let dir = next_dir(&mut args);
            let record_index_arg = args.next();
            let record_index = record_index_arg.as_deref().unwrap_or("15");
            print_planet_build_report(&dir, parse_usize_1_based(record_index, "planet record index")?)?;
        }
        "planet-build-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((target, record_index, slot_raw, kind_raw)) =
                parse_target_and_planet_spec(remaining)
            else {
                print_usage();
                return Ok(());
            };
            init_planet_build_scenario(&post_maint_fixture_dir(), &target, record_index, slot_raw, kind_raw)?;
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
            let dir = next_dir(&mut args);
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
        "guard-starbase-report" => print_guard_starbase_report(&next_dir(&mut args))?,
        "guard-starbase-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target, target_x, target_y)) =
                parse_optional_source_target_and_xy(remaining, post_maint_fixture_dir())
            else {
                print_usage();
                return Ok(());
            };
            init_guard_starbase_onebase(&source, &target, target_x, target_y)?;
        }
        "ipbm-report" => print_ipbm_report(&next_dir(&mut args))?,
        "ipbm-zero" => {
            let dir = next_dir(&mut args);
            let Some(count) = args.next() else {
                print_usage();
                return Ok(());
            };
            set_ipbm_zero_records(&dir, count.parse::<u16>()?)?;
        }
        "ipbm-record-set" => {
            let dir = next_dir(&mut args);
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
        "ipbm-validate" => validate_ipbm(next_dir(&mut args).as_path())?,
        "ipbm-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target, count)) =
                parse_optional_source_target_and_count(remaining, post_maint_fixture_dir())
            else {
                print_usage();
                return Ok(());
            };
            init_ipbm_zero_records(&source, &target, count)?;
        }
        "compliance-report" => print_compliance_report(&next_dir(&mut args))?,
        "guard-starbase-batch-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target_root, coords)) =
                parse_optional_source_target_and_coord_list(remaining, post_maint_fixture_dir())
            else {
                print_usage();
                return Ok(());
            };
            init_guard_starbase_batch(&source, &target_root, &coords)?;
        }
        "ipbm-batch-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target_root, counts)) =
                parse_optional_source_target_and_count_list(remaining, post_maint_fixture_dir())
            else {
                print_usage();
                return Ok(());
            };
            init_ipbm_batch(&source, &target_root, &counts)?;
        }
        "scenario" => {
            let dir = next_dir(&mut args);
            let selector = args.next();
            if selector.as_deref() == Some("list") {
                print_known_scenarios();
            } else if selector.as_deref() == Some("show") {
                match args.next().as_deref().and_then(KnownScenario::parse) {
                    Some(scenario) => print_known_scenario_details(scenario),
                    None => print_usage(),
                }
            } else if selector.as_deref() == Some("compose") {
                let Some(scenarios) = parse_known_scenarios(args.collect()) else {
                    print_usage();
                    return Ok(());
                };
                apply_known_scenarios(&dir, &scenarios)?;
            } else {
                match selector.as_deref().and_then(KnownScenario::parse) {
                    Some(scenario) => apply_known_scenario(&dir, scenario)?,
                    None => print_usage(),
                }
            }
        }
        "scenario-init-all" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target_root)) =
                parse_optional_source_and_target(remaining, post_maint_fixture_dir())
            else {
                print_usage();
                return Ok(());
            };
            init_all_known_scenarios(&source, &target_root)?;
        }
        "scenario-init" => {
            let remaining = args.collect::<Vec<_>>();
            let Some((source, target, scenario_name)) =
                parse_optional_source_target_and_name(remaining, post_maint_fixture_dir())
            else {
                print_usage();
                return Ok(());
            };
            match KnownScenario::parse(&scenario_name) {
                Some(scenario) => init_known_scenario(&source, &target, scenario)?,
                None => print_usage(),
            }
        }
        "scenario-init-compose" => {
            let Some((source, target, scenarios)) = parse_scenario_chain_init_args(args.collect()) else {
                print_usage();
                return Ok(());
            };
            init_known_scenario_chain(&source, &target, &scenarios)?;
        }
        "validate" => {
            let dir = next_dir(&mut args);
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
            let dir = next_dir(&mut args);
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
            let dir = next_dir(&mut args);
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

fn parse_known_scenarios(args: Vec<String>) -> Option<Vec<KnownScenario>> {
    if args.is_empty() {
        return None;
    }

    args.into_iter().map(|name| KnownScenario::parse(&name)).collect()
}

fn parse_scenario_chain_init_args(args: Vec<String>) -> Option<(PathBuf, PathBuf, Vec<KnownScenario>)> {
    match args.as_slice() {
        [target, scenario_names @ ..] if !scenario_names.is_empty() => Some((
            post_maint_fixture_dir(),
            resolve_repo_path(target),
            parse_known_scenarios(scenario_names.to_vec())?,
        )),
        [source, target, scenario_names @ ..] if !scenario_names.is_empty() => Some((
            resolve_repo_path(source),
            resolve_repo_path(target),
            parse_known_scenarios(scenario_names.to_vec())?,
        )),
        _ => None,
    }
}
