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

    let buffer = screen.render_menu(&view, None).expect("render menu");

    assert_eq!(buffer.plain_line(12), "Build queue: [2/10]   Stardock: [3/10]");
    assert_eq!(buffer.plain_line(13), "");
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

    assert_eq!(buffer.plain_line(4), "Unit                     Points Queue Dock");
    assert!(buffer.plain_line(6).contains("Destroyers"));
    assert!(buffer.plain_line(6).contains("3"));
    assert!(buffer.plain_line(7).contains("Armies"));
    assert!(buffer.plain_line(7).contains("N/A"));
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

    assert_eq!(buffer.plain_line(4), "Planet Name          Location  Production         PP Spent");
    assert!(buffer.plain_line(6).contains("50"));
    assert!(buffer.plain_line(6).contains("20"));
}
