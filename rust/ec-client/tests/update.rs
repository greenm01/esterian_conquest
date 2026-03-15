use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ec_client::app::{Action, AppConfig, AppOutcome, App, apply_action};
use ec_client::screen::{
    CommandMenu, FleetListMode, FleetRoeScreen, FleetRow, PlanetListMode, PlanetListSort,
    ScreenId,
};
use ec_client::startup::StartupPhase;
use ec_data::{DiplomaticRelation, EmpireProductionRankingSort};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn temp_game_copy() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ec-client-update-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ));
    copy_dir_all(&repo_root().join("fixtures/ecutil-init/v1.5"), &root);
    root
}

fn copy_dir_all(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).expect("create temp dir");
    for entry in fs::read_dir(src).expect("read src dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &target);
        } else {
            fs::copy(&path, &target).expect("copy file");
        }
    }
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn apply_action_switches_between_client_screens() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    assert_eq!(app.current_screen(), ScreenId::Startup(StartupPhase::Splash));

    assert_eq!(
        apply_action(&mut app, Action::OpenStartupIntro),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Startup(StartupPhase::Intro));

    assert_eq!(
        apply_action(&mut app, Action::AdvanceStartup),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Startup(StartupPhase::LoginSummary));

    assert_eq!(
        apply_action(&mut app, Action::AdvanceStartup),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(
        apply_action(&mut app, Action::OpenGeneralMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);

    assert_eq!(
        apply_action(&mut app, Action::OpenStarmap),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Starmap);

    assert_eq!(
        apply_action(&mut app, Action::OpenGeneralMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);

    assert_eq!(
        apply_action(&mut app, Action::OpenGeneralHelp),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralHelp);

    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);

    assert_eq!(
        apply_action(&mut app, Action::OpenFleetMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    assert_eq!(
        apply_action(&mut app, Action::OpenFleetList(FleetListMode::Brief)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList(FleetListMode::Brief));

    assert_eq!(
        apply_action(&mut app, Action::OpenFleetReview),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetReview);

    assert_eq!(
        apply_action(&mut app, Action::OpenFleetRoeSelect),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetRoeSelect);

    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetHelp),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetHelp);

    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetAutoCommissionConfirm),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetAutoCommissionDone);

    assert_eq!(
        apply_action(&mut app, Action::ConfirmPlanetAutoCommission),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetAutoCommissionDone);

    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetCommissionMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionMenu);

    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetBuildMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildMenu);

    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetBuildHelp),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildHelp);

    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetBuildReview),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildReview);

    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetBuildList),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildList);

    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetBuildAbortConfirm),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildAbortConfirm);

    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetBuildSpecify),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildSpecify);

    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetTaxPrompt),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetTaxPrompt);

    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetDatabase),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetDatabaseList);

    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetDatabaseDetail),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetDatabaseDetail);

    assert_eq!(
        apply_action(&mut app, Action::BackspacePlanetTaxInput),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::BackspacePlanetTaxInput),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::AppendPlanetTaxChar('6')),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::AppendPlanetTaxChar('5')),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::SubmitPlanetTax),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetTaxDone);

    assert_eq!(
        apply_action(
            &mut app,
            Action::SubmitPlanetListSort(
                PlanetListMode::Brief,
                PlanetListSort::CurrentProduction
            )
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetBriefList(PlanetListSort::CurrentProduction)
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::SubmitPlanetListSort(PlanetListMode::Detail, PlanetListSort::Location)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetDetailList(PlanetListSort::Location)
    );

    assert_eq!(
        apply_action(&mut app, Action::OpenGeneralMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);

    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetInfoPrompt(CommandMenu::General)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoPrompt);
    assert_eq!(app.planet_info_input(), "16,13");

    assert_eq!(
        apply_action(&mut app, Action::SubmitPlanetInfoPrompt),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoDetail);
    assert_eq!(app.selected_planet_info(), Some(14));

    assert_eq!(
        apply_action(&mut app, Action::OpenPartialStarmapPrompt(CommandMenu::General)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PartialStarmapPrompt);

    assert_eq!(
        apply_action(&mut app, Action::SubmitPartialStarmapPrompt),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PartialStarmapView);

    assert_eq!(
        apply_action(&mut app, Action::OpenReports),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Reports);

    assert_eq!(
        apply_action(&mut app, Action::OpenEmpireStatus),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::EmpireStatus);

    assert_eq!(
        apply_action(&mut app, Action::OpenEmpireProfile),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::EmpireProfile);

    assert_eq!(
        apply_action(
            &mut app,
            Action::OpenRankingsTable(EmpireProductionRankingSort::Production)
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
fn apply_action_quit_exits_loop() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    assert_eq!(apply_action(&mut app, Action::Quit), AppOutcome::Quit);
    assert_eq!(app.current_screen(), ScreenId::Startup(StartupPhase::Splash));
}

#[test]
fn main_menu_keys_open_existing_shared_screens_and_return_to_main() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    app.advance_startup();
    app.advance_startup();
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(app.handle_key(key(KeyCode::Char('b'))), Action::OpenEmpireStatus);
    assert_eq!(
        apply_action(&mut app, Action::OpenEmpireStatus),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::EmpireStatus);
    assert_eq!(app.handle_key(key(KeyCode::Enter)), Action::ReturnToCommandMenu);
    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(app.handle_key(key(KeyCode::Char('f'))), Action::OpenFleetMenu);
    assert_eq!(
        apply_action(&mut app, Action::OpenFleetMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('c'))),
        Action::OpenFleetRoeSelect
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenFleetRoeSelect),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetRoeSelect);
    assert_eq!(app.handle_key(key(KeyCode::Enter)), Action::SubmitFleetRoe);
    assert_eq!(
        apply_action(&mut app, Action::SubmitFleetRoe),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetRoeSelect);
    assert_eq!(
        apply_action(&mut app, Action::AppendFleetRoeChar('4')),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::SubmitFleetRoe),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetRoeSelect);
    assert_eq!(app.current_fleet_roe_by_id(1), Some(4));
    assert_eq!(app.handle_key(key(KeyCode::Char('q'))), Action::OpenFleetMenu);
    assert_eq!(
        apply_action(&mut app, Action::OpenFleetMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('b'))),
        Action::OpenFleetList(FleetListMode::Brief)
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenFleetList(FleetListMode::Brief)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetList(FleetListMode::Brief));
    assert_eq!(app.handle_key(key(KeyCode::Enter)), Action::OpenFleetReview);
    assert_eq!(
        apply_action(&mut app, Action::OpenFleetReview),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetReview);
    assert_eq!(app.handle_key(key(KeyCode::Char('q'))), Action::OpenFleetMenu);
    assert_eq!(
        apply_action(&mut app, Action::OpenFleetMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(app.handle_key(key(KeyCode::Char('q'))), Action::OpenMainMenu);
    assert_eq!(
        apply_action(&mut app, Action::OpenMainMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(
        app.handle_key(key(KeyCode::Char('i'))),
        Action::OpenPlanetInfoPrompt(CommandMenu::Main)
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetInfoPrompt(CommandMenu::Main)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoPrompt);
    assert_eq!(app.handle_key(key(KeyCode::Char('q'))), Action::ReturnToCommandMenu);
    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetInfoPrompt(CommandMenu::Main)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoPrompt);
    assert_eq!(
        apply_action(&mut app, Action::SubmitPlanetInfoPrompt),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoDetail);
    assert_eq!(app.handle_key(key(KeyCode::Enter)), Action::ReturnToCommandMenu);
    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(
        app.handle_key(key(KeyCode::Char('v'))),
        Action::OpenPartialStarmapPrompt(CommandMenu::Main)
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenPartialStarmapPrompt(CommandMenu::Main)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PartialStarmapPrompt);
    assert_eq!(
        apply_action(&mut app, Action::SubmitPartialStarmapPrompt),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PartialStarmapView);
    assert_eq!(app.handle_key(key(KeyCode::Enter)), Action::ReturnToCommandMenu);
    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(app.handle_key(key(KeyCode::Char('t'))), Action::OpenPlanetDatabase);
    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetDatabase),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetDatabaseList);
    assert_eq!(app.handle_key(key(KeyCode::Enter)), Action::OpenPlanetDatabaseDetail);
    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetDatabaseDetail),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetDatabaseDetail);
    assert_eq!(app.handle_key(key(KeyCode::Char('q'))), Action::OpenPlanetDatabase);
    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetDatabase),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetDatabaseList);
    assert_eq!(app.handle_key(key(KeyCode::Char('q'))), Action::ReturnToCommandMenu);
    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);
}

#[test]
fn fleet_roe_accepts_typed_fleet_selection_and_q_cancels_edit_mode() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    app.advance_startup();
    app.advance_startup();
    assert_eq!(
        apply_action(&mut app, Action::OpenFleetMenu),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenFleetRoeSelect),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetRoeSelect);

    assert_eq!(
        apply_action(&mut app, Action::AppendFleetRoeChar('4')),
        AppOutcome::Continue
    );
    assert_eq!(app.selected_fleet_roe_id(), Some(4));
    assert_eq!(
        apply_action(&mut app, Action::SubmitFleetRoe),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetRoeSelect);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::OpenFleetRoeSelect
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenFleetRoeSelect),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetRoeSelect);

    assert_eq!(
        apply_action(&mut app, Action::AppendFleetRoeChar('4')),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::SubmitFleetRoe),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::AppendFleetRoeChar('7')),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::SubmitFleetRoe),
        AppOutcome::Continue
    );
    assert_eq!(app.current_fleet_roe_by_id(4), Some(7));
    assert_eq!(app.current_fleet_roe_by_id(1), Some(6));
}

#[test]
fn fleet_roe_render_keeps_command_line_on_bottom_row() {
    let mut screen = FleetRoeScreen::new();
    let rows = vec![FleetRow {
        fleet_record_index_1_based: 1,
        fleet_id: 1,
        coords: [16, 13],
        current_speed: 0,
        max_speed: 3,
        rules_of_engagement: 5,
        order_label: "Guard/blockade world".to_string(),
        composition_label: "1 CA 1 ETAC".to_string(),
    }];

    let buffer = screen
        .render_select(&rows, 0, 0, false, "", "", None)
        .expect("roe screen renders");

    assert_eq!(buffer.plain_line(17), "");
    assert_eq!(buffer.plain_line(19), "FLEET COMMAND <- Fleet #:");
    assert_eq!(buffer.cursor(), Some((26, 19)));
}

#[test]
fn general_rankings_opens_production_table_and_returns_to_general_menu() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    app.advance_startup();
    app.advance_startup();
    assert_eq!(
        apply_action(&mut app, Action::OpenGeneralMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);

    assert_eq!(
        app.handle_key(key(KeyCode::Char('o'))),
        Action::OpenRankingsTable(EmpireProductionRankingSort::Production)
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::OpenRankingsTable(EmpireProductionRankingSort::Production)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::Rankings(EmpireProductionRankingSort::Production)
    );
    assert_eq!(app.handle_key(key(KeyCode::Enter)), Action::ReturnToCommandMenu);
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
    })
    .expect("app should load");

    let initial_autopilot = app.current_autopilot_flag();
    assert_eq!(
        apply_action(&mut app, Action::ToggleAutopilot),
        AppOutcome::Continue
    );
    assert_eq!(app.current_autopilot_flag(), if initial_autopilot == 0 { 1 } else { 0 });

    assert_eq!(
        apply_action(&mut app, Action::OpenEnemies),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Enemies);
    assert_eq!(app.current_relation_to(2), Some(DiplomaticRelation::Neutral));

    assert_eq!(
        apply_action(&mut app, Action::AppendEnemiesChar('2')),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::SubmitEnemiesInput),
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
    })
    .expect("app should load");

    assert_eq!(
        apply_action(&mut app, Action::OpenEnemies),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Enemies);

    for _ in 0..50 {
        assert_eq!(
            apply_action(&mut app, Action::ScrollEnemies(1)),
            AppOutcome::Continue
        );
    }

    assert_eq!(app.enemies_scroll_offset(), 0);
}

#[test]
fn apply_action_deletes_reviewables() {
    let fixture_dir = temp_game_copy();
    std::fs::write(fixture_dir.join("RESULTS.DAT"), b"test results").expect("seed results");
    std::fs::write(fixture_dir.join("MESSAGES.DAT"), b"test messages").expect("seed messages");

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    assert_eq!(
        apply_action(&mut app, Action::OpenDeleteReviewables),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::DeleteReviewables);

    assert_eq!(
        apply_action(&mut app, Action::ConfirmDeleteReviewables),
        AppOutcome::Continue
    );

    assert_eq!(
        std::fs::read(fixture_dir.join("RESULTS.DAT")).expect("read results"),
        Vec::<u8>::new()
    );
    assert_eq!(
        std::fs::read(fixture_dir.join("MESSAGES.DAT")).expect("read messages"),
        Vec::<u8>::new()
    );
}

#[test]
fn apply_action_queues_composed_message() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    assert_eq!(
        apply_action(&mut app, Action::OpenComposeMessageRecipient),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageRecipient);
    assert_eq!(
        apply_action(&mut app, Action::AppendComposeRecipientChar('2')),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::SubmitComposeRecipient),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageSubject);
    assert_eq!(
        apply_action(&mut app, Action::AppendComposeSubjectChar('H')),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::AppendComposeSubjectChar('i')),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::SubmitComposeSubject),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageBody);
    assert_eq!(
        apply_action(&mut app, Action::AppendComposeBodyChar('H')),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::AppendComposeBodyChar('i')),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenComposeMessageSendConfirm),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageSendConfirm);
    assert_eq!(
        apply_action(&mut app, Action::ConfirmSendComposedMessage),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageSent);
    let queue = ec_data::load_mail_queue(&fixture_dir).expect("load queued mail");
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].recipient_empire_id, 2);
    assert_eq!(queue[0].subject, "Hi");
    assert_eq!(queue[0].body, "Hi");
}

#[test]
fn apply_action_deletes_queued_message_from_outbox() {
    let fixture_dir = temp_game_copy();
    ec_data::append_mail_queue(
        &fixture_dir,
        &ec_data::QueuedPlayerMail {
            sender_empire_id: 1,
            recipient_empire_id: 2,
            year: 3000,
            subject: "Test".to_string(),
            body: "Queued".to_string(),
        },
    )
    .expect("seed queued mail");

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    assert_eq!(
        apply_action(&mut app, Action::OpenComposeMessageOutbox),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageOutbox);
    assert_eq!(
        apply_action(&mut app, Action::AppendComposeOutboxChar('1')),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::DeleteQueuedComposeMessage),
        AppOutcome::Continue
    );

    let queue = ec_data::load_mail_queue(&fixture_dir).expect("load queue after delete");
    assert!(queue.is_empty());
}

#[test]
fn apply_action_confirms_before_discarding_composed_message() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    assert_eq!(
        apply_action(&mut app, Action::OpenComposeMessageRecipient),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::AppendComposeRecipientChar('2')),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::SubmitComposeRecipient),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::SubmitComposeSubject),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageBody);

    assert_eq!(
        apply_action(&mut app, Action::OpenComposeMessageDiscardConfirm),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageDiscardConfirm);

    assert_eq!(
        apply_action(&mut app, Action::OpenComposeMessageBody),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageBody);

    assert_eq!(
        apply_action(&mut app, Action::OpenComposeMessageDiscardConfirm),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::ConfirmDiscardComposedMessage),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageRecipient);
}
