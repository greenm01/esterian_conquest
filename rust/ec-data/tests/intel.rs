use std::collections::BTreeMap;

use ec_data::{
    DatabaseDat, GameStateBuilder, build_player_starmap_projection_from_snapshots,
    merge_player_intel_from_compat, visible_hazard_intel_from_snapshots,
};

#[test]
fn sqlite_style_intel_persists_when_compat_database_row_disappears() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    game_data.planets.records[4].set_coords_raw([4, 2]);
    game_data.planets.records[4].set_owner_empire_slot_raw(2);
    game_data.planets.records[4].set_ownership_status_raw(2);

    let planet_count = game_data.planets.records.len();
    let mut observed_database =
        DatabaseDat::new_zeroed((game_data.conquest.player_count() as usize) * planet_count);
    let record = observed_database.record_mut(4, 0, planet_count);
    record.set_planet_name("Prime");
    record.raw[0x15] = 2;
    record.raw[0x1c] = 100;
    record.raw[0x23] = 9;
    record.raw[0x25] = 3;

    let known = merge_player_intel_from_compat(&game_data, &observed_database, 1, 3000, None);
    let known_world = known.get(&5).expect("known world should be stored");
    assert_eq!(known_world.known_name.as_deref(), Some("Prime"));
    assert_eq!(known_world.known_owner_empire_id, Some(2));
    assert_eq!(known_world.known_potential_production, Some(100));
    assert_eq!(known_world.known_armies, Some(9));
    assert_eq!(known_world.known_ground_batteries, Some(3));
    assert_eq!(known_world.last_intel_year, Some(3000));

    let blank_database =
        DatabaseDat::new_zeroed((game_data.conquest.player_count() as usize) * planet_count);
    let persisted =
        merge_player_intel_from_compat(&game_data, &blank_database, 1, 3001, Some(&known));
    let persisted_world = persisted
        .get(&5)
        .expect("persisted world should remain in sqlite intel");
    assert_eq!(persisted_world.known_name.as_deref(), Some("Prime"));
    assert_eq!(persisted_world.known_owner_empire_id, Some(2));
    assert_eq!(persisted_world.known_potential_production, Some(100));
    assert_eq!(persisted_world.known_armies, Some(9));
    assert_eq!(persisted_world.known_ground_batteries, Some(3));
    assert_eq!(persisted_world.last_intel_year, Some(3000));

    let projection = build_player_starmap_projection_from_snapshots(&game_data, &persisted, 1);
    let world = projection
        .worlds
        .iter()
        .find(|world| world.coords == [4, 2])
        .expect("projection world should exist");
    assert_eq!(world.known_name.as_deref(), Some("Prime"));
    assert_eq!(world.known_owner_empire_id, Some(2));

    let hazards = visible_hazard_intel_from_snapshots(&game_data, &persisted, 1);
    assert!(hazards.foreign_worlds.contains(&[4, 2]));
    assert!(hazards.hostile_homeworlds.contains(&[4, 2]));
}

#[test]
fn runtime_projection_keeps_owned_worlds_visible_without_snapshot_rows() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    game_data.planets.records[0].set_coords_raw([5, 2]);
    game_data.planets.records[0].set_owner_empire_slot_raw(1);
    game_data.planets.records[0].set_planet_name("Home");

    let projection =
        build_player_starmap_projection_from_snapshots(&game_data, &BTreeMap::new(), 1);
    let home = projection
        .worlds
        .iter()
        .find(|world| world.coords == [5, 2])
        .expect("owned world should exist");

    assert_eq!(home.known_owner_empire_id, Some(1));
    assert_eq!(home.known_name.as_deref(), Some("Home"));
    assert_eq!(
        home.known_potential_production,
        Some(game_data.planets.records[0].potential_production_points())
    );
}

#[test]
fn generated_database_defaults_to_classic_unknown_sentinels() {
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    let database = DatabaseDat::generate_from_planets_and_year(
        &game_data
            .planets
            .records
            .iter()
            .map(|planet| planet.planet_name())
            .collect::<Vec<_>>(),
        game_data.conquest.game_year(),
        game_data.conquest.player_count() as usize,
        None,
    );
    let record = database.record(0, 0, game_data.planets.records.len());
    assert_eq!(record.planet_name_bytes(), b"UNKNOWN");
    for offset in [
        0x15usize, 0x1c, 0x1d, 0x1e, 0x1f, 0x20, 0x23, 0x24, 0x25, 0x26,
    ] {
        assert_eq!(
            record.raw[offset], 0xff,
            "offset {offset:#x} should use unknown sentinel"
        );
    }
}
