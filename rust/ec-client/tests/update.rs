use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ec_client::app::{Action, App, AppConfig, AppOutcome, apply_action};
use ec_client::domains::empire::EmpireAction;
use ec_client::domains::fleet::FleetAction;
use ec_client::domains::messaging::MessagingAction;
use ec_client::domains::planet::PlanetAction;
use ec_client::domains::starbase::StarbaseAction;
use ec_client::domains::starmap::StarmapAction;
use ec_client::domains::startup::StartupAction;
use ec_client::model::ClassicLoginState;
use ec_client::screen::layout::COMMAND_LINE_ROW;
use ec_client::screen::{
    CommandMenu, FleetListMode, FleetRoeScreen, FleetRow, PlanetBuildMenuView, PlanetBuildOrder,
    PlanetBuildScreen, PlanetListMode, PlanetListSort, ScreenId,
};
use ec_client::startup::StartupPhase;
use ec_client::terminal::Terminal;
use ec_compat::{decode_report_block_rows, import_directory_snapshot};
use ec_data::{
    CampaignRuntimeState, CampaignStore, CoreGameData, DiplomaticRelation, EmpirePlanetEconomyRow,
    EmpireProductionRankingSort, IntelTier, PlanetIntelSnapshot, ProductionItemKind,
    QueuedPlayerMail,
};
use ec_engine::yearly_tax_revenue;

static TEMP_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn temp_dir(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{label}-{}-{}-{}",
        std::process::id(),
        TEMP_DIR_SEQ.fetch_add(1, Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ))
}

fn temp_game_copy() -> PathBuf {
    let root = temp_dir("ec-client-update");
    copy_dir_all(&repo_root().join("fixtures/ecutil-init/v1.5"), &root);
    let mut data = CoreGameData::load(&root).expect("load joinable fixture");
    data.join_player(1, "Codex Dominion")
        .expect("join player for standard client tests");
    data.rename_player_homeworld(1, "Codex Prime")
        .expect("name homeworld for standard client tests");
    data.save(&root).expect("save joined fixture");
    let store = CampaignStore::open_default_in_dir(&root).expect("open campaign store");
    import_directory_snapshot(&store, &root).expect("seed sqlite snapshot");
    root
}

fn temp_first_time_game_copy() -> PathBuf {
    let root = temp_dir("ec-client-first-time");
    copy_dir_all(&repo_root().join("fixtures/ecutil-init/v1.5"), &root);
    let store = CampaignStore::open_default_in_dir(&root).expect("open campaign store");
    import_directory_snapshot(&store, &root).expect("seed sqlite snapshot");
    root
}

fn temp_joined_needs_homeworld_copy() -> PathBuf {
    let root = temp_first_time_game_copy();
    let mut data = CoreGameData::load(&root).expect("load joinable fixture");
    data.join_player(1, "Codex Dominion")
        .expect("join player without naming homeworld");
    data.save(&root).expect("save partially joined fixture");
    let store = CampaignStore::open_default_in_dir(&root).expect("open campaign store");
    import_directory_snapshot(&store, &root).expect("refresh sqlite snapshot");
    root
}

fn temp_joined_no_assets_copy() -> PathBuf {
    let root = temp_game_copy();
    let mut state = latest_runtime_state(&root);
    for planet in &mut state.game_data.planets.records {
        if planet.owner_empire_slot_raw() == 1 {
            planet.set_owner_empire_slot_raw(0);
            planet.set_ownership_status_raw(0);
        }
    }
    save_runtime_state(&root, &state);
    root
}

fn temp_joined_empty_empire_copy() -> PathBuf {
    let root = temp_game_copy();
    let mut state = latest_runtime_state(&root);
    for planet in &mut state.game_data.planets.records {
        if planet.owner_empire_slot_raw() == 1 {
            planet.set_owner_empire_slot_raw(0);
            planet.set_ownership_status_raw(0);
        }
    }
    for fleet in &mut state.game_data.fleets.records {
        if fleet.owner_empire_raw() == 1 {
            fleet.set_owner_empire_raw(0);
        }
    }
    save_runtime_state(&root, &state);
    root
}

fn temp_game_with_starbase_copy() -> PathBuf {
    let root = temp_game_copy();
    let mut state = latest_runtime_state(&root);
    state
        .game_data
        .set_guard_starbase(1, 1, [6, 5], 1, 1)
        .expect("seed guard starbase");
    save_runtime_state(&root, &state);
    root
}

fn temp_game_with_same_sector_fleets_copy() -> PathBuf {
    let root = temp_game_copy();
    let mut state = latest_runtime_state(&root);
    state.game_data.fleets.records[0].set_current_location_coords_raw([6, 5]);
    state.game_data.fleets.records[0].set_standing_order_target_coords_raw([6, 5]);
    state.game_data.fleets.records[1].set_current_location_coords_raw([6, 5]);
    state.game_data.fleets.records[1].set_standing_order_target_coords_raw([6, 5]);
    save_runtime_state(&root, &state);
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
    let player_count = state.game_data.conquest.player_count();
    let planet_intel_by_viewer = (1..=player_count)
        .map(|viewer_empire_id| {
            CampaignStore::open_default_in_dir(root)
                .expect("open campaign store")
                .latest_planet_intel_for_viewer(viewer_empire_id)
                .expect("load runtime intel")
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>()
        })
        .collect::<Vec<_>>();
    save_runtime_state_with_intel(root, state, &planet_intel_by_viewer);
}

fn save_runtime_state_with_intel(
    root: &Path,
    state: &CampaignRuntimeState,
    planet_intel_by_viewer: &[BTreeMap<usize, PlanetIntelSnapshot>],
) {
    CampaignStore::open_default_in_dir(root)
        .expect("open campaign store")
        .save_runtime_state_structured_with_intel(
            &state.game_data,
            &state.report_block_rows,
            &state.queued_mail,
            planet_intel_by_viewer,
        )
        .expect("save runtime state");
}

fn partial_known_world_snapshot(
    planet_record_index_1_based: usize,
    planet: &ec_data::PlanetRecord,
    owner_empire_id: u8,
    year: u16,
) -> PlanetIntelSnapshot {
    PlanetIntelSnapshot {
        planet_record_index_1_based,
        intel_tier: IntelTier::Partial,
        compat_is_orbit_seed: false,
        last_intel_year: Some(year),
        seen_year: Some(year),
        scout_year: Some(year),
        known_name: Some(planet.status_or_name_summary()),
        known_owner_empire_id: Some(owner_empire_id),
        known_potential_production: Some(planet.potential_production_points()),
        known_armies: None,
        known_ground_batteries: None,
        known_current_production: None,
        known_stored_points: None,
        compat_word_1e: None,
    }
}

fn incoming_mail(
    sender_empire_id: u8,
    recipient_empire_id: u8,
    year: u16,
    subject: &str,
    body: &str,
) -> QueuedPlayerMail {
    QueuedPlayerMail {
        sender_empire_id,
        recipient_empire_id,
        year,
        subject: subject.to_string(),
        body: body.to_string(),
        recipient_deleted: false,
    }
}

fn classic_chunked_report_bytes(text: &str) -> Vec<u8> {
    let mut bytes = vec![0u8; 84];
    for (idx, byte) in text.bytes().take(75).enumerate() {
        bytes[idx + 1] = byte;
    }
    bytes
}

fn classic_chunked_report_blocks(texts: &[&str]) -> Vec<u8> {
    texts
        .iter()
        .flat_map(|text| classic_chunked_report_bytes(text))
        .collect()
}

fn length_prefixed_report_block(lines: &[&str]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for line in lines {
        let line_bytes = line.as_bytes();
        assert!(
            line_bytes.len() <= 72,
            "line too long for length-prefixed fixture"
        );
        let mut chunk = vec![0u8; 84];
        chunk[0] = 6;
        chunk[1] = line_bytes.len() as u8;
        chunk[2..2 + line_bytes.len()].copy_from_slice(line_bytes);
        bytes.extend_from_slice(&chunk);
    }
    bytes
}

fn set_runtime_report_blocks(state: &mut CampaignRuntimeState, bytes: impl AsRef<[u8]>) {
    state.report_block_rows = decode_report_block_rows(bytes.as_ref());
}

fn clear_runtime_report_blocks(state: &mut CampaignRuntimeState) {
    state.report_block_rows.clear();
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
        let mapped_row = if row == 19 && self.lines.len() > COMMAND_LINE_ROW {
            COMMAND_LINE_ROW
        } else {
            row
        };
        &self.lines[mapped_row]
    }
}

fn line_containing<'a>(terminal: &'a CaptureTerminal, needle: &str) -> &'a str {
    terminal
        .lines
        .iter()
        .find(|line| line.contains(needle))
        .map(String::as_str)
        .unwrap_or("")
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
        game_dir: fixture_dir.clone(),
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
        apply_action(&mut app, Action::OpenGeneralHelp),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralHelp);

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
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::OpenList(FleetListMode::Brief))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::FleetList(FleetListMode::Brief)
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReviewSelect)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetReviewSelect);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReview)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetReview);

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenRoeSelect)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetRoeSelect);

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenDetach)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetDetach);

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenHelp)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetHelp);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenHelp)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetHelp);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenAutoCommissionConfirm)
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildHelp)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildHelp);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildReview)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildReview);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildList)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildList);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenBuildAbortConfirm)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildAbortConfirm);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildSpecify)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildSpecify);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenTaxPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetTaxPrompt);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenDatabase)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetDatabaseList);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenDatabaseDetail)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetDatabaseDetail);

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
    assert_eq!(app.current_screen(), ScreenId::PlanetTaxDone);

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
        ScreenId::PlanetBriefList(PlanetListSort::CurrentProduction)
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitListSort(
                PlanetListMode::Detail,
                PlanetListSort::Location
            ))
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
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenInfoPrompt(CommandMenu::General))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoPrompt);
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
fn first_time_menu_branch_opens_help_intro_and_empire_list() {
    let fixture_dir = temp_first_time_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
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
        apply_action(&mut app, Action::Startup(StartupAction::OpenFirstTimeHelp)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHelp);

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
fn preloaded_first_login_routes_through_login_summary_before_rename_prompt() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
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
            .contains("You are a pre-loaded player and this is your first time on.")
    );
    assert!(
        terminal
            .line(19)
            .contains("Would you like to rename your empire? (This is your only chance.)")
    );
}

#[test]
fn preloaded_first_login_becomes_returning_player_after_homeworld_naming() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
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
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
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
            .contains("You are a pre-loaded player and this is your first time on.")
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
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
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
fn apply_action_quit_exits_loop() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
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
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    advance_to_main_menu(&mut app);
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

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
        Action::Fleet(FleetAction::OpenHelp)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenHelp)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetHelp);
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::Fleet(FleetAction::OpenMenu)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
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
        Action::Fleet(FleetAction::OpenRoeSelect)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenRoeSelect)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetRoeSelect);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendRoeChar('1'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitRoe)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendRoeChar('4'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitRoe)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_fleet_roe_by_id(1), Some(4));
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Fleet(FleetAction::OpenMenu)
    );
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
    assert_eq!(app.current_screen(), ScreenId::FleetEta);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Fleet(FleetAction::OpenMenu)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('b'))),
        Action::Fleet(FleetAction::OpenList(FleetListMode::Brief))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::OpenList(FleetListMode::Brief))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::FleetList(FleetListMode::Brief)
    );
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('r'))),
        Action::Fleet(FleetAction::OpenReviewSelect)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReviewSelect)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetReviewSelect);
    assert_eq!(
        app.handle_key(key(KeyCode::Down)),
        Action::Fleet(FleetAction::MoveReviewSelect(1))
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Char('7'))),
        Action::Fleet(FleetAction::AppendReviewChar('7'))
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Backspace)),
        Action::Fleet(FleetAction::BackspaceReviewInput)
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::Fleet(FleetAction::SubmitReviewSelect)
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Fleet(FleetAction::OpenMenu)
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
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenInfoPrompt(CommandMenu::Main))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetInfoPrompt);
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
    assert_eq!(app.current_screen(), ScreenId::PlanetDatabaseDetail);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Planet(PlanetAction::OpenDatabase)
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenDatabase)),
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
fn fleet_review_detail_q_returns_to_review_picker() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReviewSelect)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitReviewSelect)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetReview);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Fleet(FleetAction::OpenReviewSelect)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReviewSelect)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetReviewSelect);
}

#[test]
fn fleet_menu_matches_verified_v15_command_layout() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
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
        terminal.line(0).trim_end(),
        "FLEET COMMAND CENTER:                    E>TA Calculation    O>rder a Fleet"
    );
    assert_eq!(
        terminal.line(1).trim_end(),
        "  H>elp on Options   S>TARBASE MENU...   C>hg ROE,ID,Speed   G>ROUP FLEET ORDER"
    );
    assert_eq!(
        terminal.line(2).trim_end(),
        "  Q>uit: Main Menu   B>rief Fleet List   I>nfo about Planet  M>erge a Fleet"
    );
    assert_eq!(
        terminal.line(3).trim_end(),
        "  X>pert Mode        F>ull Fleet List    D>etach Ships       L>oad TTs w/Armies"
    );
    assert_eq!(
        terminal.line(4).trim_end(),
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
        "STARBASE COMMAND <-H,Q,X,S,R,V,I,M->"
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
    assert_eq!(terminal.line(3).trim_end(), "Starbase ID: Starbase 1");
    assert_eq!(
        terminal.line(4).trim_end(),
        "Location:    World in Solar System [ 6, 5]"
    );
    assert_eq!(
        terminal.line(5).trim_end(),
        "Destination: World in Solar System [ 6, 5]"
    );
    assert_eq!(
        terminal.line(6).trim_end(),
        "Operation:   Protection & Enhancement"
    );
    assert_eq!(
        terminal.line(7).trim_end(),
        "ETA:         Starbase 1 has already arrived and is in operation."
    );
    assert_eq!(terminal.line(8).trim_end(), "Escort:      The 1st Fleet");
}

#[test]
fn fleet_transfer_uses_two_fleet_selector_and_groups_same_sector_rows() {
    let fixture_dir = temp_game_with_same_sector_fleets_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
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
    assert_eq!(app.current_screen(), ScreenId::FleetTransfer);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("transfer screen should render");
    assert_eq!(terminal.line(0).trim_end(), "TRANSFER SHIPS:");
    assert_eq!(
        terminal.line(1).trim_end(),
        "Select two fleets in one sector. Highlight the host fleet, then press ENTER."
    );
    assert_eq!(terminal.line(2).trim_end(), "Selected fleets: 0");
    assert_eq!(
        terminal.line(3).trim_end(),
        "┌──┬───┬──────────┬───────┬───┬───┬──────────┬───────────────────────────┐"
    );
    let same_sector_rows = (6..17)
        .filter(|idx| terminal.line(*idx).contains("[ 6, 5]"))
        .count();
    assert!(same_sector_rows >= 2);
}

#[test]
fn main_menu_matches_verified_v15_command_layout() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("main menu should render");
    assert_eq!(terminal.line(0).trim_end(), "MAIN MENU:");
    assert_eq!(
        terminal.line(1).trim_end(),
        "  H>elp with commands   A>nsi color ON/OFF         T>otal Planet Database"
    );
    assert_eq!(
        terminal.line(2).trim_end(),
        "  Q>uit back to BBS     G>ENERAL COMMAND MENU...   I>nfo about a Planet"
    );
    assert_eq!(
        terminal.line(3).trim_end(),
        "  X>pert mode ON/OFF    P>LANET COMMAND MENU...    B>rief Empire Report"
    );
    assert_eq!(
        terminal.line(4).trim_end(),
        "  V>iew Partial Map     F>LEET COMMAND MENU...     D>etailed Empire Report"
    );
    assert_eq!(terminal.line(5).trim_end(), "");
    assert_eq!(
        terminal.line(6).trim_end(),
        "MAIN COMMAND <-H,Q,X,V,A,G,P,F,T,I,B,D->"
    );
    assert!(terminal.line(23).contains("-- "));
}

#[test]
fn general_menu_matches_verified_v15_command_layout() {
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

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("general menu should render");
    assert_eq!(
        terminal.line(0).trim_end(),
        "GENERAL COMMAND CENTER:   I>nfo about a Planet     C>ommunicate (send message)"
    );
    assert_eq!(
        terminal.line(1).trim_end(),
        "  H>elp with commands     A>utopilot ON/OFF        R>eview messages/results"
    );
    assert_eq!(
        terminal.line(2).trim_end(),
        "  Q>uit to main menu      S>tatus, your            D>elete ALL messages/results"
    );
    assert_eq!(
        terminal.line(3).trim_end(),
        "  X>pert mode ON/OFF      P>rofile of your empire  O>ther empires (rankings)"
    );
    assert_eq!(
        terminal.line(4).trim_end(),
        "  V>iew Partial Starmap   M>ap of the galaxy       E>nemies, declare or list"
    );
    assert_eq!(
        line_containing(&terminal, "GENERAL COMMAND <-").trim_end(),
        "GENERAL COMMAND <-H,Q,X,V,I,A,S,P,M,C,R,D,O,E->"
    );
}

#[test]
fn main_menu_notice_renders_below_fixed_command_row() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    app.command_menu_notice = Some("No ships are waiting in stardock.".into());

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("main menu should render");
    assert_eq!(
        terminal.lines[6].trim_end(),
        "MAIN COMMAND <-H,Q,X,V,A,G,P,F,T,I,B,D->"
    );
    assert_eq!(terminal.lines[7].trim_end(), "");
    assert_eq!(terminal.lines[8].trim_end(), "");
    assert_eq!(terminal.lines[9].trim_end(), "");
    assert!(terminal.lines[10].contains("Notice: No ships are waiting in stardock."));
    assert!(!terminal.lines.iter().any(|line| line.contains("-- ")));
}

#[test]
fn main_menu_x_toggles_expert_mode_and_hides_menu_chrome() {
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
        app.handle_key(key(KeyCode::Char('x'))),
        Action::ToggleExpertMode
    );
    assert_eq!(
        apply_action(&mut app, Action::ToggleExpertMode),
        AppOutcome::Continue
    );
    assert!(app.expert_mode);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("expert main menu should render");
    assert_eq!(
        terminal.lines[0].trim_end(),
        "MAIN COMMAND <-H,Q,X,V,A,G,P,F,T,I,B,D->"
    );
    assert_eq!(terminal.lines[1].trim_end(), "");
    assert_eq!(terminal.lines[23].trim_end(), "");

    assert_eq!(
        apply_action(&mut app, Action::ToggleExpertMode),
        AppOutcome::Continue
    );
    assert!(!app.expert_mode);
    app.render(&mut terminal)
        .expect("normal main menu should render");
    assert_eq!(terminal.lines[0].trim_end(), "MAIN MENU:");
}

#[test]
fn general_menu_x_toggles_expert_mode_and_hides_menu_chrome() {
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

    assert_eq!(
        app.handle_key(key(KeyCode::Char('x'))),
        Action::ToggleExpertMode
    );
    assert_eq!(
        apply_action(&mut app, Action::ToggleExpertMode),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("expert general menu should render");
    assert_eq!(
        terminal.lines[0].trim_end(),
        "GENERAL COMMAND <-H,Q,X,V,I,A,S,P,M,C,R,D,O,E->"
    );
    assert_eq!(terminal.lines[1].trim_end(), "");
}

#[test]
fn main_menu_a_key_maps_to_real_ansi_toggle() {
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
        app.handle_key(key(KeyCode::Char('a'))),
        Action::ToggleAnsiMode
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Char('A'))),
        Action::ToggleAnsiMode
    );
}

#[test]
fn main_help_describes_the_ansi_toggle() {
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
        app.handle_key(key(KeyCode::Char('h'))),
        Action::OpenMainHelp
    );
    assert_eq!(
        apply_action(&mut app, Action::OpenMainHelp),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::MainHelp);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("main help should render");
    assert_eq!(
        terminal.line(3).trim_end(),
        "<A> - toggle ANSI color mode ON/OFF"
    );
}

#[test]
fn first_time_and_main_help_share_the_same_ansi_toggle_text() {
    let fixture_dir = temp_first_time_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_first_time_menu(&mut app);

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenFirstTimeHelp)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FirstTimeHelp);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("first-time help should render");
    assert_eq!(
        terminal.line(3).trim_end(),
        "<A> - toggle ANSI color mode ON/OFF"
    );
}

#[test]
fn first_time_menu_status_renders_below_fixed_command_row() {
    let fixture_dir = temp_first_time_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_first_time_menu(&mut app);
    app.startup_state.first_time_status = Some("Only two empires remain open.".into());

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("first-time menu should render");
    assert_eq!(
        terminal.lines[4].trim_end(),
        "FIRST TIME COMMAND <-H Q L J A V->"
    );
    assert_eq!(terminal.lines[5].trim_end(), "");
    assert_eq!(terminal.lines[6].trim_end(), "");
    assert_eq!(terminal.lines[7].trim_end(), "");
    assert!(terminal.lines[8].contains("Notice: Only two empires remain open."));
}

#[test]
fn planet_menu_matches_verified_v15_command_layout() {
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet menu should render");
    assert_eq!(
        terminal.line(0).trim_end(),
        "PLANET COMMANDS:    V>iew Partial Map                       T>ax rate: Empire"
    );
    assert_eq!(
        terminal.line(1).trim_end(),
        "  H>elp on Options  C>OMMISSION MENU   D>etail Planet List  S>corch planets"
    );
    assert_eq!(
        terminal.line(2).trim_end(),
        "  Q>uit: Main Menu  A>UTO-COMMISSION   P>lanet: Brief List  L>oad TTs w/Armies"
    );
    assert_eq!(
        terminal.line(3).trim_end(),
        "  X>pert mode       B>UILD MENU...     I>nfo about Planet   U>nload TT Armies"
    );
}

#[test]
fn planet_menu_notice_renders_below_fixed_command_row() {
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );
    app.command_menu_notice = Some("No ships or starbases are waiting in stardock.".into());

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet menu should render");
    assert_eq!(
        terminal.lines[5].trim_end(),
        "PLANET COMMAND <-H,Q,X,V,C,A,B,I,D,P,T,S,L,U->"
    );
    assert_eq!(terminal.lines[6].trim_end(), "");
    assert_eq!(terminal.lines[7].trim_end(), "");
    assert_eq!(terminal.lines[8].trim_end(), "");
    assert!(terminal.lines[9].contains("Notice: No ships or starbases are waiting in stardock."));
}

#[test]
fn planet_menu_expert_mode_keeps_notice_below_top_prompt() {
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );
    app.expert_mode = true;
    app.command_menu_notice = Some("No ships or starbases are waiting in stardock.".into());

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("expert planet menu should render");
    assert_eq!(
        terminal.lines[0].trim_end(),
        "PLANET COMMAND <-H,Q,X,V,C,A,B,I,D,P,T,S,L,U->"
    );
    assert_eq!(terminal.lines[1].trim_end(), "");
    assert_eq!(terminal.lines[2].trim_end(), "");
    assert_eq!(terminal.lines[3].trim_end(), "");
    assert!(terminal.lines[4].contains("Notice: No ships or starbases are waiting in stardock."));
}

#[test]
fn planet_commission_menu_renders_without_crashing_when_no_stardock_units_exist() {
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenCommissionMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);

    app.render(&mut terminal).expect("render succeeds");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| { line.contains("No owned planets have units waiting in stardock.") })
    );
}

#[test]
fn planet_build_menu_and_subscreens_render_without_crashing_when_no_owned_planets_exist() {
    let fixture_dir = temp_joined_no_assets_copy();
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);
    app.render(&mut terminal)
        .expect("planet menu render succeeds");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("No owned planets available"))
    );

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildReview)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);
    app.render(&mut terminal)
        .expect("build review fallback render succeeds");

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildList)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);
    app.render(&mut terminal)
        .expect("build list fallback render succeeds");

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenBuildAbortConfirm)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);
    app.render(&mut terminal)
        .expect("build abort fallback render succeeds");

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildSpecify)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);
    app.render(&mut terminal)
        .expect("build specify fallback render succeeds");
}

#[test]
fn planet_build_menu_matches_verified_v15_command_layout() {
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet build menu should render");
    assert_eq!(
        terminal.line(0).trim_end(),
        "BUILD ON CURRENT PLANET: \"Codex Prime\" IN SYSTEM [16,13]:"
    );
    assert_eq!(
        terminal.line(2).trim_end(),
        "  H>elp with commands        P>lanets, List your         S>pecify Build Orders"
    );
    assert_eq!(
        terminal.line(3).trim_end(),
        "  Q>uit to Planet Menu       R>eview current planet      A>bort planet's builds"
    );
    assert_eq!(
        terminal.line(4).trim_end(),
        "  X>pert mode ON/OFF         C>hange current planet      L>ist builds"
    );
    assert_eq!(
        terminal.line(5).trim_end(),
        "  V>iew partial star map     N>ext planet                I>nfo about a Planet"
    );
    assert_eq!(
        terminal.line(7).trim_end(),
        "BUILD COMMAND <-H,Q,X,V,P,R,C,N,S,A,L,I->"
    );
    assert_eq!(
        terminal.line(13).trim_end(),
        "There are no starbases orbiting planet \"Codex Prime\"."
    );
    assert_eq!(
        terminal.line(14).trim_end(),
        "Standard building restrictions apply."
    );
    assert_eq!(
        terminal.line(15).trim_end(),
        "You have spent 0 out of 0 points.  You have 0 points left to spend."
    );
    assert_eq!(
        terminal.lines[17].trim_end(),
        "Build queue: [0/10]   Stardock: [4/10]"
    );
}

#[test]
fn expert_mode_survives_command_menu_navigation_and_non_menu_screens_render_normally() {
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
        apply_action(&mut app, Action::ToggleExpertMode),
        AppOutcome::Continue
    );
    assert!(app.expert_mode);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("expert planet menu should render");
    assert_eq!(
        terminal.lines[0].trim_end(),
        "PLANET COMMAND <-H,Q,X,V,C,A,B,I,D,P,T,S,L,U->"
    );

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildMenu)),
        AppOutcome::Continue
    );
    app.render(&mut terminal)
        .expect("expert build menu should render");
    assert_eq!(
        terminal.lines[0].trim_end(),
        "BUILD COMMAND <-H,Q,X,V,P,R,C,N,S,A,L,I->"
    );
    assert_eq!(terminal.lines[1].trim_end(), "");

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildList)),
        AppOutcome::Continue
    );
    app.render(&mut terminal)
        .expect("build list should still render normally");
    assert_eq!(
        terminal.lines[0].trim_end(),
        "BUILD LIST: \"Codex Prime\" AT [16,13]:"
    );
    assert!(terminal.lines[4].contains("┌"));
}

#[test]
fn command_menus_render_without_crashing_for_empty_empire_state() {
    let fixture_dir = temp_joined_empty_empire_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);

    let mut terminal = CaptureTerminal::new();
    for action in [
        Action::Fleet(FleetAction::OpenMenu),
        Action::Fleet(FleetAction::OpenList(FleetListMode::Brief)),
        Action::Fleet(FleetAction::OpenList(FleetListMode::Full)),
        Action::Fleet(FleetAction::OpenReviewSelect),
        Action::Fleet(FleetAction::OpenReview),
        Action::Fleet(FleetAction::OpenRoeSelect),
        Action::Fleet(FleetAction::OpenDetach),
        Action::Fleet(FleetAction::OpenEta),
        Action::Fleet(FleetAction::OpenTransportLoad),
        Action::Fleet(FleetAction::OpenTransportUnload),
        Action::Planet(PlanetAction::OpenMenu),
        Action::Planet(PlanetAction::OpenAutoCommissionConfirm),
        Action::Planet(PlanetAction::OpenCommissionMenu),
        Action::Planet(PlanetAction::OpenBuildMenu),
        Action::Planet(PlanetAction::OpenBuildReview),
        Action::Planet(PlanetAction::OpenBuildList),
        Action::Planet(PlanetAction::OpenBuildChange),
        Action::Planet(PlanetAction::OpenBuildAbortConfirm),
        Action::Planet(PlanetAction::OpenBuildSpecify),
        Action::Planet(PlanetAction::OpenTransportPlanetSelect(
            ec_client::screen::PlanetTransportMode::Load,
        )),
        Action::Planet(PlanetAction::OpenTransportPlanetSelect(
            ec_client::screen::PlanetTransportMode::Unload,
        )),
        Action::Planet(PlanetAction::OpenListSortPrompt(PlanetListMode::Brief)),
        Action::Planet(PlanetAction::SubmitListSort(
            PlanetListMode::Brief,
            PlanetListSort::Location,
        )),
        Action::Planet(PlanetAction::OpenListSortPrompt(PlanetListMode::Detail)),
        Action::Planet(PlanetAction::SubmitListSort(
            PlanetListMode::Detail,
            PlanetListSort::Location,
        )),
    ] {
        apply_action(&mut app, action);
        app.render(&mut terminal)
            .expect("screen should render without crashing");
    }
}

#[test]
fn fleet_list_stays_on_fleet_menu_with_notice_when_no_fleets_exist() {
    let fixture_dir = temp_joined_empty_empire_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::OpenList(FleetListMode::Brief))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet menu should render empty-fleet notice");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("You have no active fleets."))
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::OpenList(FleetListMode::Full))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
}

#[test]
fn planet_list_commands_stay_on_planet_menu_with_notice_when_no_owned_planets_exist() {
    let fixture_dir = temp_joined_no_assets_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
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
            Action::Planet(PlanetAction::OpenListSortPrompt(PlanetListMode::Brief))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet menu should render empty-planet notice");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("You do not currently control any planets."))
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenListSortPrompt(PlanetListMode::Detail))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);
}

#[test]
fn delete_reviewables_stays_on_general_menu_with_notice_when_nothing_is_reviewable() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    clear_runtime_report_blocks(&mut state);
    save_runtime_state(&fixture_dir, &state);

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
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenDeleteReviewables)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("general menu should render empty-reviewables notice");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("No messages or results are currently reviewable."))
    );
}

#[test]
fn delete_reviewables_opens_when_classic_pending_flags_are_set() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    clear_runtime_report_blocks(&mut state);
    state.game_data.player.records[0].raw[0x30] = 1;
    state.game_data.player.records[0].raw[0x34] = 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
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
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenDeleteReviewables)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::DeleteReviewables);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::ConfirmDeleteReviewables)
        ),
        AppOutcome::Continue
    );

    let runtime = latest_runtime_state(&fixture_dir);
    assert!(runtime.report_block_rows.is_empty());
    assert_eq!(runtime.game_data.player.records[0].raw[0x30], 0);
    assert_eq!(runtime.game_data.player.records[0].raw[0x34], 0);
}

#[test]
fn startup_uses_classic_pending_flags_even_when_report_bytes_are_empty() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    clear_runtime_report_blocks(&mut state);
    state.game_data.player.records[0].raw[0x30] = 1;
    state.game_data.player.records[0].raw[0x34] = 1;
    save_runtime_state(&fixture_dir, &state);

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
        if app.current_screen() == ScreenId::Startup(StartupPhase::LoginSummary) {
            let mut terminal = CaptureTerminal::new();
            app.render(&mut terminal)
                .expect("login summary should render");
            assert!(
                terminal
                    .lines
                    .iter()
                    .any(|line| line.contains("The year is:"))
            );
            break;
        }
        app.advance_startup();
    }

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::Advance)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Results)
    );

    let mut results_terminal = CaptureTerminal::new();
    app.render(&mut results_terminal)
        .expect("startup results should render");
    assert!(results_terminal.lines.iter().any(|line| {
        line.contains("Reports are marked pending, but no review text is available yet.")
    }));

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::Advance)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Messages)
    );

    let mut messages_terminal = CaptureTerminal::new();
    app.render(&mut messages_terminal)
        .expect("startup messages should render");
    assert!(messages_terminal.lines.iter().any(|line| {
        line.contains("Messages are marked pending, but no review text is available yet.")
    }));

    advance_to_main_menu(&mut app);
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenReports)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Reports);

    let mut reports_terminal = CaptureTerminal::new();
    app.render(&mut reports_terminal)
        .expect("reports screen should render");
    assert!(reports_terminal.lines.iter().any(|line| {
        line.contains("reports are marked pending, but no review text is available yet")
    }));
    assert!(reports_terminal.lines.iter().any(|line| {
        line.contains("messages are marked pending, but no review text is available yet")
    }));
}

#[test]
fn startup_reviews_results_then_messages_then_enters_main_menu() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(&mut state, b"Fleet battle report");
    state.queued_mail.push(incoming_mail(
        2,
        1,
        state.game_data.conquest.game_year().saturating_sub(1),
        "Diplomatic",
        "Diplomatic telegram",
    ));
    state.game_data.player.records[0].raw[0x30] = 1;
    state.game_data.player.records[0].raw[0x34] = 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    let mut saw_login_summary = false;
    let mut saw_results = false;
    let mut saw_messages = false;

    for _ in 0..16 {
        match app.current_screen() {
            ScreenId::Startup(StartupPhase::LoginSummary) => saw_login_summary = true,
            ScreenId::Startup(StartupPhase::Results) => {
                assert!(saw_login_summary);
                saw_results = true;
            }
            ScreenId::Startup(StartupPhase::Messages) => {
                assert!(saw_results);
                saw_messages = true;
            }
            ScreenId::MainMenu => break,
            _ => {}
        }
        app.advance_startup();
    }

    assert!(saw_login_summary);
    assert!(saw_results);
    assert!(saw_messages);
    assert_eq!(app.current_screen(), ScreenId::MainMenu);
}

#[test]
fn startup_results_paginate_before_advancing_to_messages() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(
        &mut state,
        (1..=24)
            .map(|idx| format!("Report line {idx:02} is long enough"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    state.queued_mail.push(incoming_mail(
        2,
        1,
        state.game_data.conquest.game_year().saturating_sub(1),
        "Message",
        "Message line 01 is long enough",
    ));
    state.game_data.player.records[0].raw[0x30] = 1;
    state.game_data.player.records[0].raw[0x34] = 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::Results) {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Results)
    );

    // Advance from ViewPrompt into ItemBody to start showing content.
    app.advance_startup();

    let mut first_page = CaptureTerminal::new();
    app.render(&mut first_page)
        .expect("first startup results page should render");
    assert!(
        first_page
            .lines
            .iter()
            .any(|line| line.contains(" -> Report line 01"))
    );
    assert!(
        !first_page
            .lines
            .iter()
            .any(|line| line.contains("Report line 28"))
    );
    assert!(
        first_page
            .lines
            .iter()
            .any(|line| line.contains("(Slap a key for more)"))
    );

    for _ in 0..18 {
        let mut screen = CaptureTerminal::new();
        app.render(&mut screen)
            .expect("scrolled startup results should render");
        if screen
            .lines
            .iter()
            .any(|line| line.contains("Delete this report Y/[N] ->"))
        {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Results)
    );

    // Advance through DeletePrompt (keep) → EndStatus.
    app.advance_startup();

    let mut end_status = CaptureTerminal::new();
    app.render(&mut end_status)
        .expect("inline startup results completion should render");
    assert!(
        end_status
            .lines
            .iter()
            .any(|line| line.contains("All reports seen. (Slap a key)"))
    );
    assert!(
        end_status
            .lines
            .iter()
            .any(|line| line.contains("Delete this report Y/[N] ->"))
    );
    assert!(
        !end_status
            .lines
            .iter()
            .any(|line| line.contains("RESULTS REVIEW:"))
    );

    // EndStatus → phase exit → Messages.
    app.advance_startup();
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Messages)
    );
}

#[test]
fn startup_messages_allow_deleting_current_message_then_advancing() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    clear_runtime_report_blocks(&mut state);
    state
        .queued_mail
        .push(incoming_mail(2, 1, 2999, "One", "Body one"));
    state
        .queued_mail
        .push(incoming_mail(3, 1, 2999, "Two", "Body two"));
    state.game_data.player.records[0].raw[0x30] = 1;
    state.game_data.player.records[0].raw[0x34] = 0;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::Messages) {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Messages)
    );

    // ViewPrompt → ItemBody (shows Alpha).
    app.advance_startup();

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("first startup message should render");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains(" -> From") && line.contains("Empire #2"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("<end of message>"))
    );

    // Accept default at the end-of-block prompt → delete Alpha → ContinuePrompt (Beta still exists).
    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::AcceptDefault)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Messages)
    );

    // ContinuePrompt → ItemBody (shows Beta).
    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::AcceptDefault)),
        AppOutcome::Continue
    );

    let mut after_delete = CaptureTerminal::new();
    app.render(&mut after_delete)
        .expect("next startup message should render");
    assert!(
        after_delete
            .lines
            .iter()
            .any(|line| line.contains(" -> From") && line.contains("Empire #3"))
    );

    let runtime = latest_runtime_state(&fixture_dir);
    let preview = ec_client::reports::ReportsPreview::from_block_rows(
        &runtime.game_data,
        1,
        &runtime.report_block_rows,
        &runtime.queued_mail,
    );
    assert_eq!(preview.message_blocks.len(), 1);
    assert!(
        preview
            .message_lines
            .iter()
            .any(|line| line.contains("From") && line.contains("Empire #3"))
    );
    assert!(
        preview
            .message_lines
            .iter()
            .any(|line| line.contains("<end of message>"))
    );
    assert!(runtime.queued_mail[0].recipient_deleted);
    assert!(!runtime.queued_mail[1].recipient_deleted);
}

#[test]
fn startup_message_review_shows_end_status_after_deleting_last_message() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    clear_runtime_report_blocks(&mut state);
    state
        .queued_mail
        .push(incoming_mail(2, 1, 2999, "One", "Body one"));
    state.game_data.player.records[0].raw[0x30] = 1;
    state.game_data.player.records[0].raw[0x34] = 0;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::Messages) {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Messages)
    );

    // ViewPrompt → ItemBody.
    app.advance_startup();
    // Accept delete at the end-of-block prompt → EndStatus (only 1 block).
    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::AcceptDefault)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Messages)
    );

    let mut end_status = CaptureTerminal::new();
    app.render(&mut end_status)
        .expect("end status should render");
    assert!(
        end_status
            .lines
            .iter()
            .any(|line| line.contains("Messages deleted."))
    );
    assert!(
        end_status
            .lines
            .iter()
            .any(|line| line.contains("All messages seen. (Slap a key)"))
    );
    assert!(
        !end_status
            .lines
            .iter()
            .any(|line| line.contains("MESSAGES REVIEW:"))
    );

    // Advance from EndStatus → phase exit → MainMenu.
    app.advance_startup();
    assert_eq!(app.current_screen(), ScreenId::MainMenu);

    let runtime = latest_runtime_state(&fixture_dir);
    assert_eq!(runtime.queued_mail.len(), 1);
    assert!(runtime.queued_mail[0].recipient_deleted);
    assert_eq!(
        runtime.game_data.player.records[0].classic_messages_pending_flag_raw(),
        0
    );
}

#[test]
fn startup_results_wrap_long_lines_within_the_playfield() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(
        &mut state,
        b"This is a deliberately long startup results line that should wrap cleanly within the eighty column playfield instead of overrunning a single row.",
    );
    state.game_data.player.records[0].raw[0x30] = 0;
    state.game_data.player.records[0].raw[0x34] = 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::Results) {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Results)
    );

    // Advance from ViewPrompt into ItemBody to start showing content.
    app.advance_startup();

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("startup results should render");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains(" -> This is a deliberately long startup results line"))
    );
    assert!(terminal.lines.iter().any(|line| line.starts_with(" -> ")));
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("should wrap cleanly"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("eighty column playfield"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("single row."))
    );
}

#[test]
fn startup_results_preserve_blank_lines_as_classic_spacers() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(
        &mut state,
        classic_chunked_report_bytes("Line one\n\nLine two"),
    );
    state.game_data.player.records[0].raw[0x30] = 0;
    state.game_data.player.records[0].raw[0x34] = 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::Results) {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Results)
    );

    // Advance from ViewPrompt into ItemBody to start showing content.
    app.advance_startup();

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("startup results should render");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains(" -> Line one"))
    );
    assert!(terminal.lines.iter().any(|line| line.trim_end() == " ->"));
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains(" -> Line two"))
    );
}

#[test]
fn startup_results_preserve_leading_spaces_from_oracle_style_reports() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(
        &mut state,
        classic_chunked_report_bytes("  Stardate 11 / 3003\n    Fleet 7 arrived"),
    );
    state.game_data.player.records[0].raw[0x30] = 0;
    state.game_data.player.records[0].raw[0x34] = 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::Results) {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Results)
    );

    app.advance_startup();

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("startup results should render");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.starts_with(" ->   Stardate 11 / 3003"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.starts_with(" ->     Fleet 7 arrived"))
    );
}

#[test]
fn startup_results_use_the_full_intro_review_page_height() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(
        &mut state,
        (1..=15)
            .map(|idx| format!("Report {idx:02}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    state.game_data.player.records[0].raw[0x30] = 0;
    state.game_data.player.records[0].raw[0x34] = 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::Results) {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Results)
    );

    app.advance_startup();

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("startup results should render");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains(" -> Report 15"))
    );
    assert!(!terminal.line(19).contains("for more"));
}

#[test]
fn startup_results_decode_length_prefixed_lines_as_separate_classic_rows() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(
        &mut state,
        length_prefixed_report_block(&[
            "From your 12th Fleet, located in System(9,14):          Stardate: 2/3003",
            "We were attacked by \"Nadir Compact\", (Empire #4) in System(9,14). Our",
            "force contained 1 destroyer and 1 ETAC ship. Alien force contained 1",
            "<end of transmission>",
        ]),
    );
    state.game_data.player.records[0].raw[0x30] = 0;
    state.game_data.player.records[0].raw[0x34] = 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::Results) {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Results)
    );

    app.advance_startup();

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("startup results should render");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| { line.contains("System(9,14):          Stardate: 02/3003") })
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| { line.contains("We were attacked by \"Nadir Compact\", (Empire #4)") })
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("<end of transmission>"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Delete this report Y/[N] ->"))
    );
    assert!(terminal.line(COMMAND_LINE_ROW - 1).trim().is_empty());
    assert!(!terminal.lines.iter().any(|line| line.contains("----")));
}

#[test]
fn startup_results_continue_prompt_preserves_blank_spacing_without_rule() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(
        &mut state,
        classic_chunked_report_blocks(&["From Alpha\nBody one", "From Beta\nBody two"]),
    );
    state.game_data.player.records[0].raw[0x30] = 0;
    state.game_data.player.records[0].raw[0x34] = 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    for _ in 0..16 {
        if app.current_screen() == ScreenId::Startup(StartupPhase::Results) {
            break;
        }
        app.advance_startup();
    }
    assert_eq!(
        app.current_screen(),
        ScreenId::Startup(StartupPhase::Results)
    );

    app.advance_startup();
    app.advance_startup();

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("startup continue prompt should render");
    assert!(
        terminal
            .line(19)
            .contains("There are more reports. Continue?")
    );
    assert!(terminal.line(COMMAND_LINE_ROW - 1).trim().is_empty());
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Delete this report Y/[N] ->"))
    );
    assert_eq!(
        terminal
            .lines
            .iter()
            .filter(|line| line.contains("There are more reports. Continue?"))
            .count(),
        1
    );
    assert!(!terminal.lines.iter().any(|line| line.contains("----")));
}

#[test]
fn reports_screen_preserves_blank_separator_lines() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(
        &mut state,
        classic_chunked_report_bytes("Line one\n\nLine two"),
    );
    state.game_data.player.records[0].raw[0x34] = 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenReports)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Reports);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("reports screen should render");
    let line_one_idx = terminal
        .lines
        .iter()
        .position(|line| line.trim_end() == "  Line one")
        .expect("reports screen should contain first line");
    assert!(terminal.lines[line_one_idx + 1].trim().is_empty());
    assert_eq!(terminal.lines[line_one_idx + 2].trim_end(), "  Line two");
}

#[test]
fn reports_screen_wraps_long_lines_within_the_playfield() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(
        &mut state,
        b"This is a deliberately long reports review line that should wrap cleanly within the eighty column playfield instead of overrunning a single row.",
    );
    state.game_data.player.records[0].raw[0x34] = 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenReports)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Reports);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("reports screen should render");
    let first_line_idx = terminal
        .lines
        .iter()
        .position(|line| line.contains("This is a deliberately long reports review line"))
        .expect("reports screen should contain wrapped first line");
    assert!(terminal.lines[first_line_idx].starts_with("  "));
    assert!(terminal.lines[first_line_idx + 1].starts_with("  "));
    assert!(
        terminal.lines[first_line_idx].contains("should wrap cleanly")
            || terminal.lines[first_line_idx + 1].contains("should wrap cleanly")
    );
    assert!(
        terminal.lines[first_line_idx + 1].contains("eighty column playfield")
            || terminal.lines[first_line_idx + 2].contains("eighty column playfield")
    );
}

#[test]
fn reports_screen_keeps_both_sections_visible_when_results_are_long() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(
        &mut state,
        (1..=8)
            .map(|idx| format!("This is long report line {idx:02} and it should wrap across rows"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    state.queued_mail.push(incoming_mail(
        2,
        1,
        state.game_data.conquest.game_year().saturating_sub(1),
        "Visible",
        "Message line 01 should still remain visible",
    ));
    state.game_data.player.records[0].raw[0x30] = 1;
    state.game_data.player.records[0].raw[0x34] = 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenReports)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Reports);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("reports screen should render");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.trim_end() == "MESSAGES")
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Message line 01 should still remain visible"))
    );
}

#[test]
fn reports_screen_shows_explicit_truncation_cue_when_wrapped_rows_overflow() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(
        &mut state,
        (1..=16)
            .map(|idx| format!("This is long report line {idx:02} and it should wrap across rows"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    state.game_data.player.records[0].raw[0x34] = 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);

    assert_eq!(
        apply_action(&mut app, Action::Startup(StartupAction::OpenReports)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::Reports);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("reports screen should render");
    assert!(terminal.lines.iter().any(|line| {
        line.contains("<...")
            && line.contains("more line(s); use startup review for full suspense>")
    }));
}

#[test]
fn preloaded_first_login_reviews_reports_before_homeworld_naming() {
    let fixture_dir = temp_joined_needs_homeworld_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    set_runtime_report_blocks(&mut state, b"Fleet battle report");
    state.queued_mail.push(incoming_mail(
        2,
        1,
        state.game_data.conquest.game_year().saturating_sub(1),
        "Diplomatic",
        "Diplomatic telegram",
    ));
    state.game_data.player.records[0].raw[0x30] = 1;
    state.game_data.player.records[0].raw[0x34] = 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    let mut saw_results = false;
    let mut saw_messages = false;
    for _ in 0..16 {
        match app.current_screen() {
            ScreenId::Startup(StartupPhase::Results) => saw_results = true,
            ScreenId::Startup(StartupPhase::Messages) => {
                assert!(saw_results);
                saw_messages = true;
            }
            ScreenId::FirstTimePreloadedRenamePrompt => break,
            _ => {}
        }
        app.advance_startup();
    }

    assert!(saw_results);
    assert!(saw_messages);
    assert_eq!(
        app.current_screen(),
        ScreenId::FirstTimePreloadedRenamePrompt
    );
}

#[test]
fn returning_player_reviews_reports_before_colony_naming() {
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
    set_runtime_report_blocks(&mut state, b"Scout report");
    state.queued_mail.push(incoming_mail(
        2,
        1,
        state.game_data.conquest.game_year().saturating_sub(1),
        "Command",
        "Command mail",
    ));
    state.game_data.player.records[0].raw[0x30] = 1;
    state.game_data.player.records[0].raw[0x34] = 1;
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    let mut saw_results = false;
    let mut saw_messages = false;
    for _ in 0..16 {
        match app.current_screen() {
            ScreenId::Startup(StartupPhase::Results) => saw_results = true,
            ScreenId::Startup(StartupPhase::Messages) => {
                assert!(saw_results);
                saw_messages = true;
            }
            ScreenId::ColonyWorldName => break,
            _ => {}
        }
        app.advance_startup();
    }

    assert!(saw_results);
    assert!(saw_messages);
    assert_eq!(app.current_screen(), ScreenId::ColonyWorldName);
}

#[test]
fn fleet_review_opens_with_a_selection_table_first() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReviewSelect)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetReviewSelect);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet review select should render");
    assert_eq!(terminal.line(0).trim_end(), "REVIEW A FLEET:");
    assert!(
        terminal
            .line(1)
            .contains("Select a fleet, then press ENTER to review its status")
    );
    let prompt = line_containing(&terminal, "COMMANDS <ARROWS J K Q> [");
    assert!(prompt.contains("COMMANDS <ARROWS J K Q> ["));
    assert!(prompt.contains("->"));
}

#[test]
fn fleet_review_select_accepts_typed_fleet_id_and_opens_that_fleet() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReviewSelect)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendReviewChar('1'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitReviewSelect)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetReview);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet review detail should render");
    assert!(terminal.line(2).contains("Fleet ID: 1"));
}

#[test]
fn fleet_review_select_navigation_updates_the_default_fleet_prompt() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReviewSelect)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::MoveReviewSelect(1))),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet review select should render after move");
    let prompt = line_containing(&terminal, "COMMANDS <ARROWS J K Q> [");
    assert!(prompt.contains("COMMANDS <ARROWS J K Q> ["));
    assert!(prompt.contains("->"));
}

#[test]
fn fleet_review_select_shows_invalid_fleet_message_on_unknown_typed_id() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReviewSelect)),
        AppOutcome::Continue
    );
    for ch in ['9', '9'] {
        assert_eq!(
            apply_action(&mut app, Action::Fleet(FleetAction::AppendReviewChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitReviewSelect)),
        AppOutcome::Continue
    );
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet review select should render invalid id notice");
    assert!(terminal.line(19).contains("Notice:"));
    assert!(terminal.line(19).contains("(slap a key)"));
}

#[test]
fn fleet_menu_load_and_unload_keys_open_fleet_transport_flow() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();
    let fleet = &mut state.game_data.fleets.records[0];
    fleet.set_current_location_coords_raw(home_coords);
    fleet.set_troop_transport_count(3);
    fleet.set_army_count(1);
    fleet.recompute_max_speed_from_composition();
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );

    assert_eq!(
        app.handle_key(key(KeyCode::Char('l'))),
        Action::Fleet(FleetAction::OpenTransportLoad)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportLoad)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetTransportPlanetSelect(ec_client::screen::PlanetTransportMode::Load)
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet load transport picker should render");
    let prompt = line_containing(&terminal, "FLEET COMMAND <-");
    assert!(prompt.contains("FLEET COMMAND"));
    assert!(prompt.contains("<Q> ->"));
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
        app.handle_key(key(KeyCode::Char('u'))),
        Action::Fleet(FleetAction::OpenTransportUnload)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportUnload)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetTransportPlanetSelect(ec_client::screen::PlanetTransportMode::Unload)
    );
    app.render(&mut terminal)
        .expect("fleet unload transport picker should render");
    let prompt = line_containing(&terminal, "FLEET COMMAND <-");
    assert!(prompt.contains("FLEET COMMAND"));
    assert!(prompt.contains("<Q> ->"));
}

#[test]
fn fleet_menu_load_and_unload_show_menu_notice_when_no_transport_action_is_available() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportLoad)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet menu should render load notice");
    assert!(terminal.lines.iter().any(|line| {
        line.contains("No planets have armies and troop transports ready to load.")
    }));

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportUnload)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    app.render(&mut terminal)
        .expect("fleet menu should render unload notice");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| { line.contains("No fleets have loaded armies ready to unload") })
    );
}

#[test]
fn fleet_transport_planet_picker_accepts_typed_coordinates() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();
    let fleet = &mut state.game_data.fleets.records[0];
    fleet.set_current_location_coords_raw(home_coords);
    fleet.set_troop_transport_count(3);
    fleet.set_army_count(1);
    fleet.recompute_max_speed_from_composition();
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportLoad)),
        AppOutcome::Continue
    );
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet load transport picker should render");
    let prompt = line_containing(&terminal, "FLEET COMMAND <-");
    let default_coords = prompt
        .split('[')
        .nth(1)
        .and_then(|tail| tail.split(']').next())
        .expect("default coords should be shown in prompt");
    for ch in default_coords.chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Planet(PlanetAction::AppendTransportPlanetChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitTransportPlanet)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetTransportFleetSelect(ec_client::screen::PlanetTransportMode::Load)
    );
}

#[test]
fn fleet_menu_long_notice_wraps_instead_of_clipping() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportUnload)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet menu should render wrapped notice");
    assert_eq!(
        terminal.lines[6].trim_end(),
        "FLEET COMMAND <-H,Q,X,V,S,B,F,R,E,C,I,D,T,O,G,M,L,U->"
    );
    assert_eq!(terminal.lines[7].trim_end(), "");
    assert_eq!(terminal.lines[8].trim_end(), "");
    assert_eq!(terminal.lines[9].trim_end(), "");
    let wrapped_notice = [
        &terminal.lines[10],
        &terminal.lines[11],
        &terminal.lines[12],
    ]
    .into_iter()
    .flat_map(|line| line.split_whitespace())
    .collect::<Vec<_>>()
    .join(" ");
    assert!(
        wrapped_notice.contains(
            "No fleets have loaded armies ready to unload onto planets with free capacity."
        )
    );
}

#[test]
fn fleet_menu_x_toggles_expert_mode_and_hides_menu_chrome() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );

    assert_eq!(
        app.handle_key(key(KeyCode::Char('x'))),
        Action::ToggleExpertMode
    );
    assert_eq!(
        apply_action(&mut app, Action::ToggleExpertMode),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert!(app.expert_mode);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("expert fleet menu should render");
    assert_eq!(
        terminal.lines[0].trim_end(),
        "FLEET COMMAND <-H,Q,X,V,S,B,F,R,E,C,I,D,T,O,G,M,L,U->"
    );
    assert_eq!(terminal.lines[1].trim_end(), "");
}

#[test]
fn fleet_merge_sets_join_order_for_selected_source_and_host() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMerge)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMerge);

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendMergeChar('1'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMerge)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMerge);

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendMergeChar('2'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMerge)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet menu should render merge success notice");
    assert_eq!(
        terminal.lines[6].trim_end(),
        "FLEET COMMAND <-H,Q,X,V,S,B,F,R,E,C,I,D,T,O,G,M,L,U->"
    );
    assert_eq!(terminal.lines[7].trim_end(), "");
    assert_eq!(terminal.lines[8].trim_end(), "");
    assert_eq!(terminal.lines[9].trim_end(), "");
    let wrapped_notice = [
        &terminal.lines[10],
        &terminal.lines[11],
        &terminal.lines[12],
    ]
    .into_iter()
    .flat_map(|line| line.split_whitespace())
    .collect::<Vec<_>>()
    .join(" ");
    assert!(wrapped_notice.contains("ordered to join Fleet #"));

    let state = latest_runtime_state(&fixture_dir);
    let source = &state.game_data.fleets.records[0];
    assert_eq!(
        source.standing_order_kind(),
        ec_data::Order::JoinAnotherFleet
    );
    assert_ne!(source.join_host_fleet_id_raw(), 0);
    let valid_host = state.game_data.fleets.records.iter().any(|fleet| {
        fleet.owner_empire_raw() == 1
            && fleet.fleet_id() == source.join_host_fleet_id_raw()
            && fleet.current_location_coords_raw() == source.standing_order_target_coords_raw()
    });
    assert!(valid_host);
}

#[test]
fn fleet_group_order_uses_select_column_and_space_toggles_rows() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Char('g'))),
        Action::Fleet(FleetAction::OpenGroupOrder)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet group order screen should render");
    assert!(terminal.line(4).contains("Sel"));
    assert!(terminal.line(4).contains("Ord"));
    assert!(terminal.line(4).contains("Target"));
    assert!(!terminal.line(6).contains(" X "));

    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    app.render(&mut terminal)
        .expect("fleet group order selection should render");
    assert!(terminal.line(6).contains("X"));
}

#[test]
fn fleet_group_order_opens_mission_picker_and_q_returns_to_group_table() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::Fleet(FleetAction::OpenMissionPicker)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMissionPicker);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet mission picker should render");
    assert_eq!(terminal.line(0), "FLEET MISSION ORDERS:");
    assert!(terminal.line(2).contains("No."));
    assert!(terminal.lines.iter().any(|line| line.contains("15")));
    let prompt = line_containing(&terminal, "COMMANDS <ARROWS J K Q> [");
    assert!(prompt.contains("COMMANDS <ARROWS J K Q> ["));
    assert!(prompt.contains("->"));

    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Fleet(FleetAction::OpenMissionPicker)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);
}

#[test]
fn fleet_order_opens_mission_picker_and_q_returns_to_order_table() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Char('o'))),
        Action::Fleet(FleetAction::OpenOrder)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetOrder);
    assert_eq!(
        app.handle_key(key(KeyCode::PageDown)),
        Action::Fleet(FleetAction::MoveOrderSelect(8))
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet order screen should render");
    let prompt = line_containing(&terminal, "COMMANDS <ARROWS J K Q> [");
    assert!(prompt.contains("COMMANDS <ARROWS J K Q> ["));
    assert!(prompt.contains("->"));

    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::Fleet(FleetAction::SubmitOrder)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMissionPicker);

    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Fleet(FleetAction::OpenMissionPicker)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetOrder);
}

#[test]
fn fleet_order_applies_move_order_to_selected_fleet_only() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendOrderChar('2'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMissionPicker);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetOrder);
    for ch in ['1', '4', ',', '9'] {
        assert_eq!(
            apply_action(&mut app, Action::Fleet(FleetAction::AppendOrderChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetOrder);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet order should render success notice");
    assert!(terminal.line(19).contains("Notice:"));
    assert!(terminal.line(19).contains("(slap a key)"));

    let state = latest_runtime_state(&fixture_dir);
    let ordered_fleet = state
        .game_data
        .fleets
        .records
        .iter()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    assert_eq!(ordered_fleet.standing_order_code_raw(), 1);
    assert_eq!(ordered_fleet.standing_order_target_coords_raw(), [14, 9]);
    assert_eq!(
        state
            .game_data
            .fleets
            .records
            .iter()
            .filter(|fleet| !(fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2))
            .filter(|fleet| fleet.standing_order_code_raw() == 1)
            .count(),
        0
    );
}

#[test]
fn fleet_order_allows_guard_starbase_from_fleet_command() {
    let fixture_dir = temp_game_with_starbase_copy();
    let before = latest_runtime_state(&fixture_dir);
    assert_eq!(
        before.game_data.fleets.records[0].standing_order_code_raw(),
        4
    );
    assert_eq!(
        before.game_data.fleets.records[1].standing_order_code_raw(),
        5
    );
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendOrderChar('2'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMissionPicker);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('4'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetOrder);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("guard starbase target prompt should render");
    assert_eq!(
        terminal.line(1).trim_end(),
        "Enter the starbase number for Guard a Starbase."
    );
    assert!(
        line_containing(&terminal, "Starbase # [").contains("Starbase # [1]"),
        "{}",
        line_containing(&terminal, "Starbase # [")
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );

    let state = latest_runtime_state(&fixture_dir);
    let ordered_fleet = state
        .game_data
        .fleets
        .records
        .iter()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    assert_eq!(ordered_fleet.standing_order_code_raw(), 4);
    assert_eq!(ordered_fleet.mission_aux_bytes(), [1, 1]);
    assert_eq!(ordered_fleet.standing_order_target_coords_raw(), [6, 5]);
}

#[test]
fn fleet_order_blocks_guard_starbase_when_player_has_no_starbases() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendOrderChar('2'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMissionPicker);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('4'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetOrder);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("no-starbase guard order notice should render");
    assert!(
        line_containing(&terminal, "You have no starbases available to guard.")
            .contains("You have no starbases available to guard.")
    );
}

#[test]
fn fleet_order_allows_join_another_fleet_from_fleet_command() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendOrderChar('1'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMissionPicker);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('3'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetOrder);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("join-fleet target prompt should render");
    assert_eq!(
        terminal.line(1).trim_end(),
        "Enter the host fleet number for Join another fleet."
    );
    assert!(
        line_containing(&terminal, "Fleet # [").contains("Fleet # ["),
        "{}",
        line_containing(&terminal, "Fleet # [")
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );

    let state = latest_runtime_state(&fixture_dir);
    let source = state
        .game_data
        .fleets
        .records
        .iter()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    assert_eq!(
        source.standing_order_kind(),
        ec_data::Order::JoinAnotherFleet
    );
    assert_ne!(source.join_host_fleet_id_raw(), 0);
    let valid_host = state.game_data.fleets.records.iter().any(|fleet| {
        fleet.owner_empire_raw() == 1
            && fleet.fleet_id() == source.join_host_fleet_id_raw()
            && fleet.current_location_coords_raw() == source.standing_order_target_coords_raw()
    });
    assert!(valid_host);
}

#[test]
fn fleet_order_persists_immediately_and_reloaded_tables_reflect_it() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendOrderChar('2'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    for ch in ['1', '4', ',', '9'] {
        assert_eq!(
            apply_action(&mut app, Action::Fleet(FleetAction::AppendOrderChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetOrder);

    let persisted = latest_runtime_state(&fixture_dir);
    assert_eq!(
        persisted.game_data.fleets.records[1].standing_order_code_raw(),
        1
    );
    assert_eq!(
        persisted.game_data.fleets.records[1].standing_order_target_coords_raw(),
        [14, 9]
    );

    let mut reloaded = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("reloaded app should load");
    advance_to_main_menu(&mut reloaded);
    assert_eq!(
        apply_action(&mut reloaded, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut reloaded, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut reloaded,
            Action::Fleet(FleetAction::AppendOrderChar('2'))
        ),
        AppOutcome::Continue
    );
    let mut terminal = CaptureTerminal::new();
    reloaded
        .render(&mut terminal)
        .expect("reloaded fleet order table should render");
    let table_text = (5..16)
        .map(|row| terminal.line(row).to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(table_text.contains("[14, 9]"));
}

#[test]
fn fleet_tables_sort_by_mission_then_newest_fleet_id() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    for fleet in state.game_data.fleets.records.iter_mut().take(4) {
        fleet.set_standing_order_code_raw(9);
    }
    state.game_data.fleets.records[0].set_standing_order_code_raw(0);
    state.game_data.fleets.records[1].set_standing_order_code_raw(1);
    state.game_data.fleets.records[2].set_standing_order_code_raw(0);
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReviewSelect)),
        AppOutcome::Continue
    );
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet review select should render");
    assert!(terminal.line(6).starts_with("│ 3│"));
    assert!(terminal.line(7).starts_with("│ 1│"));
    assert!(terminal.line(8).starts_with("│ 2│"));
}

#[test]
fn fleet_group_bombard_mission_defaults_to_closest_known_enemy_world() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let viewer_index = 0usize;
    let home_coords = state.game_data.planets.records
        [state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1]
        .coords_raw();
    let mut foreign_candidates = state
        .game_data
        .planets
        .records
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner_empire_slot_raw() != 1)
        .map(|(idx, planet)| (idx, planet.coords_raw()))
        .collect::<Vec<_>>();
    foreign_candidates.sort_by_key(|(_, coords)| {
        let dx = i32::from(home_coords[0]) - i32::from(coords[0]);
        let dy = i32::from(home_coords[1]) - i32::from(coords[1]);
        dx * dx + dy * dy
    });
    let (closest_idx, closest_coords) = foreign_candidates[0];
    let (other_idx, _) = foreign_candidates[1];
    state.game_data.planets.records[closest_idx].set_owner_empire_slot_raw(2);
    state.game_data.planets.records[other_idx].set_owner_empire_slot_raw(2);
    let mut planet_intel_by_viewer = (1..=state.game_data.conquest.player_count())
        .map(|viewer_empire_id| {
            CampaignStore::open_default_in_dir(&fixture_dir)
                .expect("open campaign store")
                .latest_planet_intel_for_viewer(viewer_empire_id)
                .expect("load runtime intel")
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>()
        })
        .collect::<Vec<_>>();
    let year = state.game_data.conquest.game_year();
    planet_intel_by_viewer[viewer_index].insert(
        closest_idx + 1,
        partial_known_world_snapshot(
            closest_idx + 1,
            &state.game_data.planets.records[closest_idx],
            2,
            year,
        ),
    );
    planet_intel_by_viewer[viewer_index].insert(
        other_idx + 1,
        partial_known_world_snapshot(
            other_idx + 1,
            &state.game_data.planets.records[other_idx],
            2,
            year,
        ),
    );
    save_runtime_state_with_intel(&fixture_dir, &state, &planet_intel_by_viewer);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('6'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("combat target prompt should render");
    assert!(line_containing(&terminal, "Target [").contains(&format!(
        "Target [{},{}] <Q> ->",
        closest_coords[0], closest_coords[1]
    )));
}

#[test]
fn fleet_group_colonize_mission_skips_worlds_claimed_by_other_friendly_etacs() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let home_coords = state.game_data.planets.records
        [state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1]
        .coords_raw();
    let mut unowned_candidates = state
        .game_data
        .planets
        .records
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner_empire_slot_raw() == 0)
        .map(|(idx, planet)| (idx, planet.coords_raw()))
        .collect::<Vec<_>>();
    unowned_candidates.sort_by_key(|(_, coords)| {
        let dx = i32::from(home_coords[0]) - i32::from(coords[0]);
        let dy = i32::from(home_coords[1]) - i32::from(coords[1]);
        dx * dx + dy * dy
    });
    let claimed_coords = unowned_candidates[0].1;
    let fallback_coords = unowned_candidates[1].1;
    for fleet in state.game_data.fleets.records.iter_mut() {
        if fleet.owner_empire_raw() == 1 {
            fleet.set_standing_order_code_raw(9);
        }
    }
    {
        let other_etac = state
            .game_data
            .fleets
            .records
            .iter_mut()
            .enumerate()
            .find(|(idx, fleet)| {
                *idx != 0 && fleet.owner_empire_raw() == 1 && fleet.etac_count() > 0
            })
            .map(|(_, fleet)| fleet)
            .expect("fixture should have a second ETAC fleet");
        other_etac.set_standing_order_kind(ec_data::Order::ColonizeWorld);
        other_etac.set_standing_order_target_coords_raw(claimed_coords);
    }
    let selected_etac = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| {
            fleet.owner_empire_raw() == 1
                && fleet.etac_count() > 0
                && fleet.standing_order_kind() != ec_data::Order::ColonizeWorld
        })
        .expect("fixture should have a selectable ETAC fleet");
    selected_etac.set_standing_order_code_raw(0);
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('2'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("colonize target prompt should render");
    assert!(line_containing(&terminal, "Target [").contains(&format!(
        "Target [{},{}] <Q> ->",
        fallback_coords[0], fallback_coords[1]
    )));
}

#[test]
fn fleet_group_colonize_mission_allows_hidden_colonized_worlds_as_targets() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let home_coords = state.game_data.planets.records
        [state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1]
        .coords_raw();
    let mut candidates = state
        .game_data
        .planets
        .records
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner_empire_slot_raw() == 0)
        .map(|(idx, planet)| (idx, planet.coords_raw()))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(_, coords)| {
        let dx = i32::from(home_coords[0]) - i32::from(coords[0]);
        let dy = i32::from(home_coords[1]) - i32::from(coords[1]);
        dx * dx + dy * dy
    });
    let hidden_colonized_idx = candidates[0].0;
    let hidden_colonized_coords = candidates[0].1;
    state.game_data.planets.records[hidden_colonized_idx].set_owner_empire_slot_raw(2);
    let mut planet_intel_by_viewer = (1..=state.game_data.conquest.player_count())
        .map(|viewer_empire_id| {
            CampaignStore::open_default_in_dir(&fixture_dir)
                .expect("open campaign store")
                .latest_planet_intel_for_viewer(viewer_empire_id)
                .expect("load runtime intel")
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>()
        })
        .collect::<Vec<_>>();
    planet_intel_by_viewer[0].remove(&(hidden_colonized_idx + 1));
    for fleet in state.game_data.fleets.records.iter_mut() {
        if fleet.owner_empire_raw() == 1 {
            fleet.set_standing_order_code_raw(9);
        }
    }
    let selected_etac = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.etac_count() > 0)
        .expect("fixture should have a selectable ETAC fleet");
    selected_etac.set_standing_order_code_raw(0);
    save_runtime_state_with_intel(&fixture_dir, &state, &planet_intel_by_viewer);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('2'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("colonize target prompt should render");
    assert!(line_containing(&terminal, "Target [").contains(&format!(
        "Target [{},{}] <Q> ->",
        hidden_colonized_coords[0], hidden_colonized_coords[1]
    )));
}

#[test]
fn fleet_mission_picker_rejects_missions_not_supported_by_all_selected_fleets() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::MoveGroupOrder(6))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMissionPicker);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('0'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("disabled mission rejection should render");
    assert!(terminal.line(19).contains("Notice:"));
    assert!(terminal.line(19).contains("(slap a key)"));
}

#[test]
fn fleet_group_order_rejects_empty_sector_for_world_targeting_mission() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('9'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);
    for ch in ['1', ',', '1'] {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Fleet(FleetAction::AppendGroupOrderChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitGroupOrder)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("world-target validation should render");
    assert!(terminal.line(19).contains("Notice:"));
    assert!(terminal.line(19).contains("(slap a key)"));
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::DismissModalNotice
    );
    assert_eq!(
        apply_action(&mut app, Action::DismissModalNotice),
        AppOutcome::Continue
    );
    app.render(&mut terminal)
        .expect("world-target prompt should return after dismiss");
    let prompt = line_containing(&terminal, "Target [");
    assert!(prompt.contains("Target ["));
    assert!(prompt.contains("->"), "{prompt}");
    assert!(!prompt.contains("1,1"));
}

#[test]
fn fleet_group_order_allows_owned_planet_for_blockade_mission() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let target_idx = state
        .game_data
        .planets
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner_empire_slot_raw() != 1)
        .map(|(idx, _)| idx)
        .expect("fixture should have a foreign world");
    state.game_data.planets.records[target_idx].set_owner_empire_slot_raw(1);
    let owned_target = state.game_data.planets.records[target_idx].coords_raw();
    save_runtime_state(&fixture_dir, &state);
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('5'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);
    for ch in format!("{},{}", owned_target[0], owned_target[1]).chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Fleet(FleetAction::AppendGroupOrderChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitGroupOrder)),
        AppOutcome::Continue
    );

    let state = latest_runtime_state(&fixture_dir);
    let ordered_fleet = state
        .game_data
        .fleets
        .records
        .iter()
        .find(|fleet| {
            fleet.owner_empire_raw() == 1
                && fleet.standing_order_kind() == ec_data::Order::GuardBlockadeWorld
                && fleet.standing_order_target_coords_raw() == owned_target
        })
        .expect("one selected fleet should accept an owned blockade target");
    assert_eq!(
        ordered_fleet.standing_order_kind(),
        ec_data::Order::GuardBlockadeWorld
    );
    assert_eq!(
        ordered_fleet.standing_order_target_coords_raw(),
        owned_target
    );
}

#[test]
fn fleet_group_order_allows_owned_planet_for_scout_mission() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    for fleet in state.game_data.fleets.records.iter_mut() {
        if fleet.owner_empire_raw() == 1 {
            fleet.set_standing_order_code_raw(9);
        }
    }
    let scout_fleet = state
        .game_data
        .fleets
        .records
        .get_mut(0)
        .expect("fleet 1 should exist");
    scout_fleet.set_scout_count(1);
    scout_fleet.set_standing_order_code_raw(0);
    let enemy_idx = state
        .game_data
        .planets
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner_empire_slot_raw() != 1)
        .map(|(idx, _)| idx)
        .expect("fixture should have a foreign world");
    state.game_data.planets.records[enemy_idx].set_owner_empire_slot_raw(1);
    let owned_target = state.game_data.planets.records[enemy_idx].coords_raw();
    save_runtime_state(&fixture_dir, &state);
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('0'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);
    for ch in format!("{},{}", owned_target[0], owned_target[1]).chars() {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Fleet(FleetAction::AppendGroupOrderChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitGroupOrder)),
        AppOutcome::Continue
    );

    let state = latest_runtime_state(&fixture_dir);
    let ordered_fleet = state
        .game_data
        .fleets
        .records
        .iter()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("player 1 fleet #1 should exist");
    assert_eq!(
        ordered_fleet.standing_order_kind(),
        ec_data::Order::ScoutSector
    );
    assert_eq!(
        ordered_fleet.standing_order_target_coords_raw(),
        owned_target
    );
}

#[test]
fn fleet_order_salvage_defaults_to_closest_owned_planet() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let extra_owned_idx = state
        .game_data
        .planets
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner_empire_slot_raw() != 1)
        .map(|(idx, _)| idx)
        .expect("fixture should have a non-owned planet");
    state.game_data.planets.records[extra_owned_idx].set_owner_empire_slot_raw(1);
    let nearest_owned = state.game_data.planets.records[extra_owned_idx].coords_raw();
    let selected_fleet = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("player 1 fleet #1 should exist");
    selected_fleet.set_current_location_coords_raw(nearest_owned);
    selected_fleet.set_standing_order_target_coords_raw(nearest_owned);
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendOrderChar('1'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('5'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("salvage target prompt should render");
    assert!(line_containing(&terminal, "Target [").contains(&format!(
        "Target [{},{}] <Q> ->",
        nearest_owned[0], nearest_owned[1]
    )));
}

#[test]
fn fleet_order_salvage_rejects_empty_sector_target() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendOrderChar('1'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('5'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    for ch in ['1', ',', '1'] {
        assert_eq!(
            apply_action(&mut app, Action::Fleet(FleetAction::AppendOrderChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("salvage empty-sector validation should render");
    assert!(
        line_containing(&terminal, "That mission needs a system with a planet")
            .contains("That mission needs a system with a planet at the target.")
    );
}

#[test]
fn fleet_order_salvage_rejects_foreign_planet_target() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let foreign_idx = state
        .game_data
        .planets
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner_empire_slot_raw() == 0)
        .map(|(idx, _)| idx)
        .expect("fixture should have an unowned planet");
    state.game_data.planets.records[foreign_idx].set_owner_empire_slot_raw(2);
    let mut planet_intel_by_viewer = (1..=state.game_data.conquest.player_count())
        .map(|viewer_empire_id| {
            CampaignStore::open_default_in_dir(&fixture_dir)
                .expect("open campaign store")
                .latest_planet_intel_for_viewer(viewer_empire_id)
                .expect("load runtime intel")
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>()
        })
        .collect::<Vec<_>>();
    planet_intel_by_viewer[0].insert(
        foreign_idx + 1,
        partial_known_world_snapshot(
            foreign_idx + 1,
            &state.game_data.planets.records[foreign_idx],
            2,
            state.game_data.conquest.game_year(),
        ),
    );
    let foreign_target = state.game_data.planets.records[foreign_idx].coords_raw();
    save_runtime_state_with_intel(&fixture_dir, &state, &planet_intel_by_viewer);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendOrderChar('1'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('5'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    for ch in format!("{},{}", foreign_target[0], foreign_target[1]).chars() {
        assert_eq!(
            apply_action(&mut app, Action::Fleet(FleetAction::AppendOrderChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("salvage foreign-planet validation should render");
    assert!(
        line_containing(
            &terminal,
            "That mission requires one of your owned planets."
        )
        .contains("That mission requires one of your owned planets.")
    );
}

#[test]
fn fleet_order_salvage_rejects_unowned_planet_target() {
    let fixture_dir = temp_game_copy();
    let state = latest_runtime_state(&fixture_dir);
    let unowned_target = state
        .game_data
        .planets
        .records
        .iter()
        .find(|planet| planet.owner_empire_slot_raw() == 0)
        .map(|planet| planet.coords_raw())
        .expect("fixture should have an unowned planet");

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendOrderChar('1'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('5'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    for ch in format!("{},{}", unowned_target[0], unowned_target[1]).chars() {
        assert_eq!(
            apply_action(&mut app, Action::Fleet(FleetAction::AppendOrderChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("salvage unowned-planet validation should render");
    assert!(
        line_containing(
            &terminal,
            "That mission requires one of your owned planets."
        )
        .contains("That mission requires one of your owned planets.")
    );
}

#[test]
fn partial_starmap_view_uses_full_80x25_layout_without_sidebar_legend() {
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

    assert!(terminal.line(0).contains("Map Center at Sector"));
    assert_eq!(terminal.line(1), "");
    assert_eq!(terminal.line(2), "");
    // Grid is centered: map_cell_start_col = (80 - 52) / 2 = 14, map_left_col = 9
    let leading_spaces = terminal.line(3).chars().take_while(|ch| *ch == ' ').count();
    assert_eq!(leading_spaces, 9);
    assert!(terminal.line(3).contains("18 |"));
    assert!(terminal.line(20).contains("01 |"));
    // x-axis at row 21 (just below the grid), command prompt at row 24
    assert!(terminal.line(21).contains("01"));
    assert!(terminal.line(21).contains("18"));
    assert_eq!(terminal.line(22), "");
    assert_eq!(terminal.line(23), "");
    assert!(terminal.line(24).contains("MAP COMMAND"));
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
fn partial_starmap_small_map_stays_centered_while_crosshair_tracks_selected_sector() {
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
    let axis_line = line_containing(&terminal, "25");
    assert!(axis_line.contains("01"));
    assert!(axis_line.contains("25"));
    assert_eq!(terminal.line(20).chars().nth(11), Some('+'));
}

#[test]
fn partial_starmap_enter_opens_planet_info_and_returns_to_map() {
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
fn fleet_group_order_allows_manual_combat_target_without_known_enemy_world() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('5'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("combat target prompt should render");
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);
    let prompt = line_containing(&terminal, "Target [");
    assert!(prompt.contains("Target ["));
    assert!(!prompt.contains("Notice:"));
}

#[test]
fn fleet_group_order_allows_manual_scout_target_without_known_enemy_world() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    for fleet in state.game_data.fleets.records.iter_mut() {
        if fleet.owner_empire_raw() == 1 {
            fleet.set_standing_order_code_raw(9);
        }
    }
    let scout_fleet = state
        .game_data
        .fleets
        .records
        .get_mut(0)
        .expect("fleet 1 should exist");
    scout_fleet.set_scout_count(1);
    scout_fleet.set_standing_order_code_raw(0);
    save_runtime_state(&fixture_dir, &state);
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('0'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("scout target prompt should render");
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);
    let prompt = line_containing(&terminal, "Target [");
    assert!(prompt.contains("Target ["));
    assert!(!prompt.contains("Notice:"));
}

#[test]
fn fleet_group_order_applies_move_order_to_selected_fleets() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    for fleet in state.game_data.fleets.records.iter_mut() {
        if fleet.owner_empire_raw() == 1 {
            fleet.set_standing_order_code_raw(9);
        }
    }
    state.game_data.fleets.records[0].set_standing_order_code_raw(0);
    state.game_data.fleets.records[1].set_standing_order_code_raw(0);
    save_runtime_state(&fixture_dir, &state);
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::MoveGroupOrder(1))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMissionPicker);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    for ch in ['1', '0', ',', '1', '3'] {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Fleet(FleetAction::AppendGroupOrderChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet group order should render success notice");
    assert!(terminal.line(19).contains("Notice:"));
    assert!(terminal.line(19).contains("(slap a key)"));

    let state = latest_runtime_state(&fixture_dir);
    assert_eq!(
        state.game_data.fleets.records[0].standing_order_code_raw(),
        1
    );
    assert_eq!(
        state.game_data.fleets.records[0].standing_order_target_coords_raw(),
        [10, 13]
    );
    assert_eq!(
        state.game_data.fleets.records[1].standing_order_code_raw(),
        1
    );
    assert_eq!(
        state.game_data.fleets.records[1].standing_order_target_coords_raw(),
        [10, 13]
    );
}

#[test]
fn fleet_group_order_accepts_join_fleet_mission_number() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenGroupOrder)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::ToggleGroupOrderSelection)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMissionPicker)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMissionPicker);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Fleet(FleetAction::AppendMissionPickerChar('3'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet group join target prompt should render");
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);
    assert_eq!(
        terminal.line(1).trim_end(),
        "Enter the host fleet number for Join another fleet."
    );
    assert!(
        line_containing(&terminal, "Fleet # [").contains("Fleet # ["),
        "{}",
        line_containing(&terminal, "Fleet # [")
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitGroupOrder)),
        AppOutcome::Continue
    );

    let state = latest_runtime_state(&fixture_dir);
    let joined_fleets = state
        .game_data
        .fleets
        .records
        .iter()
        .filter(|fleet| fleet.owner_empire_raw() == 1 && fleet.standing_order_code_raw() == 13)
        .collect::<Vec<_>>();
    assert_eq!(joined_fleets.len(), 1);
    let ordered_fleet = joined_fleets[0];
    assert_eq!(ordered_fleet.standing_order_code_raw(), 13);
    assert_ne!(ordered_fleet.join_host_fleet_id_raw(), 0);
    assert_ne!(
        ordered_fleet.join_host_fleet_id_raw(),
        ordered_fleet.fleet_id()
    );
    assert_eq!(ordered_fleet.standing_order_target_coords_raw(), [16, 13]);
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenRoeSelect)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetRoeSelect);

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendRoeChar('4'))),
        AppOutcome::Continue
    );
    assert_eq!(app.selected_fleet_roe_id(), Some(4));
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitRoe)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetRoeSelect);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Fleet(FleetAction::OpenRoeSelect)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenRoeSelect)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetRoeSelect);

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendRoeChar('4'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitRoe)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendRoeChar('7'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitRoe)),
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenRoeSelect)),
        AppOutcome::Continue
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendRoeChar('4'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitRoe)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitRoe)),
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenRoeSelect)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendRoeChar('4'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitRoe)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendRoeChar('9'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitRoe)),
        AppOutcome::Continue
    );

    app.render(&mut terminal).expect("render succeeds");
    assert_eq!(
        line_containing(&terminal, "COMMANDS <ARROWS J K Q> ["),
        "COMMANDS <ARROWS J K Q> [4] ->"
    );
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenDatabase)),
        AppOutcome::Continue
    );

    app.render(&mut terminal).expect("render succeeds");
    assert!(terminal.line(1).starts_with("┌"));
    assert!(terminal.line(2).contains("(X,Y)"));
    assert!(terminal.line(2).contains("Planet Name"));
    assert!(terminal.line(2).contains("Max"));
    assert!(terminal.line(2).contains("Curr"));
    assert!(terminal.line(2).contains("Seen"));
    assert!(terminal.line(2).contains("Scout"));
    assert!(terminal.line(2).contains("Intel"));
    assert!(terminal.lines.iter().any(|line| line.contains("3000")));
    assert!(terminal.lines.iter().any(|line| line.contains("owned")));
    let prompt = line_containing(&terminal, "COMMANDS <");
    assert!(prompt.starts_with("COMMANDS <"));
    assert!(prompt.contains("["));
    assert!(prompt.contains("->"));
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
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenInfoPrompt(CommandMenu::Main))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitInfoPrompt)),
        AppOutcome::Continue
    );

    app.render(&mut terminal).expect("render succeeds");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Last Intel: "))
    );
    assert!(terminal.lines.iter().any(|line| line.contains("3000")));
    assert!(terminal.lines.iter().any(|line| line.contains("owned")));
}

#[test]
fn fleet_roe_render_keeps_command_line_on_bottom_row() {
    let mut screen = FleetRoeScreen::new();
    let rows = vec![FleetRow {
        fleet_record_index_1_based: 1,
        fleet_number: 1,
        coords: [16, 13],
        target_coords: [16, 13],
        order_code: 5,
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
    assert_eq!(buffer.plain_line(8), "COMMANDS <ARROWS J K Q> [1] ->");
    let (cursor_col, cursor_row) = buffer.cursor().expect("cursor on command row");
    assert_eq!(cursor_row as usize, 8);
    assert!(cursor_col < 80);
}

#[test]
fn fleet_roe_render_shows_edit_errors_on_bottom_line() {
    let mut screen = FleetRoeScreen::new();
    let rows = vec![FleetRow {
        fleet_record_index_1_based: 1,
        fleet_number: 6,
        coords: [16, 13],
        target_coords: [16, 13],
        order_code: 0,
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
        buffer.plain_line(8),
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
            order_code: 0,
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
            order_code: 0,
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
            order_code: 0,
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

    assert!(buffer.plain_line(6).contains("│001│"));
    assert!(buffer.plain_line(7).contains("│010│"));
    assert!(buffer.plain_line(8).contains("│100│"));
}

#[test]
fn fleet_eta_screen_renders_bottom_line_prompt() {
    let mut screen = ec_client::screen::FleetEtaScreen::new();
    let rows = vec![FleetRow {
        fleet_record_index_1_based: 1,
        fleet_number: 7,
        coords: [16, 13],
        target_coords: [19, 13],
        order_code: 1,
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
    assert!(buffer.plain_line(4).contains("Ord"));
    assert!(buffer.plain_line(4).contains("Target"));
    assert!(buffer.plain_line(6).contains("│  1│"));
    assert!(buffer.plain_line(6).contains("[19,13]"));
    assert!(
        buffer
            .plain_line(8)
            .contains("COMMANDS <ARROWS J K Q> [7] ->")
    );
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenEta)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetEta);

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendEtaChar('4'))),
        AppOutcome::Continue
    );
    assert_eq!(app.selected_fleet_eta_id(), Some(4));
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitEta)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetEta);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendEtaChar('1'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendEtaChar('0'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendEtaChar(','))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendEtaChar('1'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendEtaChar('3'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitEta)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitEta)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetEta);
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::Fleet(FleetAction::SubmitEta)
    );
}

#[test]
fn fleet_eta_uses_max_speed_when_selected_fleet_is_stopped() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let fleet = state
        .game_data
        .fleets
        .records
        .get_mut(0)
        .expect("fleet 1 should exist");
    let current_coords = fleet.current_location_coords_raw();
    fleet.set_current_speed(0);
    save_runtime_state(&fixture_dir, &state);
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenEta)),
        AppOutcome::Continue
    );
    for ch in ['1'] {
        assert_eq!(
            apply_action(&mut app, Action::Fleet(FleetAction::AppendEtaChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitEta)),
        AppOutcome::Continue
    );
    for ch in format!("{},{}", current_coords[0], current_coords[1]).chars() {
        assert_eq!(
            apply_action(&mut app, Action::Fleet(FleetAction::AppendEtaChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitEta)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitEta)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet eta result should render");
    let prompt = line_containing(&terminal, "Fleet 1 reaches [");
    assert!(
        prompt.contains(&format!(
            "Fleet 1 reaches [{},{}] in 0 year(s)",
            current_coords[0], current_coords[1]
        )),
        "{}",
        prompt
    );
    assert!(!prompt.contains("is stopped"));
}

#[test]
fn fleet_eta_allows_empty_sector_targets_for_resting_hold_fleets() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let fleet = state
        .game_data
        .fleets
        .records
        .get_mut(0)
        .expect("fleet 1 should exist");
    fleet.set_current_speed(0);
    fleet.set_standing_order_kind(ec_data::Order::HoldPosition);
    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenEta)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendEtaChar('1'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitEta)),
        AppOutcome::Continue
    );
    for ch in ['1', ',', '1'] {
        assert_eq!(
            apply_action(&mut app, Action::Fleet(FleetAction::AppendEtaChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitEta)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitEta)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet eta empty-sector result should render");
    let prompt = line_containing(&terminal, "Fleet 1 reaches [1,1] in");
    assert!(prompt.contains("Fleet 1 reaches [1,1] in"));
    assert!(!prompt.contains("No route found"));
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
            .plain_line(14)
            .contains("BUILD COMMAND <- Unit number or 0 if done")
    );
    assert!(buffer.plain_line(14).contains("[0] <Q> ->"));
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
            .plain_line(14)
            .contains("BUILD COMMAND <- How many new destroyers to build")
    );
    assert!(buffer.plain_line(14).contains("[6] <Q> ->"));
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
    })
    .expect("app should load");

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenDeleteReviewables)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::DeleteReviewables);

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
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenComposeRecipient)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageRecipient);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::AppendComposeRecipientChar('2'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::SubmitComposeRecipient)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageSubject);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::AppendComposeSubjectChar('H'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::AppendComposeSubjectChar('i'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::SubmitComposeSubject)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageBody);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::AppendComposeBodyChar('H'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::AppendComposeBodyChar('i'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenComposeSendConfirm)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageSendConfirm);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::ConfirmSendComposedMessage)
        ),
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
        recipient_deleted: false,
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
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenComposeOutbox)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageOutbox);
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::AppendComposeOutboxChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::DeleteQueuedComposeMessage)
        ),
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
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenComposeRecipient)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::AppendComposeRecipientChar('2'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::SubmitComposeRecipient)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::SubmitComposeSubject)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageBody);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenComposeDiscardConfirm)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageDiscardConfirm);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenComposeBody)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::ComposeMessageBody);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::OpenComposeDiscardConfirm)
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::ConfirmDiscardComposedMessage)
        ),
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
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    import_directory_snapshot(&store, &fixture_dir).expect("refresh sqlite snapshot");

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir.clone(),
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenDetach)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetDetach);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("render detach select");
    let prompt = line_containing(&terminal, "Detach ships from fleet # [");
    assert!(prompt.contains("Detach ships from fleet # ["));
    assert!(prompt.contains("<Q> ->"));
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendDetachChar('1'))),
        AppOutcome::Continue
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitDetach)),
        AppOutcome::Continue
    );
    app.render(&mut terminal).expect("render destroyer prompt");
    assert!(
        line_containing(&terminal, "Destroyers to detach [")
            .contains("Destroyers to detach [0] <Q> ->")
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendDetachChar('1'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitDetach)),
        AppOutcome::Continue
    );
    app.render(&mut terminal).expect("render etac prompt");
    assert!(
        line_containing(&terminal, "ETAC ships to detach [")
            .contains("ETAC ships to detach [0] <Q> ->")
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendDetachChar('1'))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitDetach)),
        AppOutcome::Continue
    );
    app.render(&mut terminal).expect("render roe prompt");
    assert!(line_containing(&terminal, "New fleet ROE [").contains("New fleet ROE [6] <Q> ->"));

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitDetach)),
        AppOutcome::Continue
    );
    app.render(&mut terminal)
        .expect("render detach select after save");
    let prompt = line_containing(&terminal, "Detach ships from fleet # [");
    assert!(prompt.contains("Detach ships from fleet # ["));
    assert!(prompt.contains("<Q> ->"));

    let updated = latest_runtime_state(&fixture_dir).game_data;
    assert_eq!(updated.fleets.records.len(), initial_fleet_count + 1);
    assert_eq!(updated.fleets.records[0].destroyer_count(), 1);
    assert_eq!(updated.fleets.records[0].etac_count(), 0);
    let detached = updated.fleets.records.last().expect("detached fleet");
    assert_eq!(detached.destroyer_count(), 1);
    assert_eq!(detached.etac_count(), 1);
}

#[test]
fn fleet_detach_with_zero_selected_ships_returns_to_the_table_without_a_warning() {
    let fixture_dir = temp_game_copy();
    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("app should load");

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenDetach)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetDetach);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("render detach select");
    let prompt = line_containing(&terminal, "Detach ships from fleet # [");
    assert!(prompt.contains("Detach ships from fleet # ["));
    assert!(prompt.contains("<Q> ->"));
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendDetachChar('1'))),
        AppOutcome::Continue
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitDetach)),
        AppOutcome::Continue
    );
    app.render(&mut terminal)
        .expect("render first quantity prompt");

    for _ in 0..8 {
        let prompt = line_containing(&terminal, "Detach ships from fleet # [");
        if prompt.contains("Detach ships from fleet # [") && prompt.contains("<Q> ->") {
            break;
        }
        assert_eq!(
            apply_action(&mut app, Action::Fleet(FleetAction::SubmitDetach)),
            AppOutcome::Continue
        );
        app.render(&mut terminal)
            .expect("advance zero-detach prompt sequence");
    }

    let prompt = line_containing(&terminal, "Detach ships from fleet # [");
    assert!(prompt.contains("Detach ships from fleet # [1] <Q> ->"));
    assert!(!prompt.contains("Detach at least one ship."));
}
