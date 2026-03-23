use ec_client::screen::layout::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};
use ec_client::screen::table::{
    SplitTableRow, TableColumn, TableRowState, write_split_table,
    write_stacked_table_window_with_states, write_table_window_with_states,
};
use ec_client::screen::{
    PlanetBuildMenuView, PlanetBuildOrder, PlanetBuildScreen, PlanetDatabaseRow,
    PlanetDatabaseScreen, PlayfieldBuffer,
};
use ec_client::theme::classic;
use ec_data::{EmpirePlanetEconomyRow, ProductionItemKind};

fn row_text(buffer: &PlayfieldBuffer, row: usize) -> String {
    buffer.row(row).iter().map(|cell| cell.ch).collect()
}

#[test]
fn playfield_geometry_is_80x25() {
    let buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    assert_eq!(PLAYFIELD_WIDTH, 80);
    assert_eq!(PLAYFIELD_HEIGHT, 25);
    assert_eq!(buffer.width(), 80);
    assert_eq!(buffer.height(), 25);
}

#[test]
fn standard_table_uses_right_edge_scroll_gutter() {
    let columns = [TableColumn::left("Name", 10)];
    let rows = vec![
        vec!["Alpha".to_string()],
        vec!["Beta".to_string()],
        vec!["Gamma".to_string()],
        vec!["Delta".to_string()],
    ];
    let row_states = vec![
        TableRowState::Normal,
        TableRowState::Disabled,
        TableRowState::Normal,
        TableRowState::Normal,
    ];
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    write_table_window_with_states(
        &mut buffer,
        2,
        &columns,
        &rows,
        1,
        3,
        classic::status_value_style(),
        classic::status_value_style(),
        Some(2),
        Some(&row_states),
    );

    assert_eq!(buffer.row(4)[79].ch, '^');
    assert_eq!(buffer.row(6)[79].ch, 'v');
    assert!(row_text(&buffer, 4).contains("Beta"));
    assert!(row_text(&buffer, 5).contains("Gamma"));
}

#[test]
fn stacked_header_table_renders_top_and_bottom_headers() {
    let columns = [
        TableColumn::left("Coord", 7),
        TableColumn::left("Planet", 14),
        TableColumn::right("Own", 3),
    ];
    let rows = vec![vec![
        "12,34".to_string(),
        "Aurora".to_string(),
        "01".to_string(),
    ]];
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    write_stacked_table_window_with_states(
        &mut buffer,
        1,
        "                      Meta",
        &columns,
        &rows,
        0,
        1,
        classic::status_value_style(),
        classic::status_value_style(),
        Some(0),
        None,
    );

    assert!(buffer.plain_line(1).contains("Meta"));
    assert!(buffer.plain_line(2).starts_with("Coord"));
    assert!(buffer.plain_line(4).contains("Aurora"));
}

#[test]
fn split_table_renders_both_halves() {
    let columns = [
        TableColumn::left("NO.", 4),
        TableColumn::left("UNIT TYPE", 19),
        TableColumn::right("COST", 4),
        TableColumn::right("QTY.", 5),
    ];
    let rows = vec![SplitTableRow {
        left_cells: vec![
            "<0>".to_string(),
            "DONE".to_string(),
            String::new(),
            String::new(),
        ],
        right_cells: vec![
            "<5>".to_string(),
            "Scouts".to_string(),
            "1".to_string(),
            "(0)".to_string(),
        ],
    }];
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    write_split_table(
        &mut buffer,
        5,
        &columns,
        &columns,
        &rows,
        classic::status_value_style(),
    );

    let header = row_text(&buffer, 5);
    assert_eq!(header.matches("NO.").count(), 2);
    assert!(buffer.plain_line(7).contains("DONE"));
    assert!(buffer.plain_line(7).contains("Scouts"));
}

#[test]
fn planet_database_screen_uses_stacked_header_table() {
    let mut screen = PlanetDatabaseScreen::new();
    let rows = vec![PlanetDatabaseRow {
        planet_record_index_1_based: 1,
        coords: [12, 34],
        name_label: "Aurora".to_string(),
        owner_label: "01".to_string(),
        max_prod_label: "120".to_string(),
        year_seen_label: "3001".to_string(),
        armies_label: "10".to_string(),
        batteries_label: "4".to_string(),
        current_prod_label: "80".to_string(),
        stored_points_label: "25".to_string(),
        year_scout_label: "3001".to_string(),
        intel_label: "Good".to_string(),
    }];

    let buffer = screen
        .render_list(
            &rows,
            0,
            0,
            [12, 34],
            "",
            None,
            ec_client::screen::CommandMenu::Planet,
        )
        .expect("render database list");

    assert!(buffer.plain_line(1).contains("Max  Year"));
    assert!(buffer.plain_line(2).starts_with("Coord"));
    assert!(buffer.plain_line(4).contains("Aurora"));
}

#[test]
fn planet_build_specify_screen_uses_split_table() {
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
    let orders = vec![PlanetBuildOrder {
        kind: ProductionItemKind::Destroyer,
        points_remaining: 10,
    }];

    let buffer = screen
        .render_specify(&view, &orders, "", None)
        .expect("render specify");

    assert_eq!(buffer.plain_line(5).matches("NO.").count(), 2);
    assert!(buffer.plain_line(7).contains("DONE"));
    assert!(buffer.plain_line(7).contains("<5>"));
    assert!(buffer.plain_line(8).contains("Destroyers"));
}
