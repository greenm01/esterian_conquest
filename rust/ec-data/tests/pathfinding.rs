use ec_data::{
    DatabaseDat, GameStateBuilder, Order, VisibleHazardIntel, plan_route, plan_route_with_intel,
    run_maintenance_turn, run_maintenance_turn_with_visible_hazards,
    visible_hazard_intel_from_database,
};

#[test]
fn pathfinder_prefers_direct_route_when_space_is_safe() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw([2, 2]);
    fleet.set_standing_order_kind(Order::MoveOnly);
    fleet.set_standing_order_target_coords_raw([6, 2]);
    fleet.set_current_speed(3);

    let route = plan_route(&game_data, 0).expect("route should exist");
    assert_eq!(route.steps.first().map(|step| step.coords), Some([2, 2]));
    assert_eq!(route.steps.last().map(|step| step.coords), Some([6, 2]));
    assert!(route.steps.iter().all(|step| step.coords[1] == 2));
}

#[test]
fn pathfinder_avoids_foreign_guarded_system_when_alternate_route_exists() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw([2, 2]);
    fleet.set_standing_order_kind(Order::MoveOnly);
    fleet.set_standing_order_target_coords_raw([6, 2]);
    fleet.set_current_speed(3);

    let mut intel = VisibleHazardIntel::default();
    intel.foreign_worlds.insert([4, 2]);
    intel.foreign_starbases.insert([4, 2]);

    let route = plan_route_with_intel(&game_data, 0, &intel).expect("route should exist");
    assert!(!route.steps.iter().any(|step| step.coords == [4, 2]));
}

#[test]
fn maintenance_preserves_fog_of_war_and_uses_direct_route_without_visible_intel() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw([2, 2]);
    fleet.set_standing_order_kind(Order::MoveOnly);
    fleet.set_standing_order_target_coords_raw([6, 2]);
    fleet.set_current_speed(3);

    let foreign_world = &mut game_data.planets.records[4];
    foreign_world.set_coords_raw([4, 2]);
    foreign_world.set_owner_empire_slot_raw(2);
    foreign_world.set_ownership_status_raw(2);
    foreign_world.set_ground_batteries_raw(5);

    run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(game_data.fleets.records[0].current_location_coords_raw(), [4, 2]);
}

#[test]
fn visible_hazard_intel_derives_known_foreign_worlds_from_database_view() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    game_data.planets.records[4].set_coords_raw([4, 2]);

    let planet_count = game_data.planets.records.len();
    let mut database = DatabaseDat::new_zeroed((game_data.conquest.player_count() as usize) * planet_count);
    let record = database.record_mut(4, 0, planet_count);
    record.set_planet_name("Prime");
    record.raw[0x15] = 2;
    record.raw[0x1c] = 100;

    let intel = visible_hazard_intel_from_database(&game_data, &database, 1);
    assert!(intel.foreign_worlds.contains(&[4, 2]));
    assert!(intel.hostile_homeworlds.contains(&[4, 2]));
}

#[test]
fn maintenance_avoids_known_foreign_world_when_visible_hazard_intel_is_supplied() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw([2, 2]);
    fleet.set_standing_order_kind(Order::MoveOnly);
    fleet.set_standing_order_target_coords_raw([6, 2]);
    fleet.set_current_speed(3);

    let mut hazards = vec![VisibleHazardIntel::default(); game_data.conquest.player_count() as usize];
    hazards[0].foreign_worlds.insert([4, 2]);

    run_maintenance_turn_with_visible_hazards(&mut game_data, &hazards)
        .expect("maintenance should succeed");

    assert_ne!(game_data.fleets.records[0].current_location_coords_raw(), [4, 2]);
}
