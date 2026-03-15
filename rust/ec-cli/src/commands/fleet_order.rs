use std::fs;
use std::path::Path;

use ec_data::CoreGameData;

use crate::commands::runtime::with_runtime_game_mut_and_export;
use crate::workspace::copy_init_files;

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
    let final_aux = with_runtime_game_mut_and_export(dir, |data| {
        data.set_fleet_order(
            record_index_1_based,
            speed,
            order_code,
            [target_x, target_y],
            aux0,
            aux1,
        )
        .map_err(|err| err.to_string().into())
    })?;

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

pub(crate) fn apply_move_scenario(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    set_fleet_order(dir, 1, 0x03, 0x01, 0x1A, 0x0D, None, None)?;
    println!("Applied scenario: move");
    Ok(())
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
    println!(
        "  next_fleet_link_word={}",
        record.next_fleet_link_word_raw()
    );
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
    copy_init_files(source, target)?;
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
