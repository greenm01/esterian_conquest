use std::fs;
use std::path::Path;

use ec_data::{ConquestDat, CoreGameData, SetupDat};

pub(crate) fn inspect_dir(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = CoreGameData::load(dir)?;

    println!("Directory: {}", dir.display());
    print_header_summary(&data.setup, &data.conquest);
    println!();

    println!("Players:");
    for (idx, record) in data.player.records.iter().enumerate() {
        println!(
            "  slot {}: owner_mode={} assigned_player_flag={} tax={} stored_prod_pts={} autopilot={} summary={}",
            idx + 1,
            record.owner_mode_raw(),
            record.assigned_player_flag_raw(),
            record.tax_rate(),
            record.stored_production_pts_raw(),
            record.autopilot_flag(),
            record.ownership_summary()
        );
        println!("    starbase_count_raw={}", record.starbase_count_raw());
    }
    println!();

    println!("Planets:");
    for (idx, record) in data.planets.records.iter().enumerate().take(5) {
        println!(
            "  planet {:02}: coords={:?} hdr={:02x?} len={} text='{}' tail58={:02x?} fact_word={:04x} summary='{}'",
            idx + 1,
            record.coords_raw(),
            record.header_bytes(),
            record.string_len(),
            record.status_or_name_summary(),
            &record.raw[0x58..0x61],
            record.factories_word_raw(),
            record.derived_summary()
        );
    }
    println!("  ... {} total planet records", data.planets.records.len());

    let homeworld_like = data
        .planets
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

    println!();
    println!("Fleets:");
    let fleet_display_count = data.fleets.records.len().min(16);
    for (idx, record) in data
        .fleets
        .records
        .iter()
        .enumerate()
        .take(fleet_display_count)
    {
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
    if data.fleets.records.len() > fleet_display_count {
        println!("  ... {} total fleet records", data.fleets.records.len());
    }

    let looks_like_initialized_blocks = !data.fleets.records.is_empty()
        && data.fleets.records.len() % 4 == 0
        && data
            .fleets
            .records
            .chunks_exact(4)
            .all(|group| group.iter().map(|r| r.local_slot()).eq([1, 2, 3, 4]));

    if looks_like_initialized_blocks {
        println!();
        println!("Fleet Groups:");
        for (group_idx, group) in data.fleets.records.chunks_exact(4).enumerate() {
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

    println!();
    println!("Bases:");
    for (idx, record) in data.bases.records.iter().enumerate() {
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

    println!();
    println!("IPBM:");
    for (idx, record) in data.ipbm.records.iter().enumerate() {
        println!(
            "  record {:02}: primary={} owner={} gate={} follow_on={}",
            idx + 1,
            record.primary_word_raw(),
            record.owner_empire_raw(),
            record.gate_word_raw(),
            record.follow_on_word_raw()
        );
    }

    Ok(())
}

pub(crate) fn dump_headers(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    let conquest = ConquestDat::parse(&fs::read(dir.join("CONQUEST.DAT"))?)?;

    println!("Directory: {}", dir.display());
    println!(
        "SETUP.version={}",
        String::from_utf8_lossy(setup.version_tag())
    );
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
    println!(
        "SETUP.purge_after_turns_raw={}",
        setup.purge_after_turns_raw()
    );
    println!(
        "SETUP.autopilot_inactive_turns_raw={}",
        setup.autopilot_inactive_turns_raw()
    );
    println!("CONQUEST.game_year={}", conquest.game_year());
    println!("CONQUEST.player_count={}", conquest.player_count());
    println!(
        "CONQUEST.player_config_word={:04x}",
        conquest.player_config_word()
    );
    println!(
        "CONQUEST.maintenance_schedule={:02x?}",
        conquest.maintenance_schedule_bytes()
    );
    println!("CONQUEST.header_len={}", conquest.control_header().len());
    println!("CONQUEST.header_words={:04x?}", conquest.header_words());

    Ok(())
}

fn print_header_summary(setup: &SetupDat, conquest: &ConquestDat) {
    println!(
        "SETUP version: {}",
        String::from_utf8_lossy(setup.version_tag())
    );
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
    println!(
        "SETUP snoop enabled: {}",
        if setup.snoop_enabled() { "yes" } else { "no" }
    );
    println!(
        "SETUP local timeout enabled: {}",
        if setup.local_timeout_enabled() {
            "yes"
        } else {
            "no"
        }
    );
    println!(
        "SETUP remote timeout enabled: {}",
        if setup.remote_timeout_enabled() {
            "yes"
        } else {
            "no"
        }
    );
    println!(
        "SETUP max time between keys (raw minutes): {}",
        setup.max_time_between_keys_minutes_raw()
    );
    println!(
        "SETUP minimum time granted (raw minutes): {}",
        setup.minimum_time_granted_minutes_raw()
    );
    println!(
        "SETUP purge after turns (raw): {}",
        setup.purge_after_turns_raw()
    );
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

fn yes_no(value: bool) -> &'static str {
    if value {
        "Yes"
    } else {
        "No"
    }
}
