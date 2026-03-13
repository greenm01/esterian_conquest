use std::path::Path;

use ec_data::CoreGameData;

/// Apply the fleet-battle scenario to an already-initialized game directory.
///
/// Fixture-specific constants for this scenario:
/// - Fleet 1 (empire=1, slot=1, index=0): moved to (10,10) with RoE=100, BB=50, CA=50, DD=50
/// - Fleet 3 (empire=1, slot=3, index=2): BombardWorld (0x06) order, speed=3/3, target (15,13),
///   CA=50, DD=50 (in addition to existing CA=1 for a total of 50 cruisers)
/// - Fleet 5 (empire=2, slot=1, index=4): Patrol (0x03) order at (10,10), speed=0/6,
///   SC=10, BB=100, CA=0
/// - Fleet 9 (empire=3, slot=1, index=8): MoveOnly (0x01) to (10,10), speed=3/6,
///   loc=(9,10), SC=10, BB=1, CA=0
/// - Planet 14 (index=13): set via direct raw byte assignment (Dust Bowl-type seeded world
///   at (15,13), owned by empire 2, armies=142, batteries=15)
///
/// All record indices and constants here are scenario-specific; the general mutators live in
/// ec-data and accept parameters.
pub(crate) fn apply_fleet_battle_scenario(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut data = CoreGameData::load(dir)?;

    // Fleet 0 (empire=1, slot=1): move to (10,10), RoE=100, BB=50, CA=50, DD=50
    {
        let f = &mut data.fleets.records[0];
        f.set_current_location_coords_raw([0x0a, 0x0a]);
        f.set_standing_order_target_coords_raw([0x0a, 0x0a]);
        f.set_rules_of_engagement(0x64);
        f.set_battleship_count(50);
        f.set_cruiser_count(50);
        f.set_destroyer_count(50);
    }

    // Fleet 2 (empire=1, slot=3): BombardWorld order targeting (15,13), speed=3/3, CA=50, DD=50
    {
        let f = &mut data.fleets.records[2];
        f.set_max_speed(3);
        f.set_current_speed(3);
        f.set_standing_order_code_raw(0x06); // BombardWorld
        f.set_standing_order_target_coords_raw([0x0f, 0x0d]);
        f.set_cruiser_count(50);
        f.set_destroyer_count(50);
    }

    // Fleet 4 (empire=2, slot=1): Patrol (0x03) at (10,10), speed=0/6, SC=10, BB=100, CA=0
    {
        let f = &mut data.fleets.records[4];
        f.set_max_speed(6);
        f.set_current_location_coords_raw([0x0a, 0x0a]);
        f.set_standing_order_code_raw(0x03); // PatrolSector
        f.set_standing_order_target_coords_raw([0x0a, 0x0a]);
        f.set_scout_count(10);
        f.set_battleship_count(100);
        f.set_cruiser_count(0);
    }

    // Fleet 8 (empire=3, slot=1): MoveOnly (0x01) to (10,10), speed=3/6, loc=(9,10),
    // SC=10, BB=1, CA=0
    {
        let f = &mut data.fleets.records[8];
        f.set_max_speed(6);
        f.set_current_speed(3);
        f.set_current_location_coords_raw([0x09, 0x0a]);
        f.set_standing_order_code_raw(0x01); // MoveOnly
        f.set_standing_order_target_coords_raw([0x0a, 0x0a]);
        f.set_scout_count(10);
        f.set_battleship_count(1);
        f.set_cruiser_count(0);
    }

    // Planet 14 (index 13): direct raw assignment — Dust Bowl-type world at (15,13)
    // owned by empire 2, armies=142, batteries=15, named "TargetPrime" (with stale suffix bytes)
    {
        let p = data
            .planets
            .records
            .get_mut(13)
            .ok_or("planet record 14 missing")?;
        p.raw = [
            0x0f, 0x0d, 0x64, 0x87, 0x00, 0x00, 0x00, 0x00, // [00..07]
            0x48, 0x87, 0x00, 0x00, 0x00, 0x00, 0x04, 0x0b, // [08..0f]
            0x54, 0x61, 0x72, 0x67, 0x65, 0x74, 0x50, 0x72, // [10..17] "TargetPr"
            0x69, 0x6d, 0x65, 0x65, 0x74, 0x05, 0x1d, 0x0b, // [18..1f] "imeet\x05\x1d\x0b"
            0x11, 0x25, 0x1c, 0x05, 0x00, 0x00, 0x00, 0x00, // [20..27]
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // [28..2f]
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // [30..37]
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // [38..3f]
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // [40..47]
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // [48..4f]
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // [50..57]
            0x8e, 0x00, 0x0f, 0x00, 0x02, 0x02, 0x00, 0x00, // [58..5f]
            0x00, // [60]
        ];
    }

    data.save(dir)?;
    println!("Applied scenario: fleet-battle");
    Ok(())
}

pub(crate) fn validate_fleet_battle_data(
    data: &CoreGameData,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut errors = Vec::new();

    // Fleet 0 (empire=1, slot=1): at (10,10), RoE=100, BB=50, CA=50, DD=50
    match data.fleets.records.get(0) {
        None => errors.push("missing fleet record 1".to_string()),
        Some(f) => {
            if f.current_location_coords_raw() != [0x0a, 0x0a] {
                errors.push(format!(
                    "FLEET[1].location expected (10,10), got {:?}",
                    f.current_location_coords_raw()
                ));
            }
            if f.standing_order_target_coords_raw() != [0x0a, 0x0a] {
                errors.push(format!(
                    "FLEET[1].target expected (10,10), got {:?}",
                    f.standing_order_target_coords_raw()
                ));
            }
            if f.rules_of_engagement() != 0x64 {
                errors.push(format!(
                    "FLEET[1].roe expected 100 (0x64), got {}",
                    f.rules_of_engagement()
                ));
            }
            if f.battleship_count() != 50 {
                errors.push(format!(
                    "FLEET[1].bb expected 50, got {}",
                    f.battleship_count()
                ));
            }
            if f.cruiser_count() != 50 {
                errors.push(format!(
                    "FLEET[1].ca expected 50, got {}",
                    f.cruiser_count()
                ));
            }
            if f.destroyer_count() != 50 {
                errors.push(format!(
                    "FLEET[1].dd expected 50, got {}",
                    f.destroyer_count()
                ));
            }
        }
    }

    // Fleet 2 (empire=1, slot=3): BombardWorld, speed=3/3, target (15,13), CA=50, DD=50
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

    // Fleet 4 (empire=2, slot=1): Patrol at (10,10), speed=0/6, SC=10, BB=100, CA=0
    match data.fleets.records.get(4) {
        None => errors.push("missing fleet record 5".to_string()),
        Some(f) => {
            if f.max_speed() != 6 {
                errors.push(format!(
                    "FLEET[5].max_speed expected 6, got {}",
                    f.max_speed()
                ));
            }
            if f.current_location_coords_raw() != [0x0a, 0x0a] {
                errors.push(format!(
                    "FLEET[5].location expected (10,10), got {:?}",
                    f.current_location_coords_raw()
                ));
            }
            if f.standing_order_code_raw() != 0x03 {
                errors.push(format!(
                    "FLEET[5].order expected 0x03 (PatrolSector), got {:#04x}",
                    f.standing_order_code_raw()
                ));
            }
            if f.standing_order_target_coords_raw() != [0x0a, 0x0a] {
                errors.push(format!(
                    "FLEET[5].target expected (10,10), got {:?}",
                    f.standing_order_target_coords_raw()
                ));
            }
            if f.scout_count() != 10 {
                errors.push(format!("FLEET[5].sc expected 10, got {}", f.scout_count()));
            }
            if f.battleship_count() != 100 {
                errors.push(format!(
                    "FLEET[5].bb expected 100, got {}",
                    f.battleship_count()
                ));
            }
            if f.cruiser_count() != 0 {
                errors.push(format!("FLEET[5].ca expected 0, got {}", f.cruiser_count()));
            }
        }
    }

    // Fleet 8 (empire=3, slot=1): MoveOnly to (10,10), speed=3/6, loc=(9,10), SC=10, BB=1, CA=0
    match data.fleets.records.get(8) {
        None => errors.push("missing fleet record 9".to_string()),
        Some(f) => {
            if f.max_speed() != 6 {
                errors.push(format!(
                    "FLEET[9].max_speed expected 6, got {}",
                    f.max_speed()
                ));
            }
            if f.current_speed() != 3 {
                errors.push(format!(
                    "FLEET[9].current_speed expected 3, got {}",
                    f.current_speed()
                ));
            }
            if f.current_location_coords_raw() != [0x09, 0x0a] {
                errors.push(format!(
                    "FLEET[9].location expected (9,10), got {:?}",
                    f.current_location_coords_raw()
                ));
            }
            if f.standing_order_code_raw() != 0x01 {
                errors.push(format!(
                    "FLEET[9].order expected 0x01 (MoveOnly), got {:#04x}",
                    f.standing_order_code_raw()
                ));
            }
            if f.standing_order_target_coords_raw() != [0x0a, 0x0a] {
                errors.push(format!(
                    "FLEET[9].target expected (10,10), got {:?}",
                    f.standing_order_target_coords_raw()
                ));
            }
            if f.scout_count() != 10 {
                errors.push(format!("FLEET[9].sc expected 10, got {}", f.scout_count()));
            }
            if f.battleship_count() != 1 {
                errors.push(format!(
                    "FLEET[9].bb expected 1, got {}",
                    f.battleship_count()
                ));
            }
            if f.cruiser_count() != 0 {
                errors.push(format!("FLEET[9].ca expected 0, got {}", f.cruiser_count()));
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
        println!("Valid fleet-battle scenario");
        println!("  FLEET[1]: loc=(10,10) tgt=(10,10) RoE=100 BB=50 CA=50 DD=50");
        println!("  FLEET[3]: order=0x06 (BombardWorld) tgt=(15,13) speed=3/3 CA=50 DD=50");
        println!("  FLEET[5]: order=0x03 (PatrolSector) loc=(10,10) speed=0/6 SC=10 BB=100");
        println!("  FLEET[9]: order=0x01 (MoveOnly) loc=(9,10) tgt=(10,10) speed=3/6 SC=10 BB=1");
        println!("  PLANET[14]: (15,13) empire=2 armies=142 batteries=15");
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}
