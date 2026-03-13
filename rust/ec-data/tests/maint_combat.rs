mod common;

use ec_data::{run_maintenance_turn, CoreGameData};
use std::path::Path;

fn load_fixture(name: &str) -> CoreGameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
        .join("v1.5");
    CoreGameData::load(&dir).unwrap_or_else(|e| panic!("Failed to load fixture {}: {}", name, e))
}

#[test]
fn canonical_bombardment_consumes_order_and_devastates_target() {
    let mut game_data = load_fixture("ecmaint-bombard-arrive");

    let pre_target = game_data.planets.records[13].clone();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(events.bombard_events.len(), 1);
    assert_eq!(events.bombard_events[0].planet_idx, 13);
    assert_eq!(events.bombard_events[0].attacker_empire_raw, 1);

    let attacker = &game_data.fleets.records[2];
    assert_eq!(attacker.current_location_coords_raw(), [15, 13]);
    assert_eq!(attacker.standing_order_code_raw(), 0);
    assert_eq!(attacker.current_speed(), 0);
    assert_eq!(attacker.cruiser_count(), 0);
    assert_eq!(attacker.destroyer_count(), 0);

    let post_target = &game_data.planets.records[13];
    assert_eq!(post_target.owner_empire_slot_raw(), pre_target.owner_empire_slot_raw());
    assert_eq!(post_target.army_count_raw(), 5);
    assert_eq!(post_target.ground_batteries_raw(), 0);
    assert!(post_target.army_count_raw() < pre_target.army_count_raw());
}

#[test]
fn canonical_fleet_battle_removes_losers_without_garbage_counts() {
    let mut game_data = load_fixture("ecmaint-fleet-battle-pre");

    run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

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
