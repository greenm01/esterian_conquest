use ec_data::GameStateBuilder;

#[test]
fn builder_creates_valid_gamestate() {
    let data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3001)
        .build_initialized_baseline()
        .expect("Should build successfully");

    assert_eq!(data.conquest.game_year(), 3001);
    assert_eq!(data.conquest.player_count(), 4);
    assert_eq!(data.fleets.records.len(), 16); // 4 players * 4 fleets

    // Validate with preflight
    let errors = data.ecmaint_preflight_errors();
    assert!(errors.is_empty(), "Preflight errors: {:?}", errors);
    assert_eq!(data.player.records[0].tax_rate(), 50);
    assert_eq!(data.planets.records[0].present_production_points(), Some(100));
    assert_eq!(data.planets.records[0].economy_marker_raw(), 50);
}

#[test]
fn builder_with_fleet_order() {
    let data = GameStateBuilder::new()
        .with_fleet_order(1, 3, 0x0C, [15, 13], [0x01, 0x00])
        .build_initialized_baseline()
        .expect("Should build successfully");

    let fleet = &data.fleets.records[0];
    assert_eq!(fleet.standing_order_code_raw(), 0x0C);
    assert_eq!(fleet.current_speed(), 3);
}

#[test]
fn builder_with_planet_build() {
    let data = GameStateBuilder::new()
        .with_planet_build(15, 0x03, 0x01)
        .build_initialized_baseline()
        .expect("Should build successfully");

    let planet = &data.planets.records[14]; // 15 is 1-based, so index 14
    assert_eq!(planet.build_count_raw(0), 0x03);
    assert_eq!(planet.build_kind_raw(0), 0x01);
}

#[test]
fn builder_with_guard_starbase() {
    let data = GameStateBuilder::new()
        .with_player_count(1)
        .with_guard_starbase(1, 1, [16, 13], 1)
        .build_initialized_baseline()
        .expect("Should build successfully");

    // Should have one base
    assert_eq!(data.bases.records.len(), 1);

    // Player should have starbase_count = 1
    assert_eq!(data.player.records[0].starbase_count_raw(), 1);

    // Fleet should have guard order
    let fleet = &data.fleets.records[0];
    assert_eq!(fleet.standing_order_code_raw(), 0x04); // Guard Starbase
    assert_eq!(fleet.mission_aux_bytes(), [0x01, 0x01]);

    // Validate with preflight
    let errors = data.ecmaint_preflight_errors();
    assert!(errors.is_empty(), "Preflight errors: {:?}", errors);
}

#[test]
fn builder_varies_player_count() {
    // Test 1 player
    let data1 = GameStateBuilder::new()
        .with_player_count(1)
        .build_initialized_baseline()
        .expect("Should build successfully");
    assert_eq!(data1.fleets.records.len(), 4); // 1 player * 4 fleets

    // Test 2 players
    let data2 = GameStateBuilder::new()
        .with_player_count(2)
        .build_initialized_baseline()
        .expect("Should build successfully");
    assert_eq!(data2.fleets.records.len(), 8); // 2 players * 4 fleets

    // Test max 4 players
    let data4 = GameStateBuilder::new()
        .with_player_count(4)
        .build_initialized_baseline()
        .expect("Should build successfully");
    assert_eq!(data4.fleets.records.len(), 16); // 4 players * 4 fleets
}

#[test]
fn builder_can_create_joinable_new_game_baseline() {
    let data = GameStateBuilder::new()
        .with_player_count(2)
        .build_joinable_new_game_baseline()
        .expect("Should build successfully");

    assert_eq!(data.player.records[0].owner_mode_raw(), 0);
    assert_eq!(data.player.records[1].owner_mode_raw(), 0);
    assert_eq!(data.player.records[0].autopilot_flag(), 0);
    assert_eq!(data.player.records[0].fleet_chain_head_raw(), 1);
    assert_eq!(data.player.records[1].fleet_chain_head_raw(), 5);
    assert_eq!(data.planets.records[0].planet_name(), "Not Named Yet");
    assert_eq!(data.planets.records[0].owner_empire_slot_raw(), 1);
    assert_eq!(data.planets.records[0].army_count_raw(), 10);
    assert_eq!(data.planets.records[0].ground_batteries_raw(), 4);
    assert_eq!(data.player.records[0].tax_rate(), 50);
    assert_eq!(data.planets.records[0].economy_marker_raw(), 50);
    assert_eq!(data.planets.records[0].present_production_points(), Some(100));
    assert!(data.ecmaint_preflight_errors().is_empty());
}
