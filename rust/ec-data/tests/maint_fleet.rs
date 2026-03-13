//! Regression tests for fleet maintenance mechanics (Milestone 4 Phase 2)
//!
//! Validates that the Rust maintenance implementation matches the original ECMAINT
//! behavior on the fleet-scenario fixture pair.

use ec_data::{run_maintenance_turn, CoreGameData};
use std::path::Path;

/// Helper to load a fixture directory.
fn load_fixture(name: &str) -> CoreGameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
        .join("v1.5");
    CoreGameData::load(&dir).unwrap_or_else(|e| panic!("Failed to load fixture {}: {}", name, e))
}

#[test]
fn test_fleet_movement_and_colonization_fleets_dat() {
    // After one maintenance turn on the fleet pre-fixture, FLEETS.DAT should match
    // the post-fixture byte-for-byte.
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let post_data = load_fixture("ecmaint-fleet-post");

    run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert_eq!(
        game_data.fleets.to_bytes(),
        post_data.fleets.to_bytes(),
        "FLEETS.DAT should match post-fixture after 1 turn"
    );
}

#[test]
fn test_fleet_movement_and_colonization_planets_dat() {
    // After one maintenance turn, PLANETS.DAT should match the post-fixture.
    // This validates planet colonization (fleet 0 ColonizeWorld to (15,13)).
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let post_data = load_fixture("ecmaint-fleet-post");

    run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert_eq!(
        game_data.planets.to_bytes(),
        post_data.planets.to_bytes(),
        "PLANETS.DAT should match post-fixture after 1 turn"
    );
}

#[test]
fn test_fleet_movement_and_colonization_player_dat() {
    // After one maintenance turn, PLAYER.DAT should match the post-fixture.
    // This validates planet count and economic updates after colonization.
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let post_data = load_fixture("ecmaint-fleet-post");

    run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert_eq!(
        game_data.player.to_bytes(),
        post_data.player.to_bytes(),
        "PLAYER.DAT should match post-fixture after 1 turn"
    );
}

#[test]
fn test_fleet_movement_arrival_state() {
    // Specific validation: fleet 0 should arrive at (15,13) with cleared speed and order.
    let mut game_data = load_fixture("ecmaint-fleet-pre");

    // Pre-state: fleet 0 at (16,13) with ColonizeWorld order, speed=3
    assert_eq!(
        game_data.fleets.records[0].current_location_coords_raw(),
        [16, 13],
        "Fleet 0 should start at (16,13)"
    );
    assert_eq!(
        game_data.fleets.records[0].current_speed(),
        3,
        "Fleet 0 should start with speed 3"
    );

    run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    // Post-state: fleet 0 should be at (15,13) with speed=0 and HoldPosition order
    assert_eq!(
        game_data.fleets.records[0].current_location_coords_raw(),
        [15, 13],
        "Fleet 0 should arrive at (15,13)"
    );
    assert_eq!(
        game_data.fleets.records[0].current_speed(),
        0,
        "Fleet 0 should have speed 0 after arrival"
    );
    assert_eq!(
        game_data.fleets.records[0].standing_order_code_raw(),
        0,
        "Fleet 0 order should be cleared to HoldPosition (0) after arrival"
    );
}

#[test]
fn test_colonization_planet_state() {
    // Planet at (15,13) should be colonized by empire 1 after fleet 0 arrives.
    let mut game_data = load_fixture("ecmaint-fleet-pre");

    // Pre-state: planet 13 at (15,13) should be unowned
    let planet_13_pre = &game_data.planets.records[13];
    assert_eq!(
        planet_13_pre.coords_raw(),
        [15, 13],
        "Planet 13 should be at (15,13)"
    );
    assert_eq!(
        planet_13_pre.owner_empire_slot_raw(),
        0,
        "Planet 13 should be unowned pre-maint"
    );

    run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    // Post-state: planet 13 should be owned by empire 1
    let planet_13_post = &game_data.planets.records[13];
    assert_eq!(
        planet_13_post.owner_empire_slot_raw(),
        1,
        "Planet 13 should be owned by empire 1 after colonization"
    );
    assert_eq!(
        planet_13_post.ownership_status_raw(),
        2,
        "Planet 13 ownership_status should be 2 after colonization"
    );
    assert_eq!(
        planet_13_post.army_count_raw(),
        1,
        "Planet 13 should have 1 colonist army"
    );
    assert_eq!(
        planet_13_post.planet_name(),
        "Not Named Yet",
        "Planet 13 should be named 'Not Named Yet' after colonization"
    );
}
