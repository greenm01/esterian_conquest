use crate::support::*;

#[test]
fn fleet_list_repeated_sort_toggles_direction_and_updates_title() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenList)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);
    assert_eq!(app.fleet.list_sort_direction, SortDirection::Desc);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("fleet list renders");
    assert!(
        line_containing(&terminal, "FLEET LIST: ID DESC ALL").contains("FLEET LIST: ID DESC ALL")
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenListSortPrompt)),
        AppOutcome::Continue
    );
    terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("sort prompt renders");
    assert_eq!(
        line_containing(&terminal, "SORT DESC <- ? I L O E T <Q> ->").trim(),
        "SORT DESC <- ? I L O E T <Q> ->"
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::SubmitListSort(FleetListSort::Id))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);
    assert_eq!(app.fleet.list_sort_direction, SortDirection::Asc);

    terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("fleet list rerenders");
    assert!(
        line_containing(&terminal, "FLEET LIST: ID ASC ALL").contains("FLEET LIST: ID ASC ALL")
    );
}

#[test]
fn planet_list_repeated_sort_toggles_direction_and_updates_title() {
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenListSortPrompt(PlanetListMode::Brief)),
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetListSortPrompt(PlanetListMode::Brief)
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitListSort(
                PlanetListMode::Brief,
                PlanetListSort::CurrentProduction,
            )),
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetList(PlanetListMode::Brief, PlanetListSort::CurrentProduction)
    );
    assert_eq!(app.planet.list_sort_direction, SortDirection::Asc);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("planet list renders");
    assert!(
        line_containing(&terminal, "PLANET LIST: CURR ASC ALL")
            .contains("PLANET LIST: CURR ASC ALL")
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenListSortPrompt(PlanetListMode::Brief)),
        ),
        AppOutcome::Continue
    );
    terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet sort prompt renders");
    assert_eq!(
        line_containing(&terminal, "SORT ASC <- ? C L M <Q> ->").trim(),
        "SORT ASC <- ? C L M <Q> ->"
    );
}

#[test]
fn planet_database_same_range_anchor_toggles_and_new_anchor_resets() {
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenDatabase)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenDatabaseSortPrompt)
        ),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("database sort prompt renders");
    assert_eq!(
        line_containing(&terminal, "SORT ASC <- ? L R E M <Q> ->").trim(),
        "SORT ASC <- ? L R E M <Q> ->"
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitDatabaseSort(
                PlanetDatabaseSortMode::Range
            )),
        ),
        AppOutcome::Continue
    );
    for ch in "05,05".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Planet(PlanetAction::AppendDatabaseChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitDatabaseSort(
                PlanetDatabaseSortMode::Range
            )),
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.planet.database_sort_direction, SortDirection::Asc);

    terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("database list renders");
    assert!(
        line_containing(&terminal, "TOTAL PLANET DATABASE: RNG ASC ALL")
            .contains("TOTAL PLANET DATABASE: RNG ASC ALL")
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenDatabaseSortPrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitDatabaseSort(
                PlanetDatabaseSortMode::Range
            )),
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitDatabaseSort(
                PlanetDatabaseSortMode::Range
            )),
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.planet.database_sort_direction, SortDirection::Desc);

    terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("database list rerenders");
    assert!(
        line_containing(&terminal, "TOTAL PLANET DATABASE: RNG DESC ALL")
            .contains("TOTAL PLANET DATABASE: RNG DESC ALL")
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenDatabaseSortPrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitDatabaseSort(
                PlanetDatabaseSortMode::Range
            )),
        ),
        AppOutcome::Continue
    );
    for ch in "06,06".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Planet(PlanetAction::AppendDatabaseChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitDatabaseSort(
                PlanetDatabaseSortMode::Range
            )),
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.planet.database_sort_direction, SortDirection::Asc);
}
