use nc_data::{
    ContactReportSource, EncounterDispositionEvent, EncounterDispositionReason, FleetBattleEvent,
    FleetDestroyedEvent, GameStateBuilder, JoinMissionHostEvent, MaintenanceEvents, Mission,
    MissionEvent, MissionOutcome, MissionRetargetEvent, ScoutContactEvent, ShipLosses,
};
use nc_engine::{build_results_report_blocks, maint::FleetBattlePerspective};

fn viewer_report_texts(viewer_empire_id: u8, rows: &[nc_data::ReportBlockRow]) -> Vec<String> {
    rows.iter()
        .filter(|row| row.viewer_empire_id == viewer_empire_id)
        .map(|row| row.decoded_text.clone())
        .collect()
}

fn seeded_game_data() -> nc_data::CoreGameData {
    GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3018)
        .build_initialized_baseline()
        .expect("baseline should build")
}

fn configure_fleet(
    game_data: &mut nc_data::CoreGameData,
    fleet_idx: usize,
    owner_empire_raw: u8,
    fleet_number: u8,
    coords: [u8; 2],
) {
    let fleet = &mut game_data.fleets.records[fleet_idx];
    fleet.set_owner_empire_raw(owner_empire_raw);
    fleet.set_local_slot_word_raw(fleet_number as u16);
    fleet.set_current_location_coords_raw(coords);
}

#[test]
fn fleet_destroyed_event_supersedes_generic_battle_report() {
    let mut game_data = seeded_game_data();
    configure_fleet(&mut game_data, 0, 1, 11, [6, 6]);

    let mut events = MaintenanceEvents::default();
    events.fleet_battle_events.push(FleetBattleEvent {
        reporting_empire_raw: 1,
        reporting_fleet_number: Some(11),
        reporting_mission: Some(Mission::GuardBlockadeWorld),
        perspective: FleetBattlePerspective::Intercepted,
        coords: [6, 6],
        enemy_empires_raw: vec![2],
        primary_enemy_fleet_number: Some(7),
        held_field: false,
        friendly_initial: ShipLosses {
            cruisers: 1,
            ..ShipLosses::default()
        },
        friendly_initial_starbases: 0,
        friendly_loaded_armies_initial: 0,
        friendly_losses: ShipLosses {
            cruisers: 1,
            ..ShipLosses::default()
        },
        friendly_starbases_lost: 0,
        enemy_initial: ShipLosses {
            destroyers: 2,
            ..ShipLosses::default()
        },
        enemy_initial_starbases: 0,
        enemy_loaded_armies_initial: 0,
        enemy_losses: ShipLosses::default(),
        enemy_starbases_destroyed: 0,
        stardate_week: Some(1),
    });
    events.fleet_destroyed_events.push(FleetDestroyedEvent {
        reporting_empire_raw: 1,
        fleet_number: 11,
        coords: [6, 6],
        was_intercepting: true,
        friendly_initial: ShipLosses {
            cruisers: 1,
            ..ShipLosses::default()
        },
        friendly_loaded_armies_initial: 0,
        enemy_initial: ShipLosses {
            destroyers: 2,
            ..ShipLosses::default()
        },
        enemy_initial_starbases: 0,
        enemy_loaded_armies_initial: 0,
        enemy_losses: ShipLosses::default(),
        enemy_starbases_destroyed: 0,
        primary_enemy_empire_raw: Some(2),
        primary_enemy_fleet_number: Some(7),
        stardate_week: Some(1),
    });

    let texts = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events));
    assert_eq!(
        texts.len(),
        1,
        "destroyed fleet should suppress generic battle: {texts:?}"
    );
    assert!(texts[0].contains("We lost all contact with the 11th Fleet"));
    assert!(!texts[0].contains("We successfully intercepted"));
}

#[test]
fn roe_retreat_and_abort_merge_into_one_report() {
    let mut game_data = seeded_game_data();
    configure_fleet(&mut game_data, 0, 1, 12, [6, 6]);

    let mut events = MaintenanceEvents::default();
    events.mission_events.push(MissionEvent {
        fleet_idx: 0,
        owner_empire_raw: 1,
        kind: Mission::InvadeWorld,
        outcome: MissionOutcome::Aborted,
        planet_idx: None,
        location_coords: Some([6, 6]),
        target_coords: Some([6, 6]),
        stardate_week: Some(1),
    });
    events
        .encounter_disposition_events
        .push(EncounterDispositionEvent::Retreated {
            fleet_idx: 0,
            owner_empire_raw: 1,
            mission: Some(Mission::InvadeWorld),
            coords: [6, 6],
            friendly_initial: ShipLosses {
                cruisers: 1,
                transports: 2,
                ..ShipLosses::default()
            },
            friendly_loaded_armies_initial: 3,
            target_empire_raw: 2,
            target_fleet_number: Some(3),
            enemy_initial: ShipLosses {
                cruisers: 2,
                ..ShipLosses::default()
            },
            retreat_target_coords: [5, 5],
            losses_sustained: ShipLosses {
                destroyers: 1,
                ..ShipLosses::default()
            },
            enemy_losses_inflicted: ShipLosses::default(),
            reason: EncounterDispositionReason::RoeWithdrawal,
            stardate_week: Some(1),
        });

    let texts = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events));
    let joined = texts.join(" ").replace('\n', " ");
    assert_eq!(texts.len(), 1, "ROE-backed abort should merge: {texts:?}");
    assert!(joined.contains("Invasion mission report"));
    assert!(joined.contains("In accordance with our ROE"));
    assert!(joined.contains("abort the invasion"));
    assert!(joined.contains("We had 1 cruiser and 2 troop transport ships carrying 3 armies."));
}

#[test]
fn unknown_join_host_never_renders_zero_fleet_number() {
    let mut game_data = seeded_game_data();
    configure_fleet(&mut game_data, 0, 1, 10, [3, 9]);

    let mut events = MaintenanceEvents::default();
    events
        .join_host_events
        .push(JoinMissionHostEvent::HostDestroyed {
            fleet_idx: 0,
            owner_empire_raw: 1,
            destroyed_host_fleet_number: None,
            coords: [3, 9],
        });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("Our intended host fleet was destroyed."));
    assert!(!text.contains("(0th Fleet)"));
}

#[test]
fn retarget_report_source_uses_current_location_not_new_target() {
    let mut game_data = seeded_game_data();
    configure_fleet(&mut game_data, 0, 1, 11, [6, 6]);

    let mut events = MaintenanceEvents::default();
    events
        .mission_retarget_events
        .push(MissionRetargetEvent::Retargeted {
            fleet_idx: 0,
            reporting_fleet_number: Some(11),
            owner_empire_raw: 1,
            mission: Mission::JoinAnotherFleet,
            current_coords: [6, 6],
            previous_target_coords: [4, 4],
            new_target_coords: [8, 8],
        });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("From your 11th Fleet, located in Sector(6,6):"));
    assert!(text.contains("continuing pursuit to Sector(8,8)"));
    assert!(!text.contains("located in Sector(8,8):"));
}

#[test]
fn retarget_report_source_uses_stored_reporting_fleet_number() {
    let mut game_data = seeded_game_data();
    configure_fleet(&mut game_data, 0, 1, 99, [6, 6]);

    let mut events = MaintenanceEvents::default();
    events
        .mission_retarget_events
        .push(MissionRetargetEvent::Retargeted {
            fleet_idx: 0,
            reporting_fleet_number: Some(11),
            owner_empire_raw: 1,
            mission: Mission::JoinAnotherFleet,
            current_coords: [6, 6],
            previous_target_coords: [4, 4],
            new_target_coords: [8, 8],
        });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("From your 11th Fleet, located in Sector(6,6):"));
    assert!(!text.contains("From your 99th Fleet"));
}

#[test]
fn destroyed_fleet_telemetry_reports_starbase_only_opponent() {
    let mut game_data = seeded_game_data();
    configure_fleet(&mut game_data, 0, 1, 20, [11, 8]);

    let mut events = MaintenanceEvents::default();
    events.fleet_destroyed_events.push(FleetDestroyedEvent {
        reporting_empire_raw: 1,
        fleet_number: 20,
        coords: [11, 8],
        was_intercepting: true,
        friendly_initial: ShipLosses {
            cruisers: 1,
            ..ShipLosses::default()
        },
        friendly_loaded_armies_initial: 0,
        enemy_initial: ShipLosses::default(),
        enemy_initial_starbases: 1,
        enemy_loaded_armies_initial: 0,
        enemy_losses: ShipLosses::default(),
        enemy_starbases_destroyed: 0,
        primary_enemy_empire_raw: Some(2),
        primary_enemy_fleet_number: None,
        stardate_week: Some(2),
    });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("alien force contained 1 starbase"));
    assert!(!text.contains("alien force contained no ships"));
}

#[test]
fn no_engagement_report_includes_reporting_fleet_composition() {
    let mut game_data = seeded_game_data();
    configure_fleet(&mut game_data, 0, 1, 8, [9, 5]);

    let mut events = MaintenanceEvents::default();
    events
        .encounter_disposition_events
        .push(EncounterDispositionEvent::NoEngagement {
            fleet_idx: 0,
            owner_empire_raw: 1,
            mission: Some(Mission::ScoutSector),
            coords: [9, 5],
            friendly_initial: ShipLosses {
                cruisers: 1,
                destroyers: 1,
                ..ShipLosses::default()
            },
            friendly_loaded_armies_initial: 0,
            target_empire_raw: 2,
            target_fleet_number: Some(4),
            small_vessels: 1,
            medium_vessels: 2,
            large_vessels: 1,
            reason: EncounterDispositionReason::RoeDeclined,
            stardate_week: Some(2),
        });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("We had 1 cruiser and 1 destroyer."));
    assert!(text.contains("Their fleet contains"));
}

#[test]
fn fleet_contact_report_includes_reporting_fleet_composition() {
    let mut game_data = seeded_game_data();
    configure_fleet(&mut game_data, 0, 1, 13, [24, 14]);

    let mut events = MaintenanceEvents::default();
    events.scout_contact_events.push(ScoutContactEvent {
        viewer_empire_raw: 1,
        source: ContactReportSource::FleetMission(Mission::ScoutSector),
        reporting_fleet_number: Some(13),
        reporting_initial: ShipLosses {
            scouts: 1,
            ..ShipLosses::default()
        },
        reporting_loaded_armies_initial: 0,
        coords: [24, 14],
        target_empire_raw: 2,
        target_fleet_number: Some(5),
        small_vessels: 2,
        medium_vessels: 0,
        large_vessels: 0,
        stardate_week: Some(52),
    });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("We had 1 scout ship."));
    assert!(text.contains("Their fleet contains"));
}

#[test]
fn join_host_retarget_report_describes_host_merge_not_movement() {
    let mut game_data = seeded_game_data();
    configure_fleet(&mut game_data, 0, 1, 12, [2, 7]);

    let mut events = MaintenanceEvents::default();
    events
        .join_host_events
        .push(JoinMissionHostEvent::Retargeted {
            fleet_idx: 0,
            owner_empire_raw: 1,
            previous_host_fleet_number: Some(14),
            new_host_fleet_number: Some(2),
            coords: [2, 7],
        });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("Our intended host fleet (14th Fleet) merged into the 2nd Fleet."));
    assert!(text.contains("joining that surviving fleet instead"));
    assert!(!text.contains("has moved"));
}
