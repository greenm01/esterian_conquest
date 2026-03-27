use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ec_data::{CoreGameData, EmpirePlanetEconomyRow, ProductionItemKind};
use ec_game::model::{ClassicLoginState, PlayerContext};
use ec_game::screen::layout::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};
use ec_game::screen::table::{
    SplitTableRow, TableColumn, TableRowState, table_render_width, write_split_table,
    write_stacked_table_window_with_states, write_table_row, write_table_window_with_states,
};
use ec_game::screen::{
    PlanetBuildMenuView, PlanetBuildOrder, PlanetBuildScreen, PlanetDatabaseRow,
    PlanetDatabaseScreen, PlanetListScreen, PlanetListSort, PlayfieldBuffer, ScreenFrame,
    ScreenGeometry,
};
use ec_game::theme::classic;

fn row_text(buffer: &PlayfieldBuffer, row: usize) -> String {
    buffer.row(row).iter().map(|cell| cell.ch).collect()
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn joined_player_context() -> PlayerContext {
    PlayerContext {
        record_index_1_based: 1,
        is_joined: true,
        classic_login_state: ClassicLoginState::ReturningPlayer,
        empire_name: "Player 1".to_string(),
        handle: "P1".to_string(),
    }
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
fn standard_table_places_scrollbar_just_right_of_table_border() {
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

    let scrollbar_col = table_render_width(&columns);
    assert_eq!(buffer.row(5)[scrollbar_col].ch, '^');
    assert_eq!(buffer.row(7)[scrollbar_col].ch, 'v');
    assert_eq!(buffer.row(2)[0].style, classic::table_chrome_style());
    assert_eq!(buffer.row(4)[0].style, classic::table_chrome_style());
    assert_eq!(
        buffer.row(5)[scrollbar_col].style,
        classic::table_chrome_style()
    );
    assert!(row_text(&buffer, 5).contains("Beta"));
    assert!(row_text(&buffer, 6).contains("Gamma"));
}

#[test]
#[should_panic(expected = "scrollable table must leave a gutter to the right of its border")]
fn scrollable_table_panics_when_border_would_consume_last_playfield_col() {
    let columns = [TableColumn::left("Name", 78)];
    let rows = vec![
        vec!["Alpha".to_string()],
        vec!["Beta".to_string()],
        vec!["Gamma".to_string()],
        vec!["Delta".to_string()],
    ];
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    write_table_window_with_states(
        &mut buffer,
        2,
        &columns,
        &rows,
        0,
        3,
        classic::status_value_style(),
        classic::status_value_style(),
        None,
        None,
    );
}

#[test]
fn centered_table_column_centers_single_character_values() {
    let columns = [TableColumn::center("Sel", 3)];
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    write_table_row(
        &mut buffer,
        2,
        &columns,
        &["X"],
        classic::status_value_style(),
    );

    assert_eq!(row_text(&buffer, 2).trim_end(), "│ X │");
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
        &["Meta", "", ""],
        &columns,
        &rows,
        0,
        1,
        classic::status_value_style(),
        classic::status_value_style(),
        Some(0),
        None,
    );

    assert!(buffer.plain_line(1).starts_with("┌"));
    assert!(buffer.plain_line(2).contains("│Meta"));
    assert_eq!(buffer.plain_line(2).matches('│').count(), columns.len() + 1);
    assert!(buffer.plain_line(3).contains("Coord"));
    assert!(buffer.plain_line(5).contains("Aurora"));
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

    assert!(buffer.plain_line(5).starts_with("┌"));
    assert!(buffer.plain_line(5).ends_with("┐"));
    assert!(!buffer.plain_line(5).contains("          "));
    let header = row_text(&buffer, 6);
    assert_eq!(header.matches("NO.").count(), 2);
    assert!(header.contains("QTY."));
    assert!(buffer.plain_line(8).contains("DONE"));
    assert!(buffer.plain_line(8).contains("Scouts"));
}

#[test]
fn planet_database_screen_uses_stacked_header_table() {
    let mut screen = PlanetDatabaseScreen::new();
    let rows = vec![PlanetDatabaseRow {
        planet_record_index_1_based: 1,
        coords: [12, 34],
        known_owner_empire_id: Some(1),
        known_max_production: Some(120),
        name_label: "Aurora".to_string(),
        owner_label: "01".to_string(),
        max_prod_label: "120".to_string(),
        year_seen_label: "3001".to_string(),
        armies_label: "10".to_string(),
        batteries_label: "4".to_string(),
        starbase_count_label: "1".to_string(),
        current_prod_label: "80".to_string(),
        stored_points_label: "25".to_string(),
        year_scout_label: "3001".to_string(),
    }];

    let buffer = screen
        .render_list(
            ScreenGeometry::local_default(),
            &rows,
            0,
            0,
            [12, 34],
            "",
            None,
            ec_game::screen::CommandMenu::Planet,
        )
        .expect("render database list");

    let title_col = buffer
        .plain_line(0)
        .find("TOTAL PLANET DATABASE:")
        .expect("title col");
    let border_col = buffer.plain_line(1).find('┌').expect("table col");
    assert_eq!(title_col, border_col);
    assert!(buffer.plain_line(2).contains("│Coord"));
    assert!(buffer.plain_line(2).contains("Max"));
    assert!(buffer.plain_line(2).contains("Year"));
    assert!(buffer.plain_line(2).contains("Curr"));
    assert!(buffer.plain_line(2).contains("Stored"));
    assert_eq!(buffer.plain_line(2).matches('│').count(), 12);
    assert!(buffer.plain_line(2).trim_end().ends_with('│'));
    assert!(buffer.plain_line(3).contains("(XX,YY)"));
    assert!(buffer.plain_line(3).contains("Planet Name"));
    assert!(buffer.plain_line(3).contains("ARs"));
    assert!(buffer.plain_line(3).contains("GBs"));
    assert!(buffer.plain_line(3).contains("SBs"));
    assert!(buffer.plain_line(3).contains("Scout"));
    assert!(!buffer.plain_line(3).contains("Intel"));
    assert!(buffer.plain_line(5).contains("(12,34)"));
    assert!(buffer.plain_line(5).contains("Aurora"));
    assert_eq!(
        buffer.plain_line(7).find("COMMANDS").expect("command col"),
        border_col
    );
}

#[test]
fn planet_database_filter_prompt_aligns_with_centered_table() {
    let mut screen = PlanetDatabaseScreen::new();
    let rows = vec![PlanetDatabaseRow {
        planet_record_index_1_based: 1,
        coords: [12, 34],
        known_owner_empire_id: Some(1),
        known_max_production: Some(120),
        name_label: "Aurora".to_string(),
        owner_label: "01".to_string(),
        max_prod_label: "120".to_string(),
        year_seen_label: "3001".to_string(),
        armies_label: "10".to_string(),
        batteries_label: "4".to_string(),
        starbase_count_label: "1".to_string(),
        current_prod_label: "80".to_string(),
        stored_points_label: "25".to_string(),
        year_scout_label: "3001".to_string(),
    }];

    let buffer = screen
        .render_filter_prompt(
            ScreenGeometry::local_default(),
            &rows,
            0,
            0,
            ec_game::screen::PlanetDatabasePromptMode::FilterMenu,
            "",
            "",
            None,
            ec_game::screen::CommandMenu::Planet,
        )
        .expect("render database filter prompt");

    let border_col = buffer.plain_line(1).find('┌').expect("table col");
    let prompt_row = (0..PLAYFIELD_HEIGHT)
        .find(|row| buffer.plain_line(*row).contains("Filter <A>"))
        .expect("filter prompt row");
    assert_eq!(
        buffer
            .plain_line(prompt_row)
            .find("COMMANDS")
            .expect("prompt col"),
        border_col
    );
}

#[test]
fn planet_database_24_row_door_keeps_bottom_border_above_command_line() {
    let mut screen = PlanetDatabaseScreen::new();
    let rows = (0..30)
        .map(|idx| PlanetDatabaseRow {
            planet_record_index_1_based: idx + 1,
            coords: [idx as u8 % 20, idx as u8 / 20],
            known_owner_empire_id: Some(1),
            known_max_production: Some(120),
            name_label: format!("Aurora {idx:02}"),
            owner_label: "01".to_string(),
            max_prod_label: "120".to_string(),
            year_seen_label: "3001".to_string(),
            armies_label: "10".to_string(),
            batteries_label: "4".to_string(),
            starbase_count_label: "1".to_string(),
            current_prod_label: "80".to_string(),
            stored_points_label: "25".to_string(),
            year_scout_label: "3001".to_string(),
        })
        .collect::<Vec<_>>();

    let buffer = screen
        .render_list(
            ScreenGeometry::for_door(Some(24)),
            &rows,
            0,
            0,
            [0, 0],
            "",
            None,
            ec_game::screen::CommandMenu::Planet,
        )
        .expect("render 24-row database list");

    assert_eq!(buffer.height(), 24);
    assert!(buffer.plain_line(22).contains('└'));
    assert!(buffer.plain_line(23).contains("COMMANDS"));
}

#[test]
fn planet_brief_list_uses_database_style_stacked_header_and_owned_planet_columns() {
    let mut screen = PlanetListScreen::new();
    let game_data = CoreGameData::load(&repo_root().join("fixtures/ecutil-init/v1.5"))
        .expect("load init fixture");
    let player = joined_player_context();
    let planet_intel_snapshots = BTreeMap::new();
    let frame = ScreenFrame {
        game_dir: Path::new("."),
        game_data: &game_data,
        player: &player,
        campaign_seed: 0,
        planet_intel_snapshots: &planet_intel_snapshots,
        geometry: ScreenGeometry::local_default(),
    };
    let rows = vec![EmpirePlanetEconomyRow {
        planet_record_index_1_based: 1,
        coords: [3, 3],
        planet_name: "Player 1 HW".to_string(),
        present_production: 100,
        potential_production: 100,
        stored_production_points: 165,
        yearly_tax_revenue: 65,
        yearly_growth_delta: 0,
        build_capacity: 100,
        has_friendly_starbase: false,
        armies: 10,
        ground_batteries: 4,
        is_homeworld_seed: true,
    }];

    let buffer = screen
        .render_brief_list(
            &frame,
            ec_game::screen::PlanetListMode::Brief,
            &rows,
            PlanetListSort::CurrentProduction,
            0,
            0,
            "",
        )
        .expect("render brief list");

    let title_col = buffer
        .plain_line(0)
        .find("PLANET COMMAND:")
        .expect("title col");
    let border_col = buffer.plain_line(1).find('┌').expect("table col");
    assert_eq!(title_col, border_col);
    assert!(border_col > 0);
    assert!(buffer.plain_line(2).contains("│Coord"));
    assert!(buffer.plain_line(2).contains("Max"));
    assert!(buffer.plain_line(2).contains("Curr"));
    assert!(buffer.plain_line(2).contains("Stored"));
    assert_eq!(buffer.plain_line(2).matches('│').count(), 12);
    assert!(buffer.plain_line(3).contains("(XX,YY)"));
    assert!(buffer.plain_line(3).contains("Planet Name"));
    assert!(buffer.plain_line(3).contains("Prod"));
    assert!(buffer.plain_line(3).contains("Points"));
    assert!(buffer.plain_line(3).contains("Docked"));
    assert!(buffer.plain_line(3).contains("SBs"));
    assert!(buffer.plain_line(3).contains("ARs"));
    assert!(buffer.plain_line(3).contains("GBs"));
    assert!(buffer.plain_line(5).contains("Player 1 HW"));
    assert!(buffer.plain_line(5).contains("165"));
    assert!(buffer.plain_line(5).contains("0"));
    assert_eq!(
        buffer.plain_line(7).find("COMMANDS").expect("command col"),
        border_col
    );
    assert_eq!(
        buffer.row(5)[border_col + 1].style,
        classic::selected_row_style()
    );
    assert_ne!(
        buffer.row(5)[border_col].style,
        classic::selected_row_style()
    );
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
        .render_specify(&view, &orders, "", None, None)
        .expect("render specify");

    assert_eq!(buffer.plain_line(1), "");
    assert!(buffer.plain_line(2).starts_with("┌"));
    assert_eq!(buffer.plain_line(3).matches("NO.").count(), 2);
    assert!(buffer.plain_line(3).contains("QTY."));
    assert!(buffer.plain_line(5).contains("<01>"));
    assert!(buffer.plain_line(5).contains("<06>"));
    assert!(buffer.plain_line(5).contains("Destroyers"));
    assert!(buffer.plain_line(5).contains("05"));
    assert!(buffer.plain_line(7).contains("<09>"));
    assert!(buffer.plain_line(7).contains("02"));
    assert!(buffer.plain_line(8).contains("<10>"));
    assert!(buffer.plain_line(8).contains("20"));
    assert!(buffer.plain_line(9).contains("<05>"));
    assert!(!buffer.plain_line(5).contains("DONE"));
    assert!(
        buffer
            .plain_line(13)
            .contains("You have spent 10 out of 50 points.")
    );
}
