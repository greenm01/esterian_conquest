use crate::support::*;

#[test]
fn partial_starmap_view_uses_full_80x25_layout_without_sidebar_legend() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Starmap(StarmapAction::OpenPartialView(CommandMenu::Main))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PartialStarmapView);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("partial starmap view should render");

    let title = format!(
        "Map Center at Sector {}",
        nc_game::screen::format_sector_coords(app.starmap_state.partial_center)
    );
    let title_row = 2;
    let title_leading_spaces = terminal
        .line(title_row)
        .chars()
        .take_while(|ch| *ch == ' ')
        .count();
    assert_eq!(title_leading_spaces, 9);
    assert!(terminal.line(title_row).contains(&title));
    assert_eq!(terminal.line(0), "");
    assert_eq!(terminal.line(1), "");
    // Grid is centered: map_cell_start_col = (80 - 52) / 2 = 14, map_left_col = 9
    let leading_spaces = terminal.line(3).chars().take_while(|ch| *ch == ' ').count();
    assert_eq!(leading_spaces, 9);
    assert!(terminal.line(3).contains("18 |"));
    assert!(terminal.line(20).contains("01 |"));
    // x-axis at row 21 (just below the grid), command prompt directly below at row 22
    assert!(terminal.line(21).contains("01"));
    assert!(terminal.line(21).contains("18"));
    assert!(terminal.line(22).contains("MAP COMMAND"));
    assert_eq!(terminal.line(23), "");
    assert_eq!(terminal.line(24), "");
    assert!(!line_containing(&terminal, "STARMAP MENU").contains("STARMAP MENU"));
    assert!(!line_containing(&terminal, "Unowned Planet").contains("Unowned Planet"));
    assert!(!line_containing(&terminal, "Col: 8, Row: 2 in red").contains("Col: 8, Row: 2 in red"));
    assert!(
        terminal.line(21).contains("18"),
        "expanded x-axis should show the full current map width instead of the old 17-column slice"
    );
    assert!(
        terminal
            .lines
            .iter()
            .filter(|line| line.contains("---"))
            .all(|line| !line.contains("|-")),
        "horizontal crosshair should not run directly out of the label separator"
    );
}

#[test]
fn partial_starmap_view_24_row_door_keeps_command_prompt_visible_and_title_centered() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    app.screen_geometry = nc_game::screen::ScreenGeometry::for_door(Some(24));
    advance_to_main_menu(&mut app);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Starmap(StarmapAction::OpenPartialView(CommandMenu::Main))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PartialStarmapView);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("partial starmap view should render on 24-row door");

    assert_eq!(terminal.lines.len(), 24);
    let title = format!(
        "Map Center at Sector {}",
        nc_game::screen::format_sector_coords(app.starmap_state.partial_center)
    );
    let expected_title_col = 9;
    let title_row = 2;
    let leading_spaces = terminal
        .line(title_row)
        .chars()
        .take_while(|ch| *ch == ' ')
        .count();
    assert_eq!(leading_spaces, expected_title_col);
    assert!(terminal.line(title_row).contains(&title));
    assert!(terminal.line(22).contains("MAP COMMAND"));
    assert_eq!(terminal.line(23), "");
}
#[test]
fn opening_reports_from_general_menu_with_empty_inbox_stays_on_menu_with_notice() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::OpenGeneralMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenReports)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("general menu should render after empty inbox notice");
    assert!(line_containing(&terminal, "Inbox is empty.").contains("Inbox is empty."));
}
#[test]
fn partial_starmap_small_map_keeps_title_and_prompt_left_aligned_to_grid() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Starmap(StarmapAction::OpenPartialView(CommandMenu::Main))
        ),
        AppOutcome::Continue
    );
    app.starmap_state.partial_center = [6, 9];

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("partial starmap selected-sector view should render");

    // Grid is centered: map_left_col = 9, map_top_row = 3
    let top_margin = terminal.line(3).chars().take_while(|ch| *ch == ' ').count();
    assert_eq!(top_margin, 9);
    assert_eq!(
        terminal.line(2).chars().take_while(|ch| *ch == ' ').count(),
        9
    );
    assert!(terminal.line(22).contains("MAP COMMAND"));
    // center [6,9]: center_row = 20 - (9 - 1) = 12, center_col = 14 + (6 - 1) * 3 = 29
    let crosshair_row = terminal.line(12);
    assert_eq!(crosshair_row.chars().nth(29), Some('+'));
    assert!(
        terminal.line(21).contains("01 02 03 04 05 06"),
        "small-map x-axis should be grid-centered with the full 1-based padded label run"
    );
}

#[test]
fn partial_starmap_large_map_anchors_axes_and_clamps_crosshair_near_edges() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    app.game_data.conquest.set_player_count(5);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Starmap(StarmapAction::OpenPartialView(CommandMenu::Main))
        ),
        AppOutcome::Continue
    );
    app.starmap_state.partial_center = [3, 3];

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("oversized partial starmap should render");

    let top_visible_row = line_containing(&terminal, "22 |");
    assert!(top_visible_row.starts_with("22 |"));
    assert!(terminal.line(0).starts_with("Map Center at Sector "));
    let axis_line = line_containing(&terminal, "25");
    assert!(axis_line.contains("01"));
    assert!(axis_line.contains("25"));
    assert_eq!(terminal.line(20).chars().nth(11), Some('+'));
    assert!(terminal.line(24).starts_with("MAP COMMAND"));
}

#[test]
fn partial_starmap_enter_opens_planet_info_and_returns_to_map() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Starmap(StarmapAction::OpenPartialView(CommandMenu::Main))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PartialStarmapView);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Starmap(StarmapAction::OpenPlanetInfoAtCenter)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoDetail);

    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PartialStarmapView);

    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);
}

#[test]
fn partial_starmap_enter_on_empty_sector_is_noop() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Starmap(StarmapAction::OpenPartialView(CommandMenu::Main))
        ),
        AppOutcome::Continue
    );

    let empty_coords = (1..=18u8)
        .flat_map(|y| (1..=18u8).map(move |x| [x, y]))
        .find(|coords| {
            app.game_data
                .planet_record_index_at_coords(*coords)
                .is_none()
        })
        .expect("fixture should contain at least one empty sector");
    app.starmap_state.partial_center = empty_coords;

    assert_eq!(
        apply_action(
            &mut app,
            Action::Starmap(StarmapAction::OpenPlanetInfoAtCenter)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PartialStarmapView);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("partial starmap should render without status line");
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("No world found")),
        "enter on empty sector should not show an error status"
    );
}

#[test]
fn starmap_dump_page_uses_plain_bottom_left_slap_a_key_prompt() {
    let mut screen = nc_game::screen::StarmapScreen::new();
    let lines = (1..=21)
        .map(|idx| format!("line {idx:02}"))
        .collect::<Vec<_>>();

    let buffer = screen
        .render_dump_page(nc_game::screen::ScreenGeometry::local_default(), &lines, 0)
        .expect("starmap dump page renders");

    assert_eq!(buffer.plain_line(22), "line 21");
    assert_eq!(buffer.plain_line(24), "(slap a key)");
    assert!(!buffer.plain_line(24).contains("GALAXY MAP"));
    assert!(!buffer.plain_line(24).contains("->"));
    assert!(!buffer.plain_line(24).contains("<-"));
}

#[test]
fn starmap_prompt_uses_plain_dismiss_prompt_below_last_text_line() {
    let mut screen = nc_game::screen::StarmapScreen::new();

    let buffer = screen
        .render_prompt(nc_game::screen::ScreenGeometry::local_default(), None)
        .expect("starmap prompt renders");

    assert_eq!(buffer.plain_line(8), "");
    assert_eq!(buffer.plain_line(9), "(slap a key)");
    assert!(!buffer.plain_line(9).contains("GALAXY MAP"));
    assert!(!buffer.plain_line(9).contains("->"));
    assert!(!buffer.plain_line(9).contains("<-"));
}

#[test]
fn planet_info_intel_detail_shows_last_intel_and_tier() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    let mut terminal = CaptureTerminal::new();

    let (planet_idx, coords) = app
        .game_data
        .planets
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| {
            planet.owner_empire_slot_raw() as usize != app.player.record_index_1_based
        })
        .map(|(idx, planet)| (idx, planet.coords_raw()))
        .expect("fixture should contain a non-owned world");

    app.planet_intel_snapshots.insert(
        planet_idx + 1,
        nc_data::PlanetIntelSnapshot {
            planet_record_index_1_based: planet_idx + 1,
            intel_tier: nc_data::IntelTier::Full,
            compat_is_orbit_seed: false,
            last_intel_year: Some(3000),
            seen_year: Some(3000),
            scout_year: Some(3000),
            known_name: Some("?".to_string()),
            known_owner_empire_id: Some(2),
            known_potential_production: Some(100),
            known_armies: Some(4),
            known_ground_batteries: Some(2),
            known_starbase_count: Some(1),
            known_current_production: Some(75),
            known_stored_points: Some(12),
            known_docked_summary: Some("Nothing".to_string()),
            known_orbit_summary: Some("Nothing".to_string()),
            compat_word_1e: None,
        },
    );
    app.current_screen = ScreenId::PlanetInfoDetail;
    app.planet.info_selected = Some(planet_idx);

    app.render(&mut terminal).expect("render succeeds");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Last Viewed/Scouted"))
    );
    assert!(terminal.lines.iter().any(|line| line.contains("Y3000")));
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Intel Tier"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains(&format!("[{:02},{:02}]", coords[0], coords[1])))
    );
}

#[test]
fn main_menu_planet_info_prompt_renders_inline_command_and_message() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    let mut terminal = CaptureTerminal::new();

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenInfoPrompt(CommandMenu::Main))
        ),
        AppOutcome::Continue
    );

    app.render(&mut terminal).expect("render succeeds");
    assert_eq!(
        line_containing(&terminal, "COMMAND <- Planet coords [").trim_end(),
        " COMMAND <- Planet coords [16,13] <Q> ->"
    );
    assert_eq!(terminal.line(8).trim_end(), "");
    assert_eq!(
        line_containing(&terminal, "Enter coordinates of the planet to view.").trim_end(),
        " Enter coordinates of the planet to view."
    );
}

#[test]
fn main_menu_planet_info_prompt_renders_error_below_message() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    let mut terminal = CaptureTerminal::new();

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenInfoPrompt(CommandMenu::Main))
        ),
        AppOutcome::Continue
    );
    for ch in ['9', '9', ',', '9', '9'] {
        assert_eq!(
            apply_action(&mut app, Action::Planet(PlanetAction::AppendInfoChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitInfoPrompt)),
        AppOutcome::Continue
    );

    app.render(&mut terminal).expect("render succeeds");
    assert_eq!(
        line_containing(&terminal, "COMMAND <- Planet coords [").trim_end(),
        " COMMAND <- Planet coords [16,13] <Q> -> 99,99"
    );
    assert_eq!(
        line_containing(&terminal, "Enter coordinates of the planet to view.").trim_end(),
        " Enter coordinates of the planet to view."
    );
    assert_eq!(terminal.line(10).trim_end(), "");
    assert!(
        line_containing(&terminal, "Error: ").contains("No world found at [99,99]"),
        "expected inline error below the general message"
    );
}

#[test]
fn build_menu_planet_info_prompt_clears_stale_build_notice() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    let mut terminal = CaptureTerminal::new();

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildMenu)),
        AppOutcome::Continue
    );
    app.planet.build_status = Some("Build orders aborted.".to_string());

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenInfoPrompt(CommandMenu::PlanetBuild))
        ),
        AppOutcome::Continue
    );

    app.render(&mut terminal).expect("render succeeds");
    assert_eq!(
        line_containing(&terminal, "COMMAND <- Planet coords [").trim_end(),
        " COMMAND <- Planet coords [16,13] <Q> ->"
    );
    assert_eq!(
        line_containing(&terminal, "Enter coordinates of the planet to view.").trim_end(),
        " Enter coordinates of the planet to view."
    );
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Build orders aborted.")),
        "stale build notice should not leak into the inline planet info prompt"
    );
}

#[test]
fn build_menu_review_shortcut_opens_owned_planet_info_with_build_queue_and_returns() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    let mut terminal = CaptureTerminal::new();

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildMenu)),
        AppOutcome::Continue
    );

    let planet_idx = app
        .game_data
        .planet_record_index_at_coords([16, 13])
        .expect("current build planet should exist");
    let planet = &mut app.game_data.planets.records[planet_idx];
    planet.set_build_count_raw(0, 5);
    planet.set_build_kind_raw(0, 1);
    planet.set_stardock_count_raw(0, 2);
    planet.set_stardock_kind_raw(0, 1);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenCurrentBuildPlanetInfo)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoDetail);

    app.render(&mut terminal).expect("render succeeds");
    let build_queue_line = line_containing(&terminal, "Build Queue");
    assert!(build_queue_line.contains("Build Queue"));
    assert!(build_queue_line.contains("5DD"));
    assert!(line_containing(&terminal, "Stardock").contains("2DD"));

    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildMenu);
}

#[test]
fn planet_info_compact_queue_and_docked_summaries_fit_with_full_entries() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    let mut terminal = CaptureTerminal::new();

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildMenu)),
        AppOutcome::Continue
    );

    let planet_idx = app
        .game_data
        .planet_record_index_at_coords([16, 13])
        .expect("current build planet should exist");
    let planet = &mut app.game_data.planets.records[planet_idx];

    for (slot, (points, kind_raw)) in [
        (10u8, 1u8),
        (20, 2),
        (30, 3),
        (40, 4),
        (50, 5),
        (60, 6),
        (70, 7),
        (80, 8),
        (90, 9),
    ]
    .into_iter()
    .enumerate()
    {
        planet.set_build_count_raw(slot, points);
        planet.set_build_kind_raw(slot, kind_raw);
    }

    for (slot, (count, kind_raw)) in [
        (1u8, 1u8),
        (2u8, 2u8),
        (3u8, 3u8),
        (4u8, 4u8),
        (5u8, 5u8),
        (6u8, 9u8),
    ]
    .into_iter()
    .enumerate()
    {
        planet.set_stardock_count_raw(slot, u16::from(count));
        planet.set_stardock_kind_raw(slot, kind_raw);
    }

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenCurrentBuildPlanetInfo)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoDetail);

    app.render(&mut terminal).expect("render succeeds");

    let build_queue_line = line_containing(&terminal, "Build Queue");
    for token in [
        "10DD", "20CA", "30BB", "40SC", "50TT", "60ET", "70GB", "80AR", "90SB",
    ] {
        assert!(
            build_queue_line.contains(token),
            "missing {token} in {build_queue_line}"
        );
    }

    let docked_line = line_containing(&terminal, "Stardock");
    for token in ["1DD", "2CA", "3BB", "4SC", "5TT", "6SB"] {
        assert!(
            docked_line.contains(token),
            "missing {token} in {docked_line}"
        );
    }
}
