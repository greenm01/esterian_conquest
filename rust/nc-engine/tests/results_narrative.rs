use nc_data::{
    AssaultReportEvent, BombardEvent, ContactReportSource, FleetBattleEvent, FleetDestroyedEvent,
    GameStateBuilder, MaintenanceEvents, Mission, MissionOutcome, PlanetRecord, ScoutContactEvent,
    ShipLosses,
};
use nc_engine::{
    apply_results_reviewable_flags, build_results_report_blocks, maint::FleetBattlePerspective,
    run_maintenance_turn,
};
use std::path::Path;

fn viewer_report_texts(viewer_empire_id: u8, rows: &[nc_data::ReportBlockRow]) -> Vec<String> {
    rows.iter()
        .filter(|row| row.viewer_empire_id == viewer_empire_id)
        .map(|row| row.decoded_text.clone())
        .collect()
}

fn seed_target_world(game_data: &mut nc_data::CoreGameData, coords: [u8; 2], name: &str) {
    let mut planet = PlanetRecord::new_zeroed();
    planet.set_coords_raw(coords);
    planet.set_planet_name(name);
    planet.set_owner_empire_slot_raw(1);
    planet.set_ownership_status_raw(2);
    planet.set_potential_production_raw(100u16.to_le_bytes());
    game_data.planets.records[0] = planet;
}

fn load_fixture(name: &str) -> nc_data::CoreGameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
        .join("v1.5");
    nc_data::CoreGameData::load(&dir)
        .unwrap_or_else(|e| panic!("Failed to load fixture {name}: {e}"))
}

#[test]
fn results_reports_contact_before_destroyed_fleet_notice() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3001)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [5, 13];
    seed_target_world(&mut game_data, coords, "Target");

    let mut events = MaintenanceEvents::default();
    events.scout_contact_events.push(ScoutContactEvent {
        viewer_empire_raw: 1,
        source: ContactReportSource::FleetMission(Mission::ScoutSector),
        reporting_fleet_number: Some(15),
        coords,
        target_empire_raw: 3,
        target_fleet_number: Some(4),
        small_vessels: 2,
        medium_vessels: 2,
        large_vessels: 0,
        stardate_week: Some(2),
    });
    events.fleet_destroyed_events.push(FleetDestroyedEvent {
        reporting_empire_raw: 1,
        fleet_number: 15,
        coords,
        was_intercepting: true,
        friendly_initial: ShipLosses {
            destroyers: 1,
            ..ShipLosses::default()
        },
        friendly_loaded_armies_initial: 0,
        enemy_initial: ShipLosses {
            cruisers: 2,
            destroyers: 2,
            etacs: 2,
            ..ShipLosses::default()
        },
        enemy_loaded_armies_initial: 0,
        enemy_losses: ShipLosses::default(),
        primary_enemy_empire_raw: Some(3),
        primary_enemy_fleet_number: Some(4),
        stardate_week: Some(2),
    });

    let rows = build_results_report_blocks(&mut game_data, &events);
    let texts = viewer_report_texts(1, &rows);
    let contact_idx = texts
        .iter()
        .position(|text| text.contains("Sensor contact"))
        .expect("merged contact report should exist");
    let destroyed_idx = texts
        .iter()
        .position(|text| text.contains("We lost all contact with the 15th Fleet"))
        .expect("lost-contact report should exist");
    assert!(
        contact_idx < destroyed_idx,
        "contact should precede lost-contact: {texts:?}"
    );
}

#[test]
fn results_reports_battle_before_bombard_aftermath() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3001)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [5, 13];
    seed_target_world(&mut game_data, coords, "Target");

    let mut events = MaintenanceEvents::default();
    events.scout_contact_events.push(ScoutContactEvent {
        viewer_empire_raw: 1,
        source: ContactReportSource::FleetMission(Mission::GuardBlockadeWorld),
        reporting_fleet_number: Some(7),
        coords,
        target_empire_raw: 2,
        target_fleet_number: Some(9),
        small_vessels: 1,
        medium_vessels: 1,
        large_vessels: 0,
        stardate_week: Some(2),
    });
    events.fleet_battle_events.push(FleetBattleEvent {
        reporting_empire_raw: 1,
        reporting_fleet_number: Some(7),
        reporting_mission: Some(Mission::GuardBlockadeWorld),
        perspective: FleetBattlePerspective::Intercepted,
        coords,
        enemy_empires_raw: vec![2],
        primary_enemy_fleet_number: Some(9),
        held_field: true,
        friendly_initial: ShipLosses {
            cruisers: 2,
            destroyers: 1,
            ..ShipLosses::default()
        },
        friendly_loaded_armies_initial: 0,
        friendly_losses: ShipLosses::default(),
        friendly_starbases_lost: 0,
        enemy_initial: ShipLosses {
            destroyers: 1,
            ..ShipLosses::default()
        },
        enemy_initial_starbases: 0,
        enemy_loaded_armies_initial: 0,
        enemy_losses: ShipLosses {
            destroyers: 1,
            ..ShipLosses::default()
        },
        enemy_starbases_destroyed: 0,
        stardate_week: Some(2),
    });
    events.bombard_events.push(BombardEvent {
        planet_idx: 0,
        attacker_empire_raw: 2,
        attacker_fleet_number: Some(9),
        defender_empire_raw: 1,
        attacker_initial: ShipLosses {
            destroyers: 1,
            ..ShipLosses::default()
        },
        defender_batteries_initial: 4,
        defender_armies_initial: 10,
        attacker_losses: ShipLosses::default(),
        defender_battery_losses: 2,
        defender_army_losses: 3,
        breakthrough: true,
        docked_losses: nc_data::EmpireUnitSummary::default(),
        stardock_items_destroyed: 0,
        stored_goods_destroyed: 0,
        factories_destroyed: 0,
        stardate_week: Some(2),
    });

    let rows = build_results_report_blocks(&mut game_data, &events);
    let texts = viewer_report_texts(1, &rows);
    let contact_idx = texts
        .iter()
        .position(|text| text.contains("Sensor contact"))
        .expect("merged contact report should exist");
    let battle_idx = texts
        .iter()
        .position(|text| text.contains("We successfully intercepted"))
        .expect("battle report should exist");
    let bombard_idx = texts
        .iter()
        .position(|text| text.contains("Our world has been bombarded"))
        .expect("bombard report should exist");
    assert!(
        contact_idx < battle_idx,
        "contact should precede battle: {texts:?}"
    );
    assert!(
        battle_idx < bombard_idx,
        "battle should precede bombard: {texts:?}"
    );
}

#[test]
fn bombardment_defender_report_uses_first_person_loss_wording() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3001)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [5, 13];
    seed_target_world(&mut game_data, coords, "Target");

    let mut events = MaintenanceEvents::default();
    events.bombard_events.push(BombardEvent {
        planet_idx: 0,
        attacker_empire_raw: 2,
        attacker_fleet_number: Some(9),
        defender_empire_raw: 1,
        attacker_initial: ShipLosses {
            destroyers: 1,
            ..ShipLosses::default()
        },
        defender_batteries_initial: 0,
        defender_armies_initial: 9,
        attacker_losses: ShipLosses::default(),
        defender_battery_losses: 0,
        defender_army_losses: 3,
        breakthrough: true,
        docked_losses: nc_data::EmpireUnitSummary::default(),
        stardock_items_destroyed: 2,
        stored_goods_destroyed: 0,
        factories_destroyed: 0,
        stardate_week: Some(3),
    });

    let rows = build_results_report_blocks(&game_data, &events);
    let texts = viewer_report_texts(1, &rows);
    let bombard = texts
        .iter()
        .find(|text| text.contains("bombarded"))
        .expect("bombard report should exist");

    assert!(bombard.contains("Our world has been bombarded by"));
    assert!(bombard.contains("We lost"));
    assert!(bombard.contains("3 armies"));
    assert!(bombard.contains("We also lost"));
    assert!(bombard.contains("2 stardock items"));
    assert!(!bombard.contains("Bombardment also destroyed"));
}

#[test]
fn blitz_report_distinguishes_total_army_losses_from_transport_losses() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3014)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [8, 2];
    seed_target_world(&mut game_data, coords, "half");
    game_data.planets.records[0].set_ground_batteries_raw(2);
    game_data.planets.records[0].set_army_count_raw(5);

    let mut events = MaintenanceEvents::default();
    events.assault_report_events.push(AssaultReportEvent {
        kind: Mission::BlitzWorld,
        attacker_fleet_number: Some(4),
        planet_idx: 0,
        attacker_empire_raw: 1,
        defender_empire_raw: 2,
        attacker_initial: ShipLosses {
            cruisers: 1,
            ..ShipLosses::default()
        },
        defender_batteries_initial: 2,
        defender_armies_initial: 5,
        attacker_ship_losses: ShipLosses::default(),
        attacker_army_losses: 3,
        transport_army_losses: 0,
        defender_battery_losses: 2,
        defender_army_losses: 5,
        outcome: MissionOutcome::Succeeded,
        stardate_week: Some(3),
    });

    let rows = build_results_report_blocks(&game_data, &events);
    let blitz = viewer_report_texts(1, &rows).join(" ");

    assert!(blitz.contains("Friendly losses: no ship losses and 3 armies."));
    assert!(blitz.contains("No troops were lost in"));
    assert!(blitz.contains("destroyed troop transports during the landing."));
    assert!(!blitz.contains("No troops were lost during the landing."));
}

#[test]
fn results_reports_named_hostile_fleet_with_empire_local_slot() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3001)
        .build_initialized_baseline()
        .expect("baseline should build");

    let rows = build_results_report_blocks(
        &mut game_data,
        &MaintenanceEvents {
            fleet_battle_events: vec![FleetBattleEvent {
                reporting_empire_raw: 1,
                reporting_fleet_number: Some(3),
                reporting_mission: Some(Mission::PatrolSector),
                perspective: FleetBattlePerspective::Attacked,
                coords: [8, 8],
                enemy_empires_raw: vec![2],
                primary_enemy_fleet_number: Some(2),
                held_field: false,
                friendly_initial: ShipLosses {
                    destroyers: 1,
                    ..ShipLosses::default()
                },
                friendly_loaded_armies_initial: 0,
                friendly_losses: ShipLosses {
                    destroyers: 1,
                    ..ShipLosses::default()
                },
                friendly_starbases_lost: 0,
                enemy_initial: ShipLosses {
                    cruisers: 1,
                    ..ShipLosses::default()
                },
                enemy_initial_starbases: 0,
                enemy_loaded_armies_initial: 0,
                enemy_losses: ShipLosses::default(),
                enemy_starbases_destroyed: 0,
                stardate_week: Some(2),
            }],
            ..MaintenanceEvents::default()
        },
    );

    let texts = viewer_report_texts(1, &rows);
    assert!(
        texts
            .iter()
            .any(|text| text.contains("2nd Fleet") && !text.contains("5th Fleet")),
        "hostile fleet references should use empire-local fleet numbers: {texts:?}"
    );
}

#[test]
fn results_report_invalid_capability_loss_as_aborted_seek_home_mission() {
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    game_data.fleets.records[0].set_etac_count(0);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");
    let rows = build_results_report_blocks(&mut game_data, &events);
    let texts = viewer_report_texts(1, &rows);

    assert!(texts.iter().any(|text| {
        text.contains("Maintenance aborted this fleet's colonize world mission because")
            && text.contains("lacks the required ETAC")
            && text.contains("seeking safety")
    }));
}

#[test]
fn results_projection_is_pure_until_reviewable_flags_are_applied() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3001)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [5, 13];
    seed_target_world(&mut game_data, coords, "Target");

    let mut events = MaintenanceEvents::default();
    events.scout_contact_events.push(ScoutContactEvent {
        viewer_empire_raw: 1,
        source: ContactReportSource::FleetMission(Mission::ScoutSector),
        reporting_fleet_number: Some(15),
        coords,
        target_empire_raw: 3,
        target_fleet_number: Some(4),
        small_vessels: 2,
        medium_vessels: 0,
        large_vessels: 0,
        stardate_week: Some(2),
    });

    let rows = build_results_report_blocks(&game_data, &events);
    assert!(
        game_data
            .player
            .records
            .iter()
            .all(|player| !player.has_classic_results_review_state()),
        "report projection should not mutate reviewable flags"
    );

    apply_results_reviewable_flags(&mut game_data, &rows);
    assert!(game_data.player.records[0].has_classic_results_review_state());
    assert!(
        game_data
            .player
            .records
            .iter()
            .skip(1)
            .all(|player| !player.has_classic_results_review_state())
    );
}
