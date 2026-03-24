use ec_client::screen::{
    PlanetBuildChangeRow, PlanetBuildListRow, PlanetBuildMenuView, PlanetBuildScreen,
};
use ec_data::{EmpirePlanetEconomyRow, ProductionItemKind};

#[test]
fn build_menu_renders_compact_queue_and_stardock_counts() {
    let mut screen = PlanetBuildScreen::new();
    let view = PlanetBuildMenuView {
        row: EmpirePlanetEconomyRow {
            planet_record_index_1_based: 1,
            coords: [6, 5],
            planet_name: "Not Named Yet".to_string(),
            present_production: 100,
            potential_production: 100,
            stored_production_points: 50,
            yearly_tax_revenue: 50,
            yearly_growth_delta: 0,
            build_capacity: 100,
            has_friendly_starbase: false,
            armies: 10,
            ground_batteries: 4,
            is_homeworld_seed: true,
        },
        committed_points: 10,
        available_points: 50,
        points_left: 40,
        queue_used: 2,
        queue_capacity: 10,
        stardock_used: 3,
        stardock_capacity: 10,
    };

    let buffer = screen
        .render_menu(&view, &[], None, false, false, [0, 0], "", None, false)
        .expect("render menu");

    assert_eq!(
        buffer.plain_line(7),
        "BUILD COMMAND <-H,Q,X,V,P,R,C,N,S,A,L,I->"
    );
    assert_eq!(
        buffer.plain_line(13),
        "There are no starbases orbiting planet \"Not Named Yet\"."
    );
    assert_eq!(
        buffer.plain_line(14),
        "Standard building restrictions apply."
    );
    assert_eq!(
        buffer.plain_line(15),
        "You have spent 10 out of 50 points.  You have 40 points left to spend."
    );
    assert_eq!(buffer.plain_line(16), "");
    assert_eq!(
        buffer.plain_line(17),
        "Build queue: [2/10]   Stardock: [3/10]"
    );
}

#[test]
fn build_list_renders_queue_and_stardock_columns() {
    let mut screen = PlanetBuildScreen::new();
    let view = PlanetBuildMenuView {
        row: EmpirePlanetEconomyRow {
            planet_record_index_1_based: 1,
            coords: [6, 5],
            planet_name: "Not Named Yet".to_string(),
            present_production: 100,
            potential_production: 100,
            stored_production_points: 50,
            yearly_tax_revenue: 50,
            yearly_growth_delta: 0,
            build_capacity: 100,
            has_friendly_starbase: false,
            armies: 10,
            ground_batteries: 4,
            is_homeworld_seed: true,
        },
        committed_points: 10,
        available_points: 50,
        points_left: 40,
        queue_used: 2,
        queue_capacity: 10,
        stardock_used: 0,
        stardock_capacity: 10,
    };

    let rows = vec![
        PlanetBuildListRow {
            kind: ProductionItemKind::Destroyer,
            unit_label: "Destroyers".to_string(),
            points: 5,
            queue_qty: 2,
            stardock_qty: Some(3),
        },
        PlanetBuildListRow {
            kind: ProductionItemKind::Army,
            unit_label: "Armies".to_string(),
            points: 2,
            queue_qty: 4,
            stardock_qty: None,
        },
    ];

    let buffer = screen
        .render_list(&view, &rows, 0, 0, false)
        .expect("render list");

    assert_eq!(buffer.plain_line(1), "");
    assert!(buffer.plain_line(2).starts_with("┌"));
    assert!(buffer.plain_line(3).contains("Unit"));
    assert!(buffer.plain_line(3).contains("Points"));
    assert!(buffer.plain_line(3).contains("Queue"));
    assert!(buffer.plain_line(3).contains("Dock"));
    assert!(buffer.plain_line(5).contains("Destroyers"));
    assert!(buffer.plain_line(5).contains("3"));
    assert!(buffer.plain_line(6).contains("Armies"));
    assert!(buffer.plain_line(6).contains("N/A"));
    let command_row = (0..25)
        .find(|&row| {
            buffer
                .plain_line(row)
                .contains("BUILD COMMAND <-ARROWS D(elete queued) Q->")
        })
        .expect("build list command row should render");
    let _ = command_row;
    assert!(!(0..25).any(|row| {
        buffer
            .plain_line(row)
            .contains("You have spent 10 out of 50 points.")
    }));
}

#[test]
fn build_list_confirmation_renders_delete_question_below_command_row() {
    let mut screen = PlanetBuildScreen::new();
    let view = PlanetBuildMenuView {
        row: EmpirePlanetEconomyRow {
            planet_record_index_1_based: 1,
            coords: [6, 5],
            planet_name: "Not Named Yet".to_string(),
            present_production: 100,
            potential_production: 100,
            stored_production_points: 50,
            yearly_tax_revenue: 50,
            yearly_growth_delta: 0,
            build_capacity: 100,
            has_friendly_starbase: false,
            armies: 10,
            ground_batteries: 4,
            is_homeworld_seed: true,
        },
        committed_points: 10,
        available_points: 50,
        points_left: 40,
        queue_used: 2,
        queue_capacity: 10,
        stardock_used: 0,
        stardock_capacity: 10,
    };

    let rows = vec![PlanetBuildListRow {
        kind: ProductionItemKind::Destroyer,
        unit_label: "Destroyers".to_string(),
        points: 5,
        queue_qty: 2,
        stardock_qty: Some(3),
    }];

    let buffer = screen
        .render_list(&view, &rows, 0, 0, true)
        .expect("render confirming build list");

    let command_row = (0..25)
        .find(|&row| buffer.plain_line(row).contains("BUILD COMMAND <- Y/[N] ->"))
        .expect("build list confirm prompt should render");
    let _ = command_row;
    assert!(!(0..25).any(|row| {
        buffer
            .plain_line(row)
            .contains("You have spent 10 out of 50 points.")
    }));
    assert!((0..25).any(|row| {
        buffer
            .plain_line(row)
            .contains("Notice: Delete queued build(s) for this unit?")
    }));
}

#[test]
fn build_change_renders_pp_and_spent_columns() {
    let mut screen = PlanetBuildScreen::new();
    let rows = vec![PlanetBuildChangeRow {
        planet_name: "Not Named Yet".to_string(),
        coords: [6, 5],
        present_production: 100,
        potential_production: 100,
        available_points: 50,
        committed_points: 20,
    }];

    let buffer = screen.render_change(&rows, 0, 0).expect("render change");

    assert!(buffer.plain_line(4).starts_with("┌"));
    assert!(buffer.plain_line(5).contains("Planet Name"));
    assert!(buffer.plain_line(5).contains("Location"));
    assert!(buffer.plain_line(5).contains("Production"));
    assert!(buffer.plain_line(5).contains("PP"));
    assert!(buffer.plain_line(5).contains("Spent"));
    assert!(buffer.plain_line(7).contains("50"));
    assert!(buffer.plain_line(7).contains("20"));
}
