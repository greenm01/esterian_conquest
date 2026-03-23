use std::collections::BTreeMap;
use std::path::PathBuf;

use ec_data::{CampaignStore, CoreGameData, PlanetIntelSnapshot, QueuedPlayerMail, ReportBlockRow};

use crate::domains::empire::EmpireState;
use crate::domains::fleet::FleetState;
use crate::domains::messaging::MessagingState;
use crate::domains::planet::PlanetState;
use crate::domains::starbase::StarbaseState;
use crate::domains::starmap::StarmapState;
use crate::domains::startup::StartupState;
use crate::model::{MainMenuSummary, PlayerContext, ReviewSummary};
use crate::reports::{ReportsPreview, has_visible_runtime_messages};
use crate::screen::{
    BuildHelpScreen, CommandMenu, EmpireProfileScreen, EmpireStatusScreen, EnemiesScreen,
    FirstTimeEmpiresScreen, FirstTimeHelpScreen, FirstTimeIntroScreen, FirstTimeMenuScreen,
    FleetDetachScreen, FleetEtaScreen, FleetGroupScreen, FleetHelpScreen, FleetListMode,
    FleetListScreen, FleetMenuScreen, FleetMergeScreen, FleetMissionPickerScreen,
    FleetReviewScreen, FleetRoeScreen, FleetSingleOrderScreen, FleetTransferScreen,
    GeneralHelpScreen, GeneralMenuScreen, MainHelpScreen, MainMenuScreen, MessageComposeScreen,
    PartialStarmapScreen, PlanetBuildScreen, PlanetCommissionScreen, PlanetDatabaseScreen,
    PlanetHelpScreen, PlanetInfoScreen, PlanetListScreen, PlanetMenuScreen, PlanetTaxScreen,
    PlanetTransportScreen, RankingsScreen, ReportsScreen, ScreenId, StarbaseHelpScreen,
    StarbaseListScreen, StarbaseMenuScreen, StarbaseReviewScreen, StarmapScreen, StartupScreen,
};
use crate::startup::{StartupSequence, StartupSummary};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub game_dir: PathBuf,
    pub player_record_index_1_based: usize,
    pub export_root: Option<PathBuf>,
    pub queue_dir: Option<PathBuf>,
}

pub struct App {
    pub game_dir: PathBuf,
    pub game_data: CoreGameData,
    pub player: PlayerContext,
    pub current_screen: ScreenId,
    pub startup_sequence: StartupSequence,
    pub startup: StartupScreen,
    pub first_time_menu: FirstTimeMenuScreen,
    pub first_time_help: FirstTimeHelpScreen,
    pub first_time_empires: FirstTimeEmpiresScreen,
    pub first_time_intro: FirstTimeIntroScreen,
    pub main_menu: MainMenuScreen,
    pub main_help: MainHelpScreen,
    pub general_menu: GeneralMenuScreen,
    pub general_help: GeneralHelpScreen,
    pub fleet_help: FleetHelpScreen,
    pub starbase_menu: StarbaseMenuScreen,
    pub starbase_help: StarbaseHelpScreen,
    pub starbase_list: StarbaseListScreen,
    pub starbase_review: StarbaseReviewScreen,
    pub fleet_menu: FleetMenuScreen,
    pub fleet_list: FleetListScreen,
    pub fleet_review: FleetReviewScreen,
    pub fleet_roe: FleetRoeScreen,
    pub fleet_order: FleetSingleOrderScreen,
    pub fleet_group: FleetGroupScreen,
    pub fleet_mission_picker: FleetMissionPickerScreen,
    pub fleet_merge: FleetMergeScreen,
    pub fleet_transfer: FleetTransferScreen,
    pub fleet_detach: FleetDetachScreen,
    pub fleet_eta: FleetEtaScreen,
    pub planet_menu: PlanetMenuScreen,
    pub planet_help: PlanetHelpScreen,
    pub planet_commission: PlanetCommissionScreen,
    pub planet_transport: PlanetTransportScreen,
    pub build_help: BuildHelpScreen,
    pub planet_build: PlanetBuildScreen,
    pub planet_list: PlanetListScreen,
    pub planet_tax: PlanetTaxScreen,
    pub starmap: StarmapScreen,
    pub partial_starmap: PartialStarmapScreen,
    pub planet_database: PlanetDatabaseScreen,
    pub planet_info: PlanetInfoScreen,
    pub enemies: EnemiesScreen,
    pub message_compose: MessageComposeScreen,
    pub empire_status: EmpireStatusScreen,
    pub empire_profile: EmpireProfileScreen,
    pub rankings: RankingsScreen,
    pub reports: ReportsScreen,

    pub fleet: FleetState,
    pub planet: PlanetState,
    pub starbase: StarbaseState,
    pub empire: EmpireState,
    pub messaging: MessagingState,
    pub starmap_state: StarmapState,
    pub startup_state: StartupState,

    pub command_return_menu: CommandMenu,
    pub return_screen: Option<ScreenId>,
    pub export_root: PathBuf,
    pub queue_dir: Option<PathBuf>,
    pub autopilot: bool,
    pub expert_mode: bool,
    pub snapshot_id: i64,
    pub campaign_seed: u64,
    pub report_block_rows: Vec<ReportBlockRow>,
    pub queued_mail: Vec<QueuedPlayerMail>,
    pub command_menu_notice: Option<String>,
    pub planet_intel_snapshots: BTreeMap<usize, PlanetIntelSnapshot>,
}

impl App {
    pub fn load(config: AppConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let game_dir = config.game_dir.clone();
        let export_root = config
            .export_root
            .clone()
            .unwrap_or_else(|| game_dir.join("exports"));
        let campaign_store = CampaignStore::open_default_in_dir(&game_dir)?;
        let runtime_state = campaign_store
            .load_latest_runtime_state()?
            .ok_or("campaign store has no snapshots; import with ec-cli db-import first")?;
        let snapshot_id = runtime_state.snapshot_id;
        let campaign_seed = runtime_state.campaign_seed;
        let report_block_rows = runtime_state.report_block_rows;
        let queued_mail = runtime_state.queued_mail;
        let reports = ReportsPreview::from_block_rows(
            &runtime_state.game_data,
            config.player_record_index_1_based as u8,
            &report_block_rows,
            &queued_mail,
        );
        let game_data = runtime_state.game_data;
        let player = PlayerContext::from_game_data(&game_data, config.player_record_index_1_based)?;
        let planet_intel_snapshots = campaign_store
            .latest_planet_intel_for_viewer(config.player_record_index_1_based as u8)?
            .into_iter()
            .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
            .collect::<BTreeMap<_, _>>();
        let main_menu_summary = MainMenuSummary::from_game_data(
            &game_data,
            config.player_record_index_1_based,
            !report_block_rows.is_empty(),
            has_visible_runtime_messages(config.player_record_index_1_based as u8, &queued_mail),
        );
        let review_summary = ReviewSummary::from_main_menu(&main_menu_summary);
        let startup_summary = StartupSummary::from_reports(
            main_menu_summary.game_year,
            player.classic_login_state,
            main_menu_summary.pending_results,
            main_menu_summary.pending_messages,
            &reports,
        );
        let startup_sequence = StartupSequence::new(&startup_summary, player.classic_login_state);
        Ok(Self {
            game_dir,
            game_data,
            player,
            current_screen: ScreenId::Startup(startup_sequence.current()),
            startup_sequence,
            startup: StartupScreen::new(startup_summary, reports.clone()),
            first_time_menu: FirstTimeMenuScreen::new(),
            first_time_help: FirstTimeHelpScreen::new(),
            first_time_empires: FirstTimeEmpiresScreen::new(),
            first_time_intro: FirstTimeIntroScreen::new(),
            main_menu: MainMenuScreen::new(),
            main_help: MainHelpScreen::new(),
            general_menu: GeneralMenuScreen::new(),
            general_help: GeneralHelpScreen::new(),
            fleet_help: FleetHelpScreen::new(),
            starbase_menu: StarbaseMenuScreen::new(),
            starbase_help: StarbaseHelpScreen::new(),
            starbase_list: StarbaseListScreen::new(),
            starbase_review: StarbaseReviewScreen::new(),
            fleet_menu: FleetMenuScreen::new(),
            fleet_list: FleetListScreen::new(),
            fleet_review: FleetReviewScreen::new(),
            fleet_roe: FleetRoeScreen::new(),
            fleet_order: FleetSingleOrderScreen::new(),
            fleet_group: FleetGroupScreen::new(),
            fleet_mission_picker: FleetMissionPickerScreen::new(),
            fleet_merge: FleetMergeScreen::new(),
            fleet_transfer: FleetTransferScreen::new(),
            fleet_detach: FleetDetachScreen::new(),
            fleet_eta: FleetEtaScreen::new(),
            planet_menu: PlanetMenuScreen::new(),
            planet_help: PlanetHelpScreen::new(),
            planet_commission: PlanetCommissionScreen::new(),
            planet_transport: PlanetTransportScreen::new(),
            build_help: BuildHelpScreen::new(),
            planet_build: PlanetBuildScreen::new(),
            planet_list: PlanetListScreen::new(),
            planet_tax: PlanetTaxScreen::new(),
            starmap: StarmapScreen::new(),
            partial_starmap: PartialStarmapScreen::new(),
            planet_database: PlanetDatabaseScreen::new(),
            planet_info: PlanetInfoScreen::new(),
            enemies: EnemiesScreen::new(),
            message_compose: MessageComposeScreen::new(),
            empire_status: EmpireStatusScreen::new(),
            empire_profile: EmpireProfileScreen::new(),
            rankings: RankingsScreen::new(),
            reports: ReportsScreen::new(reports, review_summary),

            fleet: FleetState {
                list_mode: FleetListMode::Brief,
                ..Default::default()
            },
            planet: PlanetState::new(campaign_store, planet_intel_snapshots.clone()),
            starbase: StarbaseState::default(),
            empire: EmpireState::default(),
            messaging: MessagingState::default(),
            starmap_state: StarmapState {
                partial_center: [8, 2],
                ..Default::default()
            },
            startup_state: StartupState::default(),

            command_return_menu: CommandMenu::General,
            return_screen: None,
            export_root,
            queue_dir: config.queue_dir,
            autopilot: false,
            expert_mode: false,
            snapshot_id,
            campaign_seed,
            report_block_rows,
            queued_mail,
            command_menu_notice: None,
            planet_intel_snapshots,
        })
    }
}
