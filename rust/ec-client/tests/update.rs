use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ec_client::app::{Action, AppConfig, AppOutcome, App, apply_action};
use ec_client::screen::{RankingsView, ScreenId};
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
        apply_action(&mut app, Action::OpenPartialStarmapPrompt),
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
