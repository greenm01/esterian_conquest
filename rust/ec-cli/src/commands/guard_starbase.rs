use std::fs;
use std::path::Path;

use ec_data::{BaseDat, BaseRecord, FleetDat, PlayerDat};

use crate::INIT_FILES;

pub(crate) fn apply_guard_starbase_scenario(
    dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    set_guard_starbase_onebase(dir, 0x10, 0x0D)?;
    println!("Applied scenario: guard-starbase");
    Ok(())
}

pub(crate) fn set_guard_starbase_onebase(
    dir: &Path,
    target_x: u8,
    target_y: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let player_path = dir.join("PLAYER.DAT");
    let mut player = PlayerDat::parse(&fs::read(&player_path)?)?;
    player.records[0].set_starbase_count_raw(1);
    fs::write(&player_path, player.to_bytes())?;

    let fleets_path = dir.join("FLEETS.DAT");
    let mut fleets = FleetDat::parse(&fs::read(&fleets_path)?)?;
    let fleet = fleets
        .records
        .get_mut(0)
        .ok_or("missing fleet record 1")?;
    fleet.set_standing_order_code_raw(0x04);
    fleet.set_standing_order_target_coords_raw([target_x, target_y]);
    fleet.set_mission_aux_bytes([0x01, 0x01]);
    let _ = fleet;
    fs::write(&fleets_path, fleets.to_bytes())?;

    let bases_path = dir.join("BASES.DAT");
    let bases = BaseDat {
        records: vec![build_guard_starbase_base_record(
            [target_x, target_y],
            0x01,
            0x0001,
            0x01,
        )],
    };
    fs::write(&bases_path, bases.to_bytes())?;

    println!("  PLAYER[1].starbase_count_raw = 1");
    println!("  FLEET[1].order = 0x04, aux = [01, 01]");
    println!(
        "  BASES.DAT = structured single-base record at ({}, {}) for empire 1",
        target_x, target_y
    );
    Ok(())
}

fn build_guard_starbase_base_record(
    coords: [u8; 2],
    base_id: u8,
    chain_word: u16,
    owner_empire: u8,
) -> BaseRecord {
    let mut record = BaseRecord::new_zeroed();
    record.set_local_slot_raw(base_id);
    record.set_active_flag_raw(0x01);
    record.set_base_id_raw(base_id);
    record.set_link_word_raw(0x0000);
    record.set_chain_word_raw(chain_word);
    record.set_coords_raw(coords);
    record.set_tuple_a_payload_raw([0x80, 0x00, 0x00, 0x00, 0x00]);
    record.set_tuple_b_payload_raw([0x80, 0x00, 0x00, 0x00, 0x00]);
    record.set_tuple_c_payload_raw([0x81, 0x00, 0x00, 0x00, 0x00]);
    record.set_trailing_coords_raw(coords);
    record.set_owner_empire_raw(owner_empire);
    record
}

pub(crate) fn validate_guard_starbase_scenario(
    dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let player = PlayerDat::parse(&fs::read(dir.join("PLAYER.DAT"))?)?;
    let fleets = FleetDat::parse(&fs::read(dir.join("FLEETS.DAT"))?)?;
    let bases = BaseDat::parse(&fs::read(dir.join("BASES.DAT"))?)?;

    let errors = guard_starbase_errors(&player, &fleets, &bases);

    if errors.is_empty() {
        let fleet = &fleets.records[0];
        let player1 = &player.records[0];
        let base = &bases.records[0];
        println!("Valid guard-starbase scenario");
        println!("  PLAYER[1].starbase_count_raw = 1");
        println!(
            "  linkage keys: player[44]={} fleet[00]={} fleet[05]={} base[07]={}",
            player1.starbase_count_raw(),
            fleet.local_slot_word_raw(),
            fleet.fleet_id_word_raw(),
            base.chain_word_raw()
        );
        println!(
            "  one-base guard-starbase linkage holds at coords {:?}",
            base.coords_raw()
        );
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}

pub(crate) fn guard_starbase_errors(
    player: &PlayerDat,
    fleets: &FleetDat,
    bases: &BaseDat,
) -> Vec<String> {
    let mut errors = Vec::new();

    match player.records.first() {
        Some(record) if record.starbase_count_raw() == 1 => {}
        Some(record) => errors.push(format!(
            "PLAYER[1].starbase_count_raw expected 1, got {}",
            record.starbase_count_raw()
        )),
        None => errors.push("PLAYER.DAT missing record 1".to_string()),
    }

    match fleets.records.first() {
        Some(record) => {
            if record.standing_order_code_raw() != 0x04 {
                errors.push(format!(
                    "FLEET[1].order expected 0x04, got {:#04x}",
                    record.standing_order_code_raw()
                ));
            }
            if record.guard_starbase_enable_raw() != 0x01 {
                errors.push(format!(
                    "FLEET[1].guard enable expected 0x01, got {:#04x}",
                    record.guard_starbase_enable_raw()
                ));
            }
            if record.guard_starbase_index_raw() == 0 {
                errors.push("FLEET[1].guard starbase index expected non-zero".to_string());
            }
        }
        None => errors.push("FLEETS.DAT missing record 1".to_string()),
    }

    let Some(fleet) = fleets.records.first() else {
        return errors;
    };
    let Some(player1) = player.records.first() else {
        return errors;
    };

    if bases.records.len() != 1 {
        errors.push(format!(
            "BASES.DAT expected 1 record, got {}",
            bases.records.len()
        ));
    } else {
        let base = &bases.records[0];
        if base.local_slot_raw() == 0 {
            errors.push("BASES[1].local_slot expected non-zero".to_string());
        }
        if base.active_flag_raw() != 0x01 {
            errors.push(format!(
                "BASES[1].active_flag expected 0x01, got {:#04x}",
                base.active_flag_raw()
            ));
        }
        if base.base_id_raw() != fleet.guard_starbase_index_raw() {
            errors.push(format!(
                "BASES[1].base_id expected FLEET[1].guard index {}, got {}",
                fleet.guard_starbase_index_raw(),
                base.base_id_raw()
            ));
        }
        if base.coords_raw() != fleet.standing_order_target_coords_raw() {
            errors.push(format!(
                "BASES[1].coords expected {:?}, got {:?}",
                fleet.standing_order_target_coords_raw(),
                base.coords_raw()
            ));
        }
        if base.trailing_coords_raw() != base.coords_raw() {
            errors.push(format!(
                "BASES[1].trailing coords expected {:?}, got {:?}",
                base.coords_raw(),
                base.trailing_coords_raw()
            ));
        }
        if base.chain_word_raw() != player1.starbase_count_raw() {
            errors.push(format!(
                "BASES[1].chain_word expected PLAYER[1].starbase_count_raw {}, got {}",
                player1.starbase_count_raw(),
                base.chain_word_raw()
            ));
        }
        if fleet.local_slot_word_raw() != player1.starbase_count_raw() {
            errors.push(format!(
                "FLEET[1].local slot word expected PLAYER[1].starbase_count_raw {}, got {}",
                player1.starbase_count_raw(),
                fleet.local_slot_word_raw()
            ));
        }
        if fleet.fleet_id_word_raw() != base.chain_word_raw() {
            errors.push(format!(
                "FLEET[1].fleet ID word expected BASES[1].chain_word {}, got {}",
                base.chain_word_raw(),
                fleet.fleet_id_word_raw()
            ));
        }
    }

    errors
}

pub(crate) fn print_guard_starbase_report(
    dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let player = PlayerDat::parse(&fs::read(dir.join("PLAYER.DAT"))?)?;
    let fleets = FleetDat::parse(&fs::read(dir.join("FLEETS.DAT"))?)?;
    let bases = BaseDat::parse(&fs::read(dir.join("BASES.DAT"))?)?;

    let player1 = player.records.first().ok_or("PLAYER.DAT missing record 1")?;
    let fleet1 = fleets.records.first().ok_or("FLEETS.DAT missing record 1")?;

    println!("Guard Starbase Report");
    println!("  dir={}", dir.display());
    println!("  player[1].fleet_chain_head_raw={}", player1.fleet_chain_head_raw());
    println!("  player[1].starbase_count_raw={}", player1.starbase_count_raw());
    println!("  fleet[1].local_slot_word_raw={}", fleet1.local_slot_word_raw());
    println!(
        "  fleet[1].next_fleet_link_word_raw={}",
        fleet1.next_fleet_link_word_raw()
    );
    println!("  fleet[1].fleet_id_word_raw={}", fleet1.fleet_id_word_raw());
    println!(
        "  fleet[1].order={:#04x} target={:?} guard_index={} guard_enable={}",
        fleet1.standing_order_code_raw(),
        fleet1.standing_order_target_coords_raw(),
        fleet1.guard_starbase_index_raw(),
        fleet1.guard_starbase_enable_raw()
    );

    if let Some(base1) = bases.records.first() {
        println!("  base_count={}", bases.records.len());
        println!(
            "  base[1].slot={} summary_word={} id={} link={:#06x} chain={:#06x} coords={:?} trailing={:?} owner={}",
            base1.local_slot_raw(),
            base1.summary_word_raw(),
            base1.base_id_raw(),
            base1.link_word_raw(),
            base1.chain_word_raw(),
            base1.coords_raw(),
            base1.trailing_coords_raw(),
            base1.owner_empire_raw()
        );
    } else {
        println!("  base_count=0");
    }

    match validate_guard_starbase_scenario(dir) {
        Ok(()) => println!("  verdict=valid one-base guard-starbase linkage"),
        Err(err) => println!("  verdict=invalid: {err}"),
    }

    Ok(())
}

pub(crate) fn init_guard_starbase_onebase(
    source: &Path,
    target: &Path,
    target_x: u8,
    target_y: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(target)?;
    for name in INIT_FILES {
        fs::copy(source.join(name), target.join(name))?;
    }
    set_guard_starbase_onebase(target, target_x, target_y)?;
    println!("Guard Starbase directory initialized at {}", target.display());
    Ok(())
}

pub(crate) fn init_guard_starbase_batch(
    source: &Path,
    target_root: &Path,
    coords: &[[u8; 2]],
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(target_root)?;
    let mut manifest = String::new();
    manifest.push_str("Guard Starbase batch\n");
    manifest.push_str(&format!("source={}\n", source.display()));
    manifest.push_str(&format!("target_root={}\n", target_root.display()));
    manifest.push('\n');

    for [x, y] in coords {
        let name = format!("x{:02}-y{:02}", x, y);
        let scenario_dir = target_root.join(&name);
        init_guard_starbase_onebase(source, &scenario_dir, *x, *y)?;
        manifest.push_str(&format!("{name}\n"));
        manifest.push_str(&format!("  target=[{}, {}]\n", x, y));
        manifest.push_str(&format!("  dir={}\n", scenario_dir.display()));
        manifest.push_str(&format!(
            "  validate=ec-cli compliance-report {}\n\n",
            scenario_dir.display()
        ));
    }

    fs::write(target_root.join("GUARD_STARBASES.txt"), manifest)?;
    println!(
        "Initialized {} Guard Starbase directories under {}",
        coords.len(),
        target_root.display()
    );
    Ok(())
}
