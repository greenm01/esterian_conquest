//! Regression tests for fleet maintenance mechanics (Milestone 4 Phase 2)
//!
//! Validates that the Rust maintenance implementation matches the original ECMAINT
//! behavior on the fleet-scenario fixture pair.

use ec_data::{
    run_maintenance_turn, ColonizationResolvedEvent, CoreGameData, FleetStandingOrderKind,
    MissionResolutionKind, MissionResolutionOutcome,
};
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

#[test]
fn test_colonization_emits_success_event() {
    let mut game_data = load_fixture("ecmaint-fleet-pre");

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert_eq!(events.colonization_events.len(), 1);
    assert_eq!(
        events.colonization_events[0],
        ColonizationResolvedEvent::Succeeded {
            fleet_idx: 0,
            planet_idx: 13,
            colonizer_empire_raw: 1,
        }
    );
    assert!(events.mission_resolution_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == MissionResolutionKind::ColonizeWorld
            && event.outcome == MissionResolutionOutcome::Succeeded
            && event.planet_idx == Some(13)
    }));
}

#[test]
fn test_colonization_emits_blocked_event_for_occupied_world() {
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let target = &mut game_data.planets.records[13];
    target.set_owner_empire_slot_raw(2);
    target.set_ownership_status_raw(2);
    target.set_planet_name("TargetPrime");
    target.set_army_count_raw(10);

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert_eq!(events.colonization_events.len(), 1);
    assert_eq!(
        events.colonization_events[0],
        ColonizationResolvedEvent::BlockedByOwner {
            fleet_idx: 0,
            planet_idx: 13,
            colonizer_empire_raw: 1,
            owner_empire_raw: 2,
        }
    );
    assert!(events.mission_resolution_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == MissionResolutionKind::ColonizeWorld
            && event.outcome == MissionResolutionOutcome::Failed
            && event.planet_idx == Some(13)
    }));
    let target = &game_data.planets.records[13];
    assert_eq!(target.owner_empire_slot_raw(), 2);
    assert_eq!(target.planet_name(), "TargetPrime");
}

#[test]
fn test_scout_sector_arrival_emits_success_event() {
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let scout = &mut game_data.fleets.records[0];
    scout.set_standing_order_code_raw(10);
    scout.set_standing_order_target_coords_raw([15, 13]);
    scout.set_scout_count(1);
    scout.set_etac_count(0);

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert!(events.colonization_events.is_empty());
    assert!(events.mission_resolution_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == MissionResolutionKind::ScoutSector
            && event.outcome == MissionResolutionOutcome::Succeeded
            && event.planet_idx.is_none()
    }));
    assert_eq!(
        game_data.fleets.records[0].standing_order_kind(),
        FleetStandingOrderKind::HoldPosition
    );
    assert_eq!(game_data.fleets.records[0].current_speed(), 0);
}

#[test]
fn test_scout_system_arrival_emits_success_event() {
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let scout = &mut game_data.fleets.records[0];
    scout.set_standing_order_code_raw(11);
    scout.set_standing_order_target_coords_raw([15, 13]);
    scout.set_scout_count(1);
    scout.set_etac_count(0);

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert!(events.colonization_events.is_empty());
    assert_eq!(events.planet_intel_events.len(), 1);
    assert_eq!(events.planet_intel_events[0].planet_idx, 13);
    assert_eq!(events.planet_intel_events[0].viewer_empire_raw, 1);
    assert!(events.mission_resolution_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == MissionResolutionKind::ScoutSolarSystem
            && event.outcome == MissionResolutionOutcome::Succeeded
            && event.planet_idx.is_none()
    }));
    assert_eq!(
        game_data.fleets.records[0].current_location_coords_raw(),
        [15, 13]
    );
}

#[test]
fn test_view_world_arrival_emits_success_and_intel_event() {
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let viewer = &mut game_data.fleets.records[0];
    viewer.set_standing_order_code_raw(9);
    viewer.set_standing_order_target_coords_raw([15, 13]);
    viewer.set_scout_count(0);
    viewer.set_etac_count(0);

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert!(events.colonization_events.is_empty());
    assert_eq!(events.planet_intel_events.len(), 1);
    assert_eq!(events.planet_intel_events[0].planet_idx, 13);
    assert_eq!(events.planet_intel_events[0].viewer_empire_raw, 1);
    assert!(events.mission_resolution_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == MissionResolutionKind::ViewWorld
            && event.outcome == MissionResolutionOutcome::Succeeded
            && event.planet_idx == Some(13)
    }));
}

#[test]
fn test_rendezvous_arrival_emits_waiting_event() {
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_standing_order_code_raw(14);
    fleet.set_standing_order_target_coords_raw([15, 13]);

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert!(events.mission_resolution_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == MissionResolutionKind::RendezvousSector
            && event.outcome == MissionResolutionOutcome::Succeeded
            && event.location_coords == Some([15, 13])
    }));
}

#[test]
fn test_join_merge_emits_merge_event() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.player.records[0].raw[0x00] = 0xff;
    let coords = game_data.fleets.records[0].current_location_coords_raw();
    game_data.fleets.records[1].set_current_location_coords_raw(coords);
    game_data.fleets.records[1].set_standing_order_code_raw(13);

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert!(events.fleet_merge_events.iter().any(|event| {
        event.fleet_idx == 1
            && event.kind == MissionResolutionKind::JoinAnotherFleet
            && event.owner_empire_raw == 1
    }));
}
