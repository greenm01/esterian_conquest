use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ec_client::app::{Action, App, AppConfig, AppOutcome, apply_action};
use ec_client::screen::{
    CommandMenu, FleetListMode, FleetRoeScreen, FleetRow, PlanetBuildMenuView, PlanetBuildOrder,
    PlanetBuildScreen, PlanetListMode, PlanetListSort, ScreenId,
};
use ec_client::startup::StartupPhase;
use ec_client::terminal::Terminal;
use ec_data::{
    CampaignRuntimeState, CampaignStore, CoreGameData, DiplomaticRelation,
    EmpirePlanetEconomyRow, EmpireProductionRankingSort, ProductionItemKind, QueuedPlayerMail,
};

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
    let mut data = CoreGameData::load(&root).expect("load joinable fixture");
    data.join_player(1, "Codex Dominion")
        .expect("join player for standard client tests");
    data.rename_player_homeworld(1, "Codex Prime")
        .expect("name homeworld for standard client tests");
    data.save(&root).expect("save joined fixture");
    CampaignStore::open_default_in_dir(&root)
        .expect("open campaign store")
        .import_directory_snapshot(&root)
        .expect("seed sqlite snapshot");
    root
}

fn temp_first_time_game_copy() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ec-client-first-time-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ));
    copy_dir_all(&repo_root().join("fixtures/ecutil-init/v1.5"), &root);
    CampaignStore::open_default_in_dir(&root)
        .expect("open campaign store")
        .import_directory_snapshot(&root)
        .expect("seed sqlite snapshot");
    root
}

fn temp_joined_needs_homeworld_copy() -> PathBuf {
    let root = temp_first_time_game_copy();
    let mut data = CoreGameData::load(&root).expect("load joinable fixture");
    data.join_player(1, "Codex Dominion")
        .expect("join player without naming homeworld");
    data.save(&root).expect("save partially joined fixture");
    CampaignStore::open_default_in_dir(&root)
        .expect("open campaign store")
        .import_directory_snapshot(&root)
        .expect("refresh sqlite snapshot");
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

fn latest_runtime_state(root: &Path) -> CampaignRuntimeState {
    CampaignStore::open_default_in_dir(root)
        .expect("open campaign store")
        .load_latest_runtime_state()
        .expect("load latest runtime state")
        .expect("campaign should have a latest runtime state")
}

fn save_runtime_state(root: &Path, state: &CampaignRuntimeState) {
    CampaignStore::open_default_in_dir(root)
        .expect("open campaign store")
        .save_runtime_state(
            &state.game_data,
            &state.database,
            &state.results_bytes,
            &state.messages_bytes,
            &state.queued_mail,
        )
        .expect("save runtime state");
}

fn advance_to_main_menu(app: &mut App) {
    for _ in 0..16 {
        if app.current_screen() == ScreenId::MainMenu {
            return;
        }
        app.advance_startup();
    }
    panic!("startup did not reach main menu");
}

fn advance_to_first_time_menu(app: &mut App) {
    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimeMenu {
            return;
        }
        app.advance_startup();
    }
    panic!("startup did not reach first-time menu");
}

struct CaptureTerminal {
    lines: Vec<String>,
}

impl CaptureTerminal {
    fn new() -> Self {
        Self { lines: Vec::new() }
    }

    fn line(&self, row: usize) -> &str {
        &self.lines[row]
    }
}

impl Terminal for CaptureTerminal {
    fn render(
        &mut self,
        playfield: &ec_client::screen::PlayfieldBuffer,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.lines = (0..playfield.height())
            .map(|row| playfield.plain_line(row))
            .collect();
        Ok(())
    }

    fn dump_text_capture(&mut self, _text: &str) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn read_key(&mut self) -> Result<KeyEvent, Box<dyn std::error::Error>> {
        Err("not used in tests".into())
    }

    fn clear_and_restore(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
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

    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Splash)
    );

    assert_eq!(
        apply_action(&mut app, Action::OpenStartupIntro),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Startup(StartupPhase::Intro));

    assert_eq!(
        apply_action(&mut app, Action::AdvanceStartup),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Startup(StartupPhase::Intro));

    assert_eq!(
        apply_action(&mut app, Action::AdvanceStartup),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::LoginSummary)
    );

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
    assert_eq!(
        app.current_screen(),
        ScreenId::FleetList(FleetListMode::Brief)
    );

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
        apply_action(&mut app, Action::OpenFleetDetach),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetDetach);

    assert_eq!(
        apply_action(&mut app, Action::OpenFleetHelp),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetHelp);

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
            Action::SubmitPlanetListSort(PlanetListMode::Brief, PlanetListSort::CurrentProduction)
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
    assert_eq!(app.planet_info_input(), "");

    assert_eq!(
        apply_action(&mut app, Action::SubmitPlanetInfoPrompt),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoDetail);
    assert_eq!(app.selected_planet_info(), Some(14));

    assert_eq!(
        apply_action(
            &mut app,
            Action::OpenPartialStarmapPrompt(CommandMenu::General)
        ),
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
fn first_time_menu_branch_opens_help_intro_and_empire_list() {
    let fixture_dir = temp_first_time_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Splash)
    );

    advance_to_first_time_menu(&mut app);
    assert_eq!(app.current_screen(), ScreenId::FirstTimeMenu);

    assert_eq!(
        apply_action(&mut app, Action::OpenFirstTimeHelp),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHelp);

    assert_eq!(
        apply_action(&mut app, Action::OpenFirstTimeEmpires),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeEmpires);

    assert_eq!(
        apply_action(&mut app, Action::OpenFirstTimeIntro),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeIntro);
}

#[test]
fn first_time_startup_skips_joined_player_login_summary() {
    let fixture_dir = temp_first_time_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Splash)
    );

    apply_action(&mut app, Action::AdvanceStartup);
    assert_eq!(app.current_screen(), ScreenId::FirstTimeMenu);
}

#[test]
fn joined_player_with_unnamed_homeworld_is_routed_to_homeworld_naming() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Splash)
    );

    for _ in 0..16 {
        if app.current_screen() == ScreenId::FirstTimeHomeworldName {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldName);
}

#[test]
fn escaping_empire_name_does_not_partially_join_player() {
    let fixture_dir = temp_first_time_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    assert_eq!(
        apply_action(&mut app, Action::OpenFirstTimeJoinName),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    for ch in "Codex Dominion".chars() {
        assert_eq!(
            apply_action(&mut app, Action::AppendFirstTimeInputChar(ch)),
            AppOutcome::Continue
        );
    }

    assert_eq!(
        apply_action(&mut app, Action::OpenFirstTimeMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeMenu);

    let reloaded = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
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
    })
    .expect("app should load");

    assert_eq!(
        apply_action(&mut app, Action::OpenFirstTimeJoinName),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireName);

    for ch in "Codex Dominion".chars() {
        assert_eq!(
            apply_action(&mut app, Action::AppendFirstTimeInputChar(ch)),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::SubmitFirstTimeInput),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinEmpireConfirm);

    assert_eq!(
        apply_action(&mut app, Action::AcceptFirstTimePrompt),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinSummary);

    assert_eq!(
        apply_action(&mut app, Action::AcceptFirstTimePrompt),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeJoinNoPending);

    assert_eq!(
        apply_action(&mut app, Action::AcceptFirstTimePrompt),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldName);

    for ch in "Codex Prime".chars() {
        assert_eq!(
            apply_action(&mut app, Action::AppendFirstTimeInputChar(ch)),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::SubmitFirstTimeInput),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHomeworldConfirm);

    assert_eq!(
        apply_action(&mut app, Action::AcceptFirstTimePrompt),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    let game_data = latest_runtime_state(&fixture_dir).game_data;
    let player = &game_data.player.records[0];
    assert_eq!(player.occupied_flag(), 1);
    assert_eq!(player.controlled_empire_name_summary(), "Codex Dominion");
    let homeworld_index = player.homeworld_planet_index_1_based_raw() as usize;
    assert_eq!(
        game_data.planets.records[homeworld_index - 1].planet_name(),
        "Codex Prime"
    );
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
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Splash)
    );
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

    advance_to_main_menu(&mut app);
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(
        app.handle_key(key(KeyCode::Char('b'))),
        Action::OpenEmpireStatus
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenEmpireStatus),
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
        Action::OpenFleetMenu
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenFleetMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('h'))),
        Action::OpenFleetHelp
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenFleetHelp),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetHelp);
    assert_eq!(app.handle_key(key(KeyCode::Enter)), Action::OpenFleetMenu);
    assert_eq!(
        apply_action(&mut app, Action::OpenFleetMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('i'))),
        Action::OpenPlanetInfoPrompt(CommandMenu::Fleet)
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetInfoPrompt(CommandMenu::Fleet)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoPrompt);
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
        app.handle_key(key(KeyCode::Char('v'))),
        Action::OpenPartialStarmapPrompt(CommandMenu::Fleet)
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::OpenPartialStarmapPrompt(CommandMenu::Fleet)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PartialStarmapPrompt);
    assert_eq!(
        apply_action(&mut app, Action::SubmitPartialStarmapPrompt),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PartialStarmapView);
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::ReturnToCommandMenu
    );
    assert_eq!(
        apply_action(&mut app, Action::ReturnToCommandMenu),
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
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::OpenFleetMenu
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenFleetMenu),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('e'))),
        Action::OpenFleetEta
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenFleetEta),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetEta);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::OpenFleetMenu
    );
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
    assert_eq!(
        app.current_screen(),
        ScreenId::FleetList(FleetListMode::Brief)
    );
    assert_eq!(app.handle_key(key(KeyCode::Enter)), Action::OpenFleetReview);
    assert_eq!(
        apply_action(&mut app, Action::OpenFleetReview),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetReview);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::OpenFleetMenu
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenFleetMenu),
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
        Action::OpenPlanetInfoPrompt(CommandMenu::Main)
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetInfoPrompt(CommandMenu::Main)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoPrompt);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::ReturnToCommandMenu
    );
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
        Action::OpenPartialStarmapPrompt(CommandMenu::Main)
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::OpenPartialStarmapPrompt(CommandMenu::Main)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PartialStarmapPrompt);
    assert_eq!(
        apply_action(&mut app, Action::SubmitPartialStarmapPrompt),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PartialStarmapView);
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
        app.handle_key(key(KeyCode::Char('t'))),
        Action::OpenPlanetDatabase
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetDatabase),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetDatabaseList);
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::SubmitPlanetDatabaseLookup
    );
    assert_eq!(
        apply_action(&mut app, Action::SubmitPlanetDatabaseLookup),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetDatabaseDetail);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::OpenPlanetDatabase
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetDatabase),
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
fn fleet_roe_accepts_typed_fleet_selection_and_q_cancels_edit_mode() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    advance_to_main_menu(&mut app);
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
fn fleet_roe_empty_enter_accepts_displayed_default() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::OpenFleetMenu),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenFleetRoeSelect),
        AppOutcome::Continue
    );

    assert_eq!(
        apply_action(&mut app, Action::AppendFleetRoeChar('4')),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::SubmitFleetRoe),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::SubmitFleetRoe),
        AppOutcome::Continue
    );

    assert_eq!(app.current_screen(), ScreenId::FleetRoeSelect);
    assert_eq!(app.current_fleet_roe_by_id(4), Some(6));
}

#[test]
fn fleet_roe_success_returns_to_selector_prompt_without_confirmation_text() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    let mut terminal = CaptureTerminal::new();

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::OpenFleetMenu),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenFleetRoeSelect),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::AppendFleetRoeChar('4')),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::SubmitFleetRoe),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::AppendFleetRoeChar('9')),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::SubmitFleetRoe),
        AppOutcome::Continue
    );

    app.render(&mut terminal).expect("render succeeds");
    assert_eq!(terminal.line(19), "FLEET COMMAND <- Fleet # [4] ->");
}

#[test]
fn planet_database_render_uses_year_and_tier_labels_on_bottom_row() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    let mut terminal = CaptureTerminal::new();

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetDatabase),
        AppOutcome::Continue
    );

    app.render(&mut terminal).expect("render succeeds");
    assert!(terminal.line(4).contains("Year"));
    assert!(terminal.lines.iter().any(|line| line.contains("Y3000")));
    assert!(terminal.lines.iter().any(|line| line.contains("owned")));
    assert!(terminal.line(19).starts_with("MAIN COMMAND <- ["));
    assert!(terminal.line(19).ends_with("] ->"));
}

#[test]
fn planet_info_intel_detail_shows_last_intel_and_tier() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    let mut terminal = CaptureTerminal::new();

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::OpenPlanetInfoPrompt(CommandMenu::Main)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::SubmitPlanetInfoPrompt),
        AppOutcome::Continue
    );

    app.render(&mut terminal).expect("render succeeds");
    assert!(terminal.lines.iter().any(|line| line.contains("Last Intel: Y3000")));
    assert!(terminal.lines.iter().any(|line| line.contains("Intel Tier: owned")));
}

#[test]
fn fleet_roe_render_keeps_command_line_on_bottom_row() {
    let mut screen = FleetRoeScreen::new();
    let rows = vec![FleetRow {
        fleet_record_index_1_based: 1,
        fleet_number: 1,
        coords: [16, 13],
        target_coords: [16, 13],
        current_speed: 0,
        max_speed: 3,
        eta_label: "0".to_string(),
        rules_of_engagement: 5,
        order_label: "Guard/blockade world".to_string(),
        composition_label: "1 CA 1 ETAC".to_string(),
    }];

    let buffer = screen
        .render_select(&rows, 0, 0, false, "", "", None)
        .expect("roe screen renders");

    assert_eq!(buffer.plain_line(17), "");
    assert_eq!(buffer.plain_line(19), "FLEET COMMAND <- Fleet # [1] ->");
    assert_eq!(buffer.cursor(), Some((32, 19)));
}

#[test]
fn fleet_roe_render_shows_edit_errors_on_bottom_line() {
    let mut screen = FleetRoeScreen::new();
    let rows = vec![FleetRow {
        fleet_record_index_1_based: 1,
        fleet_number: 6,
        coords: [16, 13],
        target_coords: [16, 13],
        current_speed: 0,
        max_speed: 3,
        eta_label: "0".to_string(),
        rules_of_engagement: 6,
        order_label: "Hold".to_string(),
        composition_label: "1 ETAC".to_string(),
    }];

    let buffer = screen
        .render_select(
            &rows,
            0,
            0,
            true,
            "",
            "1",
            Some("Non-combat fleets must use ROE 0."),
        )
        .expect("roe screen renders");

    assert_eq!(
        buffer.plain_line(19),
        "FLEET COMMAND <- Non-combat fleets must use ROE 0."
    );
}

#[test]
fn fleet_table_zero_pads_numbers_to_current_max_width() {
    let mut screen = ec_client::screen::FleetListScreen::new();
    let rows = vec![
        FleetRow {
            fleet_record_index_1_based: 1,
            fleet_number: 1,
            coords: [16, 13],
            target_coords: [16, 13],
            current_speed: 0,
            max_speed: 3,
            eta_label: "0".to_string(),
            rules_of_engagement: 6,
            order_label: "Hold".to_string(),
            composition_label: "1 CA".to_string(),
        },
        FleetRow {
            fleet_record_index_1_based: 2,
            fleet_number: 10,
            coords: [17, 13],
            target_coords: [17, 13],
            current_speed: 0,
            max_speed: 3,
            eta_label: "0".to_string(),
            rules_of_engagement: 6,
            order_label: "Hold".to_string(),
            composition_label: "1 DD".to_string(),
        },
        FleetRow {
            fleet_record_index_1_based: 3,
            fleet_number: 100,
            coords: [18, 13],
            target_coords: [18, 13],
            current_speed: 0,
            max_speed: 3,
            eta_label: "0".to_string(),
            rules_of_engagement: 6,
            order_label: "Hold".to_string(),
            composition_label: "1 BB".to_string(),
        },
    ];

    let buffer = screen
        .render(FleetListMode::Brief, &rows, 0, 0)
        .expect("fleet list renders");

    assert!(buffer.plain_line(5).starts_with("001 "));
    assert!(buffer.plain_line(6).starts_with("010 "));
    assert!(buffer.plain_line(7).starts_with("100 "));
}

#[test]
fn fleet_eta_screen_renders_bottom_line_prompt() {
    let mut screen = ec_client::screen::FleetEtaScreen::new();
    let rows = vec![FleetRow {
        fleet_record_index_1_based: 1,
        fleet_number: 7,
        coords: [16, 13],
        target_coords: [19, 13],
        current_speed: 3,
        max_speed: 3,
        eta_label: "1".to_string(),
        rules_of_engagement: 6,
        order_label: "Move fleet to Sector (19,13)".to_string(),
        composition_label: "1 CA".to_string(),
    }];

    let buffer = screen
        .render(
            &rows,
            0,
            0,
            ec_client::screen::FleetEtaMode::SelectingFleet,
            "",
            [19, 13],
            "",
            "",
            None,
        )
        .expect("fleet eta screen renders");

    assert_eq!(buffer.plain_line(0), "CALCULATE FLEET ETA:");
    assert!(buffer.plain_line(19).contains("Calculate time for fleet #"));
    assert!(buffer.plain_line(19).contains("[7] ->"));
}

#[test]
fn fleet_eta_accepts_typed_fleet_destination_and_default_include_system() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::OpenFleetEta),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetEta);

    assert_eq!(
        apply_action(&mut app, Action::AppendFleetEtaChar('4')),
        AppOutcome::Continue
    );
    assert_eq!(app.selected_fleet_eta_id(), Some(4));
    assert_eq!(
        apply_action(&mut app, Action::SubmitFleetEta),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetEta);
    assert_eq!(
        apply_action(&mut app, Action::AppendFleetEtaChar('1')),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::AppendFleetEtaChar('0')),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::AppendFleetEtaChar(',')),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::AppendFleetEtaChar('1')),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::AppendFleetEtaChar('3')),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::SubmitFleetEta),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::SubmitFleetEta),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetEta);
    assert_eq!(app.handle_key(key(KeyCode::Enter)), Action::SubmitFleetEta);
}

#[test]
fn planet_build_specify_uses_bottom_command_line_default_prompt() {
    let mut screen = PlanetBuildScreen::new();
    let view = PlanetBuildMenuView {
        row: EmpirePlanetEconomyRow {
            planet_record_index_1_based: 1,
            planet_name: "Loki".to_string(),
            coords: [16, 13],
            present_production: 50,
            potential_production: 75,
            stored_production_points: 40,
            build_capacity: 50,
            yearly_tax_revenue: 10,
            yearly_growth_delta: 5,
            armies: 10,
            ground_batteries: 5,
            has_friendly_starbase: false,
            is_homeworld_seed: false,
        },
        committed_points: 10,
        available_points: 40,
        points_left: 30,
        queue_used: 1,
        queue_capacity: 10,
        stardock_used: 0,
        stardock_capacity: 10,
    };
    let orders = vec![PlanetBuildOrder {
        kind: ProductionItemKind::Destroyer,
        points_remaining: 5,
    }];

    let buffer = screen
        .render_specify(&view, &orders, "", None)
        .expect("build specify renders");

    assert!(
        buffer
            .plain_line(19)
            .contains("BUILD COMMAND <- Unit number or 0 if done")
    );
    assert!(buffer.plain_line(19).contains("[0] ->"));
}

#[test]
fn planet_build_quantity_uses_bottom_command_line_default_prompt() {
    let mut screen = PlanetBuildScreen::new();
    let view = PlanetBuildMenuView {
        row: EmpirePlanetEconomyRow {
            planet_record_index_1_based: 1,
            planet_name: "Loki".to_string(),
            coords: [16, 13],
            present_production: 50,
            potential_production: 75,
            stored_production_points: 40,
            build_capacity: 50,
            yearly_tax_revenue: 10,
            yearly_growth_delta: 5,
            armies: 10,
            ground_batteries: 5,
            has_friendly_starbase: false,
            is_homeworld_seed: false,
        },
        committed_points: 10,
        available_points: 40,
        points_left: 30,
        queue_used: 1,
        queue_capacity: 10,
        stardock_used: 0,
        stardock_capacity: 10,
    };
    let orders = vec![PlanetBuildOrder {
        kind: ProductionItemKind::Destroyer,
        points_remaining: 5,
    }];

    let buffer = screen
        .render_quantity_prompt(
            &view,
            &orders,
            ec_client::screen::build_unit_spec(1).expect("destroyer spec"),
            6,
            "",
            None,
        )
        .expect("build quantity renders");

    assert!(
        buffer
            .plain_line(19)
            .contains("BUILD COMMAND <- How many new destroyers to build")
    );
    assert!(buffer.plain_line(19).contains("[6] ->"));
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

    advance_to_main_menu(&mut app);
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
        apply_action(&mut app, Action::OpenEnemies),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Enemies);
    assert_eq!(
        app.current_relation_to(2),
        Some(DiplomaticRelation::Neutral)
    );

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
    let mut runtime = latest_runtime_state(&fixture_dir);
    runtime.results_bytes = b"test results".to_vec();
    runtime.messages_bytes = b"test messages".to_vec();
    save_runtime_state(&fixture_dir, &runtime);

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

    let runtime = latest_runtime_state(&fixture_dir);
    assert!(runtime.results_bytes.is_empty());
    assert!(runtime.messages_bytes.is_empty());
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
    let queue = latest_runtime_state(&fixture_dir).queued_mail;
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].recipient_empire_id, 2);
    assert_eq!(queue[0].subject, "Hi");
    assert_eq!(queue[0].body, "Hi");
}

#[test]
fn apply_action_deletes_queued_message_from_outbox() {
    let fixture_dir = temp_game_copy();
    let mut runtime = latest_runtime_state(&fixture_dir);
    runtime.queued_mail.push(QueuedPlayerMail {
            sender_empire_id: 1,
            recipient_empire_id: 2,
            year: 3000,
            subject: "Test".to_string(),
            body: "Queued".to_string(),
        });
    save_runtime_state(&fixture_dir, &runtime);

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

    let queue = latest_runtime_state(&fixture_dir).queued_mail;
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

#[test]
fn fleet_detach_uses_bottom_line_prompts_and_creates_new_fleet() {
    let fixture_dir = temp_game_copy();
    let mut game_data = CoreGameData::load(&fixture_dir).expect("load fixture");
    let initial_fleet_count = game_data.fleets.records.len();
    let donor = &mut game_data.fleets.records[0];
    donor.set_destroyer_count(2);
    donor.set_etac_count(1);
    donor.set_cruiser_count(0);
    donor.set_battleship_count(0);
    donor.set_troop_transport_count(0);
    donor.set_army_count(0);
    donor.set_scout_count(0);
    donor.recompute_max_speed_from_composition();
    donor.set_current_speed(0);
    game_data.save(&fixture_dir).expect("save fixture");
    CampaignStore::open_default_in_dir(&fixture_dir)
        .expect("open campaign store")
        .import_directory_snapshot(&fixture_dir)
        .expect("refresh sqlite snapshot");

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    assert_eq!(
        apply_action(&mut app, Action::OpenFleetDetach),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetDetach);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("render detach select");
    assert!(
        terminal
            .line(19)
            .contains("Detach ships from fleet # [1] ->")
    );

    assert_eq!(
        apply_action(&mut app, Action::SubmitFleetDetach),
        AppOutcome::Continue
    );
    app.render(&mut terminal).expect("render destroyer prompt");
    assert!(terminal.line(19).contains("Destroyers to detach [0] ->"));

    assert_eq!(
        apply_action(&mut app, Action::AppendFleetDetachChar('1')),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::SubmitFleetDetach),
        AppOutcome::Continue
    );
    app.render(&mut terminal).expect("render etac prompt");
    assert!(terminal.line(19).contains("ETAC ships to detach [0] ->"));

    assert_eq!(
        apply_action(&mut app, Action::AppendFleetDetachChar('1')),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::SubmitFleetDetach),
        AppOutcome::Continue
    );
    app.render(&mut terminal).expect("render roe prompt");
    assert!(terminal.line(19).contains("New fleet ROE [6] ->"));

    assert_eq!(
        apply_action(&mut app, Action::SubmitFleetDetach),
        AppOutcome::Continue
    );
    app.render(&mut terminal)
        .expect("render detach select after save");
    assert!(
        terminal
            .line(19)
            .contains("Detach ships from fleet # [1] ->")
    );

    let updated = latest_runtime_state(&fixture_dir).game_data;
    assert_eq!(updated.fleets.records.len(), initial_fleet_count + 1);
    assert_eq!(updated.fleets.records[0].destroyer_count(), 1);
    assert_eq!(updated.fleets.records[0].etac_count(), 0);
    let detached = updated.fleets.records.last().expect("detached fleet");
    assert_eq!(detached.destroyer_count(), 1);
    assert_eq!(detached.etac_count(), 1);
}
