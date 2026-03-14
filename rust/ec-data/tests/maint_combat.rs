mod common;

use ec_data::{
    ContactReportSource, CoreGameData, MissionResolutionKind, MissionResolutionOutcome,
    run_maintenance_turn,
};
use std::path::Path;

fn load_fixture(name: &str) -> CoreGameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
        .join("v1.5");
    CoreGameData::load(&dir).unwrap_or_else(|e| panic!("Failed to load fixture {}: {}", name, e))
}

fn configured_assault_state(order_code: u8) -> CoreGameData {
    let mut game_data = load_fixture("ecmaint-post");

    // Use the unowned world at (15,13) as a deterministic assault target.
    let target = &mut game_data.planets.records[13];
    target.set_as_owned_target_world(
        [15, 13],
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        10,
        4,
        2,
        2,
    );

    // Reuse fleet 1 as the attacker and mark it as already-arrived this turn.
    let attacker = &mut game_data.fleets.records[0];
    attacker.set_current_location_coords_raw([15, 13]);
    attacker.set_standing_order_code_raw(order_code);
    attacker.set_standing_order_target_coords_raw([15, 13]);
    attacker.set_current_speed(3);
    attacker.raw[0x19] = 0x80;
    attacker.set_rules_of_engagement(10);
    attacker.set_scout_count(0);
    attacker.set_battleship_count(0);
    attacker.set_cruiser_count(0);
    attacker.set_destroyer_count(1);
    attacker.set_troop_transport_count(0);
    attacker.set_army_count(0);
    attacker.set_etac_count(0);

    game_data
}

fn add_active_starbase(game_data: &mut CoreGameData, owner: u8, coords: [u8; 2]) {
    let mut base = ec_data::BaseRecord::new_zeroed();
    base.set_local_slot_raw(1);
    base.set_active_flag_raw(1);
    base.set_base_id_raw(1);
    base.set_link_word_raw(0);
    base.set_chain_word_raw(1);
    base.set_coords_raw(coords);
    base.set_trailing_coords_raw(coords);
    base.set_owner_empire_raw(owner);
    game_data.bases.records.push(base);
}

#[test]
fn canonical_bombardment_consumes_order_and_devastates_target() {
    let mut game_data = load_fixture("ecmaint-bombard-arrive");

    let pre_target = game_data.planets.records[13].clone();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(events.bombard_events.len(), 1);
    assert_eq!(events.bombard_events[0].planet_idx, 13);
    assert_eq!(events.bombard_events[0].attacker_empire_raw, 1);
    assert_eq!(events.planet_intel_events.len(), 2);
    assert!(
        events
            .planet_intel_events
            .iter()
            .any(|event| event.planet_idx == 13 && event.viewer_empire_raw == 1)
    );
    assert!(
        events
            .planet_intel_events
            .iter()
            .any(|event| event.planet_idx == 13 && event.viewer_empire_raw == 2)
    );
    assert!(events.colonization_events.is_empty());

    let attacker = &game_data.fleets.records[2];
    assert_eq!(attacker.current_location_coords_raw(), [15, 13]);
    assert_eq!(attacker.standing_order_code_raw(), 0);
    assert_eq!(attacker.current_speed(), 0);
    assert_eq!(attacker.cruiser_count(), 0);
    assert_eq!(attacker.destroyer_count(), 0);

    let post_target = &game_data.planets.records[13];
    assert_eq!(
        post_target.owner_empire_slot_raw(),
        pre_target.owner_empire_slot_raw()
    );
    assert_eq!(post_target.army_count_raw(), 5);
    assert_eq!(post_target.ground_batteries_raw(), 0);
    assert!(post_target.army_count_raw() < pre_target.army_count_raw());
}

#[test]
fn canonical_fleet_battle_removes_losers_without_garbage_counts() {
    let mut game_data = load_fixture("ecmaint-fleet-battle-pre");
    game_data.fleets.records[0].set_standing_order_code_raw(10);
    game_data.fleets.records[0].set_scout_count(1);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let loser_one = &game_data.fleets.records[0];
    let loser_two = &game_data.fleets.records[2];
    assert_eq!(loser_one.current_location_coords_raw(), [10, 10]);
    assert_eq!(loser_one.standing_order_code_raw(), 0);
    assert_eq!(loser_one.rules_of_engagement(), 0);
    assert_eq!(loser_one.battleship_count(), 0);
    assert_eq!(loser_one.cruiser_count(), 0);
    assert_eq!(loser_one.destroyer_count(), 0);
    assert_eq!(loser_one.troop_transport_count(), 0);

    assert_eq!(loser_two.current_location_coords_raw(), [10, 10]);
    assert_eq!(loser_two.standing_order_code_raw(), 0);
    assert_eq!(loser_two.rules_of_engagement(), 0);
    assert_eq!(loser_two.battleship_count(), 0);
    assert_eq!(loser_two.cruiser_count(), 0);
    assert_eq!(loser_two.destroyer_count(), 0);
    assert_eq!(loser_two.troop_transport_count(), 0);

    assert!(events.mission_resolution_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == MissionResolutionKind::ScoutSector
            && event.outcome == MissionResolutionOutcome::Aborted
            && event.location_coords == Some([10, 10])
    }));
    assert!(events.scout_contact_events.iter().any(|event| {
        event.viewer_empire_raw == 1
            && event.source == ContactReportSource::FleetMission(MissionResolutionKind::ScoutSector)
            && event.coords == [10, 10]
            && event.target_empire_raw == 2
    }));

    let survivor = &game_data.fleets.records[6];
    assert_eq!(survivor.current_location_coords_raw(), [10, 10]);
    assert_eq!(survivor.battleship_count(), 1);
    assert_eq!(survivor.scout_count(), 10);
    assert_eq!(survivor.etac_count(), 1);

    for fleet in &game_data.fleets.records {
        assert!(fleet.battleship_count() <= 100);
        assert!(fleet.cruiser_count() <= 100);
        assert!(fleet.destroyer_count() <= 100);
        assert!(fleet.troop_transport_count() <= 100);
    }
}

#[test]
fn canonical_three_empire_open_space_battle_resolves_deterministically() {
    let mut game_data = load_fixture("ecmaint-post");

    let fleet_a = &mut game_data.fleets.records[0];
    fleet_a.set_current_location_coords_raw([15, 13]);
    fleet_a.set_standing_order_code_raw(0);
    fleet_a.set_destroyer_count(1);
    fleet_a.set_cruiser_count(0);
    fleet_a.set_battleship_count(0);
    fleet_a.set_troop_transport_count(0);
    fleet_a.set_army_count(0);
    fleet_a.set_scout_count(0);
    fleet_a.set_etac_count(0);
    fleet_a.set_rules_of_engagement(6);

    let fleet_b = &mut game_data.fleets.records[4];
    fleet_b.set_current_location_coords_raw([15, 13]);
    fleet_b.set_standing_order_code_raw(0);
    fleet_b.set_destroyer_count(1);
    fleet_b.set_cruiser_count(0);
    fleet_b.set_battleship_count(0);
    fleet_b.set_troop_transport_count(0);
    fleet_b.set_army_count(0);
    fleet_b.set_scout_count(0);
    fleet_b.set_etac_count(0);
    fleet_b.set_rules_of_engagement(6);

    let fleet_c = &mut game_data.fleets.records[8];
    fleet_c.set_current_location_coords_raw([15, 13]);
    fleet_c.set_standing_order_code_raw(0);
    fleet_c.set_destroyer_count(0);
    fleet_c.set_cruiser_count(0);
    fleet_c.set_battleship_count(1);
    fleet_c.set_troop_transport_count(0);
    fleet_c.set_army_count(0);
    fleet_c.set_scout_count(0);
    fleet_c.set_etac_count(0);
    fleet_c.set_rules_of_engagement(6);

    run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let fleet_a = &game_data.fleets.records[0];
    let fleet_b = &game_data.fleets.records[4];
    let fleet_c = &game_data.fleets.records[8];

    assert_eq!(fleet_a.destroyer_count(), 0);
    assert_eq!(fleet_a.standing_order_code_raw(), 0);
    assert_eq!(fleet_a.rules_of_engagement(), 0);

    assert_eq!(fleet_b.destroyer_count(), 0);
    assert_eq!(fleet_b.standing_order_code_raw(), 0);
    assert_eq!(fleet_b.rules_of_engagement(), 0);

    assert_eq!(fleet_c.battleship_count(), 1);
    assert_eq!(fleet_c.current_location_coords_raw(), [15, 13]);
    assert_eq!(fleet_c.standing_order_code_raw(), 0);
    assert_eq!(fleet_c.rules_of_engagement(), 6);
}

#[test]
fn canonical_starbase_defender_repels_orbital_attacker() {
    let mut game_data = load_fixture("ecmaint-post");
    add_active_starbase(&mut game_data, 1, [16, 13]);

    let defender = &mut game_data.fleets.records[0];
    defender.set_current_location_coords_raw([16, 13]);
    defender.set_standing_order_code_raw(4);
    defender.set_standing_order_target_coords_raw([16, 13]);
    defender.set_cruiser_count(1);
    defender.set_destroyer_count(0);
    defender.set_battleship_count(0);
    defender.set_troop_transport_count(0);
    defender.set_army_count(0);
    defender.set_scout_count(0);
    defender.set_etac_count(0);
    defender.set_rules_of_engagement(6);

    let attacker = &mut game_data.fleets.records[4];
    attacker.set_current_location_coords_raw([16, 13]);
    attacker.set_standing_order_code_raw(0);
    attacker.set_standing_order_target_coords_raw([16, 13]);
    attacker.set_cruiser_count(1);
    attacker.set_destroyer_count(0);
    attacker.set_battleship_count(0);
    attacker.set_troop_transport_count(0);
    attacker.set_army_count(0);
    attacker.set_scout_count(0);
    attacker.set_etac_count(0);
    attacker.set_rules_of_engagement(6);

    run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let attacker = &game_data.fleets.records[4];
    assert_eq!(attacker.cruiser_count(), 0);
    assert_eq!(attacker.standing_order_code_raw(), 0);
    assert_eq!(attacker.rules_of_engagement(), 0);

    assert_eq!(game_data.bases.records.len(), 1);
    assert_eq!(game_data.bases.records[0].owner_empire_raw(), 1);
    assert_eq!(game_data.bases.records[0].coords_raw(), [16, 13]);
}

#[test]
fn canonical_invade_failure_removes_attacker_armies_and_holds_planet() {
    let mut game_data = configured_assault_state(7);

    {
        let attacker = &mut game_data.fleets.records[0];
        attacker.set_troop_transport_count(2);
        attacker.set_army_count(2);
    }
    {
        let target = &mut game_data.planets.records[13];
        target.set_army_count_raw(10);
        target.set_ground_batteries_raw(4);
    }

    run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let attacker = &game_data.fleets.records[0];
    let target = &game_data.planets.records[13];
    assert_eq!(attacker.standing_order_code_raw(), 0);
    assert_eq!(attacker.current_speed(), 0);
    assert_eq!(attacker.army_count(), 0);
    assert_eq!(target.owner_empire_slot_raw(), 2);
    assert_eq!(target.ownership_status_raw(), 2);
    assert_eq!(target.army_count_raw(), 10);
    assert_eq!(target.ground_batteries_raw(), 4);
}

#[test]
fn canonical_blitz_success_transfers_surviving_batteries() {
    let mut game_data = configured_assault_state(8);

    {
        let attacker = &mut game_data.fleets.records[0];
        attacker.set_troop_transport_count(15);
        attacker.set_army_count(20);
    }
    {
        let target = &mut game_data.planets.records[13];
        target.set_army_count_raw(3);
        target.set_ground_batteries_raw(1);
    }

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let attacker = &game_data.fleets.records[0];
    let target = &game_data.planets.records[13];
    assert_eq!(attacker.standing_order_code_raw(), 0);
    assert_eq!(attacker.current_speed(), 0);
    assert_eq!(attacker.army_count(), 0);
    assert_eq!(target.owner_empire_slot_raw(), 1);
    assert_eq!(target.ownership_status_raw(), 2);
    assert_eq!(target.ground_batteries_raw(), 1);
    assert_eq!(target.army_count_raw(), 10);
    assert_eq!(events.ownership_change_events.len(), 1);
    assert_eq!(events.ownership_change_events[0].planet_idx, 13);
    assert_eq!(
        events.ownership_change_events[0].previous_owner_empire_raw,
        2
    );
    assert_eq!(events.ownership_change_events[0].new_owner_empire_raw, 1);
    assert!(
        events
            .planet_intel_events
            .iter()
            .any(|event| event.planet_idx == 13 && event.viewer_empire_raw == 1)
    );
    assert!(
        events
            .planet_intel_events
            .iter()
            .any(|event| event.planet_idx == 13 && event.viewer_empire_raw == 2)
    );
    assert!(events.colonization_events.is_empty());
}

#[test]
fn canonical_blitz_failure_leaves_defender_in_control() {
    let mut game_data = configured_assault_state(8);

    {
        let attacker = &mut game_data.fleets.records[0];
        attacker.set_troop_transport_count(4);
        attacker.set_army_count(4);
    }
    {
        let target = &mut game_data.planets.records[13];
        target.set_army_count_raw(10);
        target.set_ground_batteries_raw(2);
    }

    run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let attacker = &game_data.fleets.records[0];
    let target = &game_data.planets.records[13];
    assert_eq!(attacker.standing_order_code_raw(), 0);
    assert_eq!(attacker.current_speed(), 0);
    assert_eq!(attacker.army_count(), 0);
    assert_eq!(target.owner_empire_slot_raw(), 2);
    assert_eq!(target.ownership_status_raw(), 2);
    assert_eq!(target.army_count_raw(), 10);
    assert_eq!(target.ground_batteries_raw(), 2);
}
