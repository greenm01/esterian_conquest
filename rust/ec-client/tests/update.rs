use std::path::PathBuf;

use ec_client::app::{Action, AppConfig, AppOutcome, App, apply_action};
use ec_client::screen::{RankingsView, ScreenId};
use ec_client::startup::StartupPhase;
use ec_data::EmpireProductionRankingSort;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn apply_action_switches_between_client_screens() {
    let fixture_dir = repo_root().join("fixtures/ecutil-init/v1.5");
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
        apply_action(&mut app, Action::OpenPlanetInfoPrompt),
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
        ScreenId::Rankings(RankingsView::Table(EmpireProductionRankingSort::Production))
    );

    assert_eq!(
        apply_action(&mut app, Action::OpenMainMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);
}

#[test]
fn apply_action_quit_exits_loop() {
    let fixture_dir = repo_root().join("fixtures/ecutil-init/v1.5");
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
