use crate::support::*;

#[test]
fn navigation_hotkeys_map_ctrl_d_to_page_down_actions() {
    let root = temp_game_copy();
    let config = AppConfig {
        game_dir: root.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    let mut app = App::load(config).expect("load app");
    app.current_screen = ScreenId::PlanetDatabaseList;

    assert_eq!(
        app.handle_key(ctrl_key('d')),
        Action::Planet(PlanetAction::PageDatabaseList(-1))
    );
    assert_eq!(
        app.handle_key(ctrl_key('u')),
        Action::Planet(PlanetAction::PageDatabaseList(1))
    );
}

#[test]
fn compose_body_treats_hjkl_as_text_and_keeps_arrow_navigation() {
    let screen = MessageComposeScreen::new();

    assert_eq!(
        screen.handle_body_key(key(KeyCode::Char('h'))),
        Action::Messaging(MessagingAction::AppendComposeBodyChar('h'))
    );
    assert_eq!(
        screen.handle_body_key(key(KeyCode::Char('j'))),
        Action::Messaging(MessagingAction::AppendComposeBodyChar('j'))
    );
    assert_eq!(
        screen.handle_body_key(key(KeyCode::Char('k'))),
        Action::Messaging(MessagingAction::AppendComposeBodyChar('k'))
    );
    assert_eq!(
        screen.handle_body_key(key(KeyCode::Char('l'))),
        Action::Messaging(MessagingAction::AppendComposeBodyChar('l'))
    );
    assert_eq!(
        screen.handle_body_key(key(KeyCode::Left)),
        Action::Messaging(MessagingAction::MoveComposeBodyCursorLeft)
    );
    assert_eq!(
        screen.handle_body_key(key(KeyCode::Down)),
        Action::Messaging(MessagingAction::MoveComposeBodyCursorDown)
    );
    assert_eq!(
        screen.handle_body_key(key(KeyCode::Up)),
        Action::Messaging(MessagingAction::MoveComposeBodyCursorUp)
    );
    assert_eq!(
        screen.handle_body_key(key(KeyCode::Right)),
        Action::Messaging(MessagingAction::MoveComposeBodyCursorRight)
    );
}

#[test]
fn apply_action_switches_between_client_screens() {
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

    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Splash)
    );

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::Advance)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Splash)
    );

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::Advance)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Splash)
    );

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::Advance)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::LoginSummary)
    );

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::Advance)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(
        apply_action(&mut app, Action::OpenGeneralMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);

    // Continue on the joined-player surface after startup.
    assert_eq!(
        apply_action(&mut app, Action::OpenMainMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(
        apply_action(&mut app, Action::OpenGeneralMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);

    assert_eq!(
        apply_action(&mut app, Action::Starmap(StarmapAction::OpenFull)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Starmap);

    assert_eq!(
        apply_action(&mut app, Action::OpenGeneralMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);

    assert_eq!(
        apply_action(&mut app, Action::OpenPopupHelp),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);
    assert!(app.popup_help.is_some());
    assert_eq!(
        apply_action(&mut app, Action::DismissPopupHelp),
        AppOutcome::Continue
    );
    assert!(app.popup_help.is_none());

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenList)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReviewPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReview)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetReview);

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenChangePrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenDetach)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    assert_eq!(
        apply_action(&mut app, Action::OpenPopupHelp),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert!(app.popup_help.is_some());
    assert_eq!(
        apply_action(&mut app, Action::DismissPopupHelp),
        AppOutcome::Continue
    );
    assert!(app.popup_help.is_none());

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenAutoCommissionPrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::ConfirmAutoCommission)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenCommissionMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildMenu);

    assert_eq!(
        apply_action(&mut app, Action::OpenPopupHelp),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildMenu);
    assert!(app.popup_help.is_some());
    assert_eq!(
        apply_action(&mut app, Action::DismissPopupHelp),
        AppOutcome::Continue
    );
    assert!(app.popup_help.is_none());

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenCurrentBuildPlanetInfo)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoDetail);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildList)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildMenu);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("build menu should render list-empty notice");
    assert!(line_containing(&terminal, "Notice: ").contains("No build orders are queued."));

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildAbortPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildMenu);
    app.render(&mut terminal)
        .expect("build menu should render abort-empty notice");
    assert!(line_containing(&terminal, "Notice: ").contains("No build orders are queued."));

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildSpecify)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildSpecify);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenTaxPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::BackspaceTaxInput)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::BackspaceTaxInput)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::AppendTaxChar('6'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::AppendTaxChar('5'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitTax)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenDatabase)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetDatabaseList);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitDatabaseLookup)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoDetail);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitListSort(
                PlanetListMode::Brief,
                PlanetListSort::CurrentProduction
            ))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetBriefList(PlanetListMode::Brief, PlanetListSort::CurrentProduction)
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitListSort(
                PlanetListMode::Brief,
                PlanetListSort::Location
            ))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetBriefList(PlanetListMode::Brief, PlanetListSort::Location)
    );

    assert_eq!(
        apply_action(&mut app, Action::OpenGeneralMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenInfoPrompt(CommandMenu::General))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);
    assert_eq!(app.planet_info_input(), "");

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitInfoPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoDetail);
    assert_eq!(app.selected_planet_info(), Some(14));

    assert_eq!(
        apply_action(
            &mut app,
            Action::Starmap(StarmapAction::OpenPartialView(CommandMenu::General))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PartialStarmapView);

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenReports)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Reports);

    assert_eq!(
        apply_action(&mut app, Action::Empire(EmpireAction::OpenStatus)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::EmpireStatus);

    assert_eq!(
        apply_action(&mut app, Action::Empire(EmpireAction::OpenProfile)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::EmpireProfile);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Empire(EmpireAction::OpenRankingsTable(
                EmpireProductionRankingSort::Production
            ))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::Rankings(EmpireProductionRankingSort::Production)
    );

    assert_eq!(
        apply_action(&mut app, Action::OpenMainMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);
}

#[test]
fn app_load_persists_all_setup_backed_config_fields_into_runtime_snapshot() {
    let fixture_dir = temp_first_time_game_copy();
    let before_setup = latest_runtime_state(&fixture_dir).game_data.setup.clone();
    let config = GameConfig {
        game_name: "Config Persistence Test".to_string(),
        theme: None,
        setup_overrides: GameSetupOverrides {
            snoop_enabled: Some(false),
            session_max_idle_minutes: Some(19),
            session_minimum_time_minutes: Some(7),
            session_local_timeout: Some(true),
            session_remote_timeout: Some(false),
            inactivity_purge_after_turns: Some(12),
            inactivity_autopilot_after_turns: Some(5),
        },
        reservations: Vec::new(),
    };

    let first = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: config.clone(),
    })
    .expect("app should load and persist config");
    assert_ne!(
        before_setup.snoop_enabled(),
        first.game_data.setup.snoop_enabled()
    );
    assert_ne!(
        before_setup.max_time_between_keys_minutes_raw(),
        first.game_data.setup.max_time_between_keys_minutes_raw()
    );
    assert_ne!(
        before_setup.minimum_time_granted_minutes_raw(),
        first.game_data.setup.minimum_time_granted_minutes_raw()
    );
    assert_ne!(
        before_setup.local_timeout_enabled(),
        first.game_data.setup.local_timeout_enabled()
    );
    assert_ne!(
        before_setup.remote_timeout_enabled(),
        first.game_data.setup.remote_timeout_enabled()
    );
    assert_ne!(
        before_setup.purge_after_turns_raw(),
        first.game_data.setup.purge_after_turns_raw()
    );
    assert_ne!(
        before_setup.autopilot_inactive_turns_raw(),
        first.game_data.setup.autopilot_inactive_turns_raw()
    );

    let persisted = latest_runtime_state(&fixture_dir);
    let setup = &persisted.game_data.setup;
    assert!(!setup.snoop_enabled());
    assert_eq!(setup.max_time_between_keys_minutes_raw(), 19);
    assert_eq!(setup.minimum_time_granted_minutes_raw(), 7);
    assert!(setup.local_timeout_enabled());
    assert!(!setup.remote_timeout_enabled());
    assert_eq!(setup.purge_after_turns_raw(), 12);
    assert_eq!(setup.autopilot_inactive_turns_raw(), 5);

    let second = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: config,
    })
    .expect("second load should reuse persisted config snapshot");
    assert_eq!(second.game_data.setup, first.game_data.setup);
}

#[test]
fn apply_action_quit_exits_loop() {
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

    assert_eq!(apply_action(&mut app, Action::Quit), AppOutcome::Quit);
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Splash)
    );
}

#[test]
fn main_menu_keys_open_existing_shared_screens_and_return_to_main() {
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

    assert_eq!(
        app.handle_key(key(KeyCode::Char('b'))),
        Action::Empire(EmpireAction::OpenStatus)
    );
    assert_eq!(
        apply_action(&mut app, Action::Empire(EmpireAction::OpenStatus)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::EmpireStatus);
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::ReturnToCommandMenu
    );
    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(
        app.handle_key(key(KeyCode::Char('f'))),
        Action::Fleet(FleetAction::OpenMenu)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('h'))),
        Action::OpenPopupHelp
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenPopupHelp),
        AppOutcome::Continue
    );
    assert!(app.popup_help.is_some());
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::DismissPopupHelp
    );
    assert_eq!(
        apply_action(&mut app, Action::DismissPopupHelp),
        AppOutcome::Continue
    );
    assert!(app.popup_help.is_none());
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('i'))),
        Action::Planet(PlanetAction::OpenInfoPrompt(CommandMenu::Fleet))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenInfoPrompt(CommandMenu::Fleet))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Planet(PlanetAction::CloseInfoPrompt)
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::CloseInfoPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('v'))),
        Action::Starmap(StarmapAction::OpenPartialView(CommandMenu::Fleet))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Starmap(StarmapAction::OpenPartialView(CommandMenu::Fleet))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PartialStarmapView);
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::Starmap(StarmapAction::OpenPlanetInfoAtCenter)
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Starmap(StarmapAction::OpenPlanetInfoAtCenter)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoDetail);
    assert_eq!(
        app.handle_key(key(KeyCode::Char(' '))),
        Action::ReturnToCommandMenu
    );
    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PartialStarmapView);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::ReturnToCommandMenu
    );
    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('c'))),
        Action::Fleet(FleetAction::OpenChangePrompt)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenChangePrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    submit_fleet_menu_prompt(&mut app, Some(1));
    submit_fleet_menu_prompt_value(&mut app, "R");
    submit_fleet_menu_prompt_value(&mut app, "4");
    assert_eq!(app.current_fleet_roe_by_id(1), Some(4));
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::OpenMainMenu
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenMainMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('e'))),
        Action::Fleet(FleetAction::OpenEta)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenEta)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Fleet(FleetAction::CancelMenuPrompt)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CancelMenuPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(app.handle_key(key(KeyCode::Char('b'))), Action::Noop);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('f'))),
        Action::Fleet(FleetAction::OpenList)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenList)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::Fleet(FleetAction::OpenReview)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReview)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetReview);
    assert_eq!(
        app.handle_key(key(KeyCode::Esc)),
        Action::Fleet(FleetAction::CloseReview)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CloseReview)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('r'))),
        Action::Fleet(FleetAction::OpenReviewPrompt)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReviewPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('7'))),
        Action::Fleet(FleetAction::AppendMenuPromptChar('7'))
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Backspace)),
        Action::Fleet(FleetAction::BackspaceMenuPromptInput)
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::Fleet(FleetAction::SubmitMenuPrompt)
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Fleet(FleetAction::CancelMenuPrompt)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::OpenMainMenu
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenMainMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(
        app.handle_key(key(KeyCode::Char('i'))),
        Action::Planet(PlanetAction::OpenInfoPrompt(CommandMenu::Main))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenInfoPrompt(CommandMenu::Main))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Planet(PlanetAction::CloseInfoPrompt)
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::CloseInfoPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenInfoPrompt(CommandMenu::Main))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitInfoPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoDetail);
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::ReturnToCommandMenu
    );
    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(
        app.handle_key(key(KeyCode::Char('v'))),
        Action::Starmap(StarmapAction::OpenPartialView(CommandMenu::Main))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Starmap(StarmapAction::OpenPartialView(CommandMenu::Main))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PartialStarmapView);
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::Starmap(StarmapAction::OpenPlanetInfoAtCenter)
    );
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
        app.handle_key(key(KeyCode::Esc)),
        Action::ReturnToCommandMenu
    );
    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(
        app.handle_key(key(KeyCode::Char('t'))),
        Action::Planet(PlanetAction::OpenDatabase)
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenDatabase)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetDatabaseList);
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::Planet(PlanetAction::SubmitDatabaseLookup)
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitDatabaseLookup)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoDetail);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::ReturnToCommandMenu
    );
    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetDatabaseList);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::ReturnToCommandMenu
    );
    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);
}

#[test]
fn fleet_review_detail_from_menu_uses_dismiss_prompt_and_returns_to_menu() {
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
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReviewPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMenuPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetReview);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet review should render");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("(slap a key)"))
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Char('x'))),
        Action::Fleet(FleetAction::CloseReview)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CloseReview)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('r'))),
        Action::Fleet(FleetAction::OpenReviewPrompt)
    );
}

#[test]
fn fleet_menu_matches_verified_v15_command_layout() {
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
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("fleet menu should render");
    assert_eq!(
        terminal.line(1).trim_end(),
        " FLEET COMMAND CENTER:                                       O>rder a Fleet"
    );
    assert_eq!(
        terminal.line(2).trim_end(),
        "  H>elp on Options   S>TARBASE MENU...   C>hg ROE,ID,Speed   G>ROUP FLEET ORDER"
    );
    assert_eq!(
        terminal.line(3).trim_end(),
        "  Q>uit: Main Menu   E>TA Calc           I>nfo about Planet  M>erge a Fleet"
    );
    assert_eq!(
        terminal.line(4).trim_end(),
        "  X>pert Mode        F>leet List         D>etach Ships       L>oad TTs w/Armies"
    );
    assert_eq!(
        terminal.line(5).trim_end(),
        "  V>iew Partial Map  R>eview a Fleet     T>ransfer Ships     U>nload TT Armies"
    );
}

#[test]
fn starbase_menu_matches_verified_v15_command_layout() {
    let fixture_dir = temp_game_with_starbase_copy();
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
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Starbase(StarbaseAction::OpenMenu)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("starbase menu should render");
    assert_eq!(
        terminal.line(0).trim_end(),
        "STARBASE CONTROL:            X>pert mode ON/OFF     V>iew Partial Star Map"
    );
    assert_eq!(
        terminal.line(1).trim_end(),
        "  H>elp with commands        S>tarbases-List        I>nfo about a Planet"
    );
    assert_eq!(
        terminal.line(2).trim_end(),
        "  Q>uit to Fleet Command     R>eview a Starbase     M>ove/Halt Starbase"
    );
    assert_eq!(
        line_containing(&terminal, "STARBASE COMMAND <-").trim_end(),
        " STARBASE COMMAND <- ? X S R V I M <Q> ->"
    );
}

#[test]
fn starbase_review_matches_verified_v15_review_content() {
    let fixture_dir = temp_game_with_starbase_copy();
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
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Starbase(StarbaseAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Starbase(StarbaseAction::OpenReviewSelect)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Starbase(StarbaseAction::SubmitReviewSelect)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::StarbaseReview);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("starbase review should render");
    assert_eq!(terminal.line(3).trim_end(), " Starbase ID: Starbase 1");
    assert_eq!(
        terminal.line(4).trim_end(),
        " Location:    World in Solar System [ 6, 5]"
    );
    assert_eq!(
        terminal.line(5).trim_end(),
        " Destination: World in Solar System [ 6, 5]"
    );
    assert_eq!(
        terminal.line(6).trim_end(),
        " Operation:   Protection & Enhancement"
    );
    assert_eq!(
        terminal.line(7).trim_end(),
        " ETA:         Starbase 1 has already arrived and is in operation."
    );
    assert_eq!(terminal.line(8).trim_end(), " Escort:      The 1st Fleet");
}

#[test]
fn starbase_list_and_review_show_numeric_transit_eta() {
    let fixture_dir = temp_game_with_starbase_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    runtime.game_data.bases.records[0].set_coords_raw([15, 13]);
    runtime.game_data.bases.records[0].set_trailing_coords_raw([2, 12]);
    save_runtime_state(&fixture_dir, &runtime);

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
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Starbase(StarbaseAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Starbase(StarbaseAction::OpenList)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("starbase list should render");
    let list_row = line_containing(&terminal, "System(15,13)");
    assert!(list_row.contains("System(15,13)"));
    assert!(list_row.contains("System(02,12)"));
    assert!(list_row.contains("16"));
    assert!(list_row.contains("Starbase in transit"));

    assert_eq!(
        apply_action(&mut app, Action::Starbase(StarbaseAction::OpenReview)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::StarbaseReview);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("starbase review should render");
    assert_eq!(
        terminal.line(7).trim_end(),
        " ETA:         Starbase 1 is in transit with ETA 16 years."
    );
}

#[test]
fn starbase_move_prompt_accepts_non_planet_sector_and_persists_report() {
    let fixture_dir = temp_game_with_starbase_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    runtime.game_data.fleets.records[1].set_current_location_coords_raw([6, 5]);
    runtime.game_data.fleets.records[1].set_standing_order_kind(nc_data::Order::GuardStarbase);
    runtime.game_data.fleets.records[1].set_standing_order_target_coords_raw([6, 5]);
    runtime.game_data.fleets.records[1].set_mission_aux_bytes([1, 1]);
    save_runtime_state(&fixture_dir, &runtime);
    let destination = first_empty_sector(&runtime.game_data);

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
    apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu));
    apply_action(&mut app, Action::Starbase(StarbaseAction::OpenMenu));
    apply_action(&mut app, Action::Starbase(StarbaseAction::OpenMovePrompt));

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("starbase move base prompt should render");
    assert!(
        line_containing(&terminal, "STARBASE COMMAND <- Starbase #")
            .contains("Starbase # [1] <Q> ->")
    );

    apply_action(&mut app, Action::Starbase(StarbaseAction::SubmitMovePrompt));
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("starbase move decision prompt should render");
    assert!(
        line_containing(&terminal, "STARBASE COMMAND <- <H>alt or")
            .contains("<H>alt or [M]ove <Q> ->")
    );

    apply_action(&mut app, Action::Starbase(StarbaseAction::SubmitMovePrompt));
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("starbase move destination prompt should render");
    assert!(
        line_containing(&terminal, "STARBASE COMMAND <- Destination")
            .contains("Destination [06,05] <Q> ->")
    );

    for ch in format!("{:02},{:02}", destination[0], destination[1]).chars() {
        apply_action(
            &mut app,
            Action::Starbase(StarbaseAction::AppendMovePromptChar(ch)),
        );
    }
    apply_action(&mut app, Action::Starbase(StarbaseAction::SubmitMovePrompt));

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("starbase menu notice should render after move");
    assert!(line_containing(&terminal, "Notice: ").contains(&format!(
        "Starbase #1 ordered to move to ({:02},{:02}).",
        destination[0], destination[1]
    )));

    let persisted = latest_runtime_state(&fixture_dir);
    assert_eq!(
        persisted.game_data.bases.records[0].coords_raw(),
        [6, 5],
        "live base location should stay unchanged until movement execution exists"
    );
    assert_eq!(
        persisted.game_data.bases.records[0].trailing_coords_raw(),
        destination
    );
    let latest_report = persisted
        .report_block_rows
        .last()
        .expect("move should append a report block");
    assert!(latest_report.decoded_text.contains(&format!(
        "Starbase 1 is moving to ({:02},{:02}).",
        destination[0], destination[1]
    )));
    assert!(
        latest_report
            .decoded_text
            .contains("Guard Fleets 1 and 2 will follow it.")
    );
}

#[test]
fn starbase_help_uses_move_wording_not_hauling() {
    let fixture_dir = temp_game_with_starbase_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    for _ in 0..64 {
        if app.current_screen() == ScreenId::MainMenu {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(app.current_screen(), ScreenId::MainMenu);
    apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu));
    apply_action(&mut app, Action::Starbase(StarbaseAction::OpenMenu));
    apply_action(&mut app, Action::OpenPopupHelp);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("starbase help should render");
    assert!(
        line_containing(&terminal, "move to a new location").contains("move to a new location")
    );
    assert!(
        !terminal.lines.iter().any(|line| line.contains("hauled")),
        "player-facing help should not mention hauling"
    );
}

#[test]
fn starbase_move_prompt_errors_hang_directly_under_command_line() {
    let fixture_dir = temp_game_with_starbase_copy();
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
    apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu));
    apply_action(&mut app, Action::Starbase(StarbaseAction::OpenMenu));
    apply_action(&mut app, Action::Starbase(StarbaseAction::OpenMovePrompt));
    apply_action(&mut app, Action::Starbase(StarbaseAction::SubmitMovePrompt));
    apply_action(&mut app, Action::Starbase(StarbaseAction::SubmitMovePrompt));
    for ch in "99,99".chars() {
        apply_action(
            &mut app,
            Action::Starbase(StarbaseAction::AppendMovePromptChar(ch)),
        );
    }
    apply_action(&mut app, Action::Starbase(StarbaseAction::SubmitMovePrompt));

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("starbase move error should render");
    let error_line = terminal
        .lines
        .iter()
        .find(|line| line.contains("Error: "))
        .expect("error hanger should render");
    assert!(error_line.contains("Enter coordinates within 1.."));
    let error_row = terminal
        .lines
        .iter()
        .position(|line| line.contains("Error: "))
        .expect("error hanger should render");
    assert_eq!(error_row, 6, "error hanger should sit under the prompt");
}

#[test]
fn starbase_halt_prompt_resets_destination_to_live_location() {
    let fixture_dir = temp_game_with_starbase_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    runtime.game_data.bases.records[0].set_trailing_coords_raw([2, 12]);
    save_runtime_state(&fixture_dir, &runtime);

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
    apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu));
    apply_action(&mut app, Action::Starbase(StarbaseAction::OpenMenu));
    apply_action(&mut app, Action::Starbase(StarbaseAction::OpenMovePrompt));
    apply_action(&mut app, Action::Starbase(StarbaseAction::SubmitMovePrompt));
    apply_action(
        &mut app,
        Action::Starbase(StarbaseAction::AppendMovePromptChar('H')),
    );
    apply_action(&mut app, Action::Starbase(StarbaseAction::SubmitMovePrompt));

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("halt confirm should render");
    assert!(
        line_containing(&terminal, "STARBASE COMMAND <- Halt this starbase?")
            .contains("Halt this starbase? [Y]/N ->")
    );

    apply_action(&mut app, Action::Starbase(StarbaseAction::SubmitMovePrompt));
    let persisted = latest_runtime_state(&fixture_dir);
    assert_eq!(
        persisted.game_data.bases.records[0].trailing_coords_raw(),
        [6, 5]
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("starbase halt notice should render");
    assert!(line_containing(&terminal, "Notice: ").contains("Starbase #1 halted at (06,05)."));
}

#[test]
fn fleet_transfer_uses_two_inline_fleet_prompts_before_quantity_entry() {
    let fixture_dir = temp_game_with_same_sector_fleets_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    state.game_data.fleets.records[0].set_troop_transport_count(2);
    state.game_data.fleets.records[0].set_army_count(2);
    state.game_data.fleets.records[0].recompute_max_speed_from_composition();
    state.game_data.fleets.records[1].set_troop_transport_count(1);
    state.game_data.fleets.records[1].set_army_count(1);
    state.game_data.fleets.records[1].recompute_max_speed_from_composition();
    save_runtime_state(&fixture_dir, &state);
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
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransfer)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("transfer donor prompt should render");
    assert!(
        line_containing(&terminal, "COMMAND <- Transfer From Fleet #")
            .contains("Transfer From Fleet # [")
    );

    submit_fleet_menu_prompt(&mut app, Some(1));
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    app.render(&mut terminal)
        .expect("transfer host prompt should render");
    assert!(
        line_containing(&terminal, "COMMAND <- Transfer To Fleet #")
            .contains("Transfer To Fleet # [")
    );

    submit_fleet_menu_prompt(&mut app, Some(2));
    assert_eq!(app.current_screen(), ScreenId::FleetTransfer);
    app.render(&mut terminal)
        .expect("transfer quantity screen should render");
    assert_eq!(
        terminal.line(0).trim_end(),
        "TRANSFER SHIPS BETWEEN FLEETS:"
    );
    assert!(line_containing(&terminal, "Source Fleet: Fleet #1").contains("Source Fleet:"));
    assert!(
        line_containing(&terminal, "Destination Fleet: Fleet #2").contains("Destination Fleet:")
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Ships: ") && line.contains("TT*"))
    );
    assert!(terminal.lines.iter().all(|line| !line.contains("AR=")));
    assert!(line_containing(&terminal, "Class <BB,CA,DD,TT*,TT,SC,ET,C,X>").contains("<Q> ->"));
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Staged to Transfer: none"))
    );
}

#[test]
fn fleet_transfer_source_prompt_defaults_to_largest_eligible_fleet() {
    let fixture_dir = temp_game_with_same_sector_fleets_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    state.game_data.fleets.records[0].set_battleship_count(1);
    state.game_data.fleets.records[0].set_cruiser_count(1);
    state.game_data.fleets.records[0].set_destroyer_count(1);
    state.game_data.fleets.records[1].set_battleship_count(0);
    state.game_data.fleets.records[1].set_cruiser_count(0);
    state.game_data.fleets.records[1].set_destroyer_count(0);
    state.game_data.fleets.records[1].set_scout_count(1);
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    for _ in 0..64 {
        if app.current_screen() == ScreenId::MainMenu {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(app.current_screen(), ScreenId::MainMenu);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransfer)),
        AppOutcome::Continue
    );
    assert_eq!(app.fleet.menu_prompt_default_value, "1");
}

#[test]
fn fleet_transfer_source_prompt_rejects_one_ship_fleet() {
    let fixture_dir = temp_game_with_same_sector_fleets_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    state.game_data.fleets.records[0].set_battleship_count(0);
    state.game_data.fleets.records[0].set_cruiser_count(0);
    state.game_data.fleets.records[0].set_destroyer_count(0);
    state.game_data.fleets.records[0].set_troop_transport_count(0);
    state.game_data.fleets.records[0].set_scout_count(1);
    state.game_data.fleets.records[0].set_etac_count(0);
    state.game_data.fleets.records[0].recompute_max_speed_from_composition();
    state.game_data.fleets.records[1].set_scout_count(2);
    state.game_data.fleets.records[1].recompute_max_speed_from_composition();
    save_runtime_state(&fixture_dir, &state);

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransfer)),
        AppOutcome::Continue
    );
    submit_fleet_menu_prompt(&mut app, Some(1));
    assert_eq!(
        app.fleet
            .menu_prompt_status
            .as_ref()
            .map(|feedback| feedback.message()),
        Some("Fleet #1 has only one ship and is not eligible to transfer any ships.")
    );
}

#[test]
fn fleet_transfer_destination_prompt_defaults_to_smallest_colocated_fleet() {
    let fixture_dir = temp_game_with_same_sector_fleets_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    state.game_data.fleets.records[0].set_battleship_count(1);
    state.game_data.fleets.records[0].set_cruiser_count(1);
    state.game_data.fleets.records[0].set_destroyer_count(2);
    state.game_data.fleets.records[1].set_battleship_count(0);
    state.game_data.fleets.records[1].set_cruiser_count(0);
    state.game_data.fleets.records[1].set_destroyer_count(1);
    save_runtime_state(&fixture_dir, &state);

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransfer)),
        AppOutcome::Continue
    );
    submit_fleet_menu_prompt(&mut app, Some(1));
    assert_eq!(app.fleet.menu_prompt_default_value, "2");
}

#[test]
fn fleet_transfer_destination_prompt_rejects_non_colocated_fleet() {
    let fixture_dir = temp_game_with_same_sector_fleets_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    state.game_data.fleets.records[2].set_current_location_coords_raw([1, 1]);
    state.game_data.fleets.records[2].set_standing_order_target_coords_raw([1, 1]);
    save_runtime_state(&fixture_dir, &state);

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransfer)),
        AppOutcome::Continue
    );
    submit_fleet_menu_prompt(&mut app, Some(1));
    submit_fleet_menu_prompt(&mut app, Some(3));
    assert_eq!(
        app.fleet
            .menu_prompt_status
            .as_ref()
            .map(|feedback| feedback.message()),
        Some("Fleet #3 is not in the same sector as Fleet #1.")
    );
}

#[test]
fn general_rankings_opens_production_table_and_returns_to_general_menu() {
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
        app.handle_key(key(KeyCode::Char('o'))),
        Action::Empire(EmpireAction::OpenRankingsTable(
            EmpireProductionRankingSort::Production
        ))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Empire(EmpireAction::OpenRankingsTable(
                EmpireProductionRankingSort::Production
            ))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::Rankings(EmpireProductionRankingSort::Production)
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::ReturnToCommandMenu
    );
    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);
}

#[test]
fn apply_action_toggles_autopilot_and_enemy_relation() {
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

    let initial_autopilot = app.current_autopilot_flag();
    assert_eq!(
        apply_action(&mut app, Action::ToggleAutopilot),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_autopilot_flag(),
        if initial_autopilot == 0 { 1 } else { 0 }
    );

    assert_eq!(
        apply_action(&mut app, Action::Empire(EmpireAction::OpenEnemies)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Enemies);
    assert_eq!(
        app.current_relation_to(2),
        Some(DiplomaticRelation::Neutral)
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Empire(EmpireAction::AppendEnemiesChar('2'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Empire(EmpireAction::SubmitEnemiesInput)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_relation_to(2), Some(DiplomaticRelation::Enemy));
}

#[test]
fn apply_action_clamps_enemies_scroll_to_visible_window() {
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

    assert_eq!(
        apply_action(&mut app, Action::Empire(EmpireAction::OpenEnemies)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Enemies);

    for _ in 0..50 {
        assert_eq!(
            apply_action(&mut app, Action::Empire(EmpireAction::ScrollEnemies(1))),
            AppOutcome::Continue
        );
    }

    assert_eq!(app.enemies_scroll_offset(), 0);
}

#[test]
fn enemies_typed_empire_number_moves_selector_bar_immediately() {
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

    assert_eq!(
        apply_action(&mut app, Action::Empire(EmpireAction::OpenEnemies)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Enemies);
    assert_eq!(app.empire.enemies_cursor, 0);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Empire(EmpireAction::AppendEnemiesChar('3'))
        ),
        AppOutcome::Continue
    );

    assert_eq!(app.empire.enemies_cursor, 1);
    assert_eq!(app.enemies_scroll_offset(), 0);
}

#[test]
fn apply_action_deletes_reviewables() {
    let fixture_dir = temp_game_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(&mut runtime, b"test results");
    runtime.queued_mail.push(incoming_mail(
        2,
        1,
        runtime.game_data.conquest.game_year().saturating_sub(1),
        "Orders",
        "test messages",
    ));
    runtime.game_data.player.records[0].raw[0x30] = 1;
    runtime.game_data.player.records[0].raw[0x34] = 1;
    save_runtime_state(&fixture_dir, &runtime);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenDeleteReviewables)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);
    assert!(app.messaging.delete_reviewables_prompt_active);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::ConfirmDeleteReviewables)
        ),
        AppOutcome::Continue
    );

    let runtime = latest_runtime_state(&fixture_dir);
    assert!(
        runtime
            .report_block_rows
            .iter()
            .all(|row| row.recipient_deleted)
    );
    assert_eq!(runtime.queued_mail.len(), 1);
    assert!(runtime.queued_mail[0].recipient_deleted);
    assert_eq!(runtime.game_data.player.records[0].raw[0x30], 0);
    assert_eq!(runtime.game_data.player.records[0].raw[0x34], 0);
    assert!(!app.messaging.delete_reviewables_prompt_active);
}
