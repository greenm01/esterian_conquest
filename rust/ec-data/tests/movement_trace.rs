use ec_data::{GameStateBuilder, Order};
use ec_engine::{FleetEtaEstimate, estimate_fleet_eta, run_maintenance_turn};

#[derive(Clone, Copy)]
struct MovementCase {
    name: &'static str,
    speed: u8,
    start: [u8; 2],
    target: [u8; 2],
    expected_initial_eta: u16,
    expected_positions: &'static [[u8; 2]],
}

#[test]
fn move_only_traces_match_current_classic_oracle_cases() {
    let cases = [
        MovementCase {
            name: "speed3-horizontal",
            speed: 3,
            start: [10, 10],
            target: [16, 10],
            expected_initial_eta: 3,
            expected_positions: &[[10, 10], [12, 10], [15, 10], [16, 10]],
        },
        MovementCase {
            name: "speed3-diagonal",
            speed: 3,
            start: [10, 10],
            target: [16, 16],
            expected_initial_eta: 3,
            expected_positions: &[[10, 10], [11, 11], [14, 14], [16, 16]],
        },
        MovementCase {
            name: "speed6-diagonal",
            speed: 6,
            start: [10, 10],
            target: [16, 16],
            expected_initial_eta: 2,
            expected_positions: &[[10, 10], [14, 14], [16, 16]],
        },
        MovementCase {
            name: "speed1-diagonal",
            speed: 1,
            start: [10, 10],
            target: [13, 13],
            expected_initial_eta: 5,
            expected_positions: &[[10, 10], [10, 10], [11, 11], [11, 11], [12, 12], [13, 13]],
        },
        MovementCase {
            name: "speed3-shallow",
            speed: 3,
            start: [10, 10],
            target: [16, 12],
            expected_initial_eta: 3,
            expected_positions: &[[10, 10], [12, 11], [15, 12], [16, 12]],
        },
        MovementCase {
            name: "speed3-steep",
            speed: 3,
            start: [10, 10],
            target: [12, 16],
            expected_initial_eta: 3,
            expected_positions: &[[10, 10], [11, 12], [12, 15], [12, 16]],
        },
    ];

    for case in cases {
        let mut game_data = build_move_only_probe(case.start, case.target, case.speed);
        assert_eq!(
            estimate_fleet_eta(&game_data, 0),
            FleetEtaEstimate::Years(case.expected_initial_eta),
            "{} initial eta",
            case.name
        );

        for (turn, expected_coords) in case.expected_positions.iter().enumerate() {
            let fleet = &game_data.fleets.records[0];
            assert_eq!(
                fleet.current_location_coords_raw(),
                *expected_coords,
                "{} turn {} position",
                case.name,
                turn
            );

            if turn == case.expected_positions.len() - 1 {
                assert_eq!(
                    fleet.standing_order_kind(),
                    Order::HoldPosition,
                    "{} arrival order",
                    case.name
                );
                assert_eq!(fleet.current_speed(), 0, "{} arrival speed", case.name);
                assert_eq!(
                    estimate_fleet_eta(&game_data, 0),
                    FleetEtaEstimate::Arrived,
                    "{} arrival eta",
                    case.name
                );
            } else {
                assert_eq!(
                    fleet.standing_order_kind(),
                    Order::MoveOnly,
                    "{} transit order on turn {}",
                    case.name,
                    turn
                );
                assert_eq!(
                    fleet.current_speed(),
                    case.speed,
                    "{} transit speed on turn {}",
                    case.name,
                    turn
                );
            }

            if turn + 1 < case.expected_positions.len() {
                run_maintenance_turn(&mut game_data)
                    .unwrap_or_else(|err| panic!("{} maintenance failed: {err}", case.name));
            }
        }
    }
}

fn build_move_only_probe(start: [u8; 2], target: [u8; 2], speed: u8) -> ec_data::CoreGameData {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_battleship_count(0);
    fleet.set_cruiser_count(0);
    fleet.set_destroyer_count(0);
    fleet.set_troop_transport_count(0);
    fleet.set_etac_count(0);
    fleet.set_scout_count(1);
    fleet.set_rules_of_engagement(0);
    fleet.recompute_max_speed_from_composition();
    fleet.set_current_location_coords_raw(start);
    fleet.set_current_speed(speed);
    fleet.set_standing_order_kind(Order::MoveOnly);
    fleet.set_standing_order_target_coords_raw(target);

    game_data
}
