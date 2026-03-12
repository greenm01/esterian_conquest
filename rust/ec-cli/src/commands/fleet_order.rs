use std::fs;
use std::path::Path;

use ec_data::{CoreGameData, FleetDat};

use crate::INIT_FILES;

pub(crate) fn set_fleet_order(
    dir: &Path,
    record_index_1_based: usize,
    speed: u8,
    order_code: u8,
    target_x: u8,
    target_y: u8,
    aux0: Option<u8>,
    aux1: Option<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut data = CoreGameData::load(dir)?;
    let record = data
        .fleets
        .records
        .get_mut(record_index_1_based - 1)
        .ok_or_else(|| format!("fleet record index out of range: {record_index_1_based}"))?;
    record.set_current_speed(speed);
    record.set_standing_order_code_raw(order_code);
    record.set_standing_order_target_coords_raw([target_x, target_y]);
    let mut mission_aux = record.mission_aux_bytes();
    if let Some(value) = aux0 {
        mission_aux[0] = value;
    }
    if let Some(value) = aux1 {
        mission_aux[1] = value;
    }
    record.set_mission_aux_bytes(mission_aux);
    let final_aux = record.mission_aux_bytes();
    let _ = record;
    data.save(dir)?;

    println!(
        "Fleet record {} updated: speed={} order={:#04x} target=({}, {}) aux={:02x?}",
        record_index_1_based, speed, order_code, target_x, target_y, final_aux
    );
    Ok(())
}

pub(crate) fn apply_fleet_order_scenario(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    set_fleet_order(dir, 1, 0x03, 0x0C, 0x0F, 0x0D, None, None)?;
    println!("Applied scenario: fleet-order");
    Ok(())
}

pub(crate) fn fleet_order_errors(
    fleets: &FleetDat,
    record_index_1_based: usize,
    speed: u8,
    order_code: u8,
    target: [u8; 2],
    aux0: Option<u8>,
    aux1: Option<u8>,
) -> Vec<String> {
    let mut errors = Vec::new();
    match fleets.records.get(record_index_1_based - 1) {
        Some(record) => {
            if record.current_speed() != speed {
                errors.push(format!(
                    "FLEET[{}].current_speed expected {}, got {}",
                    record_index_1_based,
                    speed,
                    record.current_speed()
                ));
            }
            if record.standing_order_code_raw() != order_code {
                errors.push(format!(
                    "FLEET[{}].order expected {:#04x}, got {:#04x}",
                    record_index_1_based,
                    order_code,
                    record.standing_order_code_raw()
                ));
            }
            if record.standing_order_target_coords_raw() != target {
                errors.push(format!(
                    "FLEET[{}].target expected ({}, {}), got {:?}",
                    record_index_1_based,
                    target[0],
                    target[1],
                    record.standing_order_target_coords_raw()
                ));
            }
            let mission_aux = record.mission_aux_bytes();
            if let Some(value) = aux0 {
                if mission_aux[0] != value {
                    errors.push(format!(
                        "FLEET[{}].aux0 expected {:#04x}, got {:#04x}",
                        record_index_1_based, value, mission_aux[0]
                    ));
                }
            }
            if let Some(value) = aux1 {
                if mission_aux[1] != value {
                    errors.push(format!(
                        "FLEET[{}].aux1 expected {:#04x}, got {:#04x}",
                        record_index_1_based, value, mission_aux[1]
                    ));
                }
            }
        }
        None => errors.push(format!("FLEETS.DAT missing record {record_index_1_based}")),
    }
    errors
}

pub(crate) fn validate_fleet_order_scenario(
    dir: &Path,
    record_index_1_based: usize,
    speed: u8,
    order_code: u8,
    target_x: u8,
    target_y: u8,
    aux0: Option<u8>,
    aux1: Option<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = CoreGameData::load(dir)?;
    let errors = fleet_order_errors(
        &data.fleets,
        record_index_1_based,
        speed,
        order_code,
        [target_x, target_y],
        aux0,
        aux1,
    );
    if errors.is_empty() {
        println!("Valid fleet-order scenario");
        println!("  FLEET[{}].speed = {}", record_index_1_based, speed);
        println!("  FLEET[{}].order = {:#04x}", record_index_1_based, order_code);
        println!(
            "  FLEET[{}].target = ({}, {})",
            record_index_1_based, target_x, target_y
        );
        println!(
            "  FLEET[{}].aux = {:02x?}",
            record_index_1_based,
            data.fleets.records[record_index_1_based - 1].mission_aux_bytes()
        );
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}

pub(crate) fn print_fleet_order_report(
    dir: &Path,
    record_index_1_based: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = CoreGameData::load(dir)?;
    let record = data
        .fleets
        .records
        .get(record_index_1_based - 1)
        .ok_or_else(|| format!("fleet record index out of range: {record_index_1_based}"))?;
    println!("Fleet Order Report");
    println!("  dir={}", dir.display());
    println!("  record={}", record_index_1_based);
    println!("  current_speed={}", record.current_speed());
    println!("  order={:#04x}", record.standing_order_code_raw());
    println!("  target={:?}", record.standing_order_target_coords_raw());
    println!("  mission_aux={:02x?}", record.mission_aux_bytes());
    println!("  local_slot_word={}", record.local_slot_word_raw());
    println!("  next_fleet_link_word={}", record.next_fleet_link_word_raw());
    println!("  fleet_id_word={}", record.fleet_id_word_raw());
    Ok(())
}

pub(crate) fn init_fleet_order_scenario(
    source: &Path,
    target: &Path,
    record_index_1_based: usize,
    speed: u8,
    order_code: u8,
    target_x: u8,
    target_y: u8,
    aux0: Option<u8>,
    aux1: Option<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(target)?;
    for name in INIT_FILES {
        fs::copy(source.join(name), target.join(name))?;
    }
    set_fleet_order(
        target,
        record_index_1_based,
        speed,
        order_code,
        target_x,
        target_y,
        aux0,
        aux1,
    )?;
    println!("Fleet-order directory initialized at {}", target.display());
    Ok(())
}

pub(crate) fn init_fleet_order_batch(
    source: &Path,
    target_root: &Path,
    specs: &[(usize, u8, u8, u8, u8, Option<u8>, Option<u8>)],
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(target_root)?;
    let mut manifest = String::new();
    manifest.push_str("Fleet-order batch\n");
    manifest.push_str(&format!("source={}\n", source.display()));
    manifest.push_str(&format!("target_root={}\n", target_root.display()));
    manifest.push('\n');

    for (record_index, speed, order_code, target_x, target_y, aux0, aux1) in specs {
        let name = format!(
            "r{:02}-s{:02}-o{:02x}-x{:02}-y{:02}",
            record_index, speed, order_code, target_x, target_y
        );
        let scenario_dir = target_root.join(&name);
        init_fleet_order_scenario(
            source,
            &scenario_dir,
            *record_index,
            *speed,
            *order_code,
            *target_x,
            *target_y,
            *aux0,
            *aux1,
        )?;
        manifest.push_str(&format!("{name}\n"));
        manifest.push_str(&format!(
            "  spec={}:{:#04x}:{:#04x}:{}:{}",
            record_index, speed, order_code, target_x, target_y
        ));
        if let Some(value) = aux0 {
            manifest.push_str(&format!(":{:#04x}", value));
        }
        if let Some(value) = aux1 {
            if aux0.is_none() {
                manifest.push_str(":--");
            }
            manifest.push_str(&format!(":{:#04x}", value));
        }
        manifest.push('\n');
        manifest.push_str(&format!("  dir={}\n", scenario_dir.display()));
        manifest.push_str(&format!(
            "  validate=ec-cli validate {} fleet-order\n\n",
            scenario_dir.display()
        ));
    }

    fs::write(target_root.join("FLEET_ORDERS.txt"), manifest)?;
    println!(
        "Initialized {} fleet-order directories under {}",
        specs.len(),
        target_root.display()
    );
    Ok(())
}
