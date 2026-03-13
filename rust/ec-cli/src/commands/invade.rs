use std::path::Path;

use ec_data::CoreGameData;

/// Apply the invade scenario to an already-initialized game directory.
///
/// Fixture-specific constants for this scenario:
/// - Fleet 3 (empire=1, slot=3, index=2): InvadeWorld (0x0a) order, speed=3/3,
///   target (15,13), invasion_army_count=100, SC=100, BB=100, CA=50, DD=50, TT=50
/// - Planet 14 (index=13): set via set_as_owned_target_world (Dust Bowl-type seeded world
///   at (15,13), owned by empire 2, armies=142, batteries=15)
///
/// Order code 0x0a is empirically confirmed as InvadeWorld from fixture analysis.
/// The Rust fleet enum labels this code differently (guessed from docs); use
/// set_standing_order_code_raw directly until the enum is corrected.
///
/// All record indices and constants here are scenario-specific; the general mutators live in
/// ec-data and accept parameters.
pub(crate) fn apply_invade_scenario(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut data = CoreGameData::load(dir)?;

    // Fleet 2 (empire=1, slot=3): InvadeWorld order targeting (15,13), speed=3/3,
    // army_count=100, SC=100, BB=100, CA=50, DD=50, TT=50
    {
        let f = &mut data.fleets.records[2];
        f.set_invasion_army_count_raw(0x64); // 100 armies loaded for invasion
        f.set_max_speed(3);
        f.set_current_speed(3);
        f.set_standing_order_code_raw(0x0a); // InvadeWorld (empirically confirmed)
        f.set_standing_order_target_coords_raw([0x0f, 0x0d]); // (15,13)
        f.set_scout_count(100);
        f.set_battleship_count(100);
        f.set_cruiser_count(50);
        f.set_destroyer_count(50);
        f.set_troop_transport_count(50);
    }

    // Planet 14 (index 13): Dust Bowl-type target world at (15,13), owned by empire 2,
    // armies=142, batteries=15, named "TargetPrime" (name buffer retains stale "et" suffix).
    // Identical layout to the fleet-battle and econ pre-fixtures.
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
    println!("Applied scenario: invade");
    Ok(())
}

pub(crate) fn validate_invade_data(data: &CoreGameData) -> Result<(), Box<dyn std::error::Error>> {
    let mut errors = Vec::new();

    // Fleet 2 (empire=1, slot=3): InvadeWorld, speed=3/3, target (15,13),
    // army_count=100, SC=100, BB=100, CA=50, DD=50, TT=50
    match data.fleets.records.get(2) {
        None => errors.push("missing fleet record 3".to_string()),
        Some(f) => {
            if f.invasion_army_count_raw() != 0x64 {
                errors.push(format!(
                    "FLEET[3].invasion_army_count expected 100 (0x64), got {:#04x}",
                    f.invasion_army_count_raw()
                ));
            }
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
            if f.standing_order_code_raw() != 0x0a {
                errors.push(format!(
                    "FLEET[3].order expected 0x0a (InvadeWorld), got {:#04x}",
                    f.standing_order_code_raw()
                ));
            }
            if f.standing_order_target_coords_raw() != [0x0f, 0x0d] {
                errors.push(format!(
                    "FLEET[3].target expected (15,13), got {:?}",
                    f.standing_order_target_coords_raw()
                ));
            }
            if f.scout_count() != 100 {
                errors.push(format!("FLEET[3].sc expected 100, got {}", f.scout_count()));
            }
            if f.battleship_count() != 100 {
                errors.push(format!(
                    "FLEET[3].bb expected 100, got {}",
                    f.battleship_count()
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
            if f.troop_transport_count() != 50 {
                errors.push(format!(
                    "FLEET[3].tt expected 50, got {}",
                    f.troop_transport_count()
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
        println!("Valid invade scenario");
        println!("  FLEET[3]: order=0x0a (InvadeWorld) tgt=(15,13) speed=3/3 army=100 SC=100 BB=100 CA=50 DD=50 TT=50");
        println!("  PLANET[14]: (15,13) empire=2 armies=142 batteries=15");
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}
