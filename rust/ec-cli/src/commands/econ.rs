use std::path::Path;

use ec_data::CoreGameData;

/// Apply the econ scenario to an already-initialized game directory.
///
/// Fixture-specific constants for this scenario:
/// - Fleet 3 (empire=1, slot=3, index=2): BombardWorld (0x06) order, speed=3/3,
///   target (15,13), BB=50, CA=50
/// - Planet 14 (index=13): set via direct raw byte assignment (Dust Bowl-type seeded world
///   at (15,13), owned by empire 2, armies=142, batteries=15)
///
/// Fleet 2 mutations are identical to the fleet-battle scenario's fleet 2 changes.
/// Planet 14 raw bytes are identical to the fleet-battle, bombard, and invade pre-fixtures.
///
/// All record indices and constants here are scenario-specific; the general mutators live in
/// ec-data and accept parameters.
pub(crate) fn apply_econ_scenario(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut data = CoreGameData::load(dir)?;

    // Fleet 2 (empire=1, slot=3): BombardWorld order targeting (15,13), speed=3/3, CA=50, DD=50
    {
        let f = &mut data.fleets.records[2];
        f.set_max_speed(3);
        f.set_current_speed(3);
        f.set_standing_order_code_raw(0x06); // BombardWorld
        f.set_standing_order_target_coords_raw([0x0f, 0x0d]); // (15,13)
        f.set_cruiser_count(50);
        f.set_destroyer_count(50);
    }

    // Planet 14 (index 13): Dust Bowl-type target world at (15,13), owned by empire 2,
    // armies=142, batteries=15, named "TargetPrime" (name buffer retains stale "et" suffix).
    // Identical layout to the fleet-battle, bombard, and invade pre-fixtures.
    {
        let p = data
            .planets
            .records
            .get_mut(13)
            .ok_or("planet record 14 missing")?;
        p.set_as_owned_target_world(
            [0x0f, 0x0d],                               // coords (15,13)
            [0x64, 0x87],                               // potential_production
            [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],       // factories
            0x04,                                       // tax_rate
            0x0b,                                       // name_len = 11
            *b"TargetPrimeet",                          // name_buffer (stale "et" suffix)
            [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05], // name_suffix_raw [1d..23]
            0x8e,                                       // army_count = 142
            0x0f,                                       // ground_batteries = 15
            0x02,                                       // ownership_status
            0x02,                                       // owner_empire_slot
        );
    }

    data.save(dir)?;
    println!("Applied scenario: econ");
    Ok(())
}

pub(crate) fn validate_econ_data(data: &CoreGameData) -> Result<(), Box<dyn std::error::Error>> {
    let mut errors = Vec::new();

    // Fleet 2 (empire=1, slot=3): BombardWorld, speed=3/3, target (15,13), BB=50, CA=50
    match data.fleets.records.get(2) {
        None => errors.push("missing fleet record 3".to_string()),
        Some(f) => {
            if f.max_speed() != 3 {
                errors.push(format!(
                    "FLEET[3].max_speed expected 3, got {}",
                    f.max_speed()
                ));
            }
            if f.current_speed() != 3 {
                errors.push(format!(
                    "FLEET[3].current_speed expected 3, got {}",
                    f.current_speed()
                ));
            }
            if f.standing_order_code_raw() != 0x06 {
                errors.push(format!(
                    "FLEET[3].order expected 0x06 (BombardWorld), got {:#04x}",
                    f.standing_order_code_raw()
                ));
            }
            if f.standing_order_target_coords_raw() != [0x0f, 0x0d] {
                errors.push(format!(
                    "FLEET[3].target expected (15,13), got {:?}",
                    f.standing_order_target_coords_raw()
                ));
            }
            if f.cruiser_count() != 50 {
                errors.push(format!(
                    "FLEET[3].ca expected 50, got {}",
                    f.cruiser_count()
                ));
            }
            if f.destroyer_count() != 50 {
                errors.push(format!(
                    "FLEET[3].dd expected 50, got {}",
                    f.destroyer_count()
                ));
            }
        }
    }

    // Planet 14 (index 13): Dust Bowl-type world at (15,13), owned empire=2, armies=142, batteries=15
    match data.planets.records.get(13) {
        None => errors.push("missing planet record 14".to_string()),
        Some(p) => {
            if p.coords_raw() != [0x0f, 0x0d] {
                errors.push(format!(
                    "PLANET[14].coords expected (15,13), got {:?}",
                    p.coords_raw()
                ));
            }
            if p.army_count_raw() != 142 {
                errors.push(format!(
                    "PLANET[14].armies expected 142, got {}",
                    p.army_count_raw()
                ));
            }
            if p.ground_batteries_raw() != 15 {
                errors.push(format!(
                    "PLANET[14].batteries expected 15, got {}",
                    p.ground_batteries_raw()
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
                    "PLANET[14].owner_empire expected 2, got {}",
                    p.owner_empire_slot_raw()
                ));
            }
        }
    }

    if errors.is_empty() {
        println!("Valid econ scenario");
        println!("  FLEET[3]: order=0x06 (BombardWorld) tgt=(15,13) speed=3/3 CA=50 DD=50");
        println!("  PLANET[14]: (15,13) empire=2 armies=142 batteries=15");
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}
