use crate::support::*;

#[test]
fn first_time_menu_branch_opens_help_intro_and_empire_list() {
    let fixture_dir = temp_first_time_game_copy();
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

    advance_to_first_time_menu(&mut app);
    assert_eq!(app.current_screen(), ScreenId::FirstTimeMenu);

    assert_eq!(
        apply_action(&mut app, Action::OpenPopupHelp),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeMenu);
    assert!(app.popup_help.is_some());
    assert_eq!(
        apply_action(&mut app, Action::DismissPopupHelp),
        AppOutcome::Continue
    );
    assert!(app.popup_help.is_none());

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::OpenFirstTimeEmpires)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeEmpires);

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenFirstTimeIntro)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeIntro);
}

#[test]
fn first_time_startup_skips_joined_player_login_summary() {
    let fixture_dir = temp_first_time_game_copy();
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

    apply_action(&mut app, Action::Startup(StartupAction::SkipIntro));
    assert_eq!(app.current_screen(), ScreenId::FirstTimeMenu);
    assert_eq!(app.classic_login_state(), ClassicLoginState::FirstTimeMenu);
}

#[test]
fn joined_player_with_unnamed_homeworld_is_routed_to_homeworld_naming() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
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
        app.classic_login_state(),
        ClassicLoginState::MatchedPreloadedFirstLogin
    );

    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Splash)
    );

    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimePreloadedRenamePrompt {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );
}

#[test]
fn player_two_joined_with_unnamed_homeworld_is_not_retreated_as_first_time() {
    let fixture_dir = temp_joined_needs_homeworld_copy_for_player(2);
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 2,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    assert_eq!(
        app.classic_login_state(),
        ClassicLoginState::MatchedPreloadedFirstLogin
    );

    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimePreloadedRenamePrompt {
            break;
        }
        app.advance_startup();
    }

    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );
}

#[test]
fn reserved_first_time_player_skips_menu_and_sees_reserved_prompt() {
    let fixture_dir = temp_first_time_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: reserved_game_config(1, "SYSOP"),
    })
    .expect("app should load");
    app.door_mode = true;

    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimeReservedPrompt {
            break;
        }
        app.advance_startup();
    }

    assert_eq!(app.current_screen(), ScreenId::FirstTimeReservedPrompt);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("reserved prompt should render");
    assert!(
        terminal
            .line(2)
            .contains("This player seat is reserved for you.")
    );
    assert!(terminal.line(6).contains("You may name your empire now"));
}

#[test]
fn reserved_local_first_time_player_without_door_mode_still_sees_first_time_menu() {
    let fixture_dir = temp_first_time_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: reserved_game_config(1, "SYSOP"),
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimeMenu {
            break;
        }
        app.advance_startup();
    }

    assert_eq!(app.current_screen(), ScreenId::FirstTimeMenu);
}

#[test]
fn preloaded_first_login_routes_through_login_summary_before_rename_prompt() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    let mut saw_login_summary = false;
    let mut saw_summary_year_text = false;
    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::LoginSummary) {
            saw_login_summary = true;
            let mut terminal = CaptureTerminal::new();
            app.render(&mut terminal)
                .expect("login summary should render");
            saw_summary_year_text = terminal
                .lines
                .iter()
                .any(|line| line.contains("The year is:"));
        }
        if app.current_screen() == ScreenId::FirstTimePreloadedRenamePrompt {
            break;
        }
        app.advance_startup();
    }

    assert!(saw_login_summary);
    assert!(saw_summary_year_text);
    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("rename prompt should render");
    assert!(
        terminal
            .line(2)
            .contains("This empire is already joined, and this is your first login.")
    );
    assert!(terminal.line(6).contains("Rename your empire? Y/[N] ->"));
}

#[test]
fn preloaded_first_login_becomes_returning_player_after_homeworld_naming() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimePreloadedRenamePrompt {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::RejectFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinSummary);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinNoPending);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldName);

    for ch in "Codex Prime".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldConfirm);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    let reloaded = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("reloaded app should load");

    assert_eq!(
        reloaded.classic_login_state(),
        ClassicLoginState::ReturningPlayer
    );
    assert_eq!(
        reloaded.current_screen(),
        ScreenId::Startup(StartupPhase::Splash)
    );
}

#[test]
fn first_time_join_summary_and_no_pending_accept_any_key_dismissal() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimePreloadedRenamePrompt {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::RejectFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinSummary);
    assert_eq!(
        app.handle_key(key(KeyCode::Char(' '))),
        Action::Startup(StartupAction::AcceptFirstTimePrompt)
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinNoPending);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('x'))),
        Action::Startup(StartupAction::AcceptFirstTimePrompt)
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldName);
}

#[test]
fn preloaded_first_login_can_rename_empire_before_homeworld_naming() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimePreloadedRenamePrompt {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    let mut rename_terminal = CaptureTerminal::new();
    app.render(&mut rename_terminal)
        .expect("rename input should render");
    assert!(
        rename_terminal
            .line(2)
            .contains("This empire is already joined, and this is your first login.")
    );

    for _ in 0..24 {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::BackspaceFirstTimeInput)
            ),
            AppOutcome::Continue
        );
    }
    for ch in "Codex Dominion".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireConfirm);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinSummary);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinNoPending);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldName);

    let runtime = latest_runtime_state(&fixture_dir);
    assert_eq!(
        runtime.game_data.player.records[0].controlled_empire_name_summary(),
        "Codex Dominion"
    );
}

#[test]
fn first_time_join_empire_name_prompt_shows_esc_without_redundant_instruction() {
    let fixture_dir = temp_first_time_game_copy();
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
        apply_action(
            &mut app,
            Action::Startup(StartupAction::OpenFirstTimeJoinName)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("first-time empire naming prompt should render");

    assert!(line_containing(&terminal, "EMPIRE NAME <-").contains("<ESC> ->"));
    assert!(!terminal.lines.iter().any(|line| line.contains("Press Esc")));
}

#[test]
fn preloaded_first_login_empire_name_prompt_shows_esc_without_redundant_instruction() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimePreloadedRenamePrompt {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("preloaded empire naming prompt should render");

    assert!(line_containing(&terminal, "EMPIRE NAME <-").contains("<ESC> ->"));
    assert!(!terminal.lines.iter().any(|line| line.contains("Press Esc")));
}

#[test]
fn first_time_homeworld_name_prompt_shows_esc_without_redundant_instruction() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimePreloadedRenamePrompt {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::RejectFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldName);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("homeworld naming prompt should render");

    assert!(line_containing(&terminal, "HOMEWORLD <-").contains("<ESC> ->"));
    assert!(!terminal.lines.iter().any(|line| line.contains("Press Esc")));
}

#[test]
fn preloaded_empire_rename_failure_returns_to_name_entry_with_status() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimePreloadedRenamePrompt {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    let attempted_name = app.startup_state.first_time_input.clone();
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireConfirm);

    make_runtime_db_read_only(&fixture_dir);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);
    assert!(app.startup_state.first_time_rename_preloaded_empire);
    assert_eq!(app.startup_state.first_time_input, attempted_name);
    assert_eq!(
        app.startup_state.first_time_status.as_deref(),
        Some("Unable to save your empire name right now. Please try again.")
    );
}

#[test]
fn returning_player_with_owned_unnamed_colony_is_routed_to_colony_naming() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize;
    let colony = state
        .game_data
        .planets
        .records
        .iter_mut()
        .enumerate()
        .find(|(idx, planet)| *idx + 1 != homeworld_index && planet.owner_empire_slot_raw() != 1)
        .expect("need a non-homeworld planet for colony naming test");
    colony.1.set_owner_empire_slot_raw(1);
    colony.1.set_planet_name("Not Named Yet");
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

    for _ in 0..16 {
        if app.current_screen() == ScreenId::ColonyWorldName {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldName);
}

#[test]
fn colony_world_naming_updates_planet_and_enters_main_menu() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize;
    let colony_index = state
        .game_data
        .planets
        .records
        .iter_mut()
        .enumerate()
        .find(|(idx, planet)| *idx + 1 != homeworld_index && planet.owner_empire_slot_raw() != 1)
        .map(|(idx, planet)| {
            planet.set_owner_empire_slot_raw(1);
            planet.set_planet_name("Not Named Yet");
            idx + 1
        })
        .expect("need a non-homeworld planet for colony naming test");
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

    for _ in 0..16 {
        if app.current_screen() == ScreenId::ColonyWorldName {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldName);

    for ch in "New Horizon".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldConfirm);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    let runtime = latest_runtime_state(&fixture_dir);
    assert_eq!(
        runtime.game_data.planets.records[colony_index - 1].planet_name(),
        "New Horizon"
    );
}

#[test]
fn colony_world_name_failure_returns_to_name_entry_with_status() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize;
    state
        .game_data
        .planets
        .records
        .iter_mut()
        .enumerate()
        .find(|(idx, planet)| *idx + 1 != homeworld_index && planet.owner_empire_slot_raw() != 1)
        .map(|(_, planet)| {
            planet.set_owner_empire_slot_raw(1);
            planet.set_planet_name("Not Named Yet");
        })
        .expect("need a non-homeworld planet for colony naming test");
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

    for _ in 0..16 {
        if app.current_screen() == ScreenId::ColonyWorldName {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldName);

    for ch in "New Horizon".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldConfirm);

    app.startup_state.colony_world_planet_record_index_1_based = Some(999);
    make_runtime_db_read_only(&fixture_dir);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldName);
    assert_eq!(app.startup_state.first_time_input, "New Horizon");
    assert_eq!(
        app.startup_state.first_time_status.as_deref(),
        Some("Unable to save the world name right now. Please try again.")
    );
}

#[test]
fn colony_world_naming_cannot_be_escaped_to_main_menu() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize;
    state
        .game_data
        .planets
        .records
        .iter_mut()
        .enumerate()
        .find(|(idx, planet)| *idx + 1 != homeworld_index && planet.owner_empire_slot_raw() != 1)
        .expect("need a non-homeworld planet for colony naming test")
        .1
        .set_owner_empire_slot_raw(1);
    state
        .game_data
        .planets
        .records
        .iter_mut()
        .enumerate()
        .find(|(idx, planet)| *idx + 1 != homeworld_index && planet.owner_empire_slot_raw() == 1)
        .expect("need owned unnamed colony")
        .1
        .set_planet_name("Not Named Yet");
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

    for _ in 0..16 {
        if app.current_screen() == ScreenId::ColonyWorldName {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldName);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::RejectFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldName);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("colony world naming screen should render");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("must name this newly colonized world before continuing"))
    );
}

#[test]
fn colony_world_name_prompt_shows_esc_without_redundant_instruction() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize;
    state
        .game_data
        .planets
        .records
        .iter_mut()
        .enumerate()
        .find(|(idx, planet)| *idx + 1 != homeworld_index && planet.owner_empire_slot_raw() != 1)
        .expect("need a non-homeworld planet for colony naming test")
        .1
        .set_owner_empire_slot_raw(1);
    state
        .game_data
        .planets
        .records
        .iter_mut()
        .enumerate()
        .find(|(idx, planet)| *idx + 1 != homeworld_index && planet.owner_empire_slot_raw() == 1)
        .expect("need owned unnamed colony")
        .1
        .set_planet_name("Not Named Yet");
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

    for _ in 0..16 {
        if app.current_screen() == ScreenId::ColonyWorldName {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldName);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("colony world naming prompt should render");

    assert!(line_containing(&terminal, "WORLD NAME <-").contains("<ESC> ->"));
    assert!(!terminal.lines.iter().any(|line| line.contains("Press Esc")));
}

#[test]
fn first_time_join_routes_from_homeworld_naming_to_colony_naming_when_needed() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize;
    let colony_index = state
        .game_data
        .planets
        .records
        .iter_mut()
        .enumerate()
        .find(|(idx, planet)| *idx + 1 != homeworld_index && planet.owner_empire_slot_raw() != 1)
        .map(|(idx, planet)| {
            planet.set_owner_empire_slot_raw(1);
            planet.set_planet_name("Not Named Yet");
            idx + 1
        })
        .expect("need a non-homeworld planet for colony naming test");
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should reload");
    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimePreloadedRenamePrompt {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::RejectFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinSummary);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinNoPending);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldName);

    for ch in "Codex Prime".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldConfirm);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldName);

    for ch in "New Horizon".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldConfirm);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    let runtime = latest_runtime_state(&fixture_dir);
    assert_eq!(
        runtime.game_data.planets.records[colony_index - 1].planet_name(),
        "New Horizon"
    );
}

#[test]
fn returning_player_with_multiple_unnamed_colonies_is_prompted_for_each_in_turn() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize;
    let mut renamed_targets = Vec::new();
    for (idx, planet) in state.game_data.planets.records.iter_mut().enumerate() {
        if idx + 1 == homeworld_index || planet.owner_empire_slot_raw() == 1 {
            continue;
        }
        planet.set_owner_empire_slot_raw(1);
        planet.set_planet_name("Not Named Yet");
        renamed_targets.push(idx + 1);
        if renamed_targets.len() == 2 {
            break;
        }
    }
    assert_eq!(
        renamed_targets.len(),
        2,
        "need two colony worlds for sequencing test"
    );
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

    for _ in 0..16 {
        if app.current_screen() == ScreenId::ColonyWorldName {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldName);

    for ch in "New Horizon".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldConfirm);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldName);

    for ch in "Second Dawn".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldConfirm);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    let runtime = latest_runtime_state(&fixture_dir);
    assert_eq!(
        runtime.game_data.planets.records[renamed_targets[0] - 1].planet_name(),
        "New Horizon"
    );
    assert_eq!(
        runtime.game_data.planets.records[renamed_targets[1] - 1].planet_name(),
        "Second Dawn"
    );
}

#[test]
fn returning_player_routes_through_login_summary_before_main_menu() {
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
        app.classic_login_state(),
        ClassicLoginState::ReturningPlayer
    );

    let mut saw_login_summary = false;
    let mut saw_summary_year_text = false;
    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::LoginSummary) {
            saw_login_summary = true;
            let mut terminal = CaptureTerminal::new();
            app.render(&mut terminal)
                .expect("login summary should render");
            saw_summary_year_text = terminal
                .lines
                .iter()
                .any(|line| line.contains("The year is:"));
        }
        if app.current_screen() == ScreenId::MainMenu {
            break;
        }
        app.advance_startup();
    }

    assert!(saw_login_summary);
    assert!(saw_summary_year_text);
    assert_eq!(app.current_screen(), ScreenId::MainMenu);
}

#[test]
fn escaping_empire_name_does_not_partially_join_player() {
    let fixture_dir = temp_first_time_game_copy();
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
            Action::Startup(StartupAction::OpenFirstTimeJoinName)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    for ch in "Codex Dominion".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenFirstTimeMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeMenu);

    let reloaded = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should reload");
    assert_eq!(
        reloaded.current_screen(),
        ScreenId::Startup(StartupPhase::Splash)
    );

    let game_data = latest_runtime_state(&fixture_dir).game_data;
    assert_eq!(game_data.player.records[0].occupied_flag(), 0);
}

#[test]
fn first_time_join_flow_updates_player_and_homeworld_then_enters_main_menu() {
    let fixture_dir = temp_first_time_game_copy();
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
            Action::Startup(StartupAction::OpenFirstTimeJoinName)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    for ch in "Codex Dominion".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireConfirm);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinSummary);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinNoPending);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldName);

    for ch in "Codex Prime".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldConfirm);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    let game_data = latest_runtime_state(&fixture_dir).game_data;
    let player = &game_data.player.records[0];
    assert_eq!(player.occupied_flag(), 1);
    assert_eq!(player.controlled_empire_name_summary(), "Codex Dominion");
    assert_eq!(player.autopilot_flag(), 0);
    let homeworld_index = player.homeworld_planet_index_1_based_raw() as usize;
    let homeworld = &game_data.planets.records[homeworld_index - 1];
    assert_eq!(homeworld.planet_name(), "Codex Prime");
    assert_eq!(
        homeworld.stored_production_points(),
        yearly_tax_revenue(
            homeworld.present_production_points().unwrap_or(0),
            player.tax_rate(),
        )
    );
}

#[test]
fn hosted_first_time_join_claims_seat_when_empire_name_is_saved() {
    let fixture_dir = temp_first_time_game_copy();
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    store
        .replace_hosted_seats(&[
            HostedSeat {
                player_record_index_1_based: 1,
                invite_code: "velvet-mountain".to_string(),
                status: HostedSeatStatus::Pending,
                player_npub: None,
            },
            HostedSeat {
                player_record_index_1_based: 2,
                invite_code: "copper-sunrise".to_string(),
                status: HostedSeatStatus::Pending,
                player_npub: None,
            },
        ])
        .expect("seed hosted seats");

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    app.set_hosted_invite_session(
        "npub1hostedplayer".to_string(),
        Some("velvet-mountain".to_string()),
    );

    assert_eq!(
        store.hosted_seats().expect("load pending seats")[0].status,
        HostedSeatStatus::Pending
    );

    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimeJoinEmpireName {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    for ch in "Codex Dominion".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireConfirm);
    let confirm_screen =
        nc_game::domains::startup::views::render(&mut app).expect("render hosted join confirm");
    assert!((0..confirm_screen.height()).any(|row| {
        confirm_screen
            .plain_line(row)
            .contains("Invite code: velvet-mountain")
    }));

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinSummary);

    let claimed = store.hosted_seats().expect("reload hosted seats");
    assert_eq!(claimed[0].status, HostedSeatStatus::Claimed);
    assert_eq!(claimed[0].player_npub.as_deref(), Some("npub1hostedplayer"));
}

#[test]
fn hosted_first_time_player_skips_menu_and_reserved_prompt() {
    let fixture_dir = temp_first_time_game_copy();
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    store
        .replace_hosted_seats(&[
            HostedSeat {
                player_record_index_1_based: 1,
                invite_code: "velvet-mountain".to_string(),
                status: HostedSeatStatus::Pending,
                player_npub: None,
            },
            HostedSeat {
                player_record_index_1_based: 2,
                invite_code: "copper-sunrise".to_string(),
                status: HostedSeatStatus::Pending,
                player_npub: None,
            },
        ])
        .expect("seed hosted seats");

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    app.set_hosted_invite_session(
        "npub1hostedplayer".to_string(),
        Some("velvet-mountain".to_string()),
    );

    // The intro is shown via splash transcript pages (pressing Y at page 0).
    // The separate Startup(Intro) accent-coloured phase is not used for
    // hosted sessions.
    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimeJoinEmpireName {
            break;
        }
        assert_ne!(app.current_screen(), ScreenId::FirstTimeMenu);
        assert_ne!(app.current_screen(), ScreenId::FirstTimeReservedPrompt);
        app.advance_startup();
    }
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    let screen =
        nc_game::domains::startup::views::render(&mut app).expect("render hosted join name");
    assert!((0..screen.height()).any(|row| {
        screen
            .plain_line(row)
            .contains("Invite code: velvet-mountain")
    }));
    assert!(
        screen
            .plain_line(2)
            .contains("Enter the name of your empire (up to 20 characters).")
    );
    assert_eq!(screen.plain_line(3), "");
    assert!(
        screen
            .plain_line(4)
            .contains("Invite code: velvet-mountain")
    );
    assert!(!(0..screen.height()).any(|row| screen.plain_line(row).contains("npub1hostedplayer")));
    // The FTM menu rows must not appear on the hosted empire naming screen.
    assert!(!(0..screen.height()).any(|row| screen.plain_line(row).contains("uit back to BBS")));
    assert!(!(0..screen.height()).any(|row| screen.plain_line(row).contains("oin this game")));
}

#[test]
fn hosted_sessions_error_if_first_time_menu_renders() {
    let fixture_dir = temp_first_time_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    app.set_hosted_invite_session(
        "npub1hostedplayer".to_string(),
        Some("velvet-mountain".to_string()),
    );
    *app.current_screen_mut() = ScreenId::FirstTimeMenu;

    let mut terminal = CaptureTerminal::new();
    let err = app
        .render(&mut terminal)
        .expect_err("hosted session should reject first time menu");

    assert!(err.to_string().contains("Hosted join invariant failed"));
    assert!(err.to_string().contains("FirstTimeMenu"));
}

#[test]
fn hosted_first_time_intro_completion_redirects_to_empire_naming() {
    let fixture_dir = temp_first_time_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    app.set_hosted_invite_session(
        "npub1hostedplayer".to_string(),
        Some("velvet-mountain".to_string()),
    );
    *app.current_screen_mut() = ScreenId::FirstTimeIntro;
    app.startup_state.first_time_intro_page = FIRST_TIME_INTRO_PAGE_COUNT - 1;

    app.advance_startup();

    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);
}

#[test]
fn hosted_open_first_time_menu_redirects_back_to_empire_naming() {
    let fixture_dir = temp_first_time_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    app.set_hosted_invite_session(
        "npub1hostedplayer".to_string(),
        Some("velvet-mountain".to_string()),
    );

    app.open_first_time_menu();

    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);
}

#[test]
fn hosted_first_time_escape_requests_quit_and_warns_invite_is_not_reserved() {
    let fixture_dir = temp_first_time_game_copy();
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    store
        .replace_hosted_seats(&[
            HostedSeat {
                player_record_index_1_based: 1,
                invite_code: "velvet-mountain".to_string(),
                status: HostedSeatStatus::Pending,
                player_npub: None,
            },
            HostedSeat {
                player_record_index_1_based: 2,
                invite_code: "copper-sunrise".to_string(),
                status: HostedSeatStatus::Pending,
                player_npub: None,
            },
        ])
        .expect("seed hosted seats");

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    app.set_hosted_invite_session(
        "npub1hostedplayer".to_string(),
        Some("velvet-mountain".to_string()),
    );

    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimeJoinEmpireName {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    assert_eq!(app.handle_key(key(KeyCode::Esc)), Action::RequestQuit);
    assert_eq!(
        apply_action(&mut app, Action::RequestQuit),
        AppOutcome::Continue
    );
    assert!(app.quit_confirm_open);
    assert_eq!(
        app.startup_state.first_time_status.as_deref(),
        Some("Your seat is unreserved until you name an empire.")
    );

    assert_eq!(
        apply_action(&mut app, Action::CancelQuitPrompt),
        AppOutcome::Continue
    );
    assert!(!app.quit_confirm_open);
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    let pending = CampaignStore::open_default_in_dir(&fixture_dir)
        .expect("reopen campaign store")
        .hosted_seats()
        .expect("load pending seats");
    assert_eq!(pending[0].status, HostedSeatStatus::Pending);
    assert!(pending[0].player_npub.is_none());
}

#[test]
fn first_time_join_failure_returns_to_name_entry_with_status() {
    let fixture_dir = temp_first_time_game_copy();
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
            Action::Startup(StartupAction::OpenFirstTimeJoinName)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    for ch in "Codex Dominion".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireConfirm);

    make_runtime_db_read_only(&fixture_dir);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);
    assert_eq!(app.startup_state.first_time_input, "Codex Dominion");
    assert_eq!(
        app.startup_state.first_time_status.as_deref(),
        Some("Unable to join this empire right now. Please try again.")
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("failed join screen should render");
    assert!(
        line_containing(
            &terminal,
            "Unable to join this empire right now. Please try again."
        )
        .contains("Unable to join this empire right now. Please try again.")
    );
}

#[test]
fn homeworld_name_failure_returns_to_name_entry_with_status() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimePreloadedRenamePrompt {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::RejectFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinSummary);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinNoPending);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldName);

    for ch in "Codex Prime".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldConfirm);

    make_runtime_db_read_only(&fixture_dir);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldName);
    assert_eq!(app.startup_state.first_time_input, "Codex Prime");
    assert_eq!(
        app.startup_state.first_time_status.as_deref(),
        Some("Unable to save the homeworld name right now. Please try again.")
    );
}

#[test]
fn reserved_first_time_join_flow_updates_player_and_homeworld_then_enters_main_menu() {
    let fixture_dir = temp_first_time_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: reserved_game_config(1, "SYSOP"),
    })
    .expect("app should load");
    app.door_mode = true;

    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimeReservedPrompt {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(app.current_screen(), ScreenId::FirstTimeReservedPrompt);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("reserved join name should render");
    assert!(
        terminal
            .line(2)
            .contains("This player seat is reserved for you.")
    );

    for ch in "Codex Dominion".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireConfirm);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinSummary);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinNoPending);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldName);

    for ch in "Codex Prime".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldConfirm);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    let game_data = latest_runtime_state(&fixture_dir).game_data;
    let player = &game_data.player.records[0];
    assert_eq!(player.occupied_flag(), 1);
    assert_eq!(player.controlled_empire_name_summary(), "Codex Dominion");
    let homeworld_index = player.homeworld_planet_index_1_based_raw() as usize;
    let homeworld = &game_data.planets.records[homeworld_index - 1];
    assert_eq!(homeworld.planet_name(), "Codex Prime");
}

#[test]
fn first_time_join_from_reserved_dropfile_persists_caller_alias() {
    let fixture_dir = temp_first_time_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    app.startup_state.caller_alias = Some("SYSOP".to_string());

    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::OpenFirstTimeJoinName)
        ),
        AppOutcome::Continue
    );
    for ch in "Codex Dominion".chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Startup(StartupAction::AppendFirstTimeInputChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::SubmitFirstTimeInput)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::AcceptFirstTimePrompt)
        ),
        AppOutcome::Continue
    );

    let player = &latest_runtime_state(&fixture_dir).game_data.player.records[0];
    assert_eq!(player.assigned_player_handle_summary(), "SYSOP");
}

#[test]
fn first_time_join_from_menu_refuses_full_game_and_displays_notice() {
    let fixture_dir = temp_full_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    app.open_first_time_menu();
    assert_eq!(
        apply_action(
            &mut app,
            Action::Startup(StartupAction::OpenFirstTimeJoinName)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeMenu);
    assert_eq!(
        app.startup_state.first_time_status.as_deref(),
        Some("This game is already full. No open empires remain.")
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("first-time menu should render");
    assert!(terminal
        .lines
        .iter()
        .any(|line| line.contains("Notice: This game is already full. No open empires remain.")));
}
