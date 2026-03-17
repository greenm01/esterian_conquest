use std::collections::BTreeMap;
use std::path::PathBuf;


use ec_data::{
    CampaignStore, CoreGameData, DatabaseDat, PlanetIntelSnapshot, QueuedPlayerMail,
};

use crate::app::action::Action;
use crate::domains::empire::EmpireState;
use crate::domains::fleet::FleetState;
use crate::domains::messaging::MessagingState;
use crate::domains::planet::{PlanetAction, PlanetState};
use crate::domains::starbase::{StarbaseAction, StarbaseState};
use crate::domains::starmap::StarmapState;
use crate::domains::startup::{StartupAction, StartupState};
use crate::model::{MainMenuSummary, PlayerContext, ReviewSummary};
use crate::reports::ReportsPreview;
use crate::screen::{
    BuildHelpScreen, CommandMenu, DeleteReviewablesScreen, EmpireProfileScreen, EmpireStatusScreen,
    EnemiesScreen, FIRST_TIME_INTRO_PAGE_COUNT, FirstTimeEmpiresScreen,
    FirstTimeHelpScreen, FirstTimeIntroScreen, FirstTimeMenuScreen, FleetDetachMode,
    FleetDetachScreen, FleetEtaMode, FleetEtaScreen, FleetGroupScreen,
    FleetHelpScreen, FleetListMode, FleetListScreen, FleetMenuScreen, FleetMergeMode,
    FleetMergeScreen, FleetMissionPickerScreen, FleetReviewScreen, FleetRoeScreen, FleetSingleOrderScreen, FleetTransferMode, FleetTransferScreen,
    GeneralHelpScreen, GeneralMenuScreen, MainHelpScreen, MainMenuScreen, MessageComposeScreen,
    PartialStarmapScreen, PlanetAutoCommissionScreen, PlanetBuildScreen,
    PlanetCommissionScreen, PlanetDatabaseScreen,
    PlanetHelpScreen, PlanetInfoScreen, PlanetListMode, PlanetListScreen,
    PlanetMenuScreen, PlanetTaxScreen, PlanetTransportScreen, RankingsScreen, ReportsScreen,
    STARTUP_SPLASH_PAGE_COUNT, Screen, ScreenId, StarbaseHelpScreen,
    StarbaseListScreen, StarbaseMenuScreen, StarbaseReviewScreen, StarmapScreen, StartupScreen,
};
use crate::startup::{StartupPhase, StartupSequence, StartupSummary};
use crate::terminal::Terminal;

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
    pub database: DatabaseDat,
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
    pub planet_auto_commission: PlanetAutoCommissionScreen,
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
    pub delete_reviewables: DeleteReviewablesScreen,
    pub message_compose: MessageComposeScreen,
    pub empire_status: EmpireStatusScreen,
    pub empire_profile: EmpireProfileScreen,
    pub rankings: RankingsScreen,
    pub reports: ReportsScreen,

    // Domain States
    pub fleet: FleetState,
    pub planet: PlanetState,
    pub starbase: StarbaseState,
    pub empire: EmpireState,
    pub messaging: MessagingState,
    pub starmap_state: StarmapState,
    pub startup_state: StartupState,

    pub command_return_menu: CommandMenu,
    pub export_root: PathBuf,
    pub queue_dir: Option<PathBuf>,
    pub autopilot: bool,
    pub results_bytes: Vec<u8>,
    pub messages_bytes: Vec<u8>,
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
        let reports =
            ReportsPreview::from_bytes(&runtime_state.results_bytes, &runtime_state.messages_bytes);
        let game_data = runtime_state.game_data;
        let database = runtime_state.database;
        let queued_mail = runtime_state.queued_mail;
        let results_bytes = runtime_state.results_bytes;
        let messages_bytes = runtime_state.messages_bytes;
        let player = PlayerContext::from_game_data(&game_data, config.player_record_index_1_based)?;
        let planet_intel_snapshots = campaign_store
            .latest_planet_intel_for_viewer(config.player_record_index_1_based as u8)?
            .into_iter()
            .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
            .collect::<BTreeMap<_, _>>();
        let main_menu_summary = MainMenuSummary::from_game_data(
            &game_data,
            config.player_record_index_1_based,
            !results_bytes.is_empty(),
            !messages_bytes.is_empty(),
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
            database,
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
            planet_auto_commission: PlanetAutoCommissionScreen::new(),
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
            delete_reviewables: DeleteReviewablesScreen::new(),
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
                partial_input: "8,2".to_string(),
                partial_center: [8, 2],
                ..Default::default()
            },
            startup_state: StartupState::default(),

            command_return_menu: CommandMenu::General,
            export_root,
            queue_dir: config.queue_dir,
            autopilot: false,
            results_bytes,
            messages_bytes,
            queued_mail,
            command_menu_notice: None,
            planet_intel_snapshots,
        })
    }

    pub fn render(
        &mut self,
        terminal: &mut dyn Terminal,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::domains;
        let mut playfield = match self.current_screen {
            ScreenId::Startup(_)
            | ScreenId::FirstTimeMenu
            | ScreenId::FirstTimeHelp
            | ScreenId::FirstTimeEmpires
            | ScreenId::FirstTimeIntro
            | ScreenId::FirstTimePreloadedRenamePrompt
            | ScreenId::FirstTimeJoinEmpireName
            | ScreenId::FirstTimeJoinEmpireConfirm
            | ScreenId::FirstTimeJoinSummary
            | ScreenId::FirstTimeJoinNoPending
            | ScreenId::FirstTimeHomeworldName
            | ScreenId::FirstTimeHomeworldConfirm
            | ScreenId::ColonyWorldName
            | ScreenId::ColonyWorldConfirm
            | ScreenId::MainMenu
            | ScreenId::MainHelp
            | ScreenId::GeneralMenu
            | ScreenId::GeneralHelp
            | ScreenId::Reports => domains::startup::views::render(self)?,

            ScreenId::FleetHelp
            | ScreenId::FleetMenu
            | ScreenId::FleetList(_)
            | ScreenId::FleetReviewSelect
            | ScreenId::FleetReview
            | ScreenId::FleetRoeSelect
            | ScreenId::FleetOrder
            | ScreenId::FleetGroupOrder
            | ScreenId::FleetMissionPicker
            | ScreenId::FleetMerge
            | ScreenId::FleetTransfer
            | ScreenId::FleetDetach
            | ScreenId::FleetEta => domains::fleet::views::render(self)?,

            ScreenId::StarbaseMenu
            | ScreenId::StarbaseHelp
            | ScreenId::StarbaseList
            | ScreenId::StarbaseReviewSelect
            | ScreenId::StarbaseReview => domains::starbase::views::render(self)?,

            ScreenId::PlanetMenu
            | ScreenId::PlanetHelp
            | ScreenId::PlanetAutoCommissionConfirm
            | ScreenId::PlanetAutoCommissionDone
            | ScreenId::PlanetTransportPlanetSelect(_)
            | ScreenId::PlanetTransportFleetSelect(_)
            | ScreenId::PlanetTransportQuantityPrompt(_)
            | ScreenId::PlanetTransportDone(_)
            | ScreenId::PlanetCommissionMenu
            | ScreenId::PlanetBuildHelp
            | ScreenId::PlanetBuildMenu
            | ScreenId::PlanetBuildReview
            | ScreenId::PlanetBuildList
            | ScreenId::PlanetBuildChange
            | ScreenId::PlanetBuildAbortConfirm
            | ScreenId::PlanetBuildSpecify
            | ScreenId::PlanetBuildQuantity
            | ScreenId::PlanetListSortPrompt(_)
            | ScreenId::PlanetBriefList(_)
            | ScreenId::PlanetDetailList(_)
            | ScreenId::PlanetTaxPrompt
            | ScreenId::PlanetTaxDone
            | ScreenId::PlanetDatabaseList
            | ScreenId::PlanetDatabaseDetail
            | ScreenId::PlanetInfoPrompt
            | ScreenId::PlanetInfoDetail => domains::planet::views::render(self)?,

            ScreenId::Enemies
            | ScreenId::EmpireStatus
            | ScreenId::EmpireProfile
            | ScreenId::Rankings(_) => domains::empire::views::render(self)?,

            ScreenId::DeleteReviewables
            | ScreenId::ComposeMessageRecipient
            | ScreenId::ComposeMessageSubject
            | ScreenId::ComposeMessageBody
            | ScreenId::ComposeMessageOutbox
            | ScreenId::ComposeMessageDiscardConfirm
            | ScreenId::ComposeMessageSendConfirm
            | ScreenId::ComposeMessageSent => domains::messaging::views::render(self)?,

            ScreenId::Starmap
            | ScreenId::PartialStarmapPrompt
            | ScreenId::PartialStarmapView => domains::starmap::views::render(self)?,
        };
        if let Some(notice) = self.current_modal_notice() {
            crate::screen::draw_command_line_notice(&mut playfield, notice);
        }
        terminal.render(&playfield)
    }


    pub fn current_screen(&self) -> ScreenId {
        self.current_screen
    }

    pub fn current_screen_mut(&mut self) -> &mut ScreenId {
        &mut self.current_screen
    }

    pub fn classic_login_state(&self) -> crate::model::ClassicLoginState {
        self.player.classic_login_state
    }

    pub fn clear_command_menu_notice(&mut self) {
        self.command_menu_notice = None;
    }

    pub fn show_command_menu_notice(&mut self, menu: CommandMenu, message: impl Into<String>) {
        self.command_menu_notice = Some(message.into());
        self.command_return_menu = menu;
        self.current_screen = match menu {
            CommandMenu::Main => ScreenId::MainMenu,
            CommandMenu::General => ScreenId::GeneralMenu,
            CommandMenu::Fleet => ScreenId::FleetMenu,
            CommandMenu::Starbase => ScreenId::StarbaseMenu,
            CommandMenu::Planet => ScreenId::PlanetMenu,
            CommandMenu::PlanetBuild => ScreenId::PlanetBuildMenu,
        };
    }

    pub fn open_main_menu(&mut self) {
        self.clear_command_menu_notice();
        self.current_screen = ScreenId::MainMenu;
    }

    pub fn open_main_help(&mut self) {
        self.clear_command_menu_notice();
        self.current_screen = ScreenId::MainHelp;
    }

    pub fn show_main_menu_ansi_notice(&mut self) {
        self.show_command_menu_notice(
            CommandMenu::Main,
            "ANSI stays on. The stars look better in color.",
        );
    }

    pub fn open_general_menu(&mut self) {
        self.clear_command_menu_notice();
        self.current_screen = ScreenId::GeneralMenu;
    }

    pub fn handle_key(&self, key: crossterm::event::KeyEvent) -> crate::app::Action {
        if self.current_modal_notice().is_some() {
            return Action::DismissModalNotice;
        }
        match self.current_screen {
            ScreenId::Startup(StartupPhase::Splash)
                if self.startup_state.splash_page + 1 < STARTUP_SPLASH_PAGE_COUNT =>
            {
                match key.code {
                    crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Char('Q') => {
                        Action::Quit
                    }
                    _ => Action::Startup(StartupAction::Advance),
                }
            }
            ScreenId::Startup(phase) => self.handle_startup_key(phase, key),
            ScreenId::FirstTimeMenu => self.first_time_menu.handle_key(key),
            ScreenId::FirstTimeHelp => self.first_time_help.handle_key(key),
            ScreenId::FirstTimeEmpires => self.first_time_empires.handle_key(key),
            ScreenId::FirstTimePreloadedRenamePrompt => match key.code {
                crossterm::event::KeyCode::Char('y') | crossterm::event::KeyCode::Char('Y') => {
                    Action::Startup(StartupAction::AcceptFirstTimePrompt)
                }
                crossterm::event::KeyCode::Enter
                | crossterm::event::KeyCode::Char('n')
                | crossterm::event::KeyCode::Char('N')
                | crossterm::event::KeyCode::Esc => Action::Startup(StartupAction::RejectFirstTimePrompt),
                _ => Action::Noop,
            },
            ScreenId::FirstTimeIntro
                if self.startup_state.first_time_intro_page + 1 < FIRST_TIME_INTRO_PAGE_COUNT =>
            {
                Action::Startup(StartupAction::Advance)
            }
            ScreenId::FirstTimeIntro => self.first_time_intro.handle_key(key),
            ScreenId::FirstTimeJoinEmpireName | ScreenId::FirstTimeHomeworldName => {
                match key.code {
                    crossterm::event::KeyCode::Char(ch) => Action::Startup(StartupAction::AppendFirstTimeInputChar(ch)),
                    crossterm::event::KeyCode::Backspace => Action::Startup(StartupAction::BackspaceFirstTimeInput),
                    crossterm::event::KeyCode::Enter => Action::Startup(StartupAction::SubmitFirstTimeInput),
                    crossterm::event::KeyCode::Esc => {
                        if self.startup_state.first_time_rename_preloaded_empire {
                            Action::Startup(StartupAction::RejectFirstTimePrompt)
                        } else {
                            Action::Startup(StartupAction::OpenFirstTimeMenu)
                        }
                    }
                    _ => Action::Noop,
                }
            }
            ScreenId::ColonyWorldName => match key.code {
                crossterm::event::KeyCode::Char(ch) => Action::Startup(StartupAction::AppendFirstTimeInputChar(ch)),
                crossterm::event::KeyCode::Backspace => Action::Startup(StartupAction::BackspaceFirstTimeInput),
                crossterm::event::KeyCode::Enter => Action::Startup(StartupAction::SubmitFirstTimeInput),
                crossterm::event::KeyCode::Esc => Action::Startup(StartupAction::RejectFirstTimePrompt),
                _ => Action::Noop,
            },
            ScreenId::FirstTimeJoinEmpireConfirm => {
                if self.startup_state.first_time_rename_preloaded_empire {
                    match key.code {
                        crossterm::event::KeyCode::Char('y')
                        | crossterm::event::KeyCode::Char('Y') => Action::Startup(StartupAction::AcceptFirstTimePrompt),
                        crossterm::event::KeyCode::Enter
                        | crossterm::event::KeyCode::Char('n')
                        | crossterm::event::KeyCode::Char('N')
                        | crossterm::event::KeyCode::Esc => Action::Startup(StartupAction::RejectFirstTimePrompt),
                        _ => Action::Noop,
                    }
                } else {
                    match key.code {
                        crossterm::event::KeyCode::Enter
                        | crossterm::event::KeyCode::Char('y')
                        | crossterm::event::KeyCode::Char('Y') => Action::Startup(StartupAction::AcceptFirstTimePrompt),
                        crossterm::event::KeyCode::Char('n')
                        | crossterm::event::KeyCode::Char('N')
                        | crossterm::event::KeyCode::Esc => Action::Startup(StartupAction::RejectFirstTimePrompt),
                        _ => Action::Noop,
                    }
                }
            }
            ScreenId::FirstTimeJoinSummary | ScreenId::FirstTimeJoinNoPending => match key.code {
                crossterm::event::KeyCode::Enter => Action::Startup(StartupAction::AcceptFirstTimePrompt),
                _ => Action::Noop,
            },
            ScreenId::FirstTimeHomeworldConfirm => match key.code {
                crossterm::event::KeyCode::Char('y') | crossterm::event::KeyCode::Char('Y') => {
                    Action::Startup(StartupAction::AcceptFirstTimePrompt)
                }
                crossterm::event::KeyCode::Enter
                | crossterm::event::KeyCode::Char('n')
                | crossterm::event::KeyCode::Char('N')
                | crossterm::event::KeyCode::Esc => Action::Startup(StartupAction::RejectFirstTimePrompt),
                _ => Action::Noop,
            },
            ScreenId::ColonyWorldConfirm => match key.code {
                crossterm::event::KeyCode::Enter
                | crossterm::event::KeyCode::Char('y')
                | crossterm::event::KeyCode::Char('Y') => Action::Startup(StartupAction::AcceptFirstTimePrompt),
                crossterm::event::KeyCode::Char('n')
                | crossterm::event::KeyCode::Char('N')
                | crossterm::event::KeyCode::Esc => Action::Startup(StartupAction::RejectFirstTimePrompt),
                _ => Action::Noop,
            },
            ScreenId::MainMenu => self.main_menu.handle_key(key),
            ScreenId::MainHelp => self.main_help.handle_key(key),
            ScreenId::GeneralMenu => self.general_menu.handle_key(key),
            ScreenId::GeneralHelp => self.general_help.handle_key(key),
            ScreenId::FleetHelp => self.fleet_help.handle_key(key),
            ScreenId::StarbaseMenu => self.starbase_menu.handle_key(key),
            ScreenId::StarbaseHelp => self.starbase_help.handle_key(key),
            ScreenId::StarbaseList => self.starbase_list.handle_key(key),
            ScreenId::StarbaseReviewSelect => self.handle_starbase_review_select_key(key),
            ScreenId::StarbaseReview => Action::Starbase(StarbaseAction::OpenReviewSelect),
            ScreenId::FleetMenu => self.fleet_menu.handle_key(key),
            ScreenId::FleetList(_) => self.fleet_list.handle_key(key),
            ScreenId::FleetReviewSelect => self.handle_fleet_review_select_key(key),
            ScreenId::FleetReview => self.fleet_review.handle_key(key),
            ScreenId::FleetRoeSelect => self.handle_fleet_roe_key(key),
            ScreenId::FleetOrder => self.handle_fleet_order_key(key),
            ScreenId::FleetGroupOrder => self.handle_fleet_group_order_key(key),
            ScreenId::FleetMissionPicker => self.handle_fleet_mission_picker_key(key),
            ScreenId::FleetMerge => self.handle_fleet_merge_key(key),
            ScreenId::FleetTransfer => self.handle_fleet_transfer_key(key),
            ScreenId::FleetDetach => self.handle_fleet_detach_key(key),
            ScreenId::FleetEta => self.handle_fleet_eta_key(key),
            ScreenId::PlanetMenu => self.planet_menu.handle_key(key),
            ScreenId::PlanetHelp => self.planet_help.handle_key(key),
            ScreenId::PlanetAutoCommissionConfirm => self.planet_auto_commission.handle_key(key),
            ScreenId::PlanetAutoCommissionDone => Action::Planet(PlanetAction::OpenMenu),
            ScreenId::PlanetCommissionMenu => self.planet_commission.handle_key(key),
            ScreenId::PlanetTransportPlanetSelect(_) => {
                self.planet_transport.handle_planet_key(key)
            }
            ScreenId::PlanetTransportFleetSelect(_) => self.planet_transport.handle_fleet_key(key),
            ScreenId::PlanetTransportQuantityPrompt(_) => {
                self.planet_transport.handle_quantity_key(key)
            }
            ScreenId::PlanetTransportDone(_) => Action::Planet(PlanetAction::OpenMenu),
            ScreenId::PlanetBuildHelp => self.build_help.handle_key(key),
            ScreenId::PlanetBuildMenu => self.planet_build.handle_menu_key(key),
            ScreenId::PlanetBuildReview => self.planet_build.handle_review_key(key),
            ScreenId::PlanetBuildList => self
                .planet_build
                .handle_list_key(key, self.planet.build_list_confirming),
            ScreenId::PlanetBuildChange => self.planet_build.handle_change_key(key),
            ScreenId::PlanetBuildAbortConfirm => self.planet_build.handle_abort_key(key),
            ScreenId::PlanetBuildSpecify => self.planet_build.handle_specify_key(key),
            ScreenId::PlanetBuildQuantity => self.planet_build.handle_quantity_key(key),
            ScreenId::PlanetListSortPrompt(PlanetListMode::Stub(_)) => Action::Planet(PlanetAction::OpenMenu),
            ScreenId::PlanetListSortPrompt(_) => self.planet_list.handle_sort_prompt_key(key),
            ScreenId::PlanetBriefList(_) => self.planet_list.handle_brief_key(key),
            ScreenId::PlanetDetailList(_) => self.planet_list.handle_detail_key(key),
            ScreenId::PlanetTaxPrompt => self.planet_tax.handle_prompt_key(key),
            ScreenId::PlanetTaxDone => self.planet_tax.handle_done_key(key),
            ScreenId::Starmap if self.starmap_state.capture_complete => {
                self.starmap.handle_complete_key(key)
            }
            ScreenId::Starmap if self.starmap_state.dump_active => self.starmap.handle_dump_key(key),
            ScreenId::Starmap => self.starmap.handle_prompt_key(key),
            ScreenId::PartialStarmapPrompt => self.partial_starmap.handle_prompt_key(key),
            ScreenId::PartialStarmapView => self.partial_starmap.handle_view_key(key),
            ScreenId::PlanetDatabaseList => self.planet_database.handle_list_key(key),
            ScreenId::PlanetDatabaseDetail => self.planet_database.handle_detail_key(key),
            ScreenId::PlanetInfoPrompt => self.handle_planet_info_prompt_key(key),
            ScreenId::PlanetInfoDetail => self.planet_info.handle_detail_key(key),
            ScreenId::Enemies => self.enemies.handle_key(key),
            ScreenId::DeleteReviewables => self.delete_reviewables.handle_key(key),
            ScreenId::ComposeMessageRecipient => self.message_compose.handle_recipient_key(key),
            ScreenId::ComposeMessageSubject => self.message_compose.handle_subject_key(key),
            ScreenId::ComposeMessageBody => self.message_compose.handle_body_key(key),
            ScreenId::ComposeMessageOutbox => self.message_compose.handle_outbox_key(key),
            ScreenId::ComposeMessageDiscardConfirm => {
                self.message_compose.handle_discard_confirm_key(key)
            }
            ScreenId::ComposeMessageSendConfirm => {
                self.message_compose.handle_send_confirm_key(key)
            }
            ScreenId::ComposeMessageSent => self.message_compose.handle_sent_key(key),
            ScreenId::EmpireStatus => self.empire_status.handle_key(key),
            ScreenId::EmpireProfile => self.empire_profile.handle_key(key),
            ScreenId::Rankings(_) => self.rankings.handle_key(key),
            ScreenId::Reports => self.reports.handle_key(key),
        }
    }

    pub fn return_to_command_menu(&mut self) {
        self.current_screen = match self.command_return_menu {
            CommandMenu::Main => ScreenId::MainMenu,
            CommandMenu::General => ScreenId::GeneralMenu,
            CommandMenu::Fleet => ScreenId::FleetMenu,
            CommandMenu::Starbase => ScreenId::StarbaseMenu,
            CommandMenu::Planet => ScreenId::PlanetMenu,
            CommandMenu::PlanetBuild => ScreenId::PlanetBuildMenu,
        };
    }

    pub(super) fn origin_command_menu(&self) -> CommandMenu {
        match self.current_screen {
            ScreenId::MainMenu
            | ScreenId::MainHelp
            | ScreenId::PlanetDatabaseList
            | ScreenId::PlanetDatabaseDetail => CommandMenu::Main,
            ScreenId::FleetHelp
            | ScreenId::FleetMenu
            | ScreenId::FleetList(_)
            | ScreenId::FleetReviewSelect
            | ScreenId::FleetReview
            | ScreenId::FleetRoeSelect
            | ScreenId::FleetOrder
            | ScreenId::FleetGroupOrder
            | ScreenId::FleetMissionPicker
            | ScreenId::FleetMerge
            | ScreenId::FleetTransfer
            | ScreenId::FleetDetach
            | ScreenId::FleetEta => CommandMenu::Fleet,
            ScreenId::StarbaseMenu
            | ScreenId::StarbaseHelp
            | ScreenId::StarbaseList
            | ScreenId::StarbaseReviewSelect
            | ScreenId::StarbaseReview => CommandMenu::Starbase,
            ScreenId::GeneralMenu
            | ScreenId::GeneralHelp
            | ScreenId::Enemies
            | ScreenId::DeleteReviewables
            | ScreenId::ComposeMessageRecipient
            | ScreenId::ComposeMessageSubject
            | ScreenId::ComposeMessageBody
            | ScreenId::ComposeMessageOutbox
            | ScreenId::ComposeMessageDiscardConfirm
            | ScreenId::ComposeMessageSendConfirm
            | ScreenId::ComposeMessageSent
            | ScreenId::EmpireStatus
            | ScreenId::EmpireProfile
            | ScreenId::Rankings(_)
            | ScreenId::Reports
            | ScreenId::Starmap => CommandMenu::General,
            ScreenId::PlanetMenu
            | ScreenId::PlanetHelp
            | ScreenId::PlanetAutoCommissionConfirm
            | ScreenId::PlanetAutoCommissionDone
            | ScreenId::PlanetCommissionMenu
            | ScreenId::PlanetListSortPrompt(_)
            | ScreenId::PlanetBriefList(_)
            | ScreenId::PlanetDetailList(_)
            | ScreenId::PlanetTaxPrompt
            | ScreenId::PlanetTaxDone
            | ScreenId::PlanetTransportPlanetSelect(_)
            | ScreenId::PlanetTransportFleetSelect(_)
            | ScreenId::PlanetTransportQuantityPrompt(_)
            | ScreenId::PlanetTransportDone(_) => CommandMenu::Planet,
            ScreenId::PlanetBuildMenu
            | ScreenId::PlanetBuildHelp
            | ScreenId::PlanetBuildReview
            | ScreenId::PlanetBuildList
            | ScreenId::PlanetBuildChange
            | ScreenId::PlanetBuildAbortConfirm
            | ScreenId::PlanetBuildSpecify
            | ScreenId::PlanetBuildQuantity => CommandMenu::PlanetBuild,
            ScreenId::Startup(_)
            | ScreenId::FirstTimeMenu
            | ScreenId::FirstTimeHelp
            | ScreenId::FirstTimeEmpires
            | ScreenId::FirstTimeIntro
            | ScreenId::FirstTimePreloadedRenamePrompt
            | ScreenId::FirstTimeJoinEmpireName
            | ScreenId::FirstTimeJoinEmpireConfirm
            | ScreenId::FirstTimeJoinSummary
            | ScreenId::FirstTimeJoinNoPending
            | ScreenId::FirstTimeHomeworldName
            | ScreenId::FirstTimeHomeworldConfirm
            | ScreenId::ColonyWorldName
            | ScreenId::ColonyWorldConfirm
            | ScreenId::PartialStarmapPrompt
            | ScreenId::PartialStarmapView
            | ScreenId::PlanetInfoPrompt
            | ScreenId::PlanetInfoDetail => self.command_return_menu,
        }
    }

    pub(crate) fn status_if_no_modal<'a>(&self, status: Option<&'a str>) -> Option<&'a str> {
        if self.current_modal_notice().is_some() {
            None
        } else {
            status
        }
    }

    fn current_modal_notice(&self) -> Option<&str> {
        match self.current_screen {
            ScreenId::StarbaseReviewSelect => self.starbase.review_status.as_deref(),
            ScreenId::FleetReviewSelect => self.fleet.review_status.as_deref(),
            ScreenId::FleetRoeSelect => self.fleet.roe_status.as_deref(),
            ScreenId::FleetOrder => self.fleet.order_status.as_deref(),
            ScreenId::FleetGroupOrder => self.fleet.group_status.as_deref(),
            ScreenId::FleetMissionPicker => self.fleet.mission_picker_status.as_deref(),
            ScreenId::FleetMerge => self.fleet.merge_status.as_deref(),
            ScreenId::FleetTransfer => self.fleet.transfer_status.as_deref(),
            ScreenId::FleetDetach => self.fleet.detach_status.as_deref(),
            ScreenId::FleetEta if self.fleet.eta_mode != FleetEtaMode::ShowingResult => {
                self.fleet.eta_status.as_deref()
            }
            ScreenId::PlanetDatabaseList => self.planet.database_status.as_deref(),
            ScreenId::PlanetTransportPlanetSelect(_)
            | ScreenId::PlanetTransportFleetSelect(_)
            | ScreenId::PlanetTransportQuantityPrompt(_) => self.planet.transport_status.as_deref(),
            ScreenId::Enemies => self.empire.enemies_status.as_deref(),
            ScreenId::ComposeMessageRecipient => self.messaging.compose_recipient_status.as_deref(),
            ScreenId::ComposeMessageOutbox => self.messaging.compose_outbox_status.as_deref(),
            _ => None,
        }
    }

    pub fn dismiss_modal_notice(&mut self) {
        match self.current_screen {
            ScreenId::StarbaseReviewSelect => {
                self.starbase.review_status = None;
                self.starbase.review_input.clear();
            }
            ScreenId::FleetReviewSelect => {
                self.fleet.review_status = None;
                self.fleet.review_select_input.clear();
            }
            ScreenId::FleetRoeSelect => {
                self.fleet.roe_status = None;
                if self.fleet.roe_editing {
                    self.fleet.roe_input.clear();
                } else {
                    self.fleet.roe_select_input.clear();
                }
            }
            ScreenId::FleetOrder => {
                self.fleet.order_status = None;
                self.fleet.order_input.clear();
            }
            ScreenId::FleetGroupOrder => {
                self.fleet.group_status = None;
                self.fleet.group_input.clear();
            }
            ScreenId::FleetMissionPicker => {
                self.fleet.mission_picker_status = None;
                self.fleet.mission_picker_input.clear();
            }
            ScreenId::FleetMerge => {
                self.fleet.merge_status = None;
                match self.fleet.merge_mode {
                    FleetMergeMode::SelectingSource => self.fleet.merge_source_input.clear(),
                    FleetMergeMode::SelectingHost => self.fleet.merge_host_input.clear(),
                }
            }
            ScreenId::FleetTransfer => {
                self.fleet.transfer_status = None;
                if self.fleet.transfer_mode == FleetTransferMode::SelectingFleets {
                    self.fleet.transfer_select_input.clear();
                } else {
                    self.fleet.transfer_input.clear();
                }
            }
            ScreenId::FleetDetach => {
                self.fleet.detach_status = None;
                if self.fleet.detach_mode == FleetDetachMode::SelectingFleet {
                    self.fleet.detach_select_input.clear();
                } else {
                    self.fleet.detach_input.clear();
                }
            }
            ScreenId::FleetEta => {
                self.fleet.eta_status = None;
                match self.fleet.eta_mode {
                    FleetEtaMode::SelectingFleet => self.fleet.eta_select_input.clear(),
                    FleetEtaMode::EnteringDestination => self.fleet.eta_destination_input.clear(),
                    FleetEtaMode::ConfirmingSystemEntry => {
                        self.fleet.eta_include_system_input.clear()
                    }
                    FleetEtaMode::ShowingResult => {}
                }
            }
            ScreenId::PlanetDatabaseList => {
                self.planet.database_status = None;
                self.planet.database_input.clear();
            }
            ScreenId::PlanetTransportPlanetSelect(_) => {
                self.planet.transport_status = None;
                self.planet.transport_planet_input.clear();
            }
            ScreenId::PlanetTransportFleetSelect(_)
            | ScreenId::PlanetTransportQuantityPrompt(_) => {
                self.planet.transport_status = None;
                self.planet.transport_qty_input.clear();
            }
            ScreenId::Enemies => {
                self.empire.enemies_status = None;
                self.empire.enemies_input.clear();
            }
            ScreenId::ComposeMessageRecipient => {
                self.messaging.compose_recipient_status = None;
                self.messaging.compose_recipient_input.clear();
            }
            ScreenId::ComposeMessageOutbox => {
                self.messaging.compose_outbox_status = None;
                self.messaging.compose_outbox_input.clear();
            }
            _ => {}
        }
    }

    pub(super) fn save_game_data(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.planet.campaign_store.save_runtime_state(
            &self.game_data,
            &self.database,
            &self.results_bytes,
            &self.messages_bytes,
            &self.queued_mail,
        )?;
        self.planet_intel_snapshots = self
            .planet.campaign_store
            .latest_planet_intel_for_viewer(self.player.record_index_1_based as u8)?
            .into_iter()
            .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
            .collect::<BTreeMap<_, _>>();
        Ok(())
    }
}


fn ordinal_number(value: usize) -> String {
    let suffix = match value % 100 {
        11..=13 => "th",
        _ => match value % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    };
    format!("{value}{suffix}")
}
