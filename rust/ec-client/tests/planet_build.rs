use ec_client::screen::{PlanetBuildMenuView, PlanetBuildScreen};
use ec_data::EmpirePlanetEconomyRow;

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
