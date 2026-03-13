use std::path::Path;

use ec_data::CoreGameData;

/// Apply the bombard scenario to an already-initialized game directory.
///
/// Fixture-specific constants for this scenario:
/// - Fleet 3 (player 1, slot 3): BombardWorld order (0x06), speed 3, max_speed 3,
///   targeting planet 14 at (15, 13)
/// - Fleet 3 raw bytes [0x28]=0x03 and [0x2a]=0x05 (observed linkage words in preserved fixture)
/// - Planet 14: seeded as an owned planet for empire 2 by cloning planet 13's raw layout
///   and changing only the x-coordinate to 15 (0x0F)
///
/// All record indices and constants here are scenario-specific; the general mutators
/// (set_fleet_order, etc.) live in ec-data and accept parameters.
pub(crate) fn apply_bombard_scenario(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut data = CoreGameData::load(dir)?;

    // Fleet 3: set speed, order, target via the general mutator
    data.set_fleet_order(3, 3, 0x06, [0x0F, 0x0D], None, None)
        .map_err(|e| e.to_string())?;

    // Fleet 3: max_speed and raw linkage bytes observed in preserved fixture
    {
        let fleet3 = &mut data.fleets.records[2];
        fleet3.set_max_speed(3);
        fleet3.raw[0x28] = 0x03;
        fleet3.raw[0x2a] = 0x05;
    }

    // Planet 14: clone planet 13's raw record (empire 2 homeworld), change x-coord to 15
    {
        let template = data
            .planets
            .records
            .get(12)
            .ok_or("planet record 13 missing")?
            .raw;
        let p14 = data
            .planets
            .records
            .get_mut(13)
            .ok_or("planet record 14 missing")?;
        p14.raw = template;
        p14.raw[0x00] = 0x0F; // x = 15
        p14.raw[0x01] = 0x0D; // y = 13 (already correct from template, set explicitly)
    }

    data.save(dir)?;
    println!("Applied scenario: bombard");
    Ok(())
}

pub(crate) fn validate_bombard_data(data: &CoreGameData) -> Result<(), Box<dyn std::error::Error>> {
    let mut errors = Vec::new();

    // Fleet 3: BombardWorld order targeting (15,13), speed 3, max_speed 3
    match data.fleets.records.get(2) {
        None => errors.push("missing fleet record 3".to_string()),
        Some(fleet3) => {
            if fleet3.max_speed() != 3 {
                errors.push(format!(
                    "FLEET[3].max_speed expected 3, got {}",
                    fleet3.max_speed()
                ));
            }
            if fleet3.current_speed() != 3 {
                errors.push(format!(
                    "FLEET[3].current_speed expected 3, got {}",
                    fleet3.current_speed()
                ));
            }
            if fleet3.standing_order_code_raw() != 0x06 {
                errors.push(format!(
                    "FLEET[3].order expected 0x06 (bombard), got {:#04x}",
                    fleet3.standing_order_code_raw()
                ));
            }
            if fleet3.standing_order_target_coords_raw() != [0x0F, 0x0D] {
                errors.push(format!(
                    "FLEET[3].target expected (15,13), got {:?}",
                    fleet3.standing_order_target_coords_raw()
                ));
            }
        }
    }

    // Planet 14: owned by empire 2, at (15,13), with armies and batteries
    match data.planets.records.get(13) {
        None => errors.push("missing planet record 14".to_string()),
        Some(p) => {
            if p.coords_raw() != [0x0F, 0x0D] {
                errors.push(format!(
                    "PLANET[14].coords expected (15,13), got {:?}",
                    p.coords_raw()
                ));
            }
            if p.ownership_status_raw() != 2 {
                errors.push(format!(
                    "PLANET[14].ownership_status expected 2, got {}",
                    p.ownership_status_raw()
                ));
            }
            if p.owner_empire_slot_raw() != 2 {
                errors.push(format!(
                    "PLANET[14].owner_empire_slot expected 2, got {}",
                    p.owner_empire_slot_raw()
                ));
            }
            if p.army_count_raw() != 10 {
                errors.push(format!(
                    "PLANET[14].army_count expected 10, got {}",
                    p.army_count_raw()
                ));
            }
            if p.ground_batteries_raw() != 4 {
                errors.push(format!(
                    "PLANET[14].ground_batteries expected 4, got {}",
                    p.ground_batteries_raw()
                ));
            }
        }
    }

    if errors.is_empty() {
        println!("Valid bombard scenario");
        println!("  FLEET[3].max_speed = 3");
        println!("  FLEET[3].order = 0x06 (BombardWorld)");
        println!("  FLEET[3].target = (15, 13)");
        println!("  PLANET[14]: homeworld seed empire=2 armies=10 batteries=4");
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}
