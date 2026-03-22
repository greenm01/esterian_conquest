//! Regression tests for fleet maintenance mechanics (Milestone 4 Phase 2)
//!
//! Validates that the Rust maintenance implementation matches the original ECMAINT
//! behavior on the fleet-scenario fixture pair.

use ec_data::{
    BaseDat, BaseRecord, ColonizationResolvedEvent, CoreGameData, DiplomaticRelation,
    GameStateBuilder, JoinMissionHostEvent, Mission, MissionOutcome, MissionRetargetEvent, Order,
    PlanetIntelSource, SalvageFailureReason, SalvageResolvedEvent,
};
use ec_engine::run_maintenance_turn;
use std::path::Path;

/// Helper to load a fixture directory.
fn load_fixture(name: &str) -> CoreGameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
        .join("v1.5");
    CoreGameData::load(&dir).unwrap_or_else(|e| panic!("Failed to load fixture {}: {}", name, e))
}

fn configured_delayed_hostile_arrival_state(
    order: Order,
    ships: (u16, u16, u16, u16, u16, u16, u16),
) -> (CoreGameData, usize, [u8; 2]) {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    let target_coords = [25, 25];
    let target_idx = 4;

    let target_world = &mut game_data.planets.records[target_idx];
    target_world.set_coords_raw(target_coords);
    target_world.set_owner_empire_slot_raw(2);
    target_world.set_ownership_status_raw(2);
    target_world.set_planet_name("Target");
    target_world.set_army_count_raw(10);
    target_world.set_ground_batteries_raw(4);

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_battleship_count(ships.0);
    fleet.set_cruiser_count(ships.1);
    fleet.set_destroyer_count(ships.2);
    fleet.set_troop_transport_count(ships.3);
    fleet.set_army_count(ships.4);
    fleet.set_etac_count(ships.5);
    fleet.set_scout_count(ships.6 as u8);
    fleet.recompute_max_speed_from_composition();
    fleet.set_current_location_coords_raw([24, 25]);
    fleet.set_current_speed(3);
    fleet.set_standing_order_kind(order);
    fleet.set_standing_order_target_coords_raw(target_coords);
    fleet.raw[0x0d] = 0x80;
    fleet.raw[0x0f] = 0x00;
    fleet.raw[0x19] = 0x81;

    (game_data, target_idx, target_coords)
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
    assert!(matches!(
        events.colonization_events[0],
        ColonizationResolvedEvent::Succeeded {
            fleet_idx: 0,
            planet_idx: 13,
            colonizer_empire_raw: 1,
            ..
        }
    ));
    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == Mission::ColonizeWorld
            && event.outcome == MissionOutcome::Succeeded
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
    assert!(matches!(
        events.colonization_events[0],
        ColonizationResolvedEvent::BlockedByOwner {
            fleet_idx: 0,
            planet_idx: 13,
            colonizer_empire_raw: 1,
            owner_empire_raw: 2,
            ..
        }
    ));
    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == Mission::ColonizeWorld
            && event.outcome == MissionOutcome::Failed
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
    scout.set_standing_order_kind(Order::ScoutSector);
    scout.set_standing_order_target_coords_raw([15, 13]);
    scout.set_scout_count(1);
    scout.set_etac_count(0);

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert!(events.colonization_events.is_empty());
    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == Mission::ScoutSector
            && event.outcome == MissionOutcome::Succeeded
            && event.planet_idx.is_none()
    }));
    assert_eq!(
        game_data.fleets.records[0].standing_order_kind(),
        Order::HoldPosition
    );
    assert_eq!(game_data.fleets.records[0].current_speed(), 0);
}

#[test]
fn test_blockading_foreign_world_escalates_to_enemy() {
    let mut game_data = load_fixture("ecmaint-post");
    let (planet_idx, coords) = game_data
        .planets
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner_empire_slot_raw() == 2)
        .map(|(idx, planet)| (idx, planet.coords_raw()))
        .expect("fixture should contain an empire 2 world");

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw([coords[0].saturating_add(1), coords[1]]);
    fleet.set_standing_order_kind(Order::GuardBlockadeWorld);
    fleet.set_standing_order_target_coords_raw(coords);
    fleet.set_current_speed(3);
    fleet.raw[0x19] = 0x00;

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert!(
        events
            .diplomatic_escalation_events
            .iter()
            .any(|event| event.left_empire_raw == 1 && event.right_empire_raw == 2)
    );
    assert_eq!(
        game_data.player.records[0].diplomatic_relation_toward(2),
        Some(DiplomaticRelation::Enemy)
    );
    assert_eq!(
        game_data.player.records[1].diplomatic_relation_toward(1),
        Some(DiplomaticRelation::Enemy)
    );
    assert!(events.mission_events.iter().any(|event| {
        event.kind == Mission::GuardBlockadeWorld
            && event.owner_empire_raw == 1
            && event.planet_idx == Some(planet_idx)
    }));
}

#[test]
fn test_scout_system_arrival_emits_success_event() {
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let scout = &mut game_data.fleets.records[0];
    scout.set_standing_order_kind(Order::ScoutSolarSystem);
    scout.set_standing_order_target_coords_raw([15, 13]);
    scout.set_scout_count(1);
    scout.set_etac_count(0);

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert!(events.colonization_events.is_empty());
    assert_eq!(events.planet_intel_events.len(), 1);
    assert_eq!(events.planet_intel_events[0].planet_idx, 13);
    assert_eq!(events.planet_intel_events[0].viewer_empire_raw, 1);
    assert_eq!(
        events.planet_intel_events[0].source,
        PlanetIntelSource::ScoutSolarSystem
    );
    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == Mission::ScoutSolarSystem
            && event.outcome == MissionOutcome::Succeeded
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
    viewer.set_standing_order_kind(Order::ViewWorld);
    viewer.set_standing_order_target_coords_raw([15, 13]);
    viewer.set_scout_count(0);
    viewer.set_etac_count(0);

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert!(events.colonization_events.is_empty());
    assert_eq!(events.planet_intel_events.len(), 1);
    assert_eq!(events.planet_intel_events[0].planet_idx, 13);
    assert_eq!(events.planet_intel_events[0].viewer_empire_raw, 1);
    assert_eq!(
        events.planet_intel_events[0].source,
        PlanetIntelSource::ViewWorld
    );
    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == Mission::ViewWorld
            && event.outcome == MissionOutcome::Succeeded
            && event.planet_idx == Some(13)
    }));
    assert_eq!(
        game_data.fleets.records[0].standing_order_kind(),
        Order::HoldPosition
    );
    assert_eq!(game_data.fleets.records[0].current_speed(), 0);
    assert_eq!(
        game_data.fleets.records[0].tuple_c_payload_raw(),
        [0x81, 0x00, 0x00, 0x00, 0x00]
    );
}

#[test]
fn test_seek_home_arrival_emits_success_event() {
    let mut game_data = load_fixture("ecmaint-post");
    let target_coords = game_data
        .planets
        .records
        .iter()
        .find(|planet| planet.owner_empire_slot_raw() == 1)
        .map(|planet| planet.coords_raw())
        .expect("fixture should contain an owned world");
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw([target_coords[0].saturating_add(1), target_coords[1]]);
    fleet.set_standing_order_kind(Order::SeekHome);
    fleet.set_standing_order_target_coords_raw(target_coords);
    fleet.set_current_speed(3);
    fleet.raw[0x0d] = 0x80;
    fleet.raw[0x0f] = 0;
    fleet.raw[0x19] = 0x00;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == Mission::SeekHome
            && event.outcome == MissionOutcome::Succeeded
            && event.location_coords == Some(target_coords)
    }));
}

#[test]
fn test_rendezvous_arrival_emits_waiting_event() {
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_standing_order_kind(Order::RendezvousSector);
    fleet.set_standing_order_target_coords_raw([15, 13]);

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert_eq!(
        game_data.fleets.records[0].standing_order_kind(),
        Order::RendezvousSector
    );
    assert_eq!(
        game_data.fleets.records[0].standing_order_target_coords_raw(),
        [15, 13]
    );

    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == Mission::RendezvousSector
            && event.outcome == MissionOutcome::Arrived
            && event.location_coords == Some([15, 13])
    }));
}

#[test]
fn test_join_merge_emits_merge_event() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.player.records[0].raw[0x00] = 0xff;
    let coords = game_data.fleets.records[0].current_location_coords_raw();
    game_data.fleets.records[1].set_current_location_coords_raw(coords);
    game_data.fleets.records[1].set_standing_order_kind(Order::JoinAnotherFleet);

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert!(events.fleet_merge_events.iter().any(|event| {
        event.fleet_idx == 1
            && event.kind == Mission::JoinAnotherFleet
            && event.owner_empire_raw == 1
            && !event.survivor_side
    }));
}

#[test]
fn test_join_order_refreshes_target_to_moving_host_each_turn() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.player.records[0].raw[0x00] = 0x00;

    let host = &mut game_data.fleets.records[0];
    host.set_current_location_coords_raw([10, 10]);
    host.set_standing_order_kind(Order::MoveOnly);
    host.set_standing_order_target_coords_raw([14, 10]);
    host.set_current_speed(3);
    host.raw[0x0d] = 0x80;
    host.raw[0x0f] = 0;
    host.raw[0x19] = 0x80;

    let host_id = game_data.fleets.records[0].fleet_id();
    let joiner = &mut game_data.fleets.records[1];
    joiner.set_current_location_coords_raw([4, 10]);
    joiner.set_standing_order_kind(Order::JoinAnotherFleet);
    joiner.set_join_host_fleet_id_raw(host_id);
    joiner.set_standing_order_target_coords_raw([10, 10]);
    joiner.set_current_speed(3);
    joiner.raw[0x0d] = 0x80;
    joiner.raw[0x0f] = 0;
    joiner.raw[0x19] = 0x80;

    run_maintenance_turn(&mut game_data).expect("first maintenance turn should succeed");

    let host_after_first = game_data.fleets.records[0].current_location_coords_raw();
    assert_eq!(host_after_first, [12, 10]);
    assert_eq!(
        game_data.fleets.records[1].standing_order_target_coords_raw(),
        [10, 10],
        "joiner should chase the host's turn-start position during the first turn"
    );

    run_maintenance_turn(&mut game_data).expect("second maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.records[1].standing_order_target_coords_raw(),
        host_after_first,
        "joiner should refresh to the host's latest location on the following turn"
    );
    assert_eq!(
        game_data.fleets.records[1].join_host_fleet_id_raw(),
        host_id,
        "host identity should remain unchanged while the host survives"
    );
    assert_eq!(
        game_data.fleets.records[1].standing_order_kind(),
        Order::JoinAnotherFleet
    );
}

#[test]
fn test_join_order_survives_arrival_at_previous_host_position() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.player.records[0].raw[0x00] = 0x00;

    let host = &mut game_data.fleets.records[0];
    host.set_current_location_coords_raw([10, 10]);
    host.set_standing_order_kind(Order::MoveOnly);
    host.set_standing_order_target_coords_raw([14, 10]);
    host.set_current_speed(3);
    host.raw[0x0d] = 0x80;
    host.raw[0x0f] = 0;
    host.raw[0x19] = 0x80;

    let host_id = game_data.fleets.records[0].fleet_id();
    let joiner = &mut game_data.fleets.records[1];
    joiner.set_current_location_coords_raw([9, 10]);
    joiner.set_standing_order_kind(Order::JoinAnotherFleet);
    joiner.set_join_host_fleet_id_raw(host_id);
    joiner.set_standing_order_target_coords_raw([10, 10]);
    joiner.set_current_speed(3);
    joiner.raw[0x0d] = 0x80;
    joiner.raw[0x0f] = 0;
    joiner.raw[0x19] = 0x80;

    run_maintenance_turn(&mut game_data).expect("first maintenance turn should succeed");

    let host_after_first = game_data.fleets.records[0].current_location_coords_raw();
    assert_eq!(host_after_first, [12, 10]);
    assert_eq!(
        game_data.fleets.records[1].current_location_coords_raw(),
        [10, 10]
    );
    assert_eq!(
        game_data.fleets.records[1].standing_order_kind(),
        Order::JoinAnotherFleet
    );

    run_maintenance_turn(&mut game_data).expect("second maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.records[1].standing_order_kind(),
        Order::JoinAnotherFleet
    );
    assert_eq!(
        game_data.fleets.records[1].standing_order_target_coords_raw(),
        host_after_first
    );
}

#[test]
fn test_join_order_abandons_mission_when_host_is_destroyed() {
    let mut game_data = load_fixture("ecmaint-post");

    let host_id = game_data.fleets.records[0].fleet_id();
    game_data.fleets.records[0].set_destroyer_count(0);
    game_data.fleets.records[0].set_cruiser_count(0);
    game_data.fleets.records[0].set_battleship_count(0);
    game_data.fleets.records[0].set_scout_count(0);
    game_data.fleets.records[0].set_troop_transport_count(0);
    game_data.fleets.records[0].set_etac_count(0);

    let joiner_coords = [7, 9];
    let joiner = &mut game_data.fleets.records[1];
    joiner.set_current_location_coords_raw(joiner_coords);
    joiner.set_standing_order_kind(Order::JoinAnotherFleet);
    joiner.set_join_host_fleet_id_raw(host_id);
    joiner.set_standing_order_target_coords_raw([10, 10]);
    joiner.set_current_speed(3);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.records[1].standing_order_kind(),
        Order::HoldPosition
    );
    assert_eq!(game_data.fleets.records[1].current_speed(), 0);
    let abandoned_coords = game_data.fleets.records[1].current_location_coords_raw();
    assert_eq!(abandoned_coords, [9, 10]);
    assert_eq!(
        game_data.fleets.records[1].standing_order_target_coords_raw(),
        abandoned_coords
    );
    assert_eq!(game_data.fleets.records[1].join_host_fleet_id_raw(), 0);
    assert!(events.join_host_events.iter().any(|event| {
        matches!(
            event,
            JoinMissionHostEvent::HostDestroyed {
                fleet_idx,
                destroyed_host_fleet_id,
                coords,
                ..
            } if *fleet_idx == 1
                && *destroyed_host_fleet_id == host_id
                && *coords == abandoned_coords
        )
    }));
}

#[test]
fn test_seek_home_retargets_to_next_owned_planet_when_target_is_lost() {
    let mut game_data = load_fixture("ecmaint-post");
    let original_target = game_data.planets.records[0].coords_raw();
    game_data.planets.records[0].set_owner_empire_slot_raw(2);
    let fallback_target = game_data
        .planets
        .records
        .iter()
        .find(|planet| planet.owner_empire_slot_raw() == 1)
        .map(|planet| planet.coords_raw())
        .expect("fixture should still have another owned planet");

    let fleet = &mut game_data.fleets.records[1];
    fleet.set_current_location_coords_raw([9, 9]);
    fleet.set_standing_order_kind(Order::SeekHome);
    fleet.set_standing_order_target_coords_raw(original_target);
    fleet.set_current_speed(3);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.records[1].standing_order_target_coords_raw(),
        fallback_target
    );
    assert!(events.mission_retarget_events.iter().any(|event| {
        matches!(
            event,
            MissionRetargetEvent::Retargeted {
                fleet_idx,
                mission,
                previous_target_coords,
                new_target_coords,
                ..
            } if *fleet_idx == 1
                && *mission == Mission::SeekHome
                && *previous_target_coords == original_target
                && *new_target_coords == fallback_target
        )
    }));
}

#[test]
fn test_guard_starbase_retargets_to_live_base_coords() {
    let mut game_data = load_fixture("ecmaint-post");
    let mut base = BaseRecord::new_zeroed();
    base.set_base_id_raw(3);
    base.set_owner_empire_raw(1);
    base.set_coords_raw([12, 8]);
    game_data.bases = BaseDat {
        records: vec![base],
    };

    let fleet = &mut game_data.fleets.records[1];
    fleet.set_current_location_coords_raw([4, 8]);
    fleet.set_standing_order_kind(Order::GuardStarbase);
    fleet.set_standing_order_target_coords_raw([9, 8]);
    fleet.set_current_speed(3);
    fleet.set_mission_aux_bytes([3, 1]);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.records[1].standing_order_target_coords_raw(),
        [12, 8]
    );
    assert_eq!(game_data.fleets.records[1].mission_aux_bytes(), [0, 1]);
    assert!(events.mission_retarget_events.iter().any(|event| {
        matches!(
            event,
            MissionRetargetEvent::Retargeted {
                fleet_idx,
                mission,
                previous_target_coords,
                new_target_coords,
                ..
            } if *fleet_idx == 1
                && *mission == Mission::GuardStarbase
                && *previous_target_coords == [9, 8]
                && *new_target_coords == [12, 8]
        )
    }));
}

#[test]
fn test_guard_starbase_abandons_when_linked_base_is_missing() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.bases = BaseDat { records: vec![] };

    let fleet = &mut game_data.fleets.records[1];
    fleet.set_current_location_coords_raw([4, 8]);
    fleet.set_standing_order_kind(Order::GuardStarbase);
    fleet.set_standing_order_target_coords_raw([9, 8]);
    fleet.set_current_speed(3);
    fleet.set_mission_aux_bytes([3, 1]);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.records[1].standing_order_kind(),
        Order::HoldPosition
    );
    assert_eq!(game_data.fleets.records[1].current_speed(), 0);
    assert_eq!(
        game_data.fleets.records[1].standing_order_target_coords_raw(),
        game_data.fleets.records[1].current_location_coords_raw()
    );
    assert!(events.mission_retarget_events.iter().any(|event| {
        matches!(
            event,
            MissionRetargetEvent::Abandoned {
                fleet_idx,
                mission,
                previous_target_coords,
                ..
            } if *fleet_idx == 1
                && *mission == Mission::GuardStarbase
                && *previous_target_coords == [9, 8]
        )
    }));
}

#[test]
fn test_guard_starbase_persists_after_arrival() {
    let mut game_data = load_fixture("ecmaint-post");
    let mut base = BaseRecord::new_zeroed();
    base.set_active_flag_raw(1);
    base.set_base_id_raw(3);
    base.set_owner_empire_raw(1);
    base.set_coords_raw([11, 8]);
    game_data.bases = BaseDat {
        records: vec![base],
    };

    let fleet = &mut game_data.fleets.records[1];
    fleet.set_current_location_coords_raw([10, 8]);
    fleet.set_standing_order_kind(Order::GuardStarbase);
    fleet.set_standing_order_target_coords_raw([11, 8]);
    fleet.set_current_speed(3);
    fleet.set_mission_aux_bytes([3, 1]);
    fleet.raw[0x0d] = 0x80;
    fleet.raw[0x0f] = 0;
    fleet.raw[0x19] = 0x80;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.records[1].current_location_coords_raw(),
        [11, 8]
    );
    assert_eq!(
        game_data.fleets.records[1].standing_order_kind(),
        Order::GuardStarbase
    );
    assert_eq!(game_data.fleets.records[1].current_speed(), 0);
    assert_eq!(
        game_data.fleets.records[1].standing_order_target_coords_raw(),
        [11, 8]
    );
    assert_eq!(game_data.fleets.records[1].mission_aux_bytes(), [0, 1]);
    assert_eq!(game_data.fleets.records[1].raw[0x0d], 0x7b);
    assert_eq!(game_data.fleets.records[1].raw[0x0e], 0x00);
    assert_eq!(game_data.fleets.records[1].raw[0x0f], 0x84);
    assert_eq!(game_data.fleets.records[1].raw[0x10], 0xd8);
    assert_eq!(game_data.fleets.records[1].raw[0x11], 0x89);
    assert_eq!(game_data.fleets.records[1].raw[0x12], 0x1d);
    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 1
            && event.kind == Mission::GuardStarbase
            && event.outcome == MissionOutcome::Arrived
            && event.location_coords == Some([11, 8])
    }));
}

#[test]
fn test_guard_starbase_with_zero_index_stays_armed_when_target_base_exists() {
    let mut game_data = load_fixture("ecmaint-post");
    let mut base = BaseRecord::new_zeroed();
    base.set_active_flag_raw(1);
    base.set_base_id_raw(3);
    base.set_owner_empire_raw(1);
    base.set_coords_raw([11, 8]);
    game_data.bases = BaseDat {
        records: vec![base],
    };

    let fleet = &mut game_data.fleets.records[1];
    fleet.set_current_location_coords_raw([11, 8]);
    fleet.set_standing_order_kind(Order::GuardStarbase);
    fleet.set_standing_order_target_coords_raw([11, 8]);
    fleet.set_current_speed(0);
    fleet.set_mission_aux_bytes([0, 1]);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.records[1].standing_order_kind(),
        Order::GuardStarbase
    );
    assert_eq!(game_data.fleets.records[1].current_speed(), 0);
    assert_eq!(game_data.fleets.records[1].mission_aux_bytes(), [0, 1]);
    assert!(!events.mission_retarget_events.iter().any(|event| {
        matches!(
            event,
            MissionRetargetEvent::Abandoned {
                fleet_idx,
                mission,
                ..
            } if *fleet_idx == 1 && *mission == Mission::GuardStarbase
        )
    }));
}

#[test]
fn test_guard_starbase_with_zero_index_abandons_when_target_base_is_missing() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.bases = BaseDat { records: vec![] };

    let fleet = &mut game_data.fleets.records[1];
    fleet.set_current_location_coords_raw([11, 8]);
    fleet.set_standing_order_kind(Order::GuardStarbase);
    fleet.set_standing_order_target_coords_raw([11, 8]);
    fleet.set_current_speed(0);
    fleet.set_mission_aux_bytes([0, 1]);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.records[1].standing_order_kind(),
        Order::HoldPosition
    );
    assert_eq!(game_data.fleets.records[1].current_speed(), 0);
    assert_eq!(game_data.fleets.records[1].mission_aux_bytes(), [0, 1]);
    assert!(events.mission_retarget_events.iter().any(|event| {
        matches!(
            event,
            MissionRetargetEvent::Abandoned {
                fleet_idx,
                mission,
                previous_target_coords,
                ..
            } if *fleet_idx == 1
                && *mission == Mission::GuardStarbase
                && *previous_target_coords == [11, 8]
        )
    }));
}

#[test]
fn test_guard_blockade_world_persists_after_isolated_arrival() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    let target_coords = [11, 8];

    let target_world = &mut game_data.planets.records[4];
    target_world.set_coords_raw(target_coords);
    target_world.set_owner_empire_slot_raw(2);
    target_world.set_ownership_status_raw(2);
    target_world.set_planet_name("Target");

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw([10, 8]);
    fleet.set_standing_order_kind(Order::GuardBlockadeWorld);
    fleet.set_standing_order_target_coords_raw(target_coords);
    fleet.set_current_speed(3);
    fleet.raw[0x0d] = 0x80;
    fleet.raw[0x0f] = 0;
    fleet.raw[0x19] = 0x80;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.records[0].current_location_coords_raw(),
        target_coords
    );
    assert_eq!(
        game_data.fleets.records[0].standing_order_kind(),
        Order::GuardBlockadeWorld
    );
    assert_eq!(game_data.fleets.records[0].current_speed(), 0);
    assert_eq!(
        game_data.fleets.records[0].standing_order_target_coords_raw(),
        target_coords
    );
    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == Mission::GuardBlockadeWorld
            && event.outcome == MissionOutcome::Arrived
            && event.location_coords == Some(target_coords)
    }));
}

#[test]
fn test_patrol_sector_persists_after_arrival() {
    let mut game_data = load_fixture("ecmaint-post");
    let fleet = &mut game_data.fleets.records[1];
    fleet.set_current_location_coords_raw([10, 10]);
    fleet.set_standing_order_kind(Order::PatrolSector);
    fleet.set_standing_order_target_coords_raw([11, 10]);
    fleet.set_current_speed(3);
    fleet.raw[0x0d] = 0x80;
    fleet.raw[0x0f] = 0;
    fleet.raw[0x19] = 0x80;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.records[1].current_location_coords_raw(),
        [11, 10]
    );
    assert_eq!(
        game_data.fleets.records[1].standing_order_kind(),
        Order::PatrolSector
    );
    assert_eq!(game_data.fleets.records[1].current_speed(), 0);
    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 1
            && event.kind == Mission::PatrolSector
            && event.outcome == MissionOutcome::Arrived
    }));
}

#[test]
fn test_delayed_hostile_orders_preserve_order_speed_and_ready_bytes_on_arrival() {
    let cases = [
        (
            "bombard",
            Order::BombardWorld,
            Mission::BombardWorld,
            (0, 3, 5, 0, 0, 0, 0),
        ),
        (
            "invade",
            Order::InvadeWorld,
            Mission::InvadeWorld,
            (0, 1, 0, 10, 10, 0, 0),
        ),
        (
            "blitz",
            Order::BlitzWorld,
            Mission::BlitzWorld,
            (100, 50, 50, 50, 50, 0, 0),
        ),
    ];

    for (name, order, mission, ships) in cases {
        let (mut game_data, target_idx, target_coords) =
            configured_delayed_hostile_arrival_state(order, ships);

        let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");
        let fleet = &game_data.fleets.records[0];

        assert_eq!(
            fleet.current_location_coords_raw(),
            target_coords,
            "{name} coords"
        );
        assert_eq!(fleet.standing_order_kind(), order, "{name} order");
        assert_eq!(fleet.current_speed(), 3, "{name} arrival speed");
        assert_eq!(
            fleet.standing_order_target_coords_raw(),
            target_coords,
            "{name} target"
        );
        assert_eq!(fleet.raw[0x19], 0x80, "{name} raw[0x19]");
        assert_eq!(fleet.raw[0x1a], 0xb9, "{name} raw[0x1a]");
        assert_eq!(fleet.raw[0x1b], 0xff, "{name} raw[0x1b]");
        assert_eq!(fleet.raw[0x1c], 0xff, "{name} raw[0x1c]");
        assert_eq!(fleet.raw[0x1d], 0xff, "{name} raw[0x1d]");
        assert_eq!(fleet.raw[0x1e], 0x7f, "{name} raw[0x1e]");
        assert!(events.mission_events.iter().any(|event| {
            event.fleet_idx == 0
                && event.kind == mission
                && event.outcome == MissionOutcome::Arrived
                && event.planet_idx == Some(target_idx)
                && event.location_coords == Some(target_coords)
        }));
    }
}

#[test]
fn test_join_merge_occurs_without_combat_merge_flag() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.player.records[0].raw[0x00] = 0x00;
    let host_coords = game_data.fleets.records[0].current_location_coords_raw();
    let host_id = game_data.fleets.records[0].fleet_id();

    let joiner = &mut game_data.fleets.records[1];
    joiner.set_current_location_coords_raw(host_coords);
    joiner.set_standing_order_kind(Order::JoinAnotherFleet);
    joiner.set_join_host_fleet_id_raw(host_id);
    joiner.set_standing_order_target_coords_raw(host_coords);

    let fleet_count_before = game_data.fleets.records.len();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(game_data.fleets.records.len(), fleet_count_before - 1);
    assert!(
        events
            .fleet_merge_events
            .iter()
            .any(|event| { event.kind == Mission::JoinAnotherFleet && !event.survivor_side })
    );
}

#[test]
fn test_rendezvous_merge_occurs_without_combat_merge_flag() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.player.records[0].raw[0x00] = 0x00;
    let coords = game_data.fleets.records[0].current_location_coords_raw();
    game_data.fleets.records[0].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[0].set_standing_order_target_coords_raw(coords);
    game_data.fleets.records[1].set_current_location_coords_raw(coords);
    game_data.fleets.records[1].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[1].set_standing_order_target_coords_raw(coords);

    let fleet_count_before = game_data.fleets.records.len();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(game_data.fleets.records.len(), fleet_count_before - 1);
    assert!(
        events
            .fleet_merge_events
            .iter()
            .any(|event| { event.kind == Mission::RendezvousSector && !event.survivor_side })
    );
}

#[test]
fn test_rendezvous_does_not_merge_before_reaching_its_assigned_sector() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.player.records[0].raw[0x00] = 0x00;
    let coords = game_data.fleets.records[0].current_location_coords_raw();

    game_data.fleets.records[0].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[0].set_standing_order_target_coords_raw(coords);
    game_data.fleets.records[1].set_current_location_coords_raw(coords);
    game_data.fleets.records[1].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[1].set_standing_order_target_coords_raw([coords[0] + 1, coords[1]]);
    game_data.fleets.records[1].set_current_speed(0);

    let fleet_count_before = game_data.fleets.records.len();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(game_data.fleets.records.len(), fleet_count_before);
    assert!(
        events
            .fleet_merge_events
            .iter()
            .all(|event| { event.kind != Mission::RendezvousSector })
    );
}

#[test]
fn test_salvage_converts_fleet_value_into_owned_planet_production_and_removes_fleet() {
    let mut game_data = load_fixture("ecmaint-post");
    let (planet_idx, target_coords) = game_data
        .planets
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner_empire_slot_raw() == 1)
        .map(|(idx, planet)| (idx, planet.coords_raw()))
        .expect("fixture should contain an owned planet");
    let start_coords = if target_coords[0] > 1 {
        [target_coords[0] - 1, target_coords[1]]
    } else {
        [target_coords[0] + 1, target_coords[1]]
    };
    let stored_before = game_data.planets.records[planet_idx].stored_production_points();
    let fleet_count_before = game_data.fleets.records.len();

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw(start_coords);
    fleet.set_standing_order_kind(Order::Salvage);
    fleet.set_standing_order_target_coords_raw(target_coords);
    fleet.set_current_speed(3);
    fleet.set_destroyer_count(1);
    fleet.set_cruiser_count(1);
    fleet.set_battleship_count(0);
    fleet.set_scout_count(0);
    fleet.set_troop_transport_count(0);
    fleet.set_army_count(0);
    fleet.set_etac_count(0);
    fleet.raw[0x0d] = 0x80;
    fleet.raw[0x0f] = 0;
    fleet.raw[0x19] = 0x00;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(game_data.fleets.records.len(), fleet_count_before - 1);
    assert_eq!(
        game_data.planets.records[planet_idx].stored_production_points(),
        stored_before + 10,
    );
    assert!(events.salvage_events.iter().any(|event| {
        matches!(
            event,
            SalvageResolvedEvent::Succeeded {
                owner_empire_raw,
                planet_idx: event_planet_idx,
                coords,
                recovered_points,
                ..
            } if *owner_empire_raw == 1
                && *event_planet_idx == planet_idx
                && *coords == target_coords
                && *recovered_points == 10
        )
    }));
}

#[test]
fn test_salvage_fails_at_foreign_planet_without_removing_fleet() {
    let mut game_data = load_fixture("ecmaint-post");
    let (planet_idx, target_coords) = game_data
        .planets
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner_empire_slot_raw() == 2)
        .map(|(idx, planet)| (idx, planet.coords_raw()))
        .expect("fixture should contain a foreign planet");
    let start_coords = if target_coords[0] > 1 {
        [target_coords[0] - 1, target_coords[1]]
    } else {
        [target_coords[0] + 1, target_coords[1]]
    };
    let fleet_count_before = game_data.fleets.records.len();

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw(start_coords);
    fleet.set_standing_order_kind(Order::Salvage);
    fleet.set_standing_order_target_coords_raw(target_coords);
    fleet.set_current_speed(3);
    fleet.raw[0x0d] = 0x80;
    fleet.raw[0x0f] = 0;
    fleet.raw[0x19] = 0x00;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(game_data.fleets.records.len(), fleet_count_before);
    assert_eq!(
        game_data.fleets.records[0].standing_order_kind(),
        Order::HoldPosition
    );
    assert!(events.salvage_events.iter().any(|event| {
        matches!(
            event,
            SalvageResolvedEvent::Failed {
                owner_empire_raw,
                planet_idx: Some(idx),
                coords,
                reason,
                ..
            } if *owner_empire_raw == 1
                && *idx == planet_idx
                && *coords == target_coords
                && *reason == SalvageFailureReason::PlanetNotOwned
        )
    }));
}

#[test]
fn test_rendezvous_merge_emits_survivor_absorption_event() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.player.records[0].raw[0x00] = 0xff;
    let coords = game_data.fleets.records[0].current_location_coords_raw();
    game_data.fleets.records[0].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[1].set_current_location_coords_raw(coords);
    game_data.fleets.records[1].set_standing_order_kind(Order::RendezvousSector);

    let survivor_id = game_data.fleets.records[0].fleet_id();
    let absorbed_id = game_data.fleets.records[1].fleet_id();

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert!(events.fleet_merge_events.iter().any(|event| {
        event.kind == Mission::RendezvousSector
            && event.survivor_side
            && event.host_fleet_id == survivor_id
            && event.absorbed_fleet_id == absorbed_id
    }));
}

#[test]
fn test_rendezvous_survivor_remains_on_rendezvous_after_merge() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.player.records[0].raw[0x00] = 0x00;
    let coords = game_data.fleets.records[0].current_location_coords_raw();
    game_data.fleets.records[0].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[0].set_standing_order_target_coords_raw(coords);
    game_data.fleets.records[1].set_current_location_coords_raw(coords);
    game_data.fleets.records[1].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[1].set_standing_order_target_coords_raw(coords);

    let survivor_id = game_data.fleets.records[0].fleet_id();

    run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    let survivor = game_data
        .fleets
        .records
        .iter()
        .find(|fleet| fleet.fleet_id() == survivor_id)
        .expect("survivor fleet should remain present");
    assert_eq!(survivor.standing_order_kind(), Order::RendezvousSector);
    assert_eq!(survivor.standing_order_target_coords_raw(), coords);
}

#[test]
fn test_rendezvous_merge_recomputes_survivor_speed_to_slowest_member() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.player.records[0].raw[0x00] = 0xff;
    let coords = game_data.fleets.records[0].current_location_coords_raw();

    game_data.fleets.records[0].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[0].set_destroyer_count(1);
    game_data.fleets.records[0].set_cruiser_count(0);
    game_data.fleets.records[0].set_battleship_count(0);
    game_data.fleets.records[0].set_scout_count(0);
    game_data.fleets.records[0].set_troop_transport_count(0);
    game_data.fleets.records[0].set_etac_count(0);
    game_data.fleets.records[0].set_max_speed(6);

    game_data.fleets.records[1].set_current_location_coords_raw(coords);
    game_data.fleets.records[1].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[1].set_destroyer_count(0);
    game_data.fleets.records[1].set_cruiser_count(0);
    game_data.fleets.records[1].set_battleship_count(0);
    game_data.fleets.records[1].set_scout_count(0);
    game_data.fleets.records[1].set_troop_transport_count(0);
    game_data.fleets.records[1].set_etac_count(1);
    game_data.fleets.records[1].set_max_speed(3);

    run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert_eq!(game_data.fleets.records[0].max_speed(), 3);
}

#[test]
fn test_merge_preserves_surviving_local_fleet_numbers() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.player.records[0].raw[0x00] = 0xff;
    let coords = game_data.fleets.records[0].current_location_coords_raw();
    game_data.fleets.records[2].set_current_location_coords_raw([1, 1]);
    game_data.fleets.records[3].set_current_location_coords_raw([2, 2]);
    game_data.fleets.records[1].set_current_location_coords_raw(coords);
    game_data.fleets.records[1].set_standing_order_kind(Order::JoinAnotherFleet);

    run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    let player1_local_slots = game_data
        .fleets
        .records
        .iter()
        .filter(|fleet| fleet.owner_empire_raw() == 1)
        .map(|fleet| fleet.local_slot_word_raw())
        .collect::<Vec<_>>();
    assert_eq!(player1_local_slots, vec![1, 3, 4]);

    let player2_local_slots = game_data
        .fleets
        .records
        .iter()
        .filter(|fleet| fleet.owner_empire_raw() == 2)
        .map(|fleet| fleet.local_slot_word_raw())
        .collect::<Vec<_>>();
    assert_eq!(player2_local_slots, vec![1, 2, 3, 4]);
}
