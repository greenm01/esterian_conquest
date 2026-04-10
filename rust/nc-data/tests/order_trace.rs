use nc_data::{
    BaseDat, BaseRecord, CoreGameData, GameStateBuilder, MaintenanceEvents, Mission,
    MissionOutcome, Order, fleet_motion_state::reset_motion_state_for_new_orders,
};
use nc_engine::run_maintenance_turn;

const TRACE_START: [u8; 2] = [8, 8];
const TRACE_TARGET: [u8; 2] = [11, 8];
const TRACE_SPEED: u8 = 3;
const TARGET_PLANET_IDX: usize = 4;

#[derive(Clone, Copy)]
struct FleetTurnExpectation {
    turn: usize,
    coords: [u8; 2],
    target: [u8; 2],
    order: Order,
    speed: u8,
    ready_flag: Option<u8>,
}

struct TraceCase {
    name: &'static str,
    setup: fn() -> CoreGameData,
    fleet_idx: usize,
    turns_to_run: usize,
    expectations: &'static [FleetTurnExpectation],
    check: fn(&TraceRun),
}

struct TraceRun {
    states: Vec<CoreGameData>,
    events: Vec<MaintenanceEvents>,
}

#[derive(Clone, Copy)]
enum TargetWorldOwner {
    Unowned,
    Friendly,
    Foreign,
}

#[test]
fn one_shot_order_traces_monitor_speed_and_resolution() {
    let cases = [
        TraceCase {
            name: "move-only",
            setup: build_move_only_trace,
            fleet_idx: 0,
            turns_to_run: 2,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::MoveOnly,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: TRACE_TARGET,
                    order: Order::MoveOnly,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::HoldPosition,
                    speed: 0,
                    ready_flag: Some(0x80),
                },
            ],
            check: check_move_only_trace,
        },
        TraceCase {
            name: "colonize-world",
            setup: build_colonize_world_trace,
            fleet_idx: 0,
            turns_to_run: 2,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::ColonizeWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: TRACE_TARGET,
                    order: Order::ColonizeWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::HoldPosition,
                    speed: 0,
                    ready_flag: Some(0x80),
                },
            ],
            check: check_colonize_world_trace,
        },
        TraceCase {
            name: "view-world",
            setup: build_view_world_trace,
            fleet_idx: 0,
            turns_to_run: 2,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::ViewWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: TRACE_TARGET,
                    order: Order::ViewWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::HoldPosition,
                    speed: 0,
                    ready_flag: Some(0x80),
                },
            ],
            check: check_view_world_trace,
        },
        TraceCase {
            name: "seek-home",
            setup: build_seek_home_trace,
            fleet_idx: 0,
            turns_to_run: 2,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::SeekHome,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: TRACE_TARGET,
                    order: Order::SeekHome,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::HoldPosition,
                    speed: 0,
                    ready_flag: Some(0x80),
                },
            ],
            check: check_seek_home_trace,
        },
        TraceCase {
            name: "salvage",
            setup: build_salvage_trace,
            fleet_idx: 0,
            turns_to_run: 2,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::Salvage,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: TRACE_TARGET,
                    order: Order::Salvage,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
            ],
            check: check_salvage_trace,
        },
    ];

    for case in cases {
        run_trace_case(case);
    }
}

#[test]
fn persistent_order_traces_monitor_speed_on_arrival_and_on_station() {
    let cases = [
        TraceCase {
            name: "scout-sector",
            setup: build_scout_sector_trace,
            fleet_idx: 0,
            turns_to_run: 3,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::ScoutSector,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: TRACE_TARGET,
                    order: Order::ScoutSector,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::ScoutSector,
                    speed: 0,
                    ready_flag: Some(0x80),
                },
                FleetTurnExpectation {
                    turn: 3,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::ScoutSector,
                    speed: 0,
                    ready_flag: Some(0x80),
                },
            ],
            check: check_scout_sector_trace,
        },
        TraceCase {
            name: "scout-solar-system",
            setup: build_scout_system_trace,
            fleet_idx: 0,
            turns_to_run: 3,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::ScoutSolarSystem,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: TRACE_TARGET,
                    order: Order::ScoutSolarSystem,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::ScoutSolarSystem,
                    speed: 0,
                    ready_flag: Some(0x80),
                },
                FleetTurnExpectation {
                    turn: 3,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::ScoutSolarSystem,
                    speed: 0,
                    ready_flag: Some(0x80),
                },
            ],
            check: check_scout_system_trace,
        },
        TraceCase {
            name: "patrol-sector",
            setup: build_patrol_sector_trace,
            fleet_idx: 0,
            turns_to_run: 3,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::PatrolSector,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: TRACE_TARGET,
                    order: Order::PatrolSector,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::PatrolSector,
                    speed: 0,
                    ready_flag: Some(0x81),
                },
                FleetTurnExpectation {
                    turn: 3,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::PatrolSector,
                    speed: 0,
                    ready_flag: Some(0x81),
                },
            ],
            check: check_patrol_sector_trace,
        },
        TraceCase {
            name: "guard-starbase",
            setup: build_guard_starbase_trace,
            fleet_idx: 0,
            turns_to_run: 3,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::GuardStarbase,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: TRACE_TARGET,
                    order: Order::GuardStarbase,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::GuardStarbase,
                    speed: 0,
                    ready_flag: Some(0x00),
                },
                FleetTurnExpectation {
                    turn: 3,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::GuardStarbase,
                    speed: 0,
                    ready_flag: Some(0x00),
                },
            ],
            check: check_guard_starbase_trace,
        },
        TraceCase {
            name: "guard-blockade-world",
            setup: build_guard_blockade_world_trace,
            fleet_idx: 0,
            turns_to_run: 3,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::GuardBlockadeWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: TRACE_TARGET,
                    order: Order::GuardBlockadeWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::GuardBlockadeWorld,
                    speed: 0,
                    ready_flag: Some(0x81),
                },
                FleetTurnExpectation {
                    turn: 3,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::GuardBlockadeWorld,
                    speed: 0,
                    ready_flag: Some(0x81),
                },
            ],
            check: check_guard_blockade_trace,
        },
    ];

    for case in cases {
        run_trace_case(case);
    }
}

#[test]
fn hostile_order_traces_monitor_speed_through_delay_and_execution() {
    let cases = [
        TraceCase {
            name: "bombard-world",
            setup: build_bombard_world_trace,
            fleet_idx: 0,
            turns_to_run: 3,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::BombardWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: TRACE_TARGET,
                    order: Order::BombardWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::BombardWorld,
                    speed: 0,
                    ready_flag: Some(0x80),
                },
                FleetTurnExpectation {
                    turn: 3,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::BombardWorld,
                    speed: 0,
                    ready_flag: Some(0x80),
                },
            ],
            check: check_bombard_world_trace,
        },
        TraceCase {
            name: "invade-world",
            setup: build_invade_world_trace,
            fleet_idx: 0,
            turns_to_run: 3,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::InvadeWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: TRACE_TARGET,
                    order: Order::InvadeWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::InvadeWorld,
                    speed: 0,
                    ready_flag: Some(0x80),
                },
                FleetTurnExpectation {
                    turn: 3,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::HoldPosition,
                    speed: 0,
                    ready_flag: Some(0x81),
                },
            ],
            check: check_invade_world_trace,
        },
        TraceCase {
            name: "blitz-world",
            setup: build_blitz_world_trace,
            fleet_idx: 0,
            turns_to_run: 3,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::BlitzWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: TRACE_TARGET,
                    order: Order::BlitzWorld,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::BlitzWorld,
                    speed: 0,
                    ready_flag: Some(0x80),
                },
                FleetTurnExpectation {
                    turn: 3,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::HoldPosition,
                    speed: 0,
                    ready_flag: Some(0x81),
                },
            ],
            check: check_blitz_world_trace,
        },
    ];

    for case in cases {
        run_trace_case(case);
    }
}

#[test]
fn merge_and_pursuit_orders_trace_speed_and_completion() {
    let cases = [
        TraceCase {
            name: "join-another-fleet chase",
            setup: build_join_chase_trace,
            fleet_idx: 1,
            turns_to_run: 2,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: [4, 10],
                    target: [10, 10],
                    order: Order::JoinAnotherFleet,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [6, 10],
                    target: [10, 10],
                    order: Order::JoinAnotherFleet,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: [9, 10],
                    target: [12, 10],
                    order: Order::JoinAnotherFleet,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
            ],
            check: check_join_chase_trace,
        },
        TraceCase {
            name: "join-another-fleet merge",
            setup: build_join_merge_trace,
            fleet_idx: 1,
            turns_to_run: 1,
            expectations: &[FleetTurnExpectation {
                turn: 0,
                coords: [10, 10],
                target: [10, 10],
                order: Order::JoinAnotherFleet,
                speed: 0,
                ready_flag: Some(0x80),
            }],
            check: check_join_merge_trace,
        },
        TraceCase {
            name: "rendezvous-sector travel",
            setup: build_rendezvous_travel_trace,
            fleet_idx: 0,
            turns_to_run: 3,
            expectations: &[
                FleetTurnExpectation {
                    turn: 0,
                    coords: TRACE_START,
                    target: TRACE_TARGET,
                    order: Order::RendezvousSector,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 1,
                    coords: [10, 8],
                    target: TRACE_TARGET,
                    order: Order::RendezvousSector,
                    speed: TRACE_SPEED,
                    ready_flag: None,
                },
                FleetTurnExpectation {
                    turn: 2,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::RendezvousSector,
                    speed: 0,
                    ready_flag: Some(0x80),
                },
                FleetTurnExpectation {
                    turn: 3,
                    coords: TRACE_TARGET,
                    target: TRACE_TARGET,
                    order: Order::RendezvousSector,
                    speed: 0,
                    ready_flag: Some(0x80),
                },
            ],
            check: check_rendezvous_travel_trace,
        },
        TraceCase {
            name: "rendezvous-sector merge",
            setup: build_rendezvous_merge_trace,
            fleet_idx: 1,
            turns_to_run: 1,
            expectations: &[FleetTurnExpectation {
                turn: 0,
                coords: TRACE_TARGET,
                target: TRACE_TARGET,
                order: Order::RendezvousSector,
                speed: 0,
                ready_flag: Some(0x80),
            }],
            check: check_rendezvous_merge_trace,
        },
    ];

    for case in cases {
        run_trace_case(case);
    }
}

fn run_trace_case(case: TraceCase) {
    let mut game_data = (case.setup)();
    let mut states = vec![game_data.clone()];
    let mut events = Vec::new();

    for _ in 0..case.turns_to_run {
        let turn_events = run_maintenance_turn(&mut game_data)
            .unwrap_or_else(|err| panic!("{} maintenance failed: {err}", case.name));
        events.push(turn_events);
        states.push(game_data.clone());
    }

    for expected in case.expectations {
        assert_fleet_state(case.name, &states[expected.turn], case.fleet_idx, *expected);
    }

    (case.check)(&TraceRun { states, events });
}

fn assert_fleet_state(
    name: &str,
    state: &CoreGameData,
    fleet_idx: usize,
    expected: FleetTurnExpectation,
) {
    let fleet = &state.fleets.records[fleet_idx];
    assert_eq!(
        fleet.current_location_coords_raw(),
        expected.coords,
        "{name} turn {} coords",
        expected.turn
    );
    assert_eq!(
        fleet.standing_order_target_coords_raw(),
        expected.target,
        "{name} turn {} target",
        expected.turn
    );
    assert_eq!(
        fleet.standing_order_kind(),
        expected.order,
        "{name} turn {} order",
        expected.turn
    );
    assert_eq!(
        fleet.current_speed(),
        expected.speed,
        "{name} turn {} speed",
        expected.turn
    );
    if let Some(ready_flag) = expected.ready_flag {
        assert_eq!(
            fleet.transit_ready_flag_raw(),
            ready_flag,
            "{name} turn {} ready flag",
            expected.turn
        );
    }
}

fn assert_mission_event(
    events: &MaintenanceEvents,
    fleet_idx: usize,
    kind: Mission,
    outcome: MissionOutcome,
) {
    assert!(
        events.mission_events.iter().any(|event| {
            event.fleet_idx == fleet_idx && event.kind == kind && event.outcome == outcome
        }),
        "missing mission event {:?} {:?} for fleet {}",
        kind,
        outcome,
        fleet_idx
    );
}

fn check_move_only_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::MoveOnly,
        MissionOutcome::Succeeded,
    );
}

fn check_colonize_world_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::ColonizeWorld,
        MissionOutcome::Succeeded,
    );
    assert_eq!(
        run.states[2].planets.records[TARGET_PLANET_IDX].owner_empire_slot_raw(),
        1
    );
}

fn check_view_world_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::ViewWorld,
        MissionOutcome::Succeeded,
    );
    assert!(run.events[1].planet_intel_events.iter().any(|event| {
        event.planet_idx == TARGET_PLANET_IDX && event.source_fleet_idx == Some(0)
    }));
}

fn check_seek_home_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::SeekHome,
        MissionOutcome::Succeeded,
    );
}

fn check_salvage_trace(run: &TraceRun) {
    assert_eq!(
        run.states[2].fleets.records.len(),
        run.states[1].fleets.records.len() - 1,
        "salvage should remove the fleet on arrival"
    );
    assert!(!run.events[1].salvage_events.is_empty());
}

fn check_scout_sector_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::ScoutSector,
        MissionOutcome::Arrived,
    );
    assert_mission_event(
        &run.events[2],
        0,
        Mission::ScoutSector,
        MissionOutcome::Succeeded,
    );
}

fn check_scout_system_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::ScoutSolarSystem,
        MissionOutcome::Succeeded,
    );
    assert_mission_event(
        &run.events[2],
        0,
        Mission::ScoutSolarSystem,
        MissionOutcome::Succeeded,
    );
    assert!(run.events[1].planet_intel_events.iter().any(|event| {
        event.planet_idx == TARGET_PLANET_IDX && event.source_fleet_idx == Some(0)
    }));
    assert!(run.events[2].planet_intel_events.iter().any(|event| {
        event.planet_idx == TARGET_PLANET_IDX && event.source_fleet_idx == Some(0)
    }));
}

fn check_patrol_sector_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::PatrolSector,
        MissionOutcome::Arrived,
    );
}

fn check_guard_starbase_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::GuardStarbase,
        MissionOutcome::Arrived,
    );
}

fn check_guard_blockade_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::GuardBlockadeWorld,
        MissionOutcome::Arrived,
    );
}

fn check_bombard_world_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::BombardWorld,
        MissionOutcome::Arrived,
    );
    assert!(!run.events[2].bombard_events.is_empty());
}

fn check_invade_world_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::InvadeWorld,
        MissionOutcome::Arrived,
    );
    assert_mission_event(
        &run.events[2],
        0,
        Mission::InvadeWorld,
        MissionOutcome::Succeeded,
    );
    assert!(!run.events[2].assault_report_events.is_empty());
    assert_eq!(
        run.states[3].planets.records[TARGET_PLANET_IDX].owner_empire_slot_raw(),
        1
    );
}

fn check_blitz_world_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::BlitzWorld,
        MissionOutcome::Arrived,
    );
    assert_mission_event(
        &run.events[2],
        0,
        Mission::BlitzWorld,
        MissionOutcome::Succeeded,
    );
    assert!(!run.events[2].assault_report_events.is_empty());
    assert_eq!(
        run.states[3].planets.records[TARGET_PLANET_IDX].owner_empire_slot_raw(),
        1
    );
}

fn check_join_chase_trace(run: &TraceRun) {
    let host = &run.states[1].fleets.records[0];
    assert_eq!(host.current_location_coords_raw(), [12, 10]);
    let host_after_second = &run.states[2].fleets.records[0];
    assert_eq!(host_after_second.current_location_coords_raw(), [14, 10]);
}

fn check_join_merge_trace(run: &TraceRun) {
    assert!(
        run.events[0]
            .fleet_merge_events
            .iter()
            .any(|event| { event.kind == Mission::JoinAnotherFleet && !event.survivor_side })
    );
    assert_eq!(
        run.states[1].fleets.records[0].standing_order_kind(),
        Order::HoldPosition
    );
}

fn check_rendezvous_travel_trace(run: &TraceRun) {
    assert_mission_event(
        &run.events[1],
        0,
        Mission::RendezvousSector,
        MissionOutcome::Arrived,
    );
}

fn check_rendezvous_merge_trace(run: &TraceRun) {
    assert!(
        run.events[0]
            .fleet_merge_events
            .iter()
            .any(|event| { event.kind == Mission::RendezvousSector && !event.survivor_side })
    );
    let survivor = &run.states[1].fleets.records[0];
    assert_eq!(survivor.standing_order_kind(), Order::RendezvousSector);
    assert_eq!(survivor.standing_order_target_coords_raw(), TRACE_TARGET);
    assert_eq!(survivor.max_speed(), 3);
}

fn build_move_only_trace() -> CoreGameData {
    let mut game_data = build_trace_baseline();
    let fleet = &mut game_data.fleets.records[0];
    configure_scout_fleet(fleet);
    arm_new_order(
        fleet,
        Order::MoveOnly,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

fn build_colonize_world_trace() -> CoreGameData {
    let mut game_data = build_world_trace(TargetWorldOwner::Unowned);
    let fleet = &mut game_data.fleets.records[0];
    configure_colonizer_fleet(fleet);
    arm_new_order(
        fleet,
        Order::ColonizeWorld,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

fn build_view_world_trace() -> CoreGameData {
    let mut game_data = build_world_trace(TargetWorldOwner::Foreign);
    let fleet = &mut game_data.fleets.records[0];
    configure_cruiser_fleet(fleet);
    arm_new_order(
        fleet,
        Order::ViewWorld,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

fn build_seek_home_trace() -> CoreGameData {
    let mut game_data = build_world_trace(TargetWorldOwner::Friendly);
    let fleet = &mut game_data.fleets.records[0];
    configure_cruiser_fleet(fleet);
    arm_new_order(
        fleet,
        Order::SeekHome,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

fn build_salvage_trace() -> CoreGameData {
    let mut game_data = build_world_trace(TargetWorldOwner::Friendly);
    let fleet = &mut game_data.fleets.records[0];
    configure_salvage_fleet(fleet);
    arm_new_order(
        fleet,
        Order::Salvage,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

fn build_scout_sector_trace() -> CoreGameData {
    let mut game_data = build_world_trace(TargetWorldOwner::Foreign);
    let fleet = &mut game_data.fleets.records[0];
    configure_scout_fleet(fleet);
    arm_new_order(
        fleet,
        Order::ScoutSector,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

fn build_scout_system_trace() -> CoreGameData {
    let mut game_data = build_world_trace(TargetWorldOwner::Foreign);
    let fleet = &mut game_data.fleets.records[0];
    configure_scout_fleet(fleet);
    arm_new_order(
        fleet,
        Order::ScoutSolarSystem,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

fn build_patrol_sector_trace() -> CoreGameData {
    let mut game_data = build_trace_baseline();
    let fleet = &mut game_data.fleets.records[0];
    configure_cruiser_fleet(fleet);
    arm_new_order(
        fleet,
        Order::PatrolSector,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

fn build_guard_starbase_trace() -> CoreGameData {
    let mut game_data = build_trace_baseline();
    let mut base = BaseRecord::new_zeroed();
    base.set_active_flag_raw(1);
    base.set_base_id_raw(1);
    base.set_owner_empire_raw(1);
    base.set_coords_raw(TRACE_TARGET);
    game_data.bases = BaseDat {
        records: vec![base],
    };

    let fleet = &mut game_data.fleets.records[0];
    configure_cruiser_fleet(fleet);
    arm_new_order(
        fleet,
        Order::GuardStarbase,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    fleet.set_mission_aux_bytes([1, 1]);
    game_data
}

fn build_guard_blockade_world_trace() -> CoreGameData {
    let mut game_data = build_world_trace(TargetWorldOwner::Foreign);
    let fleet = &mut game_data.fleets.records[0];
    configure_cruiser_fleet(fleet);
    arm_new_order(
        fleet,
        Order::GuardBlockadeWorld,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

fn build_bombard_world_trace() -> CoreGameData {
    let mut game_data = build_world_trace(TargetWorldOwner::Foreign);
    let fleet = &mut game_data.fleets.records[0];
    configure_bombard_fleet(fleet);
    arm_new_order(
        fleet,
        Order::BombardWorld,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

fn build_invade_world_trace() -> CoreGameData {
    let mut game_data = build_world_trace(TargetWorldOwner::Foreign);
    {
        let target = &mut game_data.planets.records[TARGET_PLANET_IDX];
        target.set_ground_batteries_raw(1);
        target.set_army_count_raw(1);
    }
    let fleet = &mut game_data.fleets.records[0];
    configure_assault_fleet(fleet);
    arm_new_order(
        fleet,
        Order::InvadeWorld,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

fn build_blitz_world_trace() -> CoreGameData {
    let mut game_data = build_world_trace(TargetWorldOwner::Foreign);
    {
        let target = &mut game_data.planets.records[TARGET_PLANET_IDX];
        target.set_ground_batteries_raw(1);
        target.set_army_count_raw(1);
    }
    let fleet = &mut game_data.fleets.records[0];
    configure_blitz_fleet(fleet);
    arm_new_order(
        fleet,
        Order::BlitzWorld,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

fn build_join_chase_trace() -> CoreGameData {
    let mut game_data = build_trace_baseline();

    let host = &mut game_data.fleets.records[0];
    configure_cruiser_fleet(host);
    arm_new_order(host, Order::MoveOnly, [10, 10], [14, 10], TRACE_SPEED);

    let host_id = game_data.fleets.records[0].fleet_id();
    let joiner = &mut game_data.fleets.records[1];
    configure_cruiser_fleet(joiner);
    joiner.set_owner_empire_raw(1);
    arm_new_order(
        joiner,
        Order::JoinAnotherFleet,
        [4, 10],
        [10, 10],
        TRACE_SPEED,
    );
    joiner.set_join_host_fleet_id_raw(host_id);

    game_data
}

fn build_join_merge_trace() -> CoreGameData {
    let mut game_data = build_trace_baseline();
    game_data.player.records[0].raw[0x00] = 0xff;
    let coords = [10, 10];

    let host = &mut game_data.fleets.records[0];
    configure_cruiser_fleet(host);
    arm_stationary_order(host, Order::HoldPosition, coords);

    let host_id = game_data.fleets.records[0].fleet_id();
    let joiner = &mut game_data.fleets.records[1];
    configure_cruiser_fleet(joiner);
    joiner.set_owner_empire_raw(1);
    arm_stationary_order(joiner, Order::JoinAnotherFleet, coords);
    joiner.set_join_host_fleet_id_raw(host_id);

    game_data
}

fn build_rendezvous_travel_trace() -> CoreGameData {
    let mut game_data = build_trace_baseline();
    let fleet = &mut game_data.fleets.records[0];
    configure_cruiser_fleet(fleet);
    arm_new_order(
        fleet,
        Order::RendezvousSector,
        TRACE_START,
        TRACE_TARGET,
        TRACE_SPEED,
    );
    game_data
}

fn build_rendezvous_merge_trace() -> CoreGameData {
    let mut game_data = build_trace_baseline();
    game_data.player.records[0].raw[0x00] = 0xff;

    let slow = &mut game_data.fleets.records[0];
    configure_cruiser_fleet(slow);
    slow.set_max_speed(6);
    arm_stationary_order(slow, Order::RendezvousSector, TRACE_TARGET);

    let fast = &mut game_data.fleets.records[1];
    configure_etac_fleet(fast);
    fast.set_owner_empire_raw(1);
    fast.set_max_speed(3);
    arm_stationary_order(fast, Order::RendezvousSector, TRACE_TARGET);

    game_data
}

fn build_trace_baseline() -> CoreGameData {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    game_data.bases = BaseDat { records: vec![] };
    for fleet in game_data.fleets.records.iter_mut().skip(1) {
        clear_fleet_for_probe(fleet);
        fleet.set_owner_empire_raw(0);
        fleet.set_current_location_coords_raw([1, 1]);
        fleet.set_standing_order_target_coords_raw([1, 1]);
        arm_stationary_order(fleet, Order::HoldPosition, [1, 1]);
    }

    game_data
}

fn build_world_trace(owner: TargetWorldOwner) -> CoreGameData {
    let mut game_data = build_trace_baseline();
    let target = &mut game_data.planets.records[TARGET_PLANET_IDX];
    target.set_coords_raw(TRACE_TARGET);
    target.set_planet_name("Target");
    target.set_ground_batteries_raw(4);
    target.set_army_count_raw(10);
    match owner {
        TargetWorldOwner::Unowned => {
            target.set_owner_empire_slot_raw(0);
            target.set_ownership_status_raw(0);
        }
        TargetWorldOwner::Friendly => {
            target.set_owner_empire_slot_raw(1);
            target.set_ownership_status_raw(2);
        }
        TargetWorldOwner::Foreign => {
            target.set_owner_empire_slot_raw(2);
            target.set_ownership_status_raw(2);
        }
    }
    game_data
}

fn clear_fleet_for_probe(fleet: &mut nc_data::FleetRecord) {
    fleet.set_battleship_count(0);
    fleet.set_cruiser_count(0);
    fleet.set_destroyer_count(0);
    fleet.set_troop_transport_count(0);
    fleet.set_army_count(0);
    fleet.set_etac_count(0);
    fleet.set_scout_count(0);
    fleet.set_rules_of_engagement(0);
    fleet.recompute_max_speed_from_composition();
}

fn configure_cruiser_fleet(fleet: &mut nc_data::FleetRecord) {
    clear_fleet_for_probe(fleet);
    fleet.set_cruiser_count(1);
    fleet.recompute_max_speed_from_composition();
}

fn configure_scout_fleet(fleet: &mut nc_data::FleetRecord) {
    clear_fleet_for_probe(fleet);
    fleet.set_scout_count(1);
    fleet.recompute_max_speed_from_composition();
}

fn configure_colonizer_fleet(fleet: &mut nc_data::FleetRecord) {
    clear_fleet_for_probe(fleet);
    fleet.set_etac_count(3);
    fleet.recompute_max_speed_from_composition();
}

fn configure_salvage_fleet(fleet: &mut nc_data::FleetRecord) {
    clear_fleet_for_probe(fleet);
    fleet.set_destroyer_count(1);
    fleet.set_cruiser_count(1);
    fleet.recompute_max_speed_from_composition();
}

fn configure_bombard_fleet(fleet: &mut nc_data::FleetRecord) {
    clear_fleet_for_probe(fleet);
    fleet.set_destroyer_count(1);
    fleet.recompute_max_speed_from_composition();
}

fn configure_assault_fleet(fleet: &mut nc_data::FleetRecord) {
    clear_fleet_for_probe(fleet);
    fleet.set_battleship_count(4);
    fleet.set_cruiser_count(4);
    fleet.set_destroyer_count(4);
    fleet.set_troop_transport_count(6);
    fleet.set_army_count(24);
    fleet.recompute_max_speed_from_composition();
}

fn configure_blitz_fleet(fleet: &mut nc_data::FleetRecord) {
    clear_fleet_for_probe(fleet);
    fleet.set_destroyer_count(1);
    fleet.set_troop_transport_count(10);
    fleet.set_army_count(10);
    fleet.recompute_max_speed_from_composition();
}

fn configure_etac_fleet(fleet: &mut nc_data::FleetRecord) {
    clear_fleet_for_probe(fleet);
    fleet.set_etac_count(1);
    fleet.recompute_max_speed_from_composition();
}

fn arm_new_order(
    fleet: &mut nc_data::FleetRecord,
    order: Order,
    start: [u8; 2],
    target: [u8; 2],
    speed: u8,
) {
    fleet.set_current_location_coords_raw(start);
    fleet.set_standing_order_kind(order);
    fleet.set_standing_order_target_coords_raw(target);
    fleet.set_current_speed(speed);
    reset_motion_state_for_new_orders(fleet);
    fleet.set_current_speed(speed);
    fleet.set_tuple_c_payload_raw([0x81, 0x00, 0x00, 0x00, 0x00]);
}

fn arm_stationary_order(fleet: &mut nc_data::FleetRecord, order: Order, coords: [u8; 2]) {
    fleet.set_current_location_coords_raw(coords);
    fleet.set_standing_order_kind(order);
    fleet.set_standing_order_target_coords_raw(coords);
    fleet.set_current_speed(0);
    reset_motion_state_for_new_orders(fleet);
    fleet.set_current_speed(0);
    fleet.set_transit_ready_flag_raw(0x80);
}
