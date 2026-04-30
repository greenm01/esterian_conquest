use crate::support::*;

fn load_app_to_main_menu() -> App {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    app
}

fn open_fleet_list(app: &mut App) {
    assert_eq!(
        apply_action(app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(app, Action::Fleet(FleetAction::OpenList)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);
}

fn open_planet_menu(app: &mut App) {
    assert_eq!(
        apply_action(app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );
}

fn assert_prompt_advances_to_filter_value(terminal: &CaptureTerminal, code: &str) {
    let expected = format!("Filter {code} ");
    assert!(
        terminal.lines.iter().any(|line| line.contains(&expected)),
        "missing filter value prompt for {code}: {:?}",
        terminal.lines
    );
}

#[test]
fn fleet_list_repeated_sort_toggles_direction_and_updates_title() {
    let mut app = load_app_to_main_menu();
    open_fleet_list(&mut app);
    assert_eq!(app.fleet.list_sort_direction, SortDirection::Desc);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("fleet list renders");
    assert!(
        line_containing(&terminal, "FLEET LIST: ID DESCENDING ALL")
            .contains("FLEET LIST: ID DESCENDING ALL")
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenListSortPrompt)),
        AppOutcome::Continue
    );
    terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("sort prompt renders");
    assert_eq!(
        line_containing(&terminal, "COMMAND <- Sort column [?]").trim(),
        "COMMAND <- Sort column [?] [id] <Q> ->"
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitListSortPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);
    assert_eq!(app.fleet.list_sort_direction, SortDirection::Asc);

    terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("fleet list rerenders");
    assert!(
        line_containing(&terminal, "FLEET LIST: ID ASCENDING ALL")
            .contains("FLEET LIST: ID ASCENDING ALL")
    );
}

#[test]
fn fleet_list_sort_prompt_accepts_typed_column_codes() {
    let mut app = load_app_to_main_menu();
    open_fleet_list(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenListSortPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetListSortPrompt);

    for ch in "loc".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Fleet(FleetAction::AppendListFilterPromptChar(ch))
            ),
            AppOutcome::Continue
        );
    }

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitListSortPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);
    assert_eq!(
        app.fleet.list_sort,
        nc_game::screen::FleetListSort::Location
    );
    assert_eq!(app.fleet.list_sort_direction, SortDirection::Asc);
}

#[test]
fn fleet_list_sort_prompt_accepts_every_sort_column_code() {
    let cases = [
        (
            "id",
            nc_game::screen::FleetListSort::Id,
            SortDirection::Asc,
            "ID",
        ),
        (
            "sel",
            nc_game::screen::FleetListSort::Selected,
            SortDirection::Desc,
            "SEL",
        ),
        (
            "loc",
            nc_game::screen::FleetListSort::Location,
            SortDirection::Asc,
            "LOC",
        ),
        (
            "ord",
            nc_game::screen::FleetListSort::Order,
            SortDirection::Asc,
            "ORD",
        ),
        (
            "tar",
            nc_game::screen::FleetListSort::Target,
            SortDirection::Asc,
            "TAR",
        ),
        (
            "spd",
            nc_game::screen::FleetListSort::Speed,
            SortDirection::Desc,
            "SPD",
        ),
        (
            "eta",
            nc_game::screen::FleetListSort::Eta,
            SortDirection::Asc,
            "ETA",
        ),
        (
            "roe",
            nc_game::screen::FleetListSort::Roe,
            SortDirection::Desc,
            "ROE",
        ),
        (
            "ars",
            nc_game::screen::FleetListSort::Armies,
            SortDirection::Desc,
            "ARS",
        ),
        (
            "shi",
            nc_game::screen::FleetListSort::Strength,
            SortDirection::Desc,
            "SHI",
        ),
    ];

    for (code, expected_sort, expected_direction, expected_label) in cases {
        let mut app = load_app_to_main_menu();
        open_fleet_list(&mut app);
        assert_eq!(
            apply_action(&mut app, Action::Fleet(FleetAction::OpenListSortPrompt)),
            AppOutcome::Continue
        );
        assert_eq!(app.current_screen(), ScreenId::FleetListSortPrompt);
        for ch in code.chars() {
            assert_eq!(
                apply_action(
                    &mut app,
                    Action::Fleet(FleetAction::AppendListFilterPromptChar(ch))
                ),
                AppOutcome::Continue
            );
        }
        assert_eq!(
            apply_action(&mut app, Action::Fleet(FleetAction::SubmitListSortPrompt)),
            AppOutcome::Continue
        );
        assert_eq!(app.current_screen(), ScreenId::FleetList);
        assert_eq!(app.fleet.list_sort, expected_sort);
        assert_eq!(app.fleet.list_sort_direction, expected_direction);

        let mut terminal = CaptureTerminal::new();
        app.render(&mut terminal).expect("fleet list rerenders");
        let title = line_containing(&terminal, "FLEET LIST:");
        assert!(title.contains(expected_label), "{title}");
        assert!(
            title.contains(match expected_direction {
                SortDirection::Asc => "ASCENDING",
                SortDirection::Desc => "DESCENDING",
            }),
            "{title}"
        );
    }
}

#[test]
fn planet_list_repeated_sort_toggles_direction_and_updates_title() {
    let mut app = load_app_to_main_menu();
    open_planet_menu(&mut app);
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
        line_containing(&terminal, "PLANET LIST: CUR ASCENDING ALL")
            .contains("PLANET LIST: CUR ASCENDING ALL")
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
        line_containing(&terminal, "SORT ASC <- Sort column [?]").trim(),
        "SORT ASC <- Sort column [?] [cur] <Q> ->"
    );
}

#[test]
fn planet_list_sort_prompt_accepts_every_sort_column_code() {
    let cases = [
        ("coo", PlanetListSort::Location, SortDirection::Asc, "COO"),
        ("pla", PlanetListSort::PlanetName, SortDirection::Asc, "PLA"),
        (
            "max",
            PlanetListSort::PotentialProduction,
            SortDirection::Desc,
            "MAX",
        ),
        (
            "cur",
            PlanetListSort::CurrentProduction,
            SortDirection::Asc,
            "CUR",
        ),
        ("trs", PlanetListSort::Treasury, SortDirection::Desc, "TRS"),
        ("bdg", PlanetListSort::Budget, SortDirection::Desc, "BDG"),
        ("rev", PlanetListSort::Revenue, SortDirection::Desc, "REV"),
        ("gro", PlanetListSort::Growth, SortDirection::Desc, "GRO"),
        (
            "bui",
            PlanetListSort::BuildQueue,
            SortDirection::Desc,
            "BUI",
        ),
        ("sta", PlanetListSort::Stardock, SortDirection::Desc, "STA"),
        ("sbs", PlanetListSort::Starbase, SortDirection::Desc, "SBS"),
        ("ars", PlanetListSort::Armies, SortDirection::Desc, "ARS"),
        ("gbs", PlanetListSort::Batteries, SortDirection::Desc, "GBS"),
    ];

    for (code, expected_sort, expected_direction, expected_label) in cases {
        let mut app = load_app_to_main_menu();
        open_planet_menu(&mut app);
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
        for ch in code.chars() {
            assert_eq!(
                apply_action(
                    &mut app,
                    Action::Planet(PlanetAction::AppendListPromptChar(ch))
                ),
                AppOutcome::Continue
            );
        }
        assert_eq!(
            apply_action(
                &mut app,
                Action::Planet(PlanetAction::SubmitListSortPrompt(PlanetListMode::Brief)),
            ),
            AppOutcome::Continue
        );
        assert_eq!(
            app.current_screen(),
            ScreenId::PlanetList(PlanetListMode::Brief, expected_sort)
        );
        assert_eq!(app.planet.list_sort, expected_sort);
        assert_eq!(app.planet.list_sort_direction, expected_direction);

        let mut terminal = CaptureTerminal::new();
        app.render(&mut terminal).expect("planet list rerenders");
        let title = line_containing(&terminal, "PLANET LIST:");
        assert!(title.contains(expected_label), "{title}");
        assert!(
            title.contains(match expected_direction {
                SortDirection::Asc => "ASCENDING",
                SortDirection::Desc => "DESCENDING",
            }),
            "{title}"
        );
    }
}

#[test]
fn planet_database_same_range_anchor_toggles_and_new_anchor_resets() {
    let mut app = load_app_to_main_menu();

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
        line_containing(&terminal, "COMMAND <- Sort column [?]").trim(),
        "COMMAND <- Sort column [?] [coo] <Q> ->"
    );

    for ch in "rng".chars() {
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
            Action::Planet(PlanetAction::SubmitDatabaseSortPrompt),
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
        line_containing(&terminal, "TOTAL PLANET DATABASE: RNG ASCENDING ALL")
            .contains("TOTAL PLANET DATABASE: RNG ASCENDING ALL")
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
            Action::Planet(PlanetAction::SubmitDatabaseSortPrompt),
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
        line_containing(&terminal, "TOTAL PLANET DATABASE: RNG DESCENDING ALL")
            .contains("TOTAL PLANET DATABASE: RNG DESCENDING ALL")
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
            Action::Planet(PlanetAction::SubmitDatabaseSortPrompt),
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

#[test]
fn planet_database_sort_prompt_accepts_every_sort_column_code() {
    let cases = [
        (
            "coo",
            nc_game::screen::PlanetDatabaseSort::Location,
            SortDirection::Desc,
            "COO",
        ),
        (
            "pla",
            nc_game::screen::PlanetDatabaseSort::PlanetName,
            SortDirection::Asc,
            "PLA",
        ),
        (
            "own",
            nc_game::screen::PlanetDatabaseSort::Owner,
            SortDirection::Asc,
            "OWN",
        ),
        (
            "max",
            nc_game::screen::PlanetDatabaseSort::MaxProduction,
            SortDirection::Desc,
            "MAX",
        ),
        (
            "see",
            nc_game::screen::PlanetDatabaseSort::YearSeen,
            SortDirection::Desc,
            "SEE",
        ),
        (
            "ars",
            nc_game::screen::PlanetDatabaseSort::Armies,
            SortDirection::Desc,
            "ARS",
        ),
        (
            "gbs",
            nc_game::screen::PlanetDatabaseSort::Batteries,
            SortDirection::Desc,
            "GBS",
        ),
        (
            "sbs",
            nc_game::screen::PlanetDatabaseSort::Starbases,
            SortDirection::Desc,
            "SBS",
        ),
        (
            "cur",
            nc_game::screen::PlanetDatabaseSort::CurrentProduction,
            SortDirection::Desc,
            "CUR",
        ),
        (
            "trs",
            nc_game::screen::PlanetDatabaseSort::Treasury,
            SortDirection::Desc,
            "TRS",
        ),
        (
            "sco",
            nc_game::screen::PlanetDatabaseSort::ScoutYear,
            SortDirection::Desc,
            "SCO",
        ),
    ];

    for (code, expected_sort, expected_direction, expected_label) in cases {
        let mut app = load_app_to_main_menu();
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
        assert_eq!(app.current_screen(), ScreenId::PlanetDatabaseSortPrompt);
        for ch in code.chars() {
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
                Action::Planet(PlanetAction::SubmitDatabaseSortPrompt),
            ),
            AppOutcome::Continue
        );
        assert_eq!(app.current_screen(), ScreenId::PlanetDatabaseList);
        assert_eq!(app.planet.database_sort, expected_sort);
        assert_eq!(app.planet.database_sort_direction, expected_direction);

        let mut terminal = CaptureTerminal::new();
        app.render(&mut terminal).expect("database rerenders");
        let title = line_containing(&terminal, "TOTAL PLANET DATABASE:");
        assert!(title.contains(expected_label), "{title}");
        assert!(
            title.contains(match expected_direction {
                SortDirection::Asc => "ASCENDING",
                SortDirection::Desc => "DESCENDING",
            }),
            "{title}"
        );
    }
}

#[test]
fn fleet_list_filter_prompt_accepts_every_appendix_e_column_code() {
    let codes = [
        "id", "loc", "ord", "tar", "spd", "eta", "roe", "ars", "shi", "sel",
    ];

    for code in codes {
        let mut app = load_app_to_main_menu();
        open_fleet_list(&mut app);
        assert_eq!(
            apply_action(&mut app, Action::Fleet(FleetAction::OpenListFilterPrompt)),
            AppOutcome::Continue
        );
        assert_eq!(app.current_screen(), ScreenId::FleetListFilterPrompt);
        for ch in code.chars() {
            assert_eq!(
                apply_action(
                    &mut app,
                    Action::Fleet(FleetAction::AppendListFilterPromptChar(ch))
                ),
                AppOutcome::Continue
            );
        }
        assert_eq!(
            apply_action(&mut app, Action::Fleet(FleetAction::SubmitListFilterPrompt)),
            AppOutcome::Continue
        );

        let mut terminal = CaptureTerminal::new();
        app.render(&mut terminal)
            .expect("fleet filter prompt rerenders");
        assert_prompt_advances_to_filter_value(&terminal, code);
    }
}

#[test]
fn planet_list_filter_prompt_accepts_every_appendix_e_column_code() {
    let codes = [
        "coo", "pla", "max", "cur", "trs", "bdg", "rev", "gro", "bui", "sta", "sbs", "ars", "gbs",
    ];

    for code in codes {
        let mut app = load_app_to_main_menu();
        open_planet_menu(&mut app);
        assert_eq!(
            apply_action(
                &mut app,
                Action::Planet(PlanetAction::OpenListFilterPrompt(PlanetListMode::Brief)),
            ),
            AppOutcome::Continue
        );
        assert_eq!(
            app.current_screen(),
            ScreenId::PlanetListFilterPrompt(PlanetListMode::Brief)
        );
        for ch in code.chars() {
            assert_eq!(
                apply_action(
                    &mut app,
                    Action::Planet(PlanetAction::AppendListPromptChar(ch))
                ),
                AppOutcome::Continue
            );
        }
        assert_eq!(
            apply_action(
                &mut app,
                Action::Planet(PlanetAction::SubmitListFilterPrompt(PlanetListMode::Brief)),
            ),
            AppOutcome::Continue
        );

        let mut terminal = CaptureTerminal::new();
        app.render(&mut terminal)
            .expect("planet filter prompt rerenders");
        assert_prompt_advances_to_filter_value(&terminal, code);
    }
}

#[test]
fn planet_database_filter_prompt_accepts_every_appendix_e_column_code() {
    let codes = [
        "coo", "pla", "own", "max", "see", "ars", "gbs", "sbs", "cur", "trs", "sco",
    ];

    for code in codes {
        let mut app = load_app_to_main_menu();
        assert_eq!(
            apply_action(&mut app, Action::Planet(PlanetAction::OpenDatabase)),
            AppOutcome::Continue
        );
        assert_eq!(
            apply_action(
                &mut app,
                Action::Planet(PlanetAction::OpenDatabaseFilterPrompt)
            ),
            AppOutcome::Continue
        );
        assert_eq!(app.current_screen(), ScreenId::PlanetDatabaseFilterPrompt);
        for ch in code.chars() {
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
                Action::Planet(PlanetAction::SubmitDatabaseFilterPrompt),
            ),
            AppOutcome::Continue
        );

        let mut terminal = CaptureTerminal::new();
        app.render(&mut terminal)
            .expect("database filter prompt rerenders");
        assert_prompt_advances_to_filter_value(&terminal, code);
    }
}

#[test]
fn sort_prompts_accept_natural_column_names() {
    let mut app = load_app_to_main_menu();
    open_fleet_list(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenListSortPrompt)),
        AppOutcome::Continue
    );
    for ch in "speed".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Fleet(FleetAction::AppendListFilterPromptChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitListSortPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.fleet.list_sort, nc_game::screen::FleetListSort::Speed);

    let mut app = load_app_to_main_menu();
    open_planet_menu(&mut app);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenListSortPrompt(PlanetListMode::Brief)),
        ),
        AppOutcome::Continue
    );
    for ch in "dock".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Planet(PlanetAction::AppendListPromptChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitListSortPrompt(PlanetListMode::Brief)),
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.planet.list_sort, PlanetListSort::Stardock);

    let mut app = load_app_to_main_menu();
    open_planet_menu(&mut app);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenListSortPrompt(PlanetListMode::Brief)),
        ),
        AppOutcome::Continue
    );
    for ch in "treasury points".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Planet(PlanetAction::AppendListPromptChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitListSortPrompt(PlanetListMode::Brief)),
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.planet.list_sort, PlanetListSort::Treasury);

    let mut app = load_app_to_main_menu();
    open_planet_menu(&mut app);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenListSortPrompt(PlanetListMode::Brief)),
        ),
        AppOutcome::Continue
    );
    for ch in "bgdt".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Planet(PlanetAction::AppendListPromptChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitListSortPrompt(PlanetListMode::Brief)),
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.planet.list_sort, PlanetListSort::Budget);

    let mut app = load_app_to_main_menu();
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
    for ch in "year".chars() {
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
            Action::Planet(PlanetAction::SubmitDatabaseSortPrompt),
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.planet.database_sort,
        nc_game::screen::PlanetDatabaseSort::YearSeen
    );
}

#[test]
fn filter_prompts_accept_natural_column_names() {
    let mut app = load_app_to_main_menu();
    open_fleet_list(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenListFilterPrompt)),
        AppOutcome::Continue
    );
    for ch in "speed".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Fleet(FleetAction::AppendListFilterPromptChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitListFilterPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.fleet
            .list_filter_pending_column
            .expect("pending fleet column")
            .code,
        "spd"
    );

    let mut app = load_app_to_main_menu();
    open_planet_menu(&mut app);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenListFilterPrompt(PlanetListMode::Brief)),
        ),
        AppOutcome::Continue
    );
    for ch in "dock".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Planet(PlanetAction::AppendListPromptChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitListFilterPrompt(PlanetListMode::Brief)),
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.planet
            .list_filter_pending_column
            .expect("pending planet column")
            .code,
        "sta"
    );

    let mut app = load_app_to_main_menu();
    open_planet_menu(&mut app);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenListFilterPrompt(PlanetListMode::Brief)),
        ),
        AppOutcome::Continue
    );
    for ch in "treasury points".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Planet(PlanetAction::AppendListPromptChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitListFilterPrompt(PlanetListMode::Brief)),
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.planet
            .list_filter_pending_column
            .expect("pending treasury column")
            .code,
        "trs"
    );

    let mut app = load_app_to_main_menu();
    open_planet_menu(&mut app);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenListFilterPrompt(PlanetListMode::Brief)),
        ),
        AppOutcome::Continue
    );
    for ch in "bgdt".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Planet(PlanetAction::AppendListPromptChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitListFilterPrompt(PlanetListMode::Brief)),
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.planet
            .list_filter_pending_column
            .expect("pending budget column")
            .code,
        "bdg"
    );

    let mut app = load_app_to_main_menu();
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenDatabase)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenDatabaseFilterPrompt)
        ),
        AppOutcome::Continue
    );
    for ch in "year".chars() {
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
            Action::Planet(PlanetAction::SubmitDatabaseFilterPrompt),
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.planet
            .database_pending_column
            .expect("pending database column")
            .code,
        "see"
    );
}
