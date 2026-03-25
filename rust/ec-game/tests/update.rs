use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ec_compat::{decode_report_block_rows, import_directory_snapshot};
use ec_data::{
    CampaignRuntimeState, CampaignStore, CoreGameData, DiplomaticRelation, EmpirePlanetEconomyRow,
    EmpireProductionRankingSort, GameConfig, InactivityConfig, IntelTier, PlanetIntelSnapshot,
    ProductionItemKind, QueuedPlayerMail, SessionConfig,
};
use ec_engine::yearly_tax_revenue;
use ec_game::app::{Action, App, AppConfig, AppOutcome, apply_action};
use ec_game::domains::empire::EmpireAction;
use ec_game::domains::fleet::FleetAction;
use ec_game::domains::fleet::missions::{
    FLEET_MISSION_OPTIONS, FleetMissionRequirement, fleet_record_supports_mission_code,
};
use ec_game::domains::messaging::MessagingAction;
use ec_game::domains::planet::PlanetAction;
use ec_game::domains::starbase::StarbaseAction;
use ec_game::domains::starmap::StarmapAction;
use ec_game::domains::startup::StartupAction;
use ec_game::model::ClassicLoginState;
use ec_game::screen::layout::COMMAND_LINE_ROW;
use ec_game::screen::table::{TableColumn, fit_table_columns};
use ec_game::screen::{
    CommandMenu, FleetGroupOrderMode, FleetGroupScreen, FleetRow, PlanetBuildMenuView,
    PlanetBuildOrder, PlanetBuildScreen, PlanetCommissionDraftRow, PlanetListMode, PlanetListSort,
    ScreenId,
};
use ec_game::startup::StartupPhase;
use ec_game::terminal::Terminal;

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
    let root = temp_dir("ec-game-update");
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
    let root = temp_dir("ec-game-first-time");
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

fn temp_full_game_copy() -> PathBuf {
    let root = temp_first_time_game_copy();
    let mut data = CoreGameData::load(&root).expect("load full-game fixture");
    for player in 1..=4 {
        data.join_player(player, &format!("Empire {player}"))
            .expect("join player for full-game fixture");
    }
    data.save(&root).expect("save full-game fixture");
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

fn temp_game_with_auto_commission_copy() -> PathBuf {
    let root = temp_game_copy();
    let mut state = latest_runtime_state(&root);
    let homeworld = state
        .game_data
        .planets
        .records
        .iter_mut()
        .find(|planet| planet.owner_empire_slot_raw() == 1)
        .expect("owned planet exists");
    homeworld.set_stardock_kind_raw(0, 1);
    homeworld.set_stardock_count_raw(0, 4);
    homeworld.set_stardock_kind_raw(1, 2);
    homeworld.set_stardock_count_raw(1, 2);
    homeworld.set_stardock_kind_raw(2, 9);
    homeworld.set_stardock_count_raw(2, 1);
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

fn key_with_kind(code: KeyCode, kind: KeyEventKind) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind,
        state: KeyEventState::NONE,
    }
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
        known_starbase_count: None,
        known_current_production: None,
        known_stored_points: None,
        known_docked_summary: None,
        known_orbit_summary: None,
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

fn strongest_owned_fleet_number(root: &Path) -> u16 {
    latest_runtime_state(root)
        .game_data
        .fleets
        .records
        .iter()
        .filter(|fleet| fleet.owner_empire_raw() == 1)
        .max_by_key(|fleet| {
            (
                fleet.battleship_count(),
                fleet.cruiser_count(),
                fleet.destroyer_count(),
                fleet.troop_transport_count(),
                fleet.scout_count(),
                fleet.etac_count(),
                std::cmp::Reverse(fleet.local_slot_word_raw()),
            )
        })
        .expect("owned fleet exists")
        .local_slot_word_raw()
}

fn submit_fleet_menu_prompt(app: &mut App, fleet_number: Option<u16>) {
    if let Some(fleet_number) = fleet_number {
        submit_fleet_menu_prompt_value(app, &fleet_number.to_string());
        return;
    }
    assert_eq!(
        apply_action(&mut *app, Action::Fleet(FleetAction::SubmitMenuPrompt)),
        AppOutcome::Continue
    );
}

fn submit_fleet_menu_prompt_value(app: &mut App, value: &str) {
    for ch in value.chars() {
        assert_eq!(
            apply_action(
                &mut *app,
                Action::Fleet(FleetAction::AppendMenuPromptChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut *app, Action::Fleet(FleetAction::SubmitMenuPrompt)),
        AppOutcome::Continue
    );
}

fn open_review_from_fleet_menu(app: &mut App, fleet_number: Option<u16>) {
    assert_eq!(
        apply_action(app, Action::Fleet(FleetAction::OpenReviewPrompt)),
        AppOutcome::Continue
    );
    submit_fleet_menu_prompt(app, fleet_number);
}

fn open_order_mission_picker_from_fleet_menu(app: &mut App, fleet_number: Option<u16>) {
    assert_eq!(
        apply_action(app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );
    submit_fleet_menu_prompt(app, fleet_number);
    assert_eq!(app.current_screen(), ScreenId::FleetMissionPicker);
}

fn open_change_value_prompt_from_fleet_menu(app: &mut App, fleet_number: Option<u16>, field: char) {
    assert_eq!(
        apply_action(app, Action::Fleet(FleetAction::OpenChangePrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    submit_fleet_menu_prompt(app, fleet_number);
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    submit_fleet_menu_prompt_value(app, &field.to_string());
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
}

fn open_eta_from_fleet_menu(app: &mut App, fleet_number: Option<u16>) {
    assert_eq!(
        apply_action(app, Action::Fleet(FleetAction::OpenEta)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    submit_fleet_menu_prompt(app, fleet_number);
    assert_eq!(app.current_screen(), ScreenId::FleetEta);
}

fn open_detach_from_fleet_menu(app: &mut App, fleet_number: Option<u16>) {
    assert_eq!(
        apply_action(app, Action::Fleet(FleetAction::OpenDetach)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    submit_fleet_menu_prompt(app, fleet_number);
    assert_eq!(app.current_screen(), ScreenId::FleetDetach);
}

fn enter_detach_input(app: &mut App, input: &str) {
    for ch in input.chars() {
        assert_eq!(
            apply_action(app, Action::Fleet(FleetAction::AppendDetachChar(ch))),
            AppOutcome::Continue
        );
    }
}

fn submit_detach(app: &mut App) {
    assert_eq!(
        apply_action(app, Action::Fleet(FleetAction::SubmitDetach)),
        AppOutcome::Continue
    );
}

fn enter_fleet_order_target(app: &mut App, coords: [u8; 2]) {
    for ch in format!("{:02}", coords[0]).chars() {
        assert_eq!(
            apply_action(app, Action::Fleet(FleetAction::AppendOrderChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
    for ch in format!("{:02}", coords[1]).chars() {
        assert_eq!(
            apply_action(app, Action::Fleet(FleetAction::AppendOrderChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
}

fn confirm_fleet_order(app: &mut App, confirm: bool) {
    if !confirm {
        assert_eq!(
            apply_action(app, Action::Fleet(FleetAction::AppendOrderChar('N'))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
}

fn enter_fleet_group_order_target(app: &mut App, coords: [u8; 2]) {
    for ch in format!("{:02}", coords[0]).chars() {
        assert_eq!(
            apply_action(app, Action::Fleet(FleetAction::AppendGroupOrderChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut *app, Action::Fleet(FleetAction::SubmitGroupOrder)),
        AppOutcome::Continue
    );
    for ch in format!("{:02}", coords[1]).chars() {
        assert_eq!(
            apply_action(app, Action::Fleet(FleetAction::AppendGroupOrderChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut *app, Action::Fleet(FleetAction::SubmitGroupOrder)),
        AppOutcome::Continue
    );
}

fn confirm_fleet_group_order(app: &mut App, confirm: bool) {
    if !confirm {
        assert_eq!(
            apply_action(app, Action::Fleet(FleetAction::AppendGroupOrderChar('N'))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut *app, Action::Fleet(FleetAction::SubmitGroupOrder)),
        AppOutcome::Continue
    );
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
        playfield: &ec_game::screen::PlayfieldBuffer,
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildAbortPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildMenu);
    let mut terminal = CaptureTerminal::new();
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
            .contains("You are a pre-loaded player and this is your first time on.")
    );
    assert!(
        terminal
            .line(6)
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

#[test]
fn app_load_persists_all_setup_backed_config_fields_into_runtime_snapshot() {
    let fixture_dir = temp_first_time_game_copy();
    let before_setup = latest_runtime_state(&fixture_dir).game_data.setup.clone();
    let config = GameConfig {
        game_name: "Config Persistence Test".to_string(),
        theme: None,
        snoop: false,
        session: SessionConfig {
            max_idle_minutes: 19,
            minimum_time_minutes: 7,
            local_timeout: true,
            remote_timeout: false,
        },
        inactivity: InactivityConfig {
            purge_after_turns: 12,
            autopilot_after_turns: 5,
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
fn fleet_review_detail_q_returns_to_review_picker() {
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
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Fleet(FleetAction::CloseReview)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CloseReview)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
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
        terminal.line(0).trim_end(),
        "FLEET COMMAND CENTER:                                        O>rder a Fleet"
    );
    assert_eq!(
        terminal.line(1).trim_end(),
        "  H>elp on Options   S>TARBASE MENU...   C>hg ROE,ID,Speed   G>ROUP FLEET ORDER"
    );
    assert_eq!(
        terminal.line(2).trim_end(),
        "  Q>uit: Main Menu   E>TA Calc           I>nfo about Planet  M>erge a Fleet"
    );
    assert_eq!(
        terminal.line(3).trim_end(),
        "  X>pert Mode        F>leet List         D>etach Ships       L>oad TTs w/Armies"
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
        line_containing(&terminal, "FLEET COMMAND <- Transfer From Fleet #")
            .contains("Transfer From Fleet # [")
    );

    submit_fleet_menu_prompt(&mut app, Some(1));
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    app.render(&mut terminal)
        .expect("transfer host prompt should render");
    assert!(
        line_containing(&terminal, "FLEET COMMAND <- Transfer To Fleet #")
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
    assert!(line_containing(&terminal, "Class <BB,CA,DD,TT*,TT,SC,ET,C,X,Q>").contains("<Q> ->"));
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
    advance_to_main_menu(&mut app);
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
        app.fleet.menu_prompt_status.as_deref(),
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
        app.fleet.menu_prompt_status.as_deref(),
        Some("Fleet #3 is not in the same sector as Fleet #1.")
    );
}

#[test]
fn main_menu_matches_verified_v15_command_layout() {
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        "  H>elp with commands     A>utopilot [ON] [OFF]    R>eview messages/results"
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        "PLANET COMMANDS:                                            T>ax rate: Empire"
    );
    assert_eq!(
        terminal.line(1).trim_end(),
        "  H>elp on Options  C>OMMISSION MENU   V>iew Partial Map    S>corch planets"
    );
    assert_eq!(
        terminal.line(2).trim_end(),
        "  Q>uit: Main Menu  A>UTO-COMMISSION   P>lanet List         L>oad TTs w/Armies"
    );
    assert_eq!(
        terminal.line(3).trim_end(),
        "  X>pert mode       B>UILD MENU...     I>nfo about Planet   U>nload TT Armies"
    );
    assert_eq!(terminal.line(4).trim_end(), "");
}

#[test]
fn planet_menu_notice_renders_below_fixed_command_row() {
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
    app.command_menu_notice = Some("No ships or starbases are waiting in stardock.".into());

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("planet menu should render");
    assert_eq!(
        terminal.lines[5].trim_end(),
        "PLANET COMMAND <-H,Q,X,V,C,A,B,I,P,T,S,L,U->"
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        "PLANET COMMAND <-H,Q,X,V,C,A,B,I,P,T,S,L,U->"
    );
    assert_eq!(terminal.lines[1].trim_end(), "");
    assert_eq!(terminal.lines[2].trim_end(), "");
    assert_eq!(terminal.lines[3].trim_end(), "");
    assert!(terminal.lines[4].contains("Notice: No ships or starbases are waiting in stardock."));
}

#[test]
fn confirm_auto_commission_opens_paged_report_when_entries_exist() {
    let fixture_dir = temp_game_with_auto_commission_copy();
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
            Action::Planet(PlanetAction::OpenAutoCommissionPrompt)
        ),
        AppOutcome::Continue
    );
    assert!(app.planet.auto_commission_prompt_active);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::ConfirmAutoCommission)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetAutoCommissionReport);
    assert!(!app.planet.auto_commission_report_rows.is_empty());
    assert_eq!(
        app.planet.auto_commission_report_revealed_rows,
        app.planet.auto_commission_report_rows.len().min(23)
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("report should render");
    assert_eq!(terminal.line(24).trim_end(), "(slap a key)");
    assert_eq!(terminal.line(23).trim_end(), "");
    assert!(line_containing(&terminal, "Fleet").contains("commissioned from \""));
    assert!(line_containing(&terminal, "Starbase").contains("commissioned to \""));
}

#[test]
fn auto_commission_report_advances_by_page_then_returns_to_planet_menu() {
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
    app.open_planet_menu();
    app.current_screen = ScreenId::PlanetAutoCommissionReport;
    app.planet.auto_commission_report_rows = (1..=24)
        .map(|idx| {
            format!("Fleet {idx:02} commissioned from \"Foo\" in sector (08,09) with DD 01.")
        })
        .collect();
    app.planet.auto_commission_report_revealed_rows = 23;

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::AdvanceAutoCommissionReport)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetAutoCommissionReport);
    assert_eq!(app.planet.auto_commission_report_revealed_rows, 24);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::AdvanceAutoCommissionReport)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);
    assert!(app.planet.auto_commission_report_rows.is_empty());
}

#[test]
fn planet_commission_menu_renders_without_crashing_when_no_stardock_units_exist() {
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
fn planet_commission_draft_render_does_not_crash_when_picker_rows_disappear() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    for planet in &mut state.game_data.planets.records {
        for slot in 0..6 {
            planet.set_stardock_kind_raw(slot, 0);
            planet.set_stardock_count_raw(slot, 0);
        }
    }
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
    let mut terminal = CaptureTerminal::new();

    advance_to_main_menu(&mut app);
    app.current_screen = ScreenId::PlanetCommissionDraft;
    app.planet.commission_draft_rows = vec![PlanetCommissionDraftRow {
        direct_slot_0_based: None,
        kind: ProductionItemKind::Destroyer,
        unit_label: "Destroyers".to_string(),
        remaining_qty: 1,
        fleet_qty: 1,
    }];

    app.render(&mut terminal)
        .expect("commission draft render succeeds");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("DRAFT COMMISSION FLEET:"))
    );
}

#[test]
fn planet_commission_picker_render_returns_to_planet_menu_when_empty() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    for planet in &mut state.game_data.planets.records {
        for slot in 0..6 {
            planet.set_stardock_kind_raw(slot, 0);
            planet.set_stardock_count_raw(slot, 0);
        }
    }
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
    let mut terminal = CaptureTerminal::new();

    advance_to_main_menu(&mut app);
    app.current_screen = ScreenId::PlanetCommissionPicker;

    app.render(&mut terminal)
        .expect("empty commission picker should redirect");
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);
}

#[test]
fn planet_commission_uses_draft_for_ships_and_direct_result_for_starbases() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let owned_planets = state
        .game_data
        .planets
        .records
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner_empire_slot_raw() == 1)
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();
    let mut owned_planets = owned_planets;
    if owned_planets.len() < 2 {
        let extra_idx = state
            .game_data
            .planets
            .records
            .iter()
            .enumerate()
            .find(|(_, planet)| planet.owner_empire_slot_raw() != 1)
            .map(|(idx, _)| idx)
            .expect("fixture should have a spare planet");
        state.game_data.planets.records[extra_idx].set_owner_empire_slot_raw(1);
        state.game_data.planets.records[extra_idx].set_ownership_status_raw(1);
        owned_planets.push(extra_idx);
    }
    for &planet_idx in &owned_planets {
        let planet = &mut state.game_data.planets.records[planet_idx];
        for slot in 0..6 {
            planet.set_stardock_kind_raw(slot, 0);
            planet.set_stardock_count_raw(slot, 0);
        }
    }
    state.game_data.planets.records[owned_planets[0]].set_stardock_kind_raw(0, 1);
    state.game_data.planets.records[owned_planets[0]].set_stardock_count_raw(0, 2);
    state.game_data.planets.records[owned_planets[1]].set_stardock_kind_raw(0, 9);
    state.game_data.planets.records[owned_planets[1]].set_stardock_count_raw(0, 1);
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
    let mut terminal = CaptureTerminal::new();

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenCommissionMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionPicker);

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenCommissionPlanet)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionDraft);

    app.render(&mut terminal).expect("commission draft renders");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Set quantities for the ships you want in this fleet."))
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::AppendCommissionDraftChar('2'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitCommissionDraft)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionResult);
    assert!(
        app.planet
            .commission_result_notice
            .as_deref()
            .unwrap_or("")
            .contains("Fleet")
    );

    app.render(&mut terminal)
        .expect("commission result renders");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Notice: Commissioned selected ships into Fleet"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("(slap a key)"))
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::DismissCommissionResult(KeyCode::Enter))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionPicker);
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenCommissionPlanet)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionDraft);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitCommissionDraft)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionResult);
    assert!(
        app.planet
            .commission_result_notice
            .as_deref()
            .unwrap_or("")
            .contains("Starbase")
    );

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::DismissCommissionResult(KeyCode::Enter))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);
}

#[test]
fn planet_commission_draft_keeps_intermediate_success_inline() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let owned_planets = state
        .game_data
        .planets
        .records
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner_empire_slot_raw() == 1)
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();
    let owned_planet = owned_planets
        .first()
        .copied()
        .expect("fixture should have an owned planet");
    for &planet_idx in &owned_planets {
        let planet = &mut state.game_data.planets.records[planet_idx];
        for slot in 0..6 {
            planet.set_stardock_kind_raw(slot, 0);
            planet.set_stardock_count_raw(slot, 0);
        }
    }
    let planet = &mut state.game_data.planets.records[owned_planet];
    planet.set_stardock_kind_raw(0, 1);
    planet.set_stardock_count_raw(0, 5);
    planet.set_stardock_kind_raw(1, 3);
    planet.set_stardock_count_raw(1, 3);
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenCommissionMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenCommissionPlanet)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionDraft);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::AppendCommissionDraftChar('2'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::MoveCommissionDraftRow(1))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::AppendCommissionDraftChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitCommissionDraft)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionDraft);
    assert!(
        app.planet
            .commission_draft_notice
            .as_deref()
            .unwrap_or("")
            .contains("Fleet")
    );
    assert_eq!(app.planet.commission_draft_rows.len(), 2);
    assert_eq!(app.planet.commission_draft_rows[0].remaining_qty, 3);
    assert_eq!(app.planet.commission_draft_rows[1].remaining_qty, 2);
}

#[test]
fn planet_commission_result_latches_dismiss_key_until_release() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let owned_planets = state
        .game_data
        .planets
        .records
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner_empire_slot_raw() == 1)
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();
    let mut owned_planets = owned_planets;
    if owned_planets.len() < 2 {
        let extra_idx = state
            .game_data
            .planets
            .records
            .iter()
            .enumerate()
            .find(|(_, planet)| planet.owner_empire_slot_raw() != 1)
            .map(|(idx, _)| idx)
            .expect("fixture should have a spare planet");
        state.game_data.planets.records[extra_idx].set_owner_empire_slot_raw(1);
        state.game_data.planets.records[extra_idx].set_ownership_status_raw(1);
        owned_planets.push(extra_idx);
    }
    for &planet_idx in &owned_planets {
        let planet = &mut state.game_data.planets.records[planet_idx];
        for slot in 0..6 {
            planet.set_stardock_kind_raw(slot, 0);
            planet.set_stardock_count_raw(slot, 0);
        }
    }
    state.game_data.planets.records[owned_planets[0]].set_stardock_kind_raw(0, 1);
    state.game_data.planets.records[owned_planets[0]].set_stardock_count_raw(0, 2);
    state.game_data.planets.records[owned_planets[1]].set_stardock_kind_raw(0, 4);
    state.game_data.planets.records[owned_planets[1]].set_stardock_count_raw(0, 2);
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenCommissionMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionPicker);
    app.current_screen = ScreenId::PlanetCommissionResult;
    app.planet.commission_result_return_to_picker = true;
    app.planet.commission_result_notice =
        Some("Commissioned selected ships into Fleet 02.".to_string());

    let dismiss_press = app.handle_key(key_with_kind(KeyCode::Enter, KeyEventKind::Press));
    assert_eq!(apply_action(&mut app, dismiss_press), AppOutcome::Continue);
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionPicker);

    let repeat = app.handle_key(key_with_kind(KeyCode::Enter, KeyEventKind::Repeat));
    assert_eq!(apply_action(&mut app, repeat), AppOutcome::Continue);
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionPicker);

    let fresh_press = app.handle_key(key_with_kind(KeyCode::Enter, KeyEventKind::Press));
    assert_eq!(apply_action(&mut app, fresh_press), AppOutcome::Continue);
    assert_eq!(app.current_screen(), ScreenId::PlanetCommissionDraft);
}

#[test]
fn planet_build_menu_and_subscreens_render_without_crashing_when_no_owned_planets_exist() {
    let fixture_dir = temp_joined_no_assets_copy();
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildAbortPrompt)),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        "PLANET COMMAND <-H,Q,X,V,C,A,B,I,P,T,S,L,U->"
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
    assert!(terminal.lines[2].contains("┌"));
}

#[test]
fn command_menus_render_without_crashing_for_empty_empire_state() {
    let fixture_dir = temp_joined_empty_empire_copy();
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

    let mut terminal = CaptureTerminal::new();
    for action in [
        Action::Fleet(FleetAction::OpenMenu),
        Action::Fleet(FleetAction::OpenList),
        Action::Fleet(FleetAction::OpenReviewPrompt),
        Action::Fleet(FleetAction::OpenReview),
        Action::Fleet(FleetAction::OpenChangePrompt),
        Action::Fleet(FleetAction::OpenDetach),
        Action::Fleet(FleetAction::OpenEta),
        Action::Fleet(FleetAction::OpenTransportLoad),
        Action::Fleet(FleetAction::OpenTransportUnload),
        Action::Planet(PlanetAction::OpenMenu),
        Action::Planet(PlanetAction::OpenAutoCommissionPrompt),
        Action::Planet(PlanetAction::OpenCommissionMenu),
        Action::Planet(PlanetAction::OpenBuildMenu),
        Action::Planet(PlanetAction::OpenBuildReview),
        Action::Planet(PlanetAction::OpenBuildList),
        Action::Planet(PlanetAction::OpenBuildChange),
        Action::Planet(PlanetAction::OpenBuildAbortPrompt),
        Action::Planet(PlanetAction::OpenBuildSpecify),
        Action::Planet(PlanetAction::OpenTransportPlanetSelect(
            ec_game::screen::PlanetTransportMode::Load,
        )),
        Action::Planet(PlanetAction::OpenTransportPlanetSelect(
            ec_game::screen::PlanetTransportMode::Unload,
        )),
        Action::Planet(PlanetAction::OpenListSortPrompt(PlanetListMode::Brief)),
        Action::Planet(PlanetAction::OpenListSortPrompt(
            PlanetListMode::BuildSelect,
        )),
        Action::Planet(PlanetAction::SubmitListSort(
            PlanetListMode::Brief,
            PlanetListSort::Location,
        )),
        Action::Planet(PlanetAction::SubmitListSort(
            PlanetListMode::BuildSelect,
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
}

#[test]
fn planet_list_commands_stay_on_planet_menu_with_notice_when_no_owned_planets_exist() {
    let fixture_dir = temp_joined_no_assets_copy();
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
            Action::Planet(PlanetAction::SubmitListSort(
                PlanetListMode::Brief,
                PlanetListSort::CurrentProduction
            ))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);
}

#[test]
fn build_menu_planet_list_selects_build_target_and_returns_to_build_menu() {
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("build menu should render");
    let build_title = terminal.line(0).trim_end().to_string();
    assert_eq!(
        build_title,
        "BUILD ON CURRENT PLANET: \"Codex Prime\" IN SYSTEM [16,13]:"
    );

    assert_eq!(
        app.handle_key(key(KeyCode::Char('p'))),
        Action::Planet(PlanetAction::SubmitListSort(
            PlanetListMode::BuildSelect,
            PlanetListSort::CurrentProduction
        ))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitListSort(
                PlanetListMode::BuildSelect,
                PlanetListSort::CurrentProduction
            ))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetBriefList(
            PlanetListMode::BuildSelect,
            PlanetListSort::CurrentProduction
        )
    );

    assert_eq!(
        app.handle_key(key(KeyCode::Char('s'))),
        Action::Planet(PlanetAction::OpenListSortPrompt(
            PlanetListMode::BuildSelect
        ))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::OpenListSortPrompt(
                PlanetListMode::BuildSelect
            ))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetListSortPrompt(PlanetListMode::BuildSelect)
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Planet(PlanetAction::CloseListSortPrompt(
            PlanetListMode::BuildSelect
        ))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::CloseListSortPrompt(
                PlanetListMode::BuildSelect
            ))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.current_screen(),
        ScreenId::PlanetBriefList(
            PlanetListMode::BuildSelect,
            PlanetListSort::CurrentProduction
        )
    );

    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::Planet(PlanetAction::SubmitBriefInput)
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitBriefInput)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildMenu);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("build menu should render after selecting current planet");
    assert_eq!(terminal.line(0).trim_end(), build_title);

    assert_eq!(
        app.handle_key(key(KeyCode::Char('p'))),
        Action::Planet(PlanetAction::SubmitListSort(
            PlanetListMode::BuildSelect,
            PlanetListSort::CurrentProduction
        ))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitListSort(
                PlanetListMode::BuildSelect,
                PlanetListSort::CurrentProduction
            ))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Down)),
        Action::Planet(PlanetAction::MoveBrief(1))
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::MoveBrief(1))),
        AppOutcome::Continue
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        Action::Planet(PlanetAction::SubmitBriefInput)
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitBriefInput)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildMenu);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("build menu should render after choosing a new planet");
    let selected_title = terminal.line(0).trim_end().to_string();
    assert_ne!(selected_title, build_title);

    assert_eq!(
        app.handle_key(key(KeyCode::Char('p'))),
        Action::Planet(PlanetAction::SubmitListSort(
            PlanetListMode::BuildSelect,
            PlanetListSort::CurrentProduction
        ))
    );
    assert_eq!(
        apply_action(
            &mut app,
            Action::Planet(PlanetAction::SubmitListSort(
                PlanetListMode::BuildSelect,
                PlanetListSort::CurrentProduction
            ))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Planet(PlanetAction::OpenBuildMenu)
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenBuildMenu)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetBuildMenu);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("build menu should render after canceling build-select list");
    assert_eq!(terminal.line(0).trim_end(), selected_title);
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
    assert!(app.messaging.delete_reviewables_prompt_active);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("general menu should render inline delete prompt");
    assert!(line_containing(&terminal, "COMMAND <-").contains("COMMAND <- Y/[N] ->"));
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("DELETE ALL MESSAGES / RESULTS:"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("currently reviewable"))
    );

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
    assert!(!app.messaging.delete_reviewables_prompt_active);
    assert_eq!(app.current_screen(), ScreenId::GeneralMenu);
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
    let preview = ec_game::reports::ReportsPreview::from_block_rows(
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
fn fleet_review_opens_with_an_inline_prompt_first() {
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
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet menu prompt should render");
    let prompt = line_containing(&terminal, "FLEET COMMAND <- Review Fleet #");
    assert!(prompt.contains("Review Fleet # ["));
    assert!(prompt.contains("<Q> ->"));
}

#[test]
fn fleet_menu_prompts_default_to_the_most_powerful_fleet() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let strongest = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    strongest.set_battleship_count(9);
    save_runtime_state(&fixture_dir, &state);
    assert_eq!(strongest_owned_fleet_number(&fixture_dir), 2);

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReviewPrompt)),
        AppOutcome::Continue
    );
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("review prompt should render");
    assert!(
        line_containing(&terminal, "FLEET COMMAND <- Review Fleet #")
            .contains("Review Fleet # [2] <Q> ->")
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CancelMenuPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );
    app.render(&mut terminal)
        .expect("order prompt should render");
    assert!(
        line_containing(&terminal, "FLEET COMMAND <- Order Fleet #")
            .contains("Order Fleet # [2] <Q> ->")
    );
}

#[test]
fn fleet_review_prompt_accepts_typed_fleet_id_and_opens_that_fleet() {
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
    open_review_from_fleet_menu(&mut app, Some(1));
    assert_eq!(app.current_screen(), ScreenId::FleetReview);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet review detail should render");
    assert!(terminal.line(2).contains("Fleet ID: 1"));
}

#[test]
fn fleet_review_close_returns_to_prompt_for_the_current_fleet() {
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
    open_review_from_fleet_menu(&mut app, Some(2));
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::MoveReview(-1))),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CloseReview)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet menu prompt should render after closing review");
    let prompt = line_containing(&terminal, "FLEET COMMAND <- Review Fleet #");
    assert!(prompt.contains("Review Fleet # [3] <Q> ->"), "{prompt}");
}

#[test]
fn fleet_review_prompt_shows_invalid_fleet_message_on_unknown_typed_id() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReviewPrompt)),
        AppOutcome::Continue
    );
    for ch in ['9', '9'] {
        assert_eq!(
            apply_action(
                &mut app,
                Action::Fleet(FleetAction::AppendMenuPromptChar(ch))
            ),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMenuPrompt)),
        AppOutcome::Continue
    );
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet menu prompt should render invalid id notice");
    assert!(
        line_containing(&terminal, "FLEET COMMAND <- Review Fleet #").contains("Review Fleet #")
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Fleet #99 is not in your fleet list."))
    );
}

#[test]
fn fleet_menu_load_and_unload_keys_open_fleet_transport_flow() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();
    let fleet_one = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet_one.set_current_location_coords_raw(home_coords);
    fleet_one.set_battleship_count(0);
    fleet_one.set_cruiser_count(0);
    fleet_one.set_destroyer_count(0);
    fleet_one.set_troop_transport_count(5);
    fleet_one.set_army_count(1);
    fleet_one.set_scout_count(0);
    fleet_one.set_etac_count(0);
    fleet_one.recompute_max_speed_from_composition();

    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw(home_coords);
    fleet_two.set_battleship_count(4);
    fleet_two.set_cruiser_count(0);
    fleet_two.set_destroyer_count(0);
    fleet_two.set_troop_transport_count(3);
    fleet_two.set_army_count(3);
    fleet_two.set_scout_count(0);
    fleet_two.set_etac_count(0);
    fleet_two.recompute_max_speed_from_composition();
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
        app.handle_key(key(KeyCode::Char('l'))),
        Action::Fleet(FleetAction::OpenTransportLoad)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportLoad)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(app.fleet.menu_prompt_default_value, "1");

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet load prompt should render");
    let prompt = line_containing(&terminal, "FLEET COMMAND <- Load Fleet #");
    assert!(prompt.contains("Load Fleet # ["));
    assert!(prompt.contains("<Q> ->"));
    submit_fleet_menu_prompt(&mut app, Some(1));
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    app.render(&mut terminal)
        .expect("fleet load quantity prompt should render inline");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("LOAD ARMIES ONTO TROOP TRANSPORTS:"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Planet:") && line.contains("Fleet 01"))
    );
    let prompt = line_containing(&terminal, "FLEET COMMAND <- How many armies to load?");
    assert!(prompt.contains("How many armies to load? [4]"));
    assert!(prompt.contains("<Q> ->"));
    assert!(
        terminal
            .lines
            .iter()
            .all(|line| !line.contains("Load Planet XX,YY"))
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Fleet(FleetAction::CancelMenuPrompt)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CancelMenuPrompt)),
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
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(app.fleet.menu_prompt_default_value, "2");
    app.render(&mut terminal)
        .expect("fleet unload prompt should render");
    let prompt = line_containing(&terminal, "FLEET COMMAND <- Unload Fleet #");
    assert!(prompt.contains("Unload Fleet # ["));
    assert!(prompt.contains("<Q> ->"));
    submit_fleet_menu_prompt(&mut app, Some(2));
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    app.render(&mut terminal)
        .expect("fleet unload quantity prompt should render inline");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("UNLOAD ARMIES FROM TROOP TRANSPORTS:"))
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Planet:") && line.contains("Fleet 02"))
    );
    let prompt = line_containing(&terminal, "FLEET COMMAND <- How many armies to unload?");
    assert!(prompt.contains("How many armies to unload? [3]"));
    assert!(prompt.contains("<Q> ->"));
}

#[test]
fn fleet_transport_load_prompt_rejects_fleet_not_at_owned_world() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();
    let fleet_one = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet_one.set_current_location_coords_raw([1, 1]);
    fleet_one.set_troop_transport_count(4);
    fleet_one.set_army_count(0);
    fleet_one.recompute_max_speed_from_composition();
    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw(home_coords);
    fleet_two.set_troop_transport_count(2);
    fleet_two.set_army_count(0);
    fleet_two.recompute_max_speed_from_composition();
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportLoad)),
        AppOutcome::Continue
    );
    submit_fleet_menu_prompt(&mut app, Some(1));

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet load prompt should render owned-world warning");
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.fleet.menu_prompt_status.as_deref(),
        Some("That fleet is not at one of your worlds.")
    );
}

#[test]
fn fleet_transport_unload_prompt_rejects_fleet_not_at_owned_world() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();
    let fleet_one = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet_one.set_current_location_coords_raw([1, 1]);
    fleet_one.set_troop_transport_count(4);
    fleet_one.set_army_count(2);
    fleet_one.recompute_max_speed_from_composition();
    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw(home_coords);
    fleet_two.set_troop_transport_count(2);
    fleet_two.set_army_count(1);
    fleet_two.recompute_max_speed_from_composition();
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportUnload)),
        AppOutcome::Continue
    );
    submit_fleet_menu_prompt(&mut app, Some(1));

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet unload prompt should render owned-world warning");
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.fleet.menu_prompt_status.as_deref(),
        Some("That fleet is not at one of your worlds.")
    );
}

#[test]
fn fleet_transport_load_prompt_requires_armies_on_owned_world() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();
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
    state.game_data.planets.records[extra_owned_idx].set_army_count_raw(0);
    let other_coords = state.game_data.planets.records[extra_owned_idx].coords_raw();

    let fleet_one = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet_one.set_current_location_coords_raw(other_coords);
    fleet_one.set_troop_transport_count(4);
    fleet_one.set_army_count(0);
    fleet_one.recompute_max_speed_from_composition();
    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw(home_coords);
    fleet_two.set_troop_transport_count(2);
    fleet_two.set_army_count(0);
    fleet_two.recompute_max_speed_from_composition();
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportLoad)),
        AppOutcome::Continue
    );
    submit_fleet_menu_prompt(&mut app, Some(1));

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet load prompt should render no-armies warning");
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.fleet.menu_prompt_status.as_deref(),
        Some("That world has no armies available to load.")
    );
}

#[test]
fn fleet_transport_unload_prompt_requires_room_on_owned_world() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();
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
    state.game_data.planets.records[extra_owned_idx].set_army_count_raw(u8::MAX);
    let other_coords = state.game_data.planets.records[extra_owned_idx].coords_raw();

    let fleet_one = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet_one.set_current_location_coords_raw(other_coords);
    fleet_one.set_troop_transport_count(4);
    fleet_one.set_army_count(2);
    fleet_one.recompute_max_speed_from_composition();
    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw(home_coords);
    fleet_two.set_troop_transport_count(2);
    fleet_two.set_army_count(1);
    fleet_two.recompute_max_speed_from_composition();
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportUnload)),
        AppOutcome::Continue
    );
    submit_fleet_menu_prompt(&mut app, Some(1));

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet unload prompt should render no-room warning");
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(
        app.fleet.menu_prompt_status.as_deref(),
        Some("That world has no room to receive unloaded armies.")
    );
}

#[test]
fn fleet_menu_load_and_unload_show_menu_notice_when_no_transport_action_is_available() {
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
fn fleet_transport_quantity_prompt_stays_inline_on_fleet_menu() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportLoad)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet load prompt should render");
    submit_fleet_menu_prompt(&mut app, Some(1));
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    app.render(&mut terminal)
        .expect("fleet load quantity prompt should render");
    assert!(
        terminal
            .lines
            .iter()
            .all(|line| !line.contains("Load Planet XX,YY"))
    );
    assert!(
        line_containing(&terminal, "FLEET COMMAND <- How many armies to load?").contains("<Q> ->")
    );
}

#[test]
fn fleet_transport_load_prompt_rejects_fleet_already_full() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();
    state.game_data.planets.records[homeworld_index].set_army_count_raw(12);
    let fleet = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet.set_current_location_coords_raw(home_coords);
    fleet.set_troop_transport_count(4);
    fleet.set_army_count(4);
    fleet.recompute_max_speed_from_composition();
    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw(home_coords);
    fleet_two.set_troop_transport_count(2);
    fleet_two.set_army_count(0);
    fleet_two.recompute_max_speed_from_composition();
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportLoad)),
        AppOutcome::Continue
    );
    submit_fleet_menu_prompt(&mut app, Some(1));
    assert_eq!(
        app.fleet.menu_prompt_status.as_deref(),
        Some("That fleet's troop transports are already full.")
    );
}

#[test]
fn fleet_transport_unload_prompt_rejects_fleet_already_empty() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();
    let fleet = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet.set_current_location_coords_raw(home_coords);
    fleet.set_troop_transport_count(4);
    fleet.set_army_count(0);
    fleet.recompute_max_speed_from_composition();
    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw(home_coords);
    fleet_two.set_troop_transport_count(2);
    fleet_two.set_army_count(1);
    fleet_two.recompute_max_speed_from_composition();
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportUnload)),
        AppOutcome::Continue
    );
    submit_fleet_menu_prompt(&mut app, Some(1));
    assert_eq!(
        app.fleet.menu_prompt_status.as_deref(),
        Some("That fleet's troop transports are already empty.")
    );
}

#[test]
fn fleet_transport_load_default_skips_full_fleets_and_caps_qty_by_planet_armies() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    let home_planet = &mut state.game_data.planets.records[homeworld_index];
    let home_coords = home_planet.coords_raw();
    home_planet.set_army_count_raw(2);

    let fleet_one = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet_one.set_current_location_coords_raw(home_coords);
    fleet_one.set_troop_transport_count(6);
    fleet_one.set_army_count(6);
    fleet_one.recompute_max_speed_from_composition();

    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw(home_coords);
    fleet_two.set_troop_transport_count(5);
    fleet_two.set_army_count(1);
    fleet_two.recompute_max_speed_from_composition();

    let fleet_three = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 3)
        .expect("fleet #3 should exist");
    fleet_three.set_current_location_coords_raw(home_coords);
    fleet_three.set_troop_transport_count(6);
    fleet_three.set_army_count(0);
    fleet_three.recompute_max_speed_from_composition();

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportLoad)),
        AppOutcome::Continue
    );
    assert_eq!(app.fleet.menu_prompt_default_value, "3");
    submit_fleet_menu_prompt(&mut app, None);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet load quantity prompt should render");
    let prompt = line_containing(&terminal, "FLEET COMMAND <- How many armies to load?");
    assert!(prompt.contains("[2]"));
}

#[test]
fn fleet_transport_unload_default_skips_empty_fleets_and_caps_qty_by_planet_capacity() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let homeworld_index =
        state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    state.game_data.planets.records[homeworld_index].set_army_count_raw(253);
    let home_coords = state.game_data.planets.records[homeworld_index].coords_raw();

    let fleet_one = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet_one.set_current_location_coords_raw(home_coords);
    fleet_one.set_troop_transport_count(6);
    fleet_one.set_army_count(0);
    fleet_one.recompute_max_speed_from_composition();

    let fleet_two = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_current_location_coords_raw(home_coords);
    fleet_two.set_troop_transport_count(5);
    fleet_two.set_army_count(5);
    fleet_two.recompute_max_speed_from_composition();

    let fleet_three = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 3)
        .expect("fleet #3 should exist");
    fleet_three.set_current_location_coords_raw(home_coords);
    fleet_three.set_troop_transport_count(4);
    fleet_three.set_army_count(2);
    fleet_three.recompute_max_speed_from_composition();

    save_runtime_state(&fixture_dir, &state);

    let mut app = App::load(AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should reload");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportUnload)),
        AppOutcome::Continue
    );
    assert_eq!(app.fleet.menu_prompt_default_value, "2");
    submit_fleet_menu_prompt(&mut app, None);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet unload quantity prompt should render");
    let prompt = line_containing(&terminal, "FLEET COMMAND <- How many armies to unload?");
    assert!(prompt.contains("[2]"));
}

#[test]
fn fleet_menu_long_notice_wraps_instead_of_clipping() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenTransportUnload)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet menu should render wrapped notice");
    assert_eq!(
        terminal.lines[6].trim_end(),
        "FLEET COMMAND <-H,Q,X,V,S,F,R,E,C,I,D,T,O,G,M,L,U->"
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
        "FLEET COMMAND <-H,Q,X,V,S,F,R,E,C,I,D,T,O,G,M,L,U->"
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMerge)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("merge source prompt should render");
    assert!(
        line_containing(&terminal, "FLEET COMMAND <- Merge Fleet #").contains("Merge Fleet # [")
    );

    submit_fleet_menu_prompt(&mut app, Some(1));
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    app.render(&mut terminal)
        .expect("merge host prompt should render");
    assert!(line_containing(&terminal, "FLEET COMMAND <- Into Fleet #").contains("Into Fleet # ["));

    submit_fleet_menu_prompt(&mut app, Some(2));
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    app.render(&mut terminal)
        .expect("fleet menu should render merge success notice");
    assert_eq!(
        terminal.lines[6].trim_end(),
        "FLEET COMMAND <-H,Q,X,V,S,F,R,E,C,I,D,T,O,G,M,L,U->"
    );
    assert_eq!(terminal.lines[7].trim_end(), "");
    assert_eq!(terminal.lines[8].trim_end(), "");
    assert_eq!(terminal.lines[9].trim_end(), "");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("ordered to join Fleet #"))
    );

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
fn fleet_group_order_scrollbar_renders_just_right_of_table_border() {
    let rows = (1..=20)
        .map(|idx| FleetRow {
            fleet_record_index_1_based: idx,
            fleet_number: idx as u16,
            coords: [12, 6],
            target_coords: [12, 6],
            order_code: 3,
            current_speed: 0,
            max_speed: 3,
            eta_label: "0".to_string(),
            list_eta_label: "0".to_string(),
            rules_of_engagement: 6,
            order_label: "Patrol".to_string(),
            composition_label: "SC=1".to_string(),
            table_composition_label: "SC".to_string(),
        })
        .collect::<Vec<_>>();
    let mut screen = FleetGroupScreen::new();
    let buffer = screen
        .render(
            &rows,
            0,
            0,
            &BTreeSet::new(),
            FleetGroupOrderMode::SelectingFleets,
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            3015,
            None,
        )
        .expect("group fleet order screen should render");
    let mut terminal = CaptureTerminal::new();
    terminal
        .render(&buffer)
        .expect("captured group fleet order screen should render");

    let right_border_col = terminal
        .line(3)
        .chars()
        .position(|ch| ch == '┐')
        .expect("group order table should have a right border");
    let scrollbar_col = right_border_col + 1;
    let char_at = |line: &str| line.chars().nth(scrollbar_col);
    assert!(terminal.lines.iter().any(|line| char_at(line) == Some('^')));
    assert!(terminal.lines.iter().any(|line| char_at(line) == Some('#')));
    assert!(terminal.lines.iter().any(|line| char_at(line) == Some('v')));
}

#[test]
fn fleet_group_order_opens_mission_picker_and_q_returns_to_group_table() {
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
    let border = terminal.line(1);
    let left_padding = border
        .find('┌')
        .expect("mission picker border should render");
    assert!(left_padding > 0, "mission picker table should be centered");
    assert!(border.trim_end().chars().count() < 80);
    assert_eq!(
        terminal.line(0).find("FLEET MISSION ORDERS:"),
        Some(left_padding)
    );
    assert!(terminal.line(2).contains("No."));
    assert!(terminal.lines.iter().any(|line| line.contains("15")));
    let prompt = line_containing(&terminal, "COMMANDS <ARROWS J K Q> [");
    assert_eq!(prompt.find("COMMANDS"), Some(left_padding));
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
fn fleet_order_prompt_opens_mission_picker_and_q_returns_to_order_prompt() {
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
        app.handle_key(key(KeyCode::Char('o'))),
        Action::Fleet(FleetAction::OpenOrder)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenOrder)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet order prompt should render");
    let prompt = line_containing(&terminal, "FLEET COMMAND <- Order Fleet #");
    assert!(prompt.contains("Order Fleet # ["));
    assert!(prompt.contains("<Q> ->"));

    submit_fleet_menu_prompt(&mut app, Some(2));
    assert_eq!(app.current_screen(), ScreenId::FleetMissionPicker);
    assert_eq!(
        app.handle_key(key(KeyCode::PageDown)),
        Action::Fleet(FleetAction::MoveMissionPicker(8))
    );
    app.render(&mut terminal)
        .expect("fleet mission picker should render");
    let border = terminal.line(1);
    let left_padding = border
        .find('┌')
        .expect("mission picker border should render");
    assert_eq!(
        terminal.line(0).find("FLEET MISSION ORDERS:"),
        Some(left_padding)
    );
    let prompt = line_containing(&terminal, "COMMANDS <ARROWS J K Q> [");
    assert_eq!(prompt.find("COMMANDS"), Some(left_padding));
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
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    app.render(&mut terminal)
        .expect("fleet order prompt should render after cancel");
    assert!(line_containing(&terminal, "FLEET COMMAND <- Order Fleet #").contains("[2]"));
}

#[test]
fn fleet_order_applies_move_order_to_selected_fleet_only() {
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
    open_order_mission_picker_from_fleet_menu(&mut app, Some(2));
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
    enter_fleet_order_target(&mut app, [14, 9]);
    confirm_fleet_order(&mut app, true);
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet menu should render success notice");
    assert!(line_containing(&terminal, "FLEET COMMAND <- Order Fleet #").contains("[2]"));
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Applied move to Fleet #2 for sector [14,9]."))
    );

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
fn fit_table_columns_keeps_header_width_for_blank_cells() {
    let columns = [TableColumn::right("ROE", 1), TableColumn::left("Status", 1)];
    let rows = vec![
        vec!["".to_string(), "".to_string()],
        vec!["".to_string(), "OK".to_string()],
    ];

    let fitted = fit_table_columns(&columns, &rows);

    assert_eq!(fitted[0].width, "ROE".len());
    assert_eq!(fitted[1].width, "Status".len());
}

#[test]
fn fleet_mission_requirements_match_manual_summary_table() {
    let expected = [
        (0, "Any ships", FleetMissionRequirement::Any),
        (1, "Any", FleetMissionRequirement::Any),
        (2, "Any", FleetMissionRequirement::Any),
        (3, "Any", FleetMissionRequirement::Any),
        (4, "Combat ships", FleetMissionRequirement::CombatShips),
        (5, "Combat ships", FleetMissionRequirement::CombatShips),
        (6, "Combat ships", FleetMissionRequirement::CombatShips),
        (
            7,
            "Combat + loaded transports",
            FleetMissionRequirement::CombatAndLoadedTransports,
        ),
        (
            8,
            "Loaded transports (combat recommended)",
            FleetMissionRequirement::LoadedTransports,
        ),
        (9, "Any", FleetMissionRequirement::Any),
        (
            10,
            "At least one scout",
            FleetMissionRequirement::AtLeastOneScout,
        ),
        (
            11,
            "At least one scout",
            FleetMissionRequirement::AtLeastOneScout,
        ),
        (
            12,
            "At least one ETAC",
            FleetMissionRequirement::AtLeastOneEtac,
        ),
        (13, "Any", FleetMissionRequirement::Any),
        (14, "Any", FleetMissionRequirement::Any),
        (15, "Any", FleetMissionRequirement::Any),
    ];

    for (option, (code, requirements, requirement)) in
        FLEET_MISSION_OPTIONS.iter().zip(expected.into_iter())
    {
        assert_eq!(option.code, code);
        assert_eq!(option.requirements, requirements);
        assert_eq!(option.requirement, requirement);
    }
}

#[test]
fn fleet_record_supports_manual_requirement_classes() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let fleet = state
        .game_data
        .fleets
        .records
        .get_mut(0)
        .expect("fleet 1 should exist");

    fleet.set_battleship_count(0);
    fleet.set_cruiser_count(0);
    fleet.set_destroyer_count(0);
    fleet.set_army_count(0);
    fleet.set_scout_count(0);
    fleet.set_etac_count(0);

    assert!(fleet_record_supports_mission_code(fleet, 0));
    assert!(fleet_record_supports_mission_code(fleet, 15));
    assert!(!fleet_record_supports_mission_code(fleet, 4));
    assert!(!fleet_record_supports_mission_code(fleet, 8));
    assert!(!fleet_record_supports_mission_code(fleet, 10));
    assert!(!fleet_record_supports_mission_code(fleet, 12));

    fleet.set_destroyer_count(1);
    assert!(fleet_record_supports_mission_code(fleet, 4));
    assert!(fleet_record_supports_mission_code(fleet, 6));
    assert!(!fleet_record_supports_mission_code(fleet, 7));

    fleet.set_army_count(1);
    assert!(fleet_record_supports_mission_code(fleet, 7));
    assert!(fleet_record_supports_mission_code(fleet, 8));

    fleet.set_destroyer_count(0);
    assert!(!fleet_record_supports_mission_code(fleet, 7));
    assert!(fleet_record_supports_mission_code(fleet, 8));

    fleet.set_scout_count(1);
    assert!(fleet_record_supports_mission_code(fleet, 10));
    assert!(fleet_record_supports_mission_code(fleet, 11));

    fleet.set_etac_count(1);
    assert!(fleet_record_supports_mission_code(fleet, 12));
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
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(2));
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
    assert!(line_containing(&terminal, "Location: ").contains("Location: ("));
    assert!(line_containing(&terminal, "Current / Max Speed: ").contains("Current / Max Speed: "));
    assert!(line_containing(&terminal, "ROE: ").contains("ROE: "));
    assert!(line_containing(&terminal, "Order: ").contains("Order: "));
    assert!(line_containing(&terminal, "Ships: ").contains("Ships: "));
    assert!(
        line_containing(&terminal, "Enter the starbase number for Guard a Starbase.")
            .contains("Enter the starbase number for Guard a Starbase.")
    );
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("New Orders: "))
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
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(2));
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
    assert_eq!(app.current_screen(), ScreenId::FleetMissionPicker);

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
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(1));
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
    assert!(line_containing(&terminal, "Location: ").contains("Location: ("));
    assert!(line_containing(&terminal, "Current / Max Speed: ").contains("Current / Max Speed: "));
    assert!(line_containing(&terminal, "ROE: ").contains("ROE: "));
    assert!(line_containing(&terminal, "Order: ").contains("Order: "));
    assert!(line_containing(&terminal, "Ships: ").contains("Ships: "));
    assert!(
        line_containing(
            &terminal,
            "Enter the host fleet number for Join another fleet."
        )
        .contains("Enter the host fleet number for Join another fleet.")
    );
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("New Orders: "))
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
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(2));
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
    enter_fleet_order_target(&mut app, [14, 9]);
    confirm_fleet_order(&mut app, true);
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

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
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("reloaded app should load");
    advance_to_main_menu(&mut reloaded);
    assert_eq!(
        apply_action(&mut reloaded, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut reloaded, Action::Fleet(FleetAction::OpenList)),
        AppOutcome::Continue
    );
    let mut terminal = CaptureTerminal::new();
    reloaded
        .render(&mut terminal)
        .expect("reloaded fleet list should render");
    let table_text = (3..16)
        .map(|row| terminal.line(row).to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(table_text.contains("(14,09)"));
}

#[test]
fn fleet_order_screen_uses_compact_summary_and_eta_confirm() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let bombard_target = state
        .game_data
        .planets
        .records
        .iter()
        .find(|planet| planet.owner_empire_slot_raw() != 1)
        .expect("fixture should have a foreign world")
        .coords_raw();
    state.game_data.fleets.records[1].set_standing_order_code_raw(5);
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
    open_order_mission_picker_from_fleet_menu(&mut app, Some(2));
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

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("compact fleet order screen should render");
    assert!(line_containing(&terminal, "Location: ").contains("Location: ("));
    assert!(line_containing(&terminal, "Current / Max Speed: ").contains("Current / Max Speed: "));
    assert!(line_containing(&terminal, "ROE: ").contains("ROE: "));
    assert!(line_containing(&terminal, "Order: ").contains("Order: "));
    assert!(line_containing(&terminal, "Ships: ").contains("Ships: "));
    assert!(
        line_containing(&terminal, "Enter target coordinates for new order: ")
            .contains("Enter target coordinates for new order: Bombard")
    );
    let prompt = line_containing(&terminal, "Target XX ");
    assert!(prompt.contains("FLEET COMMAND <- Target XX "));
    assert!(!prompt.contains('['));
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("New Orders: "))
    );
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Selected mission:"))
    );
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Standing order:"))
    );

    enter_fleet_order_target(&mut app, bombard_target);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet order confirm should render");
    assert!(
        line_containing(&terminal, "Stardate: ")
            .contains(&format!("Stardate: {}", app.game_data.conquest.game_year()))
    );
    assert!(line_containing(&terminal, "Confirm [Y]/N").contains("Confirm [Y]/N"));
    assert!(line_containing(&terminal, "New Orders: ").contains("New Orders: Bombard"));
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("arriving in"))
    );
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Location: "))
    );
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Current / Max Speed: "))
    );
    assert!(!terminal.lines.iter().any(|line| line.contains("ROE: ")));
    assert!(!terminal.lines.iter().any(|line| line.contains("Order: ")));
    assert!(!terminal.lines.iter().any(|line| line.contains("Ships: ")));
}

#[test]
fn fleet_group_order_uses_compact_summary_and_eta_confirm() {
    let fixture_dir = temp_game_copy();
    let bombard_target = latest_runtime_state(&fixture_dir)
        .game_data
        .planets
        .records
        .iter()
        .find(|planet| planet.owner_empire_slot_raw() != 1)
        .expect("fixture should have a foreign world")
        .coords_raw();

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

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("compact fleet group target screen should render");
    let selected = line_containing(&terminal, "Selected fleets: ")
        .trim()
        .strip_prefix("Selected fleets: ")
        .expect("selected fleets line should have prefix");
    assert_eq!(selected.split(", ").count(), 1);
    assert!(selected.len() >= 2);
    assert!(selected.chars().all(|ch| ch.is_ascii_digit()));
    assert!(
        line_containing(&terminal, "Enter target coordinates for new order: ")
            .contains("Enter target coordinates for new order: Bombard")
    );
    let prompt = line_containing(&terminal, "Target XX ");
    assert!(prompt.contains("FLEET COMMAND <- Target XX "));
    assert!(!prompt.contains('['));
    assert!(!terminal.lines.iter().any(|line| line.contains("│Sel│")));
    assert!(!terminal.lines.iter().any(|line| line.contains('│')));
    assert!(!terminal.lines.iter().any(|line| line.contains('┌')));

    enter_fleet_group_order_target(&mut app, bombard_target);
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("compact fleet group confirm should render");
    assert!(
        line_containing(&terminal, "Stardate: ")
            .contains(&format!("Stardate: {}", app.game_data.conquest.game_year()))
    );
    let selected = line_containing(&terminal, "Selected fleets: ")
        .trim()
        .strip_prefix("Selected fleets: ")
        .expect("selected fleets line should have prefix");
    assert_eq!(selected.split(", ").count(), 1);
    assert!(selected.len() >= 2);
    assert!(selected.chars().all(|ch| ch.is_ascii_digit()));
    assert!(line_containing(&terminal, "New Orders: ").contains("New Orders: Bombard"));
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("arriving in"))
    );
    assert!(line_containing(&terminal, "Confirm [Y]/N").contains("Confirm [Y]/N"));
    assert!(!terminal.lines.iter().any(|line| line.contains("│Sel│")));
    assert!(!terminal.lines.iter().any(|line| line.contains('│')));
    assert!(!terminal.lines.iter().any(|line| line.contains('┌')));
}

#[test]
fn fleet_group_order_lists_selected_fleet_numbers_in_compact_target_entry() {
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
        .expect("compact fleet group target screen should render");
    let selected = line_containing(&terminal, "Selected fleets: ")
        .trim()
        .strip_prefix("Selected fleets: ")
        .expect("selected fleets line should have prefix");
    let parts = selected.split(", ").collect::<Vec<_>>();
    assert_eq!(parts.len(), 2);
    assert!(
        parts
            .iter()
            .all(|part| part.len() >= 2 && part.chars().all(|ch| ch.is_ascii_digit()))
    );
}

#[test]
fn fleet_order_scout_system_defaults_avoid_worlds_targeted_by_other_friendly_scouts() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let viewer_index = 0usize;
    let home_coords = state.game_data.planets.records
        [state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1]
        .coords_raw();
    let mut candidates = state
        .game_data
        .planets
        .records
        .iter()
        .enumerate()
        .filter(|(_, planet)| planet.owner_empire_slot_raw() != 1)
        .map(|(idx, planet)| (idx, planet.coords_raw()))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(_, coords)| {
        let dx = i32::from(home_coords[0]) - i32::from(coords[0]);
        let dy = i32::from(home_coords[1]) - i32::from(coords[1]);
        dx * dx + dy * dy
    });
    let claimed_coords = candidates[0].1;
    let fallback_coords = candidates[1].1;
    state.game_data.planets.records[candidates[0].0].set_owner_empire_slot_raw(2);
    state.game_data.planets.records[candidates[1].0].set_owner_empire_slot_raw(2);
    for fleet in state.game_data.fleets.records.iter_mut() {
        if fleet.owner_empire_raw() == 1 {
            fleet.set_standing_order_code_raw(9);
        }
    }
    let selected_fleet_number = state.game_data.fleets.records[0].local_slot_word_raw();
    state.game_data.fleets.records[0].set_scout_count(1);
    state.game_data.fleets.records[0].set_standing_order_code_raw(0);
    state.game_data.fleets.records[1].set_scout_count(1);
    state.game_data.fleets.records[1].set_standing_order_kind(ec_data::Order::ScoutSolarSystem);
    state.game_data.fleets.records[1].set_standing_order_target_coords_raw(claimed_coords);
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
    planet_intel_by_viewer[viewer_index].clear();
    planet_intel_by_viewer[viewer_index].insert(
        candidates[0].0 + 1,
        partial_known_world_snapshot(
            candidates[0].0 + 1,
            &state.game_data.planets.records[candidates[0].0],
            2,
            year,
        ),
    );
    planet_intel_by_viewer[viewer_index].insert(
        candidates[1].0 + 1,
        partial_known_world_snapshot(
            candidates[1].0 + 1,
            &state.game_data.planets.records[candidates[1].0],
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
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(selected_fleet_number));
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
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("scout system target prompt should render");
    assert!(
        line_containing(&terminal, "Target XX [")
            .contains(&format!("Target XX [{:02}] <Q> ->", fallback_coords[0]))
    );
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
    assert!(
        line_containing(&terminal, "Target XX [")
            .contains(&format!("Target XX [{:02}] <Q> ->", closest_coords[0]))
    );
}

#[test]
fn fleet_group_colonize_mission_skips_worlds_claimed_by_other_friendly_etacs() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    let viewer_index = 0usize;
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
        unowned_candidates[0].0 + 1,
        partial_known_world_snapshot(
            unowned_candidates[0].0 + 1,
            &state.game_data.planets.records[unowned_candidates[0].0],
            0,
            year,
        ),
    );
    planet_intel_by_viewer[viewer_index].insert(
        unowned_candidates[1].0 + 1,
        partial_known_world_snapshot(
            unowned_candidates[1].0 + 1,
            &state.game_data.planets.records[unowned_candidates[1].0],
            0,
            year,
        ),
    );
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
    save_runtime_state_with_intel(&fixture_dir, &state, &planet_intel_by_viewer);

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
    assert!(
        line_containing(&terminal, "Target XX [")
            .contains(&format!("Target XX [{:02}] <Q> ->", fallback_coords[0]))
    );
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
    let _hidden_colonized_coords = candidates[0].1;
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
    let prompt = line_containing(&terminal, "Target XX ");
    assert!(prompt.contains("Target XX "));
    assert!(!prompt.contains('['));
}

#[test]
fn fleet_mission_picker_rejects_missions_not_supported_by_all_selected_fleets() {
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
    assert!(line_containing(&terminal, "COMMANDS <ARROWS J K Q>").contains("COMMANDS"));
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("That mission does not apply to all selected fleets."))
    );
}

#[test]
fn fleet_group_order_rejects_empty_sector_for_world_targeting_mission() {
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
    enter_fleet_group_order_target(&mut app, [1, 1]);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("world-target validation should render");
    assert!(terminal.lines.iter().any(|line| {
        line.contains("That mission requires a system with a planet at the target coordinates.")
    }));
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);
    let prompt = line_containing(&terminal, "Target XX ");
    assert!(prompt.contains("Target XX "));
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Target YY "))
    );
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
    enter_fleet_group_order_target(&mut app, owned_target);
    confirm_fleet_group_order(&mut app, true);

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
fn fleet_order_blockade_mission_defaults_to_closest_owned_planet() {
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
        .expect("fixture should have a non-owned world");
    state.game_data.planets.records[target_idx].set_owner_empire_slot_raw(1);
    let owned_target = state.game_data.planets.records[target_idx].coords_raw();
    let selected_fleet = state
        .game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("player 1 fleet #1 should exist");
    selected_fleet.set_current_location_coords_raw(owned_target);
    selected_fleet.set_standing_order_target_coords_raw(owned_target);
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
    open_order_mission_picker_from_fleet_menu(&mut app, Some(1));
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
        .expect("blockade target prompt should render");
    assert!(
        line_containing(&terminal, "Target XX [")
            .contains(&format!("Target XX [{:02}] <Q> ->", owned_target[0]))
    );
}

#[test]
fn fleet_group_order_rejects_owned_planet_for_scout_mission() {
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
    enter_fleet_group_order_target(&mut app, owned_target);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("owned scout target rejection should render");
    assert!(terminal.lines.iter().any(|line| {
        line.contains("You cannot order scouts to target your own planet or system.")
    }));
    let prompt = line_containing(&terminal, "Target XX ");
    assert!(prompt.contains("Target XX "));
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Target YY "))
    );
}

#[test]
fn fleet_order_rejects_owned_planet_for_scout_system_mission() {
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
    let owned_target = state.game_data.planets.records
        [state.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1]
        .coords_raw();
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
    open_order_mission_picker_from_fleet_menu(&mut app, Some(1));
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
            Action::Fleet(FleetAction::AppendMissionPickerChar('1'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );
    enter_fleet_order_target(&mut app, owned_target);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("owned scout-system target rejection should render");
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| { line.contains("You cannot scout your own planet or system.") })
    );
    let prompt = line_containing(&terminal, "Target XX ");
    assert!(prompt.contains("Target XX "));
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Target YY "))
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
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(1));
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
    assert!(
        line_containing(&terminal, "Target XX [")
            .contains(&format!("Target XX [{:02}] <Q> ->", nearest_owned[0]))
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitOrder)),
        AppOutcome::Continue
    );
    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("salvage target y prompt should render");
    assert!(
        line_containing(&terminal, "Target YY [")
            .contains(&format!("Target YY [{:02}] <Q> ->", nearest_owned[1]))
    );
}

#[test]
fn fleet_order_salvage_rejects_empty_sector_target() {
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
    open_order_mission_picker_from_fleet_menu(&mut app, Some(1));
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
    enter_fleet_order_target(&mut app, [1, 1]);

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
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(1));
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
    enter_fleet_order_target(&mut app, foreign_target);

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
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");
    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_order_mission_picker_from_fleet_menu(&mut app, Some(1));
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
    enter_fleet_order_target(&mut app, unowned_target);

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
fn fleet_group_order_allows_manual_combat_target_without_known_enemy_world() {
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
    let prompt = line_containing(&terminal, "Target XX [");
    assert!(prompt.contains("Target XX ["));
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
    let prompt = line_containing(&terminal, "Target XX ");
    assert!(prompt.contains("Target XX "));
    assert!(!prompt.contains('['));
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
    enter_fleet_group_order_target(&mut app, [10, 13]);
    confirm_fleet_group_order(&mut app, true);
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet group order should render success notice");
    assert!(line_containing(&terminal, "COMMANDS <ARROWS J K SPACE Q>").contains("COMMANDS"));
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Applied move order to"))
    );

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
    let selected = line_containing(&terminal, "Selected fleets: ")
        .trim()
        .strip_prefix("Selected fleets: ")
        .expect("selected fleets line should have prefix");
    assert_eq!(selected.split(", ").count(), 1);
    assert!(selected.len() >= 2);
    assert!(selected.chars().all(|ch| ch.is_ascii_digit()));
    assert!(
        line_containing(
            &terminal,
            "Enter the host fleet number for Join another fleet."
        )
        .contains("Enter the host fleet number for Join another fleet.")
    );
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Enter target for new order: Join fleet"))
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
fn fleet_group_guard_starbase_target_prompt_uses_named_target_layout() {
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
            Action::Fleet(FleetAction::AppendMissionPickerChar('4'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMissionPicker)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("fleet group guard-starbase target prompt should render");
    assert_eq!(app.current_screen(), ScreenId::FleetGroupOrder);
    let selected = line_containing(&terminal, "Selected fleets: ")
        .trim()
        .strip_prefix("Selected fleets: ")
        .expect("selected fleets line should have prefix");
    assert_eq!(selected.split(", ").count(), 1);
    assert!(selected.len() >= 2);
    assert!(selected.chars().all(|ch| ch.is_ascii_digit()));
    assert!(
        line_containing(&terminal, "Enter the starbase number for Guard a Starbase.")
            .contains("Enter the starbase number for Guard a Starbase.")
    );
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Enter target for new order: Guard starbase"))
    );
    assert!(
        line_containing(&terminal, "Starbase # [").contains("Starbase # ["),
        "{}",
        line_containing(&terminal, "Starbase # [")
    );
}

#[test]
fn fleet_change_roe_accepts_typed_fleet_selection_and_q_cancels_prompt() {
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
    open_change_value_prompt_from_fleet_menu(&mut app, Some(4), 'R');
    assert_eq!(
        app.handle_key(key(KeyCode::Char('q'))),
        Action::Fleet(FleetAction::CancelMenuPrompt)
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::CancelMenuPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);

    open_change_value_prompt_from_fleet_menu(&mut app, Some(4), 'R');
    submit_fleet_menu_prompt_value(&mut app, "7");
    assert_eq!(app.current_fleet_roe_by_id(4), Some(7));
    assert_eq!(app.current_fleet_roe_by_id(1), Some(6));
}

#[test]
fn fleet_change_roe_empty_enter_accepts_displayed_default() {
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
    open_change_value_prompt_from_fleet_menu(&mut app, Some(4), 'R');
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::SubmitMenuPrompt)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(app.current_fleet_roe_by_id(4), Some(6));
}

#[test]
fn fleet_change_success_returns_to_menu_with_notice() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_change_value_prompt_from_fleet_menu(&mut app, Some(4), 'R');
    submit_fleet_menu_prompt_value(&mut app, "9");

    app.render(&mut terminal).expect("render succeeds");
    assert!(
        line_containing(&terminal, "Fleet #4 ROE set to 9.").contains("Fleet #4 ROE set to 9.")
    );
}

#[test]
fn fleet_change_id_updates_visible_fleet_number_inline() {
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
    let mut terminal = CaptureTerminal::new();

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_change_value_prompt_from_fleet_menu(&mut app, Some(4), 'I');
    submit_fleet_menu_prompt_value(&mut app, "12");

    app.render(&mut terminal).expect("fleet menu should render");
    assert!(
        line_containing(&terminal, "Fleet #4 renumbered to Fleet #12.")
            .contains("Fleet #4 renumbered to Fleet #12.")
    );

    let state = latest_runtime_state(&fixture_dir);
    assert!(
        state
            .game_data
            .fleets
            .records
            .iter()
            .any(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 12)
    );
}

#[test]
fn fleet_change_id_rejects_duplicate_fleet_number_inline() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_change_value_prompt_from_fleet_menu(&mut app, Some(4), 'I');
    submit_fleet_menu_prompt_value(&mut app, "1");

    app.render(&mut terminal)
        .expect("change prompt should render");
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert!(
        line_containing(&terminal, "Fleet ID is already in use.")
            .contains("Fleet ID is already in use.")
    );
    assert!(
        line_containing(&terminal, "FLEET COMMAND <- New Fleet ID")
            .contains("New Fleet ID [4] <Q> ->")
    );
}

#[test]
fn fleet_change_speed_updates_current_speed_inline() {
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
    let mut terminal = CaptureTerminal::new();

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_change_value_prompt_from_fleet_menu(&mut app, Some(4), 'S');
    submit_fleet_menu_prompt_value(&mut app, "0");

    app.render(&mut terminal).expect("fleet menu should render");
    assert!(
        line_containing(&terminal, "Fleet #4 speed set to 0.").contains("Fleet #4 speed set to 0.")
    );

    let state = latest_runtime_state(&fixture_dir);
    let fleet = state
        .game_data
        .fleets
        .records
        .iter()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 4)
        .expect("fleet #4 should exist");
    assert_eq!(fleet.current_speed(), 0);
}

#[test]
fn planet_database_render_uses_classic_stacked_headers() {
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenDatabase)),
        AppOutcome::Continue
    );

    app.render(&mut terminal).expect("render succeeds");
    let title_col = terminal
        .line(0)
        .find("TOTAL PLANET DATABASE:")
        .expect("title col");
    let border_col = terminal.line(1).find('┌').expect("table col");
    assert_eq!(title_col, border_col);
    assert!(terminal.line(2).contains("│Coord"));
    assert!(terminal.line(2).contains("Max"));
    assert!(terminal.line(2).contains("Year"));
    assert!(terminal.line(2).contains("Curr"));
    assert!(terminal.line(2).contains("Stored"));
    assert_eq!(terminal.line(2).matches('│').count(), 12);
    assert!(terminal.line(3).contains("(XX,YY)"));
    assert!(terminal.line(3).contains("Planet Name"));
    assert!(terminal.line(3).contains("Prod"));
    assert!(terminal.line(3).contains("Seen"));
    assert!(terminal.line(3).contains("Scout"));
    assert!(terminal.line(3).contains("ARs"));
    assert!(terminal.line(3).contains("GBs"));
    assert!(terminal.line(3).contains("SBs"));
    assert!(!terminal.line(3).contains("Intel"));
    assert!(terminal.lines.iter().any(|line| line.contains("3000")));
    let prompt = line_containing(&terminal, "COMMANDS <");
    assert_eq!(prompt.find("COMMANDS").expect("commands col"), border_col);
    assert!(prompt.contains("["));
    assert!(prompt.contains("->"));
}

#[test]
fn starmap_dump_page_uses_plain_bottom_left_slap_a_key_prompt() {
    let mut screen = ec_game::screen::StarmapScreen::new();
    let lines = (1..=21)
        .map(|idx| format!("line {idx:02}"))
        .collect::<Vec<_>>();

    let buffer = screen
        .render_dump_page(&lines, 0)
        .expect("starmap dump page renders");

    assert_eq!(buffer.plain_line(22), "line 21");
    assert_eq!(buffer.plain_line(24), "(slap a key)");
    assert!(!buffer.plain_line(24).contains("GALAXY MAP"));
    assert!(!buffer.plain_line(24).contains("->"));
    assert!(!buffer.plain_line(24).contains("<-"));
}

#[test]
fn starmap_prompt_uses_plain_dismiss_prompt_below_last_text_line() {
    let mut screen = ec_game::screen::StarmapScreen::new();

    let buffer = screen.render_prompt(None).expect("starmap prompt renders");

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
        ec_data::PlanetIntelSnapshot {
            planet_record_index_1_based: planet_idx + 1,
            intel_tier: ec_data::IntelTier::Full,
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
            .any(|line| line.contains("Last Viewed/Scouted: "))
    );
    assert!(terminal.lines.iter().any(|line| line.contains("Y3000")));
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Intel Tier: "))
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
        "COMMAND <- Planet coords [16,13] <Q> ->"
    );
    assert_eq!(terminal.line(7).trim_end(), "");
    assert_eq!(
        line_containing(&terminal, "Enter coordinates of the planet to view.").trim_end(),
        "Enter coordinates of the planet to view."
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
        "COMMAND <- Planet coords [16,13] <Q> -> 99,99"
    );
    assert_eq!(
        line_containing(&terminal, "Enter coordinates of the planet to view.").trim_end(),
        "Enter coordinates of the planet to view."
    );
    assert_eq!(terminal.line(9).trim_end(), "");
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
        "COMMAND <- Planet coords [16,13] <Q> ->"
    );
    assert_eq!(
        line_containing(&terminal, "Enter coordinates of the planet to view.").trim_end(),
        "Enter coordinates of the planet to view."
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
fn planet_menu_tax_prompt_renders_inline_command_and_warning_stack() {
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
    let current_tax = app.game_data.player.records[app.player.record_index_1_based - 1].tax_rate();

    advance_to_main_menu(&mut app);
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenTaxPrompt)),
        AppOutcome::Continue
    );

    app.render(&mut terminal).expect("render succeeds");
    assert_eq!(
        line_containing(&terminal, "PLANET COMMAND <- Empire tax rate").trim_end(),
        format!("PLANET COMMAND <- Empire tax rate (0 - 100) [{current_tax}] <Q> ->")
    );
    assert_eq!(terminal.line(6).trim_end(), "");
    assert_eq!(
        line_containing(&terminal, "PLANET TAX: ").trim_end(),
        "PLANET TAX: Set empire tax rate."
    );
    assert!(
        line_containing(&terminal, "Warning: ")
            .contains("Taxes in excess of 65% may actually REDUCE"),
        "expected inline tax warning block below the helper message"
    );
}

#[test]
fn planet_menu_tax_prompt_stays_inline_for_errors_and_success() {
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
        apply_action(&mut app, Action::Planet(PlanetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::OpenTaxPrompt)),
        AppOutcome::Continue
    );
    for ch in ['9', '9', '9'] {
        assert_eq!(
            apply_action(&mut app, Action::Planet(PlanetAction::AppendTaxChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitTax)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);

    app.render(&mut terminal).expect("render succeeds");
    assert!(
        line_containing(&terminal, "Error: ").contains("Enter an integer tax rate from 0 to 100."),
        "expected inline tax validation error"
    );

    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::BackspaceTaxInput)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::BackspaceTaxInput)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::BackspaceTaxInput)),
        AppOutcome::Continue
    );
    for ch in ['6', '5'] {
        assert_eq!(
            apply_action(&mut app, Action::Planet(PlanetAction::AppendTaxChar(ch))),
            AppOutcome::Continue
        );
    }
    assert_eq!(
        apply_action(&mut app, Action::Planet(PlanetAction::SubmitTax)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::PlanetMenu);

    app.render(&mut terminal).expect("render succeeds");
    assert_eq!(
        line_containing(&terminal, "PLANET COMMAND <- Empire tax rate").trim_end(),
        "PLANET COMMAND <- Empire tax rate (0 - 100) [65] <Q> ->"
    );
    assert!(
        line_containing(&terminal, "Notice: ").contains("Empire tax rate set to 65%."),
        "expected inline success notice after saving tax"
    );
}

#[test]
fn fleet_table_zero_pads_numbers_to_current_max_width() {
    let mut screen = ec_game::screen::FleetListScreen::new();
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
            list_eta_label: "0".to_string(),
            rules_of_engagement: 6,
            order_label: "Hold".to_string(),
            composition_label: "CA=1".to_string(),
            table_composition_label: "CA".to_string(),
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
            list_eta_label: "0".to_string(),
            rules_of_engagement: 6,
            order_label: "Hold".to_string(),
            composition_label: "DD=1".to_string(),
            table_composition_label: "DD".to_string(),
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
            list_eta_label: "0".to_string(),
            rules_of_engagement: 6,
            order_label: "Hold".to_string(),
            composition_label: "BB=1".to_string(),
            table_composition_label: "BB".to_string(),
        },
    ];

    let buffer = screen
        .render(&rows, 0, 0, "", None)
        .expect("fleet list renders");

    assert!(buffer.plain_line(4).contains("│001│"));
    assert!(buffer.plain_line(5).contains("│010│"));
    assert!(buffer.plain_line(6).contains("│100│"));
}

#[test]
fn fleet_list_table_uses_order_target_eta_columns_and_current_speed() {
    let mut screen = ec_game::screen::FleetListScreen::new();
    let rows = vec![FleetRow {
        fleet_record_index_1_based: 1,
        fleet_number: 4,
        coords: [8, 9],
        target_coords: [16, 13],
        order_code: 5,
        current_speed: 2,
        max_speed: 6,
        eta_label: "3000".to_string(),
        list_eta_label: "0".to_string(),
        rules_of_engagement: 6,
        order_label: "Guard/blockade world in System (16,13)".to_string(),
        composition_label: "DD=1".to_string(),
        table_composition_label: "DD".to_string(),
    }];

    let buffer = screen
        .render(&rows, 0, 0, "", None)
        .expect("fleet list renders");

    assert_eq!(buffer.plain_line(0), "FLEET LIST:");
    assert!(!buffer.plain_line(1).contains("ENTER reviews a fleet."));
    assert!(buffer.plain_line(1).starts_with("┌"));
    assert!(buffer.plain_line(1).ends_with("┐"));
    assert!(buffer.plain_line(2).contains("│ID│Location│Order"));
    assert!(buffer.plain_line(2).contains("│Target"));
    assert!(buffer.plain_line(2).contains("│Spd│"));
    assert!(buffer.plain_line(2).contains("ETA"));
    assert!(buffer.plain_line(2).contains("ROE"));
    assert!(buffer.plain_line(2).contains("Ships"));
    assert!(buffer.plain_line(4).contains("Grd/Blkd"));
    assert!(buffer.plain_line(4).contains("(16,13)"));
    assert!(buffer.plain_line(4).contains("│  2│"));
    assert!(!buffer.plain_line(4).contains("2/6"));
    assert!(buffer.plain_line(4).contains("0"));
    assert!(buffer.plain_line(4).contains("DD"));
    assert_eq!(buffer.plain_line(6), "COMMANDS <ARROWS J K Q> [4] ->");
}

#[test]
fn fleet_list_eta_column_shows_turns_remaining_for_arrived_fleets() {
    let fixture_dir = temp_game_copy();
    let mut state = latest_runtime_state(&fixture_dir);
    state.game_data.conquest.set_game_year(3007);
    for fleet in state
        .game_data
        .fleets
        .records
        .iter_mut()
        .filter(|fleet| fleet.owner_empire_raw() == 1)
    {
        fleet.set_current_location_coords_raw([16, 13]);
        fleet.set_standing_order_target_coords_raw([16, 13]);
    }
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenList)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("fleet list should render");
    let right_border_col = terminal
        .line(1)
        .chars()
        .position(|ch| ch == '┐')
        .expect("fleet list should have a right border");
    let scrollbar_col = (1..=22).find_map(|row| {
        terminal
            .line(row)
            .chars()
            .nth(right_border_col + 1)
            .filter(|ch| matches!(ch, '^' | '|' | '#' | 'v'))
            .map(|_| right_border_col + 1)
    });
    assert!(right_border_col < 79);
    if let Some(scrollbar_col) = scrollbar_col {
        assert_eq!(scrollbar_col, right_border_col + 1);
    }
    assert!(
        terminal
            .lines
            .iter()
            .filter(|line| line.contains("(16,13)"))
            .any(|line| line.contains("│  0│")),
        "{:#?}",
        terminal.lines
    );
}

#[test]
fn fleet_list_sorts_descending_and_typed_fleet_number_opens_review() {
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

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("fleet list should render");
    assert!(terminal.line(4).contains("│ 4│"));
    assert_eq!(
        line_containing(&terminal, "COMMANDS <ARROWS J K Q> [").trim_end(),
        "COMMANDS <ARROWS J K Q> [4] ->"
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::AppendListChar('1'))),
        AppOutcome::Continue
    );
    app.render(&mut terminal)
        .expect("fleet list should render typed fleet input");
    assert_eq!(
        line_containing(&terminal, "COMMANDS <ARROWS J K Q> [").trim_end(),
        "COMMANDS <ARROWS J K Q> [1] -> 1"
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenReview)),
        AppOutcome::Continue
    );
    assert_eq!(app.current_screen(), ScreenId::FleetReview);
    app.render(&mut terminal)
        .expect("fleet review should render");
    assert!(
        line_containing(&terminal, "Fleet ID: ").contains("Fleet ID: 1"),
        "{:#?}",
        terminal.lines
    );
}

#[test]
fn fleet_eta_screen_renders_bottom_line_prompt() {
    let mut screen = ec_game::screen::FleetEtaScreen::new();
    let row = FleetRow {
        fleet_record_index_1_based: 1,
        fleet_number: 7,
        coords: [16, 13],
        target_coords: [19, 13],
        order_code: 1,
        current_speed: 3,
        max_speed: 3,
        eta_label: "1".to_string(),
        list_eta_label: "1".to_string(),
        rules_of_engagement: 6,
        order_label: "Move fleet to Sector (19,13)".to_string(),
        composition_label: "CA=1".to_string(),
        table_composition_label: "CA".to_string(),
    };

    let buffer = screen
        .render(
            &row,
            ec_game::screen::FleetEtaMode::EnteringDestination,
            [19, 13],
            "",
            "",
            None,
        )
        .expect("fleet eta screen renders");

    assert_eq!(buffer.plain_line(0), "CALCULATE FLEET ETA:");
    assert_eq!(buffer.plain_line(1).trim_end(), "Fleet ID: 7");
    assert_eq!(buffer.plain_line(2).trim_end(), "Location: (16,13)");
    assert_eq!(buffer.plain_line(4).trim_end(), "Current Target: (19,13)");
    assert!(
        buffer
            .plain_line(7)
            .contains("FLEET COMMAND <- Destination [19,13] <Q> ->")
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
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    advance_to_main_menu(&mut app);
    open_eta_from_fleet_menu(&mut app, Some(4));
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
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    advance_to_main_menu(&mut app);
    open_eta_from_fleet_menu(&mut app, Some(1));
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
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    advance_to_main_menu(&mut app);
    open_eta_from_fleet_menu(&mut app, Some(1));
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
        .render_specify(&view, &orders, "", None, None)
        .expect("build specify renders");

    assert!(
        buffer
            .plain_line(11)
            .contains("BUILD COMMAND <- Unit number or 0 if done")
    );
    assert!(buffer.plain_line(11).contains("[0] <Q> ->"));
    assert!(
        buffer
            .plain_line(13)
            .contains("You have spent 10 out of 40 points. You have 30 points left to spend.")
    );
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
            ec_game::screen::build_unit_spec(1).expect("destroyer spec"),
            6,
            "",
            None,
        )
        .expect("build quantity renders");

    assert!(
        buffer
            .plain_line(11)
            .contains("BUILD COMMAND <- How many new destroyers to build")
    );
    assert!(buffer.plain_line(11).contains("[1] <Q> ->"));
    assert!(
        buffer
            .plain_line(13)
            .contains("You have spent 10 out of 40 points. You have 30 points left to spend.")
    );
}

#[test]
fn planet_build_specify_renders_success_as_notice_not_error() {
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
        points_remaining: 10,
    }];

    let buffer = screen
        .render_specify(&view, &orders, "", None, Some("Queued 2 Destroyers."))
        .expect("build specify renders with notice");

    assert!(
        buffer
            .plain_line(15)
            .contains("Notice: Queued 2 Destroyers.")
    );
    assert!(!buffer.plain_line(15).contains("Error:"));
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

#[test]
fn apply_action_queues_composed_message() {
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
        session_timeout_secs: None,
        game_config: Default::default(),
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
fn compose_body_navigation_tracks_visual_wrapped_lines() {
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

    app.messaging.compose_body = format!("{} splitword", "a".repeat(78));
    app.messaging.compose_body_cursor_row = 1;
    app.messaging.compose_body_cursor_col = 4;

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::MoveComposeBodyCursorHome)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body_cursor_row, 1);
    assert_eq!(app.messaging.compose_body_cursor_col, 0);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::MoveComposeBodyCursorUp)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body_cursor_row, 0);
    assert_eq!(app.messaging.compose_body_cursor_col, 0);

    app.messaging.compose_body_cursor_row = 0;
    app.messaging.compose_body_cursor_col = 4;
    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::MoveComposeBodyCursorDown)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body_cursor_row, 1);
    assert_eq!(app.messaging.compose_body_cursor_col, 4);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::MoveComposeBodyCursorEnd)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body_cursor_row, 1);
    assert_eq!(app.messaging.compose_body_cursor_col, 9);
}

#[test]
fn compose_body_cursor_can_move_down_from_empty_editor_without_mutating_body() {
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

    app.current_screen = ScreenId::ComposeMessageBody;
    app.messaging.compose_body.clear();
    app.messaging.compose_body_cursor_row = 0;
    app.messaging.compose_body_cursor_col = 0;

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::MoveComposeBodyCursorDown)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body, "");
    assert_eq!(app.messaging.compose_body_cursor_row, 1);
    assert_eq!(app.messaging.compose_body_cursor_col, 0);
}

#[test]
fn compose_body_cursor_can_move_into_blank_lines_and_type_there() {
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

    app.current_screen = ScreenId::ComposeMessageBody;
    app.messaging.compose_body = "abc".to_string();
    app.messaging.compose_body_cursor_row = 0;
    app.messaging.compose_body_cursor_col = 3;

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::MoveComposeBodyCursorDown)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body, "abc");
    assert_eq!(app.messaging.compose_body_cursor_row, 1);
    assert_eq!(app.messaging.compose_body_cursor_col, 3);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::AppendComposeBodyChar('Z'))
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body, "abc\n   Z");
    assert_eq!(app.messaging.compose_body_cursor_row, 1);
    assert_eq!(app.messaging.compose_body_cursor_col, 4);
}

#[test]
fn compose_body_cursor_can_move_right_past_end_of_text_without_mutating_body() {
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

    app.current_screen = ScreenId::ComposeMessageBody;
    app.messaging.compose_body = "abc".to_string();
    app.messaging.compose_body_cursor_row = 0;
    app.messaging.compose_body_cursor_col = 3;

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::MoveComposeBodyCursorRight)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body, "abc");
    assert_eq!(app.messaging.compose_body_cursor_row, 0);
    assert_eq!(app.messaging.compose_body_cursor_col, 4);
}

#[test]
fn compose_body_tab_inserts_four_spaces() {
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

    app.current_screen = ScreenId::ComposeMessageBody;
    app.messaging.compose_body = "abc".to_string();
    app.messaging.compose_body_cursor_row = 0;
    app.messaging.compose_body_cursor_col = 3;

    let tab = app.handle_key(key(KeyCode::Tab));
    assert_eq!(apply_action(&mut app, tab), AppOutcome::Continue);
    assert_eq!(app.messaging.compose_body, "abc    ");
    assert_eq!(app.messaging.compose_body_cursor_row, 0);
    assert_eq!(app.messaging.compose_body_cursor_col, 7);
}

#[test]
fn compose_body_tab_pushes_existing_text_right() {
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

    app.current_screen = ScreenId::ComposeMessageBody;
    app.messaging.compose_body = "abcxyz".to_string();
    app.messaging.compose_body_cursor_row = 0;
    app.messaging.compose_body_cursor_col = 3;

    let tab = app.handle_key(key(KeyCode::Tab));
    assert_eq!(apply_action(&mut app, tab), AppOutcome::Continue);
    assert_eq!(app.messaging.compose_body, "abc    xyz");
    assert_eq!(app.messaging.compose_body_cursor_row, 0);
    assert_eq!(app.messaging.compose_body_cursor_col, 7);
}

#[test]
fn compose_body_cursor_left_and_right_do_not_page_jump_on_short_first_line() {
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

    app.current_screen = ScreenId::ComposeMessageBody;
    app.messaging.compose_body = "x".to_string();
    app.messaging.compose_body_cursor_row = 0;
    app.messaging.compose_body_cursor_col = 1;

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::MoveComposeBodyCursorLeft)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body_cursor_row, 0);
    assert_eq!(app.messaging.compose_body_cursor_col, 0);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::MoveComposeBodyCursorRight)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body_cursor_row, 0);
    assert_eq!(app.messaging.compose_body_cursor_col, 1);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::MoveComposeBodyCursorRight)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body_cursor_row, 0);
    assert_eq!(app.messaging.compose_body_cursor_col, 2);
}

#[test]
fn compose_body_cursor_preserves_visual_column_in_blank_canvas_space() {
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

    app.current_screen = ScreenId::ComposeMessageBody;
    app.messaging.compose_body = "abc".to_string();
    app.messaging.compose_body_cursor_row = 2;
    app.messaging.compose_body_cursor_col = 8;

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::MoveComposeBodyCursorUp)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body_cursor_row, 1);
    assert_eq!(app.messaging.compose_body_cursor_col, 8);

    assert_eq!(
        apply_action(
            &mut app,
            Action::Messaging(MessagingAction::MoveComposeBodyCursorDown)
        ),
        AppOutcome::Continue
    );
    assert_eq!(app.messaging.compose_body_cursor_row, 2);
    assert_eq!(app.messaging.compose_body_cursor_col, 8);
}

#[test]
fn fleet_detach_uses_staged_class_prompt_and_creates_new_fleet() {
    let fixture_dir = temp_game_copy();
    let mut game_data = CoreGameData::load(&fixture_dir).expect("load fixture");
    let initial_fleet_count = game_data.fleets.records.len();
    let donor = &mut game_data.fleets.records[0];
    donor.set_scout_count(1);
    donor.set_cruiser_count(1);
    donor.set_destroyer_count(4);
    donor.set_battleship_count(0);
    donor.set_troop_transport_count(4);
    donor.set_army_count(4);
    donor.set_etac_count(0);
    donor.set_current_location_coords_raw([8, 9]);
    donor.recompute_max_speed_from_composition();
    donor.set_current_speed(0);
    donor.set_rules_of_engagement(0);
    game_data.save(&fixture_dir).expect("save fixture");
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    import_directory_snapshot(&store, &fixture_dir).expect("refresh sqlite snapshot");

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_detach_from_fleet_menu(&mut app, Some(1));
    assert_eq!(app.current_screen(), ScreenId::FleetDetach);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal).expect("render class prompt");
    assert_eq!(terminal.line(1).trim_end(), "");
    assert_eq!(terminal.line(2).trim_end(), "Fleet: Fleet #1");
    assert_eq!(terminal.line(3).trim_end(), "");
    assert_eq!(terminal.line(4).trim_end(), "Location: (08,09)");
    assert!(terminal.line(5).starts_with("Orders: "));
    assert!(terminal.line(6).starts_with("Target: "));
    assert_eq!(terminal.line(7).trim_end(), "Speed: 0");
    assert_eq!(terminal.line(8).trim_end(), "ROE: 0");
    assert!(
        terminal
            .line(10)
            .contains("Ships: SC=1 CA=1 DD=4 TT=4 AR=4")
    );
    assert_eq!(terminal.line(12).trim_end(), "<C>ommission, <X> Cancel");
    assert!(
        line_containing(&terminal, "Class <BB,CA,DD,TT*,TT,SC,ET,C,X,Q>")
            .contains("Class <BB,CA,DD,TT*,TT,SC,ET,C,X,Q>")
    );
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Detach ships from the selected fleet"))
    );
    assert!(line_containing(&terminal, "Staged for New Fleet: ").contains("none"));
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Remaining on Donor: "))
    );

    enter_detach_input(&mut app, "dd");
    submit_detach(&mut app);
    app.render(&mut terminal).expect("render quantity prompt");
    assert!(
        line_containing(&terminal, "DD to stage (max 4)")
            .contains("DD to stage (max 4) [1] <Q> ->")
    );

    submit_detach(&mut app);
    app.render(&mut terminal).expect("render staged summary");
    assert!(line_containing(&terminal, "Staged for New Fleet: ").contains("DD=1"));
    assert!(line_containing(&terminal, "Remaining on Donor: ").contains("SC=1 CA=1 DD=3 TT*=4"));

    enter_detach_input(&mut app, "c");
    submit_detach(&mut app);
    app.render(&mut terminal)
        .expect("render menu notice after commission");
    assert_eq!(app.current_screen(), ScreenId::FleetDetach);
    assert!(line_containing(&terminal, "Staged for New Fleet: ").contains("none"));
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains("Remaining on Donor: "))
    );
    let updated = latest_runtime_state(&fixture_dir).game_data;
    let first_commission_message = format!(
        "Commissioned Fleet #{:02} from Fleet #01.",
        updated
            .fleets
            .records
            .last()
            .expect("detached fleet")
            .local_slot_word_raw()
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains(&first_commission_message))
    );

    assert_eq!(updated.fleets.records.len(), initial_fleet_count + 1);
    assert_eq!(updated.fleets.records[0].scout_count(), 1);
    assert_eq!(updated.fleets.records[0].cruiser_count(), 1);
    assert_eq!(updated.fleets.records[0].destroyer_count(), 3);
    assert_eq!(updated.fleets.records[0].troop_transport_count(), 4);
    assert_eq!(updated.fleets.records[0].army_count(), 4);
    let detached = updated.fleets.records.last().expect("detached fleet");
    assert_eq!(detached.destroyer_count(), 1);
    assert_eq!(detached.scout_count(), 0);
    assert_eq!(detached.cruiser_count(), 0);
    assert_eq!(detached.troop_transport_count(), 0);
    assert_eq!(detached.army_count(), 0);
    assert_eq!(
        detached.rules_of_engagement(),
        updated.fleets.records[0].rules_of_engagement()
    );
}

#[test]
fn fleet_detach_last_commissioned_message_persists_until_overwritten() {
    let fixture_dir = temp_game_copy();
    let mut game_data = CoreGameData::load(&fixture_dir).expect("load fixture");
    let donor = &mut game_data.fleets.records[0];
    donor.set_battleship_count(0);
    donor.set_cruiser_count(1);
    donor.set_destroyer_count(4);
    donor.set_troop_transport_count(4);
    donor.set_army_count(4);
    donor.set_scout_count(1);
    donor.set_etac_count(0);
    donor.recompute_max_speed_from_composition();
    donor.set_current_speed(0);
    donor.set_rules_of_engagement(0);
    game_data.save(&fixture_dir).expect("save fixture");
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    import_directory_snapshot(&store, &fixture_dir).expect("refresh sqlite snapshot");

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_detach_from_fleet_menu(&mut app, Some(1));

    enter_detach_input(&mut app, "dd");
    submit_detach(&mut app);
    submit_detach(&mut app);
    enter_detach_input(&mut app, "c");
    submit_detach(&mut app);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("render after first commission");
    let first_commission_message = line_containing(&terminal, "Commissioned Fleet #")
        .trim_end()
        .to_string();

    enter_detach_input(&mut app, "zz");
    submit_detach(&mut app);
    app.render(&mut terminal)
        .expect("render empty commission warning with pinned message");
    assert_eq!(
        app.fleet.detach_status.as_deref(),
        Some("Use BB, CA, DD, TT*, TT, SC, ET, C, X, or Q.")
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains(&first_commission_message))
    );

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::ClearDetachSelection)),
        AppOutcome::Continue
    );
    enter_detach_input(&mut app, "ca");
    submit_detach(&mut app);
    submit_detach(&mut app);
    enter_detach_input(&mut app, "c");
    submit_detach(&mut app);
    app.render(&mut terminal)
        .expect("render after second commission");
    let second_commission_message = line_containing(&terminal, "Commissioned Fleet #")
        .trim_end()
        .to_string();
    assert_ne!(second_commission_message, first_commission_message);
    assert!(
        !terminal
            .lines
            .iter()
            .any(|line| line.contains(&first_commission_message))
    );
}

#[test]
fn fleet_detach_commission_requires_staged_ships_and_preserves_staged_block() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_detach_from_fleet_menu(&mut app, Some(1));
    assert_eq!(app.current_screen(), ScreenId::FleetDetach);

    let mut terminal = CaptureTerminal::new();
    enter_detach_input(&mut app, "c");
    submit_detach(&mut app);
    app.render(&mut terminal)
        .expect("render empty commission warning");

    assert_eq!(app.current_screen(), ScreenId::FleetDetach);
    assert!(
        line_containing(&terminal, "Stage at least one ship before commissioning.")
            .contains("Stage at least one ship before commissioning.")
    );
    assert!(line_containing(&terminal, "Staged for New Fleet: ").contains("none"));
}

#[test]
fn fleet_detach_x_clears_staged_selection_without_leaving_screen() {
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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_detach_from_fleet_menu(&mut app, Some(1));

    enter_detach_input(&mut app, "sc");
    submit_detach(&mut app);
    submit_detach(&mut app);
    assert_eq!(app.current_screen(), ScreenId::FleetDetach);
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::ClearDetachSelection)),
        AppOutcome::Continue
    );

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("render cleared staged selection");
    assert_eq!(app.current_screen(), ScreenId::FleetDetach);
    assert!(line_containing(&terminal, "Staged for New Fleet: ").contains("none"));
}

#[test]
fn fleet_detach_leaves_at_least_one_ship_on_the_donor() {
    let fixture_dir = temp_game_copy();
    let mut game_data = CoreGameData::load(&fixture_dir).expect("load fixture");
    let donor = &mut game_data.fleets.records[0];
    donor.set_destroyer_count(2);
    donor.set_battleship_count(0);
    donor.set_cruiser_count(0);
    donor.set_troop_transport_count(0);
    donor.set_army_count(0);
    donor.set_scout_count(0);
    donor.set_etac_count(0);
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
        session_timeout_secs: None,
        game_config: Default::default(),
    })
    .expect("app should load");

    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_detach_from_fleet_menu(&mut app, Some(1));

    enter_detach_input(&mut app, "dd");
    submit_detach(&mut app);
    enter_detach_input(&mut app, "2");
    submit_detach(&mut app);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("render donor minimum warning");
    assert_eq!(app.current_screen(), ScreenId::FleetDetach);
    assert!(
        line_containing(&terminal, "Enter a quantity from 1 to 1.")
            .contains("Enter a quantity from 1 to 1.")
    );
    assert!(line_containing(&terminal, "Staged for New Fleet: ").contains("none"));
}

#[test]
fn fleet_detach_final_commission_returns_to_menu_with_new_fleet_number_notice() {
    let fixture_dir = temp_game_copy();
    let mut game_data = CoreGameData::load(&fixture_dir).expect("load fixture");
    let donor = &mut game_data.fleets.records[0];
    donor.set_destroyer_count(2);
    donor.set_battleship_count(0);
    donor.set_cruiser_count(0);
    donor.set_troop_transport_count(0);
    donor.set_army_count(0);
    donor.set_scout_count(0);
    donor.set_etac_count(0);
    donor.recompute_max_speed_from_composition();
    donor.set_current_speed(0);
    game_data.save(&fixture_dir).expect("save fixture");
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    import_directory_snapshot(&store, &fixture_dir).expect("refresh sqlite snapshot");

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    open_detach_from_fleet_menu(&mut app, Some(1));

    enter_detach_input(&mut app, "dd");
    submit_detach(&mut app);
    submit_detach(&mut app);
    enter_detach_input(&mut app, "c");
    submit_detach(&mut app);

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("render fleet menu notice after final detach");
    let updated = latest_runtime_state(&app.game_dir).game_data;
    let new_fleet_number = updated
        .fleets
        .records
        .last()
        .expect("detached fleet")
        .local_slot_word_raw();

    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert!(line_containing(&terminal, "Notice: ").contains(&format!(
        "Detached ships from Fleet #01 into Fleet #{new_fleet_number:02}."
    )));
}

#[test]
fn fleet_detach_prompt_reports_missing_fleet_number() {
    let fixture_dir = temp_game_copy();
    let mut game_data = CoreGameData::load(&fixture_dir).expect("load fixture");
    let fleet_one = game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet_one.set_battleship_count(2);
    fleet_one.set_cruiser_count(0);
    fleet_one.set_destroyer_count(0);
    fleet_one.set_troop_transport_count(0);
    fleet_one.set_army_count(0);
    fleet_one.set_scout_count(0);
    fleet_one.set_etac_count(0);
    let fleet_two = game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_battleship_count(0);
    fleet_two.set_cruiser_count(0);
    fleet_two.set_destroyer_count(6);
    fleet_two.set_troop_transport_count(0);
    fleet_two.set_army_count(0);
    fleet_two.set_scout_count(0);
    fleet_two.set_etac_count(0);
    game_data.save(&fixture_dir).expect("save fixture");
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    import_directory_snapshot(&store, &fixture_dir).expect("refresh sqlite snapshot");

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenDetach)),
        AppOutcome::Continue
    );
    assert_eq!(app.fleet.menu_prompt_default_value, "2");
    submit_fleet_menu_prompt_value(&mut app, "99");

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("render detach prompt missing fleet notice");
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(app.fleet.menu_prompt_default_value, "2");
    assert!(
        line_containing(&terminal, "FLEET COMMAND <- Detach Fleet #").contains("Detach Fleet #")
    );
    assert!(
        terminal
            .lines
            .iter()
            .any(|line| line.contains("Fleet #99 is not in your fleet list."))
    );
}

#[test]
fn fleet_detach_prompt_reports_single_ship_fleet_as_ineligible() {
    let fixture_dir = temp_game_copy();
    let mut game_data = CoreGameData::load(&fixture_dir).expect("load fixture");
    let donor = &mut game_data.fleets.records[0];
    donor.set_destroyer_count(1);
    donor.set_battleship_count(0);
    donor.set_cruiser_count(0);
    donor.set_troop_transport_count(0);
    donor.set_army_count(0);
    donor.set_scout_count(0);
    donor.set_etac_count(0);
    donor.recompute_max_speed_from_composition();
    donor.set_current_speed(0);
    let fallback = game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fallback.set_battleship_count(0);
    fallback.set_cruiser_count(0);
    fallback.set_destroyer_count(4);
    fallback.set_troop_transport_count(0);
    fallback.set_army_count(0);
    fallback.set_scout_count(0);
    fallback.set_etac_count(0);
    game_data.save(&fixture_dir).expect("save fixture");
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    import_directory_snapshot(&store, &fixture_dir).expect("refresh sqlite snapshot");

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenDetach)),
        AppOutcome::Continue
    );
    assert_eq!(app.fleet.menu_prompt_default_value, "2");
    submit_fleet_menu_prompt(&mut app, Some(1));

    let mut terminal = CaptureTerminal::new();
    app.render(&mut terminal)
        .expect("render detach prompt single-ship notice");
    assert_eq!(app.current_screen(), ScreenId::FleetMenu);
    assert_eq!(app.fleet.menu_prompt_default_value, "2");
    assert_eq!(
        app.fleet.menu_prompt_status.as_deref(),
        Some("Fleet #1 has only one ship and is not eligible to detach any ships.")
    );
}

#[test]
fn fleet_detach_prompt_defaults_to_largest_owned_fleet_by_ship_total() {
    let fixture_dir = temp_game_copy();
    let mut game_data = CoreGameData::load(&fixture_dir).expect("load fixture");
    let fleet_one = game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 1)
        .expect("fleet #1 should exist");
    fleet_one.set_battleship_count(3);
    fleet_one.set_cruiser_count(0);
    fleet_one.set_destroyer_count(0);
    fleet_one.set_troop_transport_count(0);
    fleet_one.set_army_count(0);
    fleet_one.set_scout_count(0);
    fleet_one.set_etac_count(0);
    let fleet_two = game_data
        .fleets
        .records
        .iter_mut()
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 2)
        .expect("fleet #2 should exist");
    fleet_two.set_battleship_count(0);
    fleet_two.set_cruiser_count(0);
    fleet_two.set_destroyer_count(5);
    fleet_two.set_troop_transport_count(0);
    fleet_two.set_army_count(0);
    fleet_two.set_scout_count(0);
    fleet_two.set_etac_count(0);
    game_data.save(&fixture_dir).expect("save fixture");
    let store = CampaignStore::open_default_in_dir(&fixture_dir).expect("open campaign store");
    import_directory_snapshot(&store, &fixture_dir).expect("refresh sqlite snapshot");

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
        apply_action(&mut app, Action::Fleet(FleetAction::OpenMenu)),
        AppOutcome::Continue
    );
    assert_eq!(
        apply_action(&mut app, Action::Fleet(FleetAction::OpenDetach)),
        AppOutcome::Continue
    );
    assert_eq!(app.fleet.menu_prompt_default_value, "2");
}
