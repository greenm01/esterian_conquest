use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use ec_data::{
    AutoCommissionSummary, CampaignStore, CommissionResult, CoreGameData, DatabaseDat,
    FleetDetachSelection, GameStateMutationError, PlanetIntelSnapshot, PlayerStarmapWorld,
    ProductionItemKind, QueuedPlayerMail, build_player_starmap_projection, plan_route,
};

use crate::app::Action;
use crate::model::{MainMenuSummary, PlayerContext, ReviewSummary};
use crate::reports::{ReportsPreview, clear_report_bytes};
use crate::screen::{
    BuildHelpScreen, CommandMenu, DeleteReviewablesScreen, EmpireProfileScreen, EmpireStatusScreen,
    EnemiesScreen, FIRST_TIME_INTRO_PAGE_COUNT, FirstTimeEmpiresScreen, FirstTimeHelpScreen,
    FirstTimeIntroScreen, FirstTimeMenuScreen, FleetDetachMode, FleetDetachScreen, FleetEtaMode,
    FleetEtaScreen, FleetHelpScreen, FleetListMode, FleetListScreen, FleetMenuScreen,
    FleetReviewScreen, FleetRoeScreen, FleetRow, GeneralHelpScreen, GeneralMenuScreen,
    MainMenuScreen, MessageComposeScreen, PartialStarmapScreen, PlanetAutoCommissionScreen,
    PlanetBuildChangeRow, PlanetBuildListRow, PlanetBuildMenuView, PlanetBuildOrder,
    PlanetBuildScreen, PlanetCommissionRow, PlanetCommissionScreen, PlanetCommissionView,
    PlanetDatabaseRow, PlanetDatabaseScreen, PlanetHelpScreen, PlanetInfoScreen, PlanetListMode,
    PlanetListScreen, PlanetListSort, PlanetMenuScreen, PlanetTaxScreen, PlanetTransportFleetRow,
    PlanetTransportMode, PlanetTransportPlanetRow, PlanetTransportScreen, RankingsScreen,
    ReportsScreen, STARTUP_SPLASH_PAGE_COUNT, Screen, ScreenFrame, ScreenId, StarmapScreen,
    StartupScreen, build_unit_spec, build_unit_spec_by_kind, max_quantity,
    render_first_time_homeworld_confirm, render_first_time_homeworld_name,
    render_first_time_join_name, render_first_time_join_name_confirm,
    render_first_time_join_no_pending, render_first_time_join_summary,
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
    game_dir: PathBuf,
    game_data: CoreGameData,
    database: DatabaseDat,
    player: PlayerContext,
    current_screen: ScreenId,
    startup_sequence: StartupSequence,
    startup: StartupScreen,
    first_time_menu: FirstTimeMenuScreen,
    first_time_help: FirstTimeHelpScreen,
    first_time_empires: FirstTimeEmpiresScreen,
    first_time_intro: FirstTimeIntroScreen,
    main_menu: MainMenuScreen,
    general_menu: GeneralMenuScreen,
    general_help: GeneralHelpScreen,
    fleet_help: FleetHelpScreen,
    fleet_menu: FleetMenuScreen,
    fleet_list: FleetListScreen,
    fleet_review: FleetReviewScreen,
    fleet_roe: FleetRoeScreen,
    fleet_detach: FleetDetachScreen,
    fleet_eta: FleetEtaScreen,
    planet_menu: PlanetMenuScreen,
    planet_help: PlanetHelpScreen,
    planet_auto_commission: PlanetAutoCommissionScreen,
    planet_commission: PlanetCommissionScreen,
    planet_transport: PlanetTransportScreen,
    build_help: BuildHelpScreen,
    planet_build: PlanetBuildScreen,
    planet_list: PlanetListScreen,
    planet_tax: PlanetTaxScreen,
    starmap: StarmapScreen,
    partial_starmap: PartialStarmapScreen,
    planet_database: PlanetDatabaseScreen,
    planet_info: PlanetInfoScreen,
    enemies: EnemiesScreen,
    delete_reviewables: DeleteReviewablesScreen,
    message_compose: MessageComposeScreen,
    empire_status: EmpireStatusScreen,
    empire_profile: EmpireProfileScreen,
    rankings: RankingsScreen,
    reports: ReportsScreen,
    planet_info_input: String,
    planet_info_error: Option<String>,
    planet_info_selected: Option<usize>,
    partial_starmap_input: String,
    partial_starmap_error: Option<String>,
    partial_starmap_center: [u8; 2],
    command_return_menu: CommandMenu,
    enemies_input: String,
    enemies_status: Option<String>,
    enemies_scroll_offset: usize,
    enemies_cursor: usize,
    delete_reviewables_status: Option<String>,
    compose_recipient_input: String,
    compose_recipient_status: Option<String>,
    compose_recipient_scroll_offset: usize,
    compose_recipient_cursor: usize,
    compose_recipient_empire: Option<u8>,
    compose_subject: String,
    compose_subject_status: Option<String>,
    compose_body: String,
    compose_body_cursor: usize,
    compose_body_status: Option<String>,
    compose_outbox_input: String,
    compose_outbox_status: Option<String>,
    compose_outbox_scroll_offset: usize,
    compose_outbox_cursor: usize,
    compose_sent_status: Option<String>,
    planet_list_sort_status: Option<String>,
    fleet_list_mode: FleetListMode,
    fleet_scroll_offset: usize,
    fleet_cursor: usize,
    fleet_review_index: usize,
    fleet_roe_scroll_offset: usize,
    fleet_roe_cursor: usize,
    fleet_roe_editing: bool,
    fleet_roe_select_input: String,
    fleet_roe_input: String,
    fleet_roe_status: Option<String>,
    fleet_detach_scroll_offset: usize,
    fleet_detach_cursor: usize,
    fleet_detach_mode: FleetDetachMode,
    fleet_detach_select_input: String,
    fleet_detach_input: String,
    fleet_detach_status: Option<String>,
    fleet_detach_selection: FleetDetachSelection,
    fleet_detach_donor_speed: Option<u8>,
    fleet_eta_scroll_offset: usize,
    fleet_eta_cursor: usize,
    fleet_eta_mode: FleetEtaMode,
    fleet_eta_select_input: String,
    fleet_eta_destination_input: String,
    fleet_eta_include_system_input: String,
    fleet_eta_status: Option<String>,
    planet_brief_scroll_offset: usize,
    planet_brief_cursor: usize,
    planet_detail_index: usize,
    planet_database_scroll_offset: usize,
    planet_database_cursor: usize,
    planet_database_detail_index: usize,
    planet_database_input: String,
    planet_database_status: Option<String>,
    planet_commission_index: usize,
    planet_commission_cursor: usize,
    planet_commission_scroll_offset: usize,
    planet_commission_selected_slots: BTreeSet<usize>,
    planet_commission_status: Option<String>,
    planet_auto_commission_status: Option<String>,
    planet_transport_mode: Option<PlanetTransportMode>,
    planet_transport_planet_cursor: usize,
    planet_transport_planet_scroll_offset: usize,
    planet_transport_selected_planet_record: Option<usize>,
    planet_transport_fleet_cursor: usize,
    planet_transport_fleet_scroll_offset: usize,
    planet_transport_qty_input: String,
    planet_transport_status: Option<String>,
    planet_build_index: usize,
    planet_build_status: Option<String>,
    planet_build_unit_input: String,
    planet_build_unit_status: Option<String>,
    planet_build_quantity_input: String,
    planet_build_quantity_status: Option<String>,
    planet_build_selected_kind: Option<ProductionItemKind>,
    planet_build_list_scroll_offset: usize,
    planet_build_list_cursor: usize,
    planet_build_list_confirming: bool,
    planet_build_change_cursor: usize,
    planet_build_change_scroll_offset: usize,
    planet_tax_input: String,
    planet_tax_status: Option<String>,
    starmap_view_x: usize,
    starmap_view_y: usize,
    starmap_status: Option<String>,
    starmap_dump_lines: Vec<String>,
    starmap_dump_offset: usize,
    starmap_dump_active: bool,
    starmap_capture_complete: bool,
    export_root: PathBuf,
    queue_dir: Option<PathBuf>,
    campaign_store: CampaignStore,
    planet_intel_snapshots: BTreeMap<usize, PlanetIntelSnapshot>,
    results_bytes: Vec<u8>,
    messages_bytes: Vec<u8>,
    queued_mail: Vec<QueuedPlayerMail>,
    startup_splash_page: usize,
    startup_intro_page: usize,
    first_time_intro_page: usize,
    first_time_status: Option<String>,
    first_time_input: String,
    first_time_empire_name: String,
    first_time_homeworld_name: String,
    command_menu_notice: Option<String>,
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
        let pending_results = !runtime_state.results_bytes.is_empty();
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
            pending_results,
        );
        let review_summary = ReviewSummary::from_main_menu(&main_menu_summary);
        let startup_summary = StartupSummary::from_reports(
            main_menu_summary.game_year,
            main_menu_summary.pending_results,
            main_menu_summary.pending_messages,
            &reports,
        );
        let startup_sequence = StartupSequence::new(&startup_summary, player.is_joined);
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
            general_menu: GeneralMenuScreen::new(),
            general_help: GeneralHelpScreen::new(),
            fleet_help: FleetHelpScreen::new(),
            fleet_menu: FleetMenuScreen::new(),
            fleet_list: FleetListScreen::new(),
            fleet_review: FleetReviewScreen::new(),
            fleet_roe: FleetRoeScreen::new(),
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
            planet_info_input: String::new(),
            planet_info_error: None,
            planet_info_selected: None,
            partial_starmap_input: "8,2".to_string(),
            partial_starmap_error: None,
            partial_starmap_center: [8, 2],
            command_return_menu: CommandMenu::General,
            enemies_input: String::new(),
            enemies_status: None,
            enemies_scroll_offset: 0,
            enemies_cursor: 0,
            delete_reviewables_status: None,
            compose_recipient_input: String::new(),
            compose_recipient_status: None,
            compose_recipient_scroll_offset: 0,
            compose_recipient_cursor: 0,
            compose_recipient_empire: None,
            compose_subject: String::new(),
            compose_subject_status: None,
            compose_body: String::new(),
            compose_body_cursor: 0,
            compose_body_status: None,
            compose_outbox_input: String::new(),
            compose_outbox_status: None,
            compose_outbox_scroll_offset: 0,
            compose_outbox_cursor: 0,
            compose_sent_status: None,
            planet_list_sort_status: None,
            fleet_list_mode: FleetListMode::Brief,
            fleet_scroll_offset: 0,
            fleet_cursor: 0,
            fleet_review_index: 0,
            fleet_roe_scroll_offset: 0,
            fleet_roe_cursor: 0,
            fleet_roe_editing: false,
            fleet_roe_select_input: String::new(),
            fleet_roe_input: String::new(),
            fleet_roe_status: None,
            fleet_detach_scroll_offset: 0,
            fleet_detach_cursor: 0,
            fleet_detach_mode: FleetDetachMode::SelectingFleet,
            fleet_detach_select_input: String::new(),
            fleet_detach_input: String::new(),
            fleet_detach_status: None,
            fleet_detach_selection: FleetDetachSelection::default(),
            fleet_detach_donor_speed: None,
            fleet_eta_scroll_offset: 0,
            fleet_eta_cursor: 0,
            fleet_eta_mode: FleetEtaMode::SelectingFleet,
            fleet_eta_select_input: String::new(),
            fleet_eta_destination_input: String::new(),
            fleet_eta_include_system_input: String::new(),
            fleet_eta_status: None,
            planet_brief_scroll_offset: 0,
            planet_brief_cursor: 0,
            planet_detail_index: 0,
            planet_database_scroll_offset: 0,
            planet_database_cursor: 0,
            planet_database_detail_index: 0,
            planet_database_input: String::new(),
            planet_database_status: None,
            planet_commission_index: 0,
            planet_commission_cursor: 0,
            planet_commission_scroll_offset: 0,
            planet_commission_selected_slots: BTreeSet::new(),
            planet_commission_status: None,
            planet_auto_commission_status: None,
            planet_transport_mode: None,
            planet_transport_planet_cursor: 0,
            planet_transport_planet_scroll_offset: 0,
            planet_transport_selected_planet_record: None,
            planet_transport_fleet_cursor: 0,
            planet_transport_fleet_scroll_offset: 0,
            planet_transport_qty_input: String::new(),
            planet_transport_status: None,
            planet_build_index: 0,
            planet_build_status: None,
            planet_build_unit_input: String::new(),
            planet_build_unit_status: None,
            planet_build_quantity_input: String::new(),
            planet_build_quantity_status: None,
            planet_build_selected_kind: None,
            planet_build_list_scroll_offset: 0,
            planet_build_list_cursor: 0,
            planet_build_list_confirming: false,
            planet_build_change_cursor: 0,
            planet_build_change_scroll_offset: 0,
            planet_tax_input: "50".to_string(),
            planet_tax_status: None,
            starmap_view_x: 1,
            starmap_view_y: 1,
            starmap_status: None,
            starmap_dump_lines: Vec::new(),
            starmap_dump_offset: 0,
            starmap_dump_active: false,
            starmap_capture_complete: false,
            export_root,
            queue_dir: config.queue_dir,
            campaign_store,
            planet_intel_snapshots,
            results_bytes,
            messages_bytes,
            queued_mail,
            startup_splash_page: 0,
            startup_intro_page: 0,
            first_time_intro_page: 0,
            first_time_status: None,
            first_time_input: String::new(),
            first_time_empire_name: String::new(),
            first_time_homeworld_name: String::new(),
            command_menu_notice: None,
        })
    }

    pub fn render(
        &mut self,
        terminal: &mut dyn Terminal,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let frame = ScreenFrame {
            game_dir: &self.game_dir,
            game_data: &self.game_data,
            database: &self.database,
            player: &self.player,
            planet_intel_snapshots: &self.planet_intel_snapshots,
        };

        let playfield = match self.current_screen {
            ScreenId::Startup(phase) => self.startup.render_phase(
                &frame,
                phase,
                self.startup_splash_page,
                self.startup_intro_page,
            )?,
            ScreenId::FirstTimeMenu => self
                .first_time_menu
                .render(self.first_time_status.as_deref())?,
            ScreenId::FirstTimeHelp => self.first_time_help.render(&frame)?,
            ScreenId::FirstTimeEmpires => self
                .first_time_empires
                .render_rows(&self.first_time_empire_rows())?,
            ScreenId::FirstTimeIntro => self
                .first_time_intro
                .render_page(self.first_time_intro_page)?,
            ScreenId::FirstTimeJoinEmpireName => render_first_time_join_name(
                &self.first_time_input,
                self.first_time_status.as_deref(),
            )?,
            ScreenId::FirstTimeJoinEmpireConfirm => {
                render_first_time_join_name_confirm(&self.first_time_empire_name)?
            }
            ScreenId::FirstTimeJoinSummary => render_first_time_join_summary(
                &self.first_time_empire_name,
                self.player.record_index_1_based,
                self.game_data.conquest.game_year(),
            )?,
            ScreenId::FirstTimeJoinNoPending => render_first_time_join_no_pending()?,
            ScreenId::FirstTimeHomeworldName => {
                let (coords, present, potential) = self.first_time_homeworld_summary()?;
                render_first_time_homeworld_name(
                    coords,
                    present,
                    potential,
                    &self.first_time_input,
                    self.first_time_status.as_deref(),
                )?
            }
            ScreenId::FirstTimeHomeworldConfirm => {
                let (coords, present, potential) = self.first_time_homeworld_summary()?;
                render_first_time_homeworld_confirm(
                    coords,
                    present,
                    potential,
                    &self.first_time_homeworld_name,
                )?
            }
            ScreenId::MainMenu => self
                .main_menu
                .render_with_notice(self.command_menu_notice.as_deref())?,
            ScreenId::GeneralMenu => self
                .general_menu
                .render_with_notice(&frame, self.command_menu_notice.as_deref())?,
            ScreenId::GeneralHelp => self.general_help.render(&frame)?,
            ScreenId::FleetHelp => self.fleet_help.render(&frame)?,
            ScreenId::FleetMenu => self
                .fleet_menu
                .render_with_notice(self.command_menu_notice.as_deref())?,
            ScreenId::FleetList(mode) => self.fleet_list.render(
                mode,
                &self.fleet_rows(),
                self.fleet_scroll_offset,
                self.fleet_cursor,
            )?,
            ScreenId::FleetReview => {
                let rows = self.fleet_rows();
                let row = rows
                    .get(self.fleet_review_index)
                    .ok_or("fleet review row missing")?;
                self.fleet_review
                    .render(row, self.fleet_review_index, rows.len())?
            }
            ScreenId::FleetRoeSelect => self.fleet_roe.render_select(
                &self.fleet_rows(),
                self.fleet_roe_scroll_offset,
                self.fleet_roe_cursor,
                self.fleet_roe_editing,
                &self.fleet_roe_select_input,
                &self.fleet_roe_input,
                self.fleet_roe_status.as_deref(),
            )?,
            ScreenId::FleetDetach => {
                let rows = self.fleet_rows();
                let (prompt, default) = self.fleet_detach_prompt_and_default(&rows);
                let input = self.fleet_detach_current_input().to_string();
                let status = self.fleet_detach_status.clone();
                self.fleet_detach.render(
                    &rows,
                    self.fleet_detach_scroll_offset,
                    self.fleet_detach_cursor,
                    &prompt,
                    &default,
                    &input,
                    status.as_deref(),
                )?
            }
            ScreenId::FleetEta => self.fleet_eta.render(
                &self.fleet_rows(),
                self.fleet_eta_scroll_offset,
                self.fleet_eta_cursor,
                self.fleet_eta_mode,
                &self.fleet_eta_select_input,
                self.fleet_eta_default_destination(),
                &self.fleet_eta_destination_input,
                &self.fleet_eta_include_system_input,
                self.fleet_eta_status.as_deref(),
            )?,
            ScreenId::PlanetMenu => self
                .planet_menu
                .render_with_notice(self.command_menu_notice.as_deref())?,
            ScreenId::PlanetHelp => self.planet_help.render(&frame)?,
            ScreenId::PlanetAutoCommissionConfirm => {
                self.planet_auto_commission.render_confirm()?
            }
            ScreenId::PlanetAutoCommissionDone => self.planet_auto_commission.render_done(
                self.planet_auto_commission_status
                    .as_deref()
                    .unwrap_or("Auto-commission complete."),
            )?,
            ScreenId::PlanetTransportPlanetSelect(mode) => {
                self.planet_transport.render_planet_select(
                    mode,
                    &self.planet_transport_planet_rows(mode),
                    self.planet_transport_planet_scroll_offset,
                    self.planet_transport_planet_cursor,
                    self.planet_transport_status.as_deref(),
                )?
            }
            ScreenId::PlanetTransportFleetSelect(mode) => {
                self.planet_transport.render_fleet_select(
                    mode,
                    &self.current_planet_transport_planet_row(mode)?,
                    &self.current_planet_transport_fleet_rows(mode)?,
                    self.planet_transport_fleet_scroll_offset,
                    self.planet_transport_fleet_cursor,
                    &self.planet_transport_qty_input,
                    self.planet_transport_status.as_deref(),
                )?
            }
            ScreenId::PlanetTransportQuantityPrompt(mode) => {
                self.planet_transport.render_quantity_prompt(
                    mode,
                    &self.current_planet_transport_planet_row(mode)?,
                    &self.current_planet_transport_fleet_row(mode)?,
                    &self.planet_transport_qty_input,
                    self.planet_transport_status.as_deref(),
                )?
            }
            ScreenId::PlanetTransportDone(mode) => self.planet_transport.render_done(
                mode,
                self.planet_transport_status
                    .as_deref()
                    .unwrap_or("Transport order completed."),
            )?,
            ScreenId::PlanetCommissionMenu => self.planet_commission.render_menu(
                &self.current_planet_commission_view()?,
                self.planet_commission_scroll_offset,
                self.planet_commission_cursor,
                &self.planet_commission_selected_slots,
                self.planet_commission_status.as_deref(),
            )?,
            ScreenId::PlanetBuildHelp => self.build_help.render(&frame)?,
            ScreenId::PlanetBuildMenu => self.planet_build.render_menu(
                &self.current_planet_build_view()?,
                self.planet_build_status.as_deref(),
            )?,
            ScreenId::PlanetBuildReview => self.planet_build.render_review(
                &self.current_planet_build_view()?,
                &self.current_planet_build_orders(),
            )?,
            ScreenId::PlanetBuildList => self.planet_build.render_list(
                &self.current_planet_build_view()?,
                &self.planet_build_list_rows(),
                self.planet_build_list_scroll_offset,
                self.planet_build_list_cursor,
                self.planet_build_list_confirming,
            )?,
            ScreenId::PlanetBuildChange => self.planet_build.render_change(
                &self.build_change_rows(),
                self.planet_build_change_scroll_offset,
                self.planet_build_change_cursor,
            )?,
            ScreenId::PlanetBuildAbortConfirm => self.planet_build.render_abort_confirm(
                &self.current_planet_build_view()?,
                &self.current_planet_build_orders(),
            )?,
            ScreenId::PlanetBuildSpecify => self.planet_build.render_specify(
                &self.current_planet_build_view()?,
                &self.current_planet_build_orders(),
                &self.planet_build_unit_input,
                self.planet_build_unit_status.as_deref(),
            )?,
            ScreenId::PlanetBuildQuantity => self.planet_build.render_quantity_prompt(
                &self.current_planet_build_view()?,
                &self.current_planet_build_orders(),
                build_unit_spec_by_kind(
                    self.planet_build_selected_kind
                        .ok_or("planet build kind not selected")?,
                )
                .ok_or("planet build unit missing")?,
                self.current_planet_build_max_quantity()?,
                &self.planet_build_quantity_input,
                self.planet_build_quantity_status.as_deref(),
            )?,
            ScreenId::PlanetListSortPrompt(mode) => self
                .planet_list
                .render_sort_prompt(mode, self.planet_list_sort_status.as_deref())?,
            ScreenId::PlanetBriefList(sort) => self.planet_list.render_brief_list(
                &self.sorted_planet_rows(sort),
                sort,
                self.planet_brief_scroll_offset,
                self.planet_brief_cursor,
            )?,
            ScreenId::PlanetDetailList(sort) => self.planet_list.render_detail(
                &frame,
                &self.sorted_planet_rows(sort),
                self.planet_detail_index,
            )?,
            ScreenId::PlanetTaxPrompt => {
                let current_tax = self.game_data.player.records
                    [self.player.record_index_1_based - 1]
                    .tax_rate()
                    .to_string();
                self.planet_tax.render_prompt(
                    &current_tax,
                    &self.planet_tax_input,
                    self.planet_tax_status.as_deref(),
                )?
            }
            ScreenId::PlanetTaxDone => self.planet_tax.render_done(
                self.planet_tax_status
                    .as_deref()
                    .unwrap_or("Tax rate updated."),
            )?,
            ScreenId::Starmap if self.starmap_capture_complete => self.starmap.render_complete()?,
            ScreenId::Starmap if self.starmap_dump_active => self
                .starmap
                .render_dump_page(&self.starmap_dump_lines, self.starmap_dump_offset)?,
            ScreenId::Starmap => self.starmap.render_prompt(self.starmap_status.as_deref())?,
            ScreenId::PartialStarmapPrompt => self.partial_starmap.render_prompt(
                self.partial_starmap_center,
                &self.partial_starmap_input,
                self.partial_starmap_error.as_deref(),
                self.command_return_menu,
            )?,
            ScreenId::PartialStarmapView => self.partial_starmap.render_view(
                &frame,
                &self.database,
                self.partial_starmap_center,
            )?,
            ScreenId::PlanetDatabaseList => self.planet_database.render_list(
                &self.planet_database_rows(),
                self.planet_database_scroll_offset,
                self.planet_database_cursor,
                self.default_planet_prompt_coords(),
                &self.planet_database_input,
                self.planet_database_status.as_deref(),
                self.command_return_menu,
            )?,
            ScreenId::PlanetDatabaseDetail => {
                let rows = self.planet_database_rows();
                let row = rows
                    .get(self.planet_database_detail_index)
                    .ok_or("planet database row missing")?;
                self.planet_database.render_detail(
                    row,
                    self.planet_database_detail_index,
                    rows.len(),
                )?
            }
            ScreenId::PlanetInfoPrompt => self.planet_info.render_prompt(
                self.default_planet_prompt_coords(),
                &self.planet_info_input,
                self.planet_info_error.as_deref(),
                self.command_return_menu,
            )?,
            ScreenId::PlanetInfoDetail => self.planet_info.render_detail(
                &frame,
                self.planet_info_selected
                    .ok_or("planet info detail not selected")?,
                self.command_return_menu,
            )?,
            ScreenId::Enemies => self.enemies.render(
                &frame,
                &self.enemies_input,
                self.enemies_status.as_deref(),
                self.enemies_scroll_offset,
                self.enemies_cursor,
            )?,
            ScreenId::DeleteReviewables => self
                .delete_reviewables
                .render(self.delete_reviewables_status.as_deref())?,
            ScreenId::ComposeMessageRecipient => self.message_compose.render_recipient(
                &frame,
                &self.compose_recipient_input,
                self.compose_recipient_status.as_deref(),
                self.compose_recipient_scroll_offset,
                self.compose_recipient_cursor,
            )?,
            ScreenId::ComposeMessageSubject => self.message_compose.render_subject(
                &compose_recipient_label(&self.game_data, self.compose_recipient_empire),
                &self.compose_subject,
                self.compose_subject_status.as_deref(),
            )?,
            ScreenId::ComposeMessageBody => self.message_compose.render_body(
                &compose_recipient_label(&self.game_data, self.compose_recipient_empire),
                &self.compose_subject,
                &self.compose_body,
                self.compose_body_cursor,
                self.compose_body_status.as_deref(),
            )?,
            ScreenId::ComposeMessageOutbox => self.message_compose.render_outbox(
                &self.compose_outbox_queue()?,
                &self.compose_outbox_input,
                self.compose_outbox_status.as_deref(),
                self.compose_outbox_scroll_offset,
                self.compose_outbox_cursor,
                &self.game_data,
            )?,
            ScreenId::ComposeMessageDiscardConfirm => {
                self.message_compose.render_discard_confirm()?
            }
            ScreenId::ComposeMessageSendConfirm => self.message_compose.render_send_confirm(
                &compose_recipient_label(&self.game_data, self.compose_recipient_empire),
                &self.compose_subject,
                &self.compose_body,
            )?,
            ScreenId::ComposeMessageSent => self.message_compose.render_sent(
                self.compose_sent_status
                    .as_deref()
                    .unwrap_or("Message queued."),
            )?,
            ScreenId::EmpireStatus => self
                .empire_status
                .render_with_menu(&frame, self.command_return_menu)?,
            ScreenId::EmpireProfile => self
                .empire_profile
                .render_with_menu(&frame, self.command_return_menu)?,
            ScreenId::Rankings(sort) => {
                self.rankings
                    .render_table(&frame, sort, self.command_return_menu)?
            }
            ScreenId::Reports => self
                .reports
                .render_with_menu(&frame, self.command_return_menu)?,
        };
        terminal.render(&playfield)
    }

    pub fn current_screen(&self) -> ScreenId {
        self.current_screen
    }

    pub fn current_screen_mut(&mut self) -> &mut ScreenId {
        &mut self.current_screen
    }

    pub fn advance_startup(&mut self) {
        if self.current_screen == ScreenId::FirstTimeIntro {
            if self.first_time_intro_page + 1 < FIRST_TIME_INTRO_PAGE_COUNT {
                self.first_time_intro_page += 1;
            } else {
                self.current_screen = ScreenId::FirstTimeMenu;
            }
            return;
        }
        if self.current_screen == ScreenId::Startup(StartupPhase::Splash)
            && self.startup_splash_page + 1 < STARTUP_SPLASH_PAGE_COUNT
        {
            self.startup_splash_page += 1;
            return;
        }
        if self.current_screen == ScreenId::Startup(StartupPhase::Splash) {
            let next = self.startup_sequence.skip_intro();
            self.current_screen = self.startup_target_screen(next);
            return;
        }
        if self.current_screen == ScreenId::Startup(StartupPhase::Intro)
            && self.startup_intro_page + 1 < crate::screen::STARTUP_INTRO_PAGE_COUNT
        {
            self.startup_intro_page += 1;
            return;
        }
        let next = self.startup_sequence.advance();
        self.current_screen = self.startup_target_screen(next);
    }

    pub fn open_startup_intro(&mut self) {
        self.startup_intro_page = 0;
        let next = self.startup_sequence.open_intro();
        self.current_screen = self.startup_target_screen(next);
    }

    pub fn open_first_time_menu(&mut self) {
        self.first_time_status = None;
        self.first_time_input.clear();
        self.current_screen = ScreenId::FirstTimeMenu;
    }

    pub fn open_first_time_help(&mut self) {
        self.first_time_status = None;
        self.current_screen = ScreenId::FirstTimeHelp;
    }

    pub fn open_first_time_empires(&mut self) {
        self.first_time_status = None;
        self.current_screen = ScreenId::FirstTimeEmpires;
    }

    pub fn open_first_time_intro(&mut self) {
        self.first_time_status = None;
        self.first_time_intro_page = 0;
        self.current_screen = ScreenId::FirstTimeIntro;
    }

    pub fn open_first_time_join_name(&mut self) {
        self.first_time_status = None;
        self.first_time_input.clear();
        self.current_screen = ScreenId::FirstTimeJoinEmpireName;
    }

    pub fn show_first_time_ansi_notice(&mut self) {
        self.first_time_status = Some("ANSI stays on. The stars look better in color.".to_string());
        self.current_screen = ScreenId::FirstTimeMenu;
    }

    pub fn append_first_time_input_char(&mut self, ch: char) {
        if !matches!(
            self.current_screen,
            ScreenId::FirstTimeJoinEmpireName | ScreenId::FirstTimeHomeworldName
        ) {
            return;
        }
        if !ch.is_ascii_graphic() && ch != ' ' {
            return;
        }
        if self.first_time_input.chars().count() >= 20 {
            return;
        }
        self.first_time_input.push(ch);
    }

    pub fn backspace_first_time_input(&mut self) {
        if !matches!(
            self.current_screen,
            ScreenId::FirstTimeJoinEmpireName | ScreenId::FirstTimeHomeworldName
        ) {
            return;
        }
        self.first_time_input.pop();
    }

    pub fn submit_first_time_input(&mut self) {
        match self.current_screen {
            ScreenId::FirstTimeJoinEmpireName => {
                let value = self.first_time_input.trim();
                if value.is_empty() {
                    self.first_time_status =
                        Some("Empire names need at least one visible character.".to_string());
                    return;
                }
                self.first_time_status = None;
                self.first_time_empire_name = value.to_string();
                self.first_time_input.clear();
                self.current_screen = ScreenId::FirstTimeJoinEmpireConfirm;
            }
            ScreenId::FirstTimeHomeworldName => {
                let value = self.first_time_input.trim();
                if value.is_empty() {
                    self.first_time_status =
                        Some("Homeworld names need at least one visible character.".to_string());
                    return;
                }
                self.first_time_status = None;
                self.first_time_homeworld_name = value.to_string();
                self.first_time_input.clear();
                self.current_screen = ScreenId::FirstTimeHomeworldConfirm;
            }
            _ => {}
        }
    }

    pub fn accept_first_time_prompt(&mut self) {
        match self.current_screen {
            ScreenId::FirstTimeJoinEmpireConfirm => {
                if self.complete_first_time_join().is_ok() {
                    self.current_screen = ScreenId::FirstTimeJoinSummary;
                }
            }
            ScreenId::FirstTimeJoinSummary => {
                self.current_screen = ScreenId::FirstTimeJoinNoPending;
            }
            ScreenId::FirstTimeJoinNoPending => {
                self.first_time_status = None;
                self.first_time_input.clear();
                self.current_screen = self.pending_naming_screen().unwrap_or(ScreenId::MainMenu);
            }
            ScreenId::FirstTimeHomeworldConfirm => {
                if self.complete_first_time_homeworld_name().is_ok() {
                    self.current_screen = ScreenId::MainMenu;
                }
            }
            _ => {}
        }
    }

    pub fn reject_first_time_prompt(&mut self) {
        match self.current_screen {
            ScreenId::FirstTimeJoinEmpireConfirm => {
                self.first_time_input = self.first_time_empire_name.clone();
                self.current_screen = ScreenId::FirstTimeJoinEmpireName;
            }
            ScreenId::FirstTimeHomeworldConfirm => {
                self.first_time_input = self.first_time_homeworld_name.clone();
                self.current_screen = ScreenId::FirstTimeHomeworldName;
            }
            _ => {}
        }
    }

    fn clear_command_menu_notice(&mut self) {
        self.command_menu_notice = None;
    }

    fn show_command_menu_notice(&mut self, menu: CommandMenu, message: impl Into<String>) {
        self.command_menu_notice = Some(message.into());
        self.command_return_menu = menu;
        self.current_screen = match menu {
            CommandMenu::Main => ScreenId::MainMenu,
            CommandMenu::General => ScreenId::GeneralMenu,
            CommandMenu::Fleet => ScreenId::FleetMenu,
            CommandMenu::Planet => ScreenId::PlanetMenu,
            CommandMenu::PlanetBuild => ScreenId::PlanetBuildMenu,
        };
    }

    pub fn open_main_menu(&mut self) {
        self.clear_command_menu_notice();
        self.current_screen = ScreenId::MainMenu;
    }

    pub fn open_general_menu(&mut self) {
        self.clear_command_menu_notice();
        self.current_screen = ScreenId::GeneralMenu;
    }

    pub fn open_planet_menu(&mut self) {
        self.clear_command_menu_notice();
        self.command_return_menu = CommandMenu::Planet;
        self.current_screen = ScreenId::PlanetMenu;
    }

    pub fn open_fleet_menu(&mut self) {
        self.clear_command_menu_notice();
        self.current_screen = ScreenId::FleetMenu;
    }

    pub fn open_fleet_help(&mut self) {
        self.clear_command_menu_notice();
        self.current_screen = ScreenId::FleetHelp;
    }

    pub fn open_fleet_list(&mut self, mode: FleetListMode) {
        self.clear_command_menu_notice();
        self.fleet_list_mode = mode;
        self.fleet_scroll_offset = 0;
        self.fleet_cursor = 0;
        self.current_screen = ScreenId::FleetList(mode);
    }

    pub fn open_fleet_review(&mut self) {
        let total = self.fleet_rows().len();
        if total == 0 {
            self.show_command_menu_notice(CommandMenu::Fleet, "You have no active fleets.");
            return;
        }
        self.clear_command_menu_notice();
        self.fleet_review_index = self.fleet_cursor.min(total - 1);
        self.current_screen = ScreenId::FleetReview;
    }

    pub fn open_fleet_roe_select(&mut self) {
        if self.current_screen == ScreenId::FleetRoeSelect {
            self.fleet_roe_editing = false;
            self.fleet_roe_select_input.clear();
            self.fleet_roe_input.clear();
            self.fleet_roe_status = None;
            return;
        }
        if self.fleet_rows().is_empty() {
            self.show_command_menu_notice(CommandMenu::Fleet, "You have no active fleets.");
            return;
        }
        self.clear_command_menu_notice();
        self.fleet_roe_status = None;
        self.fleet_roe_select_input.clear();
        self.fleet_roe_input.clear();
        self.fleet_roe_scroll_offset = 0;
        self.fleet_roe_cursor = 0;
        self.fleet_roe_editing = false;
        self.current_screen = ScreenId::FleetRoeSelect;
    }

    pub fn open_fleet_detach(&mut self) {
        if self.current_screen == ScreenId::FleetDetach {
            self.fleet_detach_mode = FleetDetachMode::SelectingFleet;
            self.fleet_detach_input.clear();
            self.fleet_detach_status = None;
            self.fleet_detach_selection = FleetDetachSelection::default();
            self.fleet_detach_donor_speed = None;
            return;
        }
        if self.fleet_rows().is_empty() {
            self.show_command_menu_notice(CommandMenu::Fleet, "You have no active fleets.");
            return;
        }
        self.clear_command_menu_notice();
        self.fleet_detach_status = None;
        self.fleet_detach_select_input.clear();
        self.fleet_detach_input.clear();
        self.fleet_detach_scroll_offset = 0;
        self.fleet_detach_cursor = 0;
        self.fleet_detach_mode = FleetDetachMode::SelectingFleet;
        self.fleet_detach_selection = FleetDetachSelection::default();
        self.fleet_detach_donor_speed = None;
        self.current_screen = ScreenId::FleetDetach;
    }

    pub fn open_fleet_eta(&mut self) {
        if self.fleet_rows().is_empty() {
            self.show_command_menu_notice(CommandMenu::Fleet, "You have no active fleets.");
            return;
        }
        self.clear_command_menu_notice();
        self.fleet_eta_status = None;
        self.fleet_eta_select_input.clear();
        self.fleet_eta_destination_input.clear();
        self.fleet_eta_include_system_input.clear();
        self.fleet_eta_scroll_offset = 0;
        self.fleet_eta_cursor = 0;
        self.fleet_eta_mode = FleetEtaMode::SelectingFleet;
        self.current_screen = ScreenId::FleetEta;
    }

    pub fn open_planet_help(&mut self) {
        self.clear_command_menu_notice();
        self.current_screen = ScreenId::PlanetHelp;
    }

    pub fn open_planet_auto_commission_confirm(&mut self) {
        self.planet_auto_commission_status = None;
        if self.commission_planet_rows().is_empty() {
            self.show_command_menu_notice(
                CommandMenu::Planet,
                "No ships or starbases are waiting in stardock.",
            );
        } else {
            self.clear_command_menu_notice();
            self.current_screen = ScreenId::PlanetAutoCommissionConfirm;
        }
    }

    pub fn open_planet_transport_planet_select(&mut self, mode: PlanetTransportMode) {
        self.planet_transport_mode = Some(mode);
        self.planet_transport_planet_cursor = 0;
        self.planet_transport_planet_scroll_offset = 0;
        self.planet_transport_selected_planet_record = None;
        self.planet_transport_fleet_cursor = 0;
        self.planet_transport_fleet_scroll_offset = 0;
        self.planet_transport_qty_input.clear();
        self.planet_transport_status = None;
        if self.planet_transport_planet_rows(mode).is_empty() {
            self.show_command_menu_notice(CommandMenu::Planet, match mode {
                PlanetTransportMode::Load => {
                    "No planets have armies and troop transports ready to load."
                }
                PlanetTransportMode::Unload => {
                    "No fleets have loaded armies ready to unload onto planets with free capacity."
                }
            });
        } else {
            self.clear_command_menu_notice();
            self.current_screen = ScreenId::PlanetTransportPlanetSelect(mode);
        }
    }

    pub fn open_planet_commission_menu(&mut self) {
        self.command_return_menu = CommandMenu::Planet;
        self.planet_commission_status = None;
        self.planet_commission_cursor = 0;
        self.planet_commission_scroll_offset = 0;
        self.planet_commission_selected_slots.clear();
        let total = self.commission_planet_rows().len();
        if total == 0 {
            self.planet_commission_index = 0;
            self.show_command_menu_notice(
                CommandMenu::Planet,
                "No owned planets have units waiting in stardock.",
            );
            return;
        } else {
            self.clear_command_menu_notice();
            self.planet_commission_index = self.planet_commission_index.min(total - 1);
        }
        self.current_screen = ScreenId::PlanetCommissionMenu;
    }

    pub fn open_planet_build_help(&mut self) {
        self.clear_command_menu_notice();
        self.current_screen = ScreenId::PlanetBuildHelp;
    }

    pub fn open_planet_build_menu(&mut self) {
        self.command_return_menu = CommandMenu::PlanetBuild;
        self.planet_build_status = None;
        self.planet_build_unit_input.clear();
        self.planet_build_unit_status = None;
        self.planet_build_quantity_input.clear();
        self.planet_build_quantity_status = None;
        self.planet_build_selected_kind = None;
        self.planet_build_list_scroll_offset = 0;
        let total = self.build_planet_rows().len();
        if total == 0 {
            self.planet_build_index = 0;
            self.show_command_menu_notice(
                CommandMenu::Planet,
                "No owned planets available for building.",
            );
            return;
        }
        self.clear_command_menu_notice();
        self.planet_build_index = self.planet_build_index.min(total - 1);
        self.current_screen = ScreenId::PlanetBuildMenu;
    }

    pub fn open_planet_build_review(&mut self) {
        if self.build_planet_rows().is_empty() {
            self.open_planet_build_menu();
            return;
        }
        self.current_screen = ScreenId::PlanetBuildReview;
    }

    pub fn open_planet_build_list(&mut self) {
        if self.build_planet_rows().is_empty() {
            self.open_planet_build_menu();
            return;
        }
        self.planet_build_list_scroll_offset = 0;
        self.planet_build_list_cursor = 0;
        self.planet_build_list_confirming = false;
        self.current_screen = ScreenId::PlanetBuildList;
    }

    pub fn open_planet_build_change(&mut self) {
        // Pre-position cursor on the current planet so it's already highlighted.
        self.planet_build_change_cursor = self.planet_build_index;
        self.planet_build_change_scroll_offset = 0;
        sync_scroll_to_cursor(
            &mut self.planet_build_change_scroll_offset,
            self.planet_build_change_cursor,
            crate::screen::PLANET_BUILD_CHANGE_VISIBLE_ROWS,
        );
        self.current_screen = ScreenId::PlanetBuildChange;
    }

    pub fn move_planet_build_change_cursor(&mut self, delta: i8) {
        if self.current_screen != ScreenId::PlanetBuildChange {
            return;
        }
        let total = self.build_planet_rows().len();
        if total == 0 {
            return;
        }
        let next = self.planet_build_change_cursor as isize + delta as isize;
        self.planet_build_change_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.planet_build_change_scroll_offset,
            self.planet_build_change_cursor,
            crate::screen::PLANET_BUILD_CHANGE_VISIBLE_ROWS,
        );
    }

    pub fn confirm_planet_build_change(&mut self) {
        let total = self.build_planet_rows().len();
        if total == 0 {
            self.current_screen = ScreenId::PlanetBuildMenu;
            return;
        }
        self.planet_build_index = self.planet_build_change_cursor.min(total - 1);
        self.planet_build_status = None;
        self.current_screen = ScreenId::PlanetBuildMenu;
    }

    pub fn open_planet_build_abort_confirm(&mut self) {
        if self.build_planet_rows().is_empty() {
            self.open_planet_build_menu();
            return;
        }
        self.current_screen = ScreenId::PlanetBuildAbortConfirm;
    }

    pub fn open_planet_build_specify(&mut self) {
        if self.build_planet_rows().is_empty() {
            self.open_planet_build_menu();
            return;
        }
        self.planet_build_unit_input.clear();
        self.planet_build_unit_status = None;
        self.planet_build_quantity_input.clear();
        self.planet_build_quantity_status = None;
        self.planet_build_selected_kind = None;
        self.current_screen = ScreenId::PlanetBuildSpecify;
    }

    pub fn open_planet_tax_prompt(&mut self) {
        self.clear_command_menu_notice();
        self.planet_tax_input = String::new();
        self.planet_tax_status = None;
        self.current_screen = ScreenId::PlanetTaxPrompt;
    }

    pub fn open_planet_database(&mut self) {
        if !matches!(
            self.current_screen,
            ScreenId::PlanetDatabaseList | ScreenId::PlanetDatabaseDetail
        ) {
            self.command_return_menu = self.origin_command_menu();
            let default_coords = self.default_planet_prompt_coords();
            let rows = self.planet_database_rows();
            let default_index = rows
                .iter()
                .position(|row| row.coords == default_coords)
                .unwrap_or(0);
            self.planet_database_cursor = default_index;
            self.planet_database_detail_index = default_index;
            self.planet_database_scroll_offset =
                default_index.saturating_sub(crate::screen::PLANET_DATABASE_VISIBLE_ROWS / 2);
            self.planet_database_input.clear();
            self.planet_database_status = None;
        }
        self.current_screen = ScreenId::PlanetDatabaseList;
    }

    pub fn open_planet_database_detail(&mut self) {
        let total = self.planet_database_rows().len();
        if total == 0 {
            self.current_screen = ScreenId::PlanetDatabaseList;
            return;
        }
        self.planet_database_detail_index = self.planet_database_cursor.min(total - 1);
        self.current_screen = ScreenId::PlanetDatabaseDetail;
    }

    pub fn open_planet_list_sort_prompt(&mut self, mode: PlanetListMode) {
        self.clear_command_menu_notice();
        self.planet_list_sort_status = None;
        self.current_screen = ScreenId::PlanetListSortPrompt(mode);
    }

    pub fn submit_planet_list_sort(&mut self, mode: PlanetListMode, sort: PlanetListSort) {
        let total = self.sorted_planet_rows(sort).len();
        if total == 0 {
            self.show_command_menu_notice(CommandMenu::Planet, "You do not currently control any planets.");
            return;
        }
        self.clear_command_menu_notice();
        self.planet_list_sort_status = None;
        self.planet_brief_scroll_offset = 0;
        self.planet_brief_cursor = 0;
        self.planet_detail_index = 0;
        self.current_screen = match mode {
            PlanetListMode::Brief => ScreenId::PlanetBriefList(sort),
            PlanetListMode::Detail => ScreenId::PlanetDetailList(sort),
            PlanetListMode::Stub(_) => ScreenId::PlanetMenu,
        };
    }

    pub fn scroll_planet_brief(&mut self, delta: i8) {
        let ScreenId::PlanetBriefList(sort) = self.current_screen else {
            return;
        };
        let total = self.sorted_planet_rows(sort).len();
        let max_offset = total.saturating_sub(crate::screen::PLANET_BRIEF_VISIBLE_ROWS);
        self.planet_brief_scroll_offset = self
            .planet_brief_scroll_offset
            .saturating_add_signed(delta as isize)
            .min(max_offset);
    }

    pub fn move_planet_brief_cursor(&mut self, delta: i8) {
        let ScreenId::PlanetBriefList(sort) = self.current_screen else {
            return;
        };
        let total = self.sorted_planet_rows(sort).len();
        if total == 0 {
            self.planet_brief_cursor = 0;
            return;
        }
        let next = self.planet_brief_cursor as isize + delta as isize;
        self.planet_brief_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.planet_brief_scroll_offset,
            self.planet_brief_cursor,
            crate::screen::PLANET_BRIEF_VISIBLE_ROWS,
        );
    }

    pub fn move_planet_detail(&mut self, delta: i8) {
        let ScreenId::PlanetDetailList(sort) = self.current_screen else {
            return;
        };
        let total = self.sorted_planet_rows(sort).len();
        if total == 0 {
            self.planet_detail_index = 0;
            return;
        }
        self.planet_detail_index = match delta {
            i8::MIN => 0,
            i8::MAX => total - 1,
            _ => self
                .planet_detail_index
                .saturating_add_signed(delta as isize)
                .min(total - 1),
        };
    }

    pub fn move_fleet_list(&mut self, delta: i8) {
        let ScreenId::FleetList(_) = self.current_screen else {
            return;
        };
        let total = self.fleet_rows().len();
        if total == 0 {
            self.fleet_cursor = 0;
            return;
        }
        let next = self.fleet_cursor as isize + delta as isize;
        self.fleet_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.fleet_scroll_offset,
            self.fleet_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
    }

    pub fn move_fleet_review(&mut self, delta: i8) {
        if self.current_screen != ScreenId::FleetReview {
            return;
        }
        let total = self.fleet_rows().len();
        if total == 0 {
            self.fleet_review_index = 0;
            return;
        }
        self.fleet_review_index = match delta {
            i8::MIN => 0,
            i8::MAX => total - 1,
            _ => self
                .fleet_review_index
                .saturating_add_signed(delta as isize)
                .min(total - 1),
        };
        self.fleet_cursor = self.fleet_review_index;
        sync_scroll_to_cursor(
            &mut self.fleet_scroll_offset,
            self.fleet_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
    }

    pub fn move_fleet_roe_select(&mut self, delta: i8) {
        if self.current_screen != ScreenId::FleetRoeSelect || self.fleet_roe_editing {
            return;
        }
        let total = self.fleet_rows().len();
        if total == 0 {
            self.fleet_roe_cursor = 0;
            return;
        }
        let next = self.fleet_roe_cursor as isize + delta as isize;
        self.fleet_roe_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.fleet_roe_scroll_offset,
            self.fleet_roe_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
        self.fleet_roe_select_input.clear();
        self.fleet_roe_status = None;
    }

    pub fn move_fleet_detach_select(&mut self, delta: i8) {
        if self.current_screen != ScreenId::FleetDetach
            || self.fleet_detach_mode != FleetDetachMode::SelectingFleet
        {
            return;
        }
        let total = self.fleet_rows().len();
        if total == 0 {
            self.fleet_detach_cursor = 0;
            return;
        }
        let next = self.fleet_detach_cursor as isize + delta as isize;
        self.fleet_detach_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.fleet_detach_scroll_offset,
            self.fleet_detach_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
        self.fleet_detach_select_input.clear();
        self.fleet_detach_status = None;
    }

    pub fn move_fleet_eta_select(&mut self, delta: i8) {
        if self.current_screen != ScreenId::FleetEta
            || self.fleet_eta_mode != FleetEtaMode::SelectingFleet
        {
            return;
        }
        let total = self.fleet_rows().len();
        if total == 0 {
            self.fleet_eta_cursor = 0;
            return;
        }
        let next = self.fleet_eta_cursor as isize + delta as isize;
        self.fleet_eta_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.fleet_eta_scroll_offset,
            self.fleet_eta_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
        self.fleet_eta_select_input.clear();
        self.fleet_eta_status = None;
    }

    pub fn append_fleet_roe_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::FleetRoeSelect
            && if self.fleet_roe_editing {
                self.fleet_roe_input.len() < 2
            } else {
                self.fleet_roe_select_input.len() < 2
            }
        {
            if self.fleet_roe_editing {
                self.fleet_roe_input.push(ch);
            } else {
                self.fleet_roe_select_input.push(ch);
                self.sync_fleet_roe_cursor_to_input();
            }
            self.fleet_roe_status = None;
        }
    }

    pub fn append_fleet_detach_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::FleetDetach || !ch.is_ascii_digit() {
            return;
        }
        let limit = if self.fleet_detach_mode == FleetDetachMode::SelectingFleet {
            4
        } else {
            3
        };
        let input = if self.fleet_detach_mode == FleetDetachMode::SelectingFleet {
            &mut self.fleet_detach_select_input
        } else {
            &mut self.fleet_detach_input
        };
        if input.len() >= limit {
            return;
        }
        input.push(ch);
        if self.fleet_detach_mode == FleetDetachMode::SelectingFleet {
            self.sync_fleet_detach_cursor_to_input();
        }
        self.fleet_detach_status = None;
    }

    pub fn append_fleet_eta_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::FleetEta {
            return;
        }
        match self.fleet_eta_mode {
            FleetEtaMode::SelectingFleet => {
                if ch.is_ascii_digit() && self.fleet_eta_select_input.len() < 4 {
                    self.fleet_eta_select_input.push(ch);
                    self.sync_fleet_eta_cursor_to_input();
                    self.fleet_eta_status = None;
                }
            }
            FleetEtaMode::EnteringDestination => {
                if self.fleet_eta_destination_input.len() < 16
                    && (ch.is_ascii_digit() || matches!(ch, ',' | ' ' | '(' | ')' | '[' | ']'))
                {
                    self.fleet_eta_destination_input.push(ch);
                    self.fleet_eta_status = None;
                }
            }
            FleetEtaMode::ConfirmingSystemEntry => {
                if matches!(ch, 'y' | 'Y' | 'n' | 'N')
                    && self.fleet_eta_include_system_input.is_empty()
                {
                    self.fleet_eta_include_system_input
                        .push(ch.to_ascii_uppercase());
                    self.fleet_eta_status = None;
                }
            }
            FleetEtaMode::ShowingResult => {}
        }
    }

    pub fn backspace_fleet_roe_input(&mut self) {
        if self.current_screen == ScreenId::FleetRoeSelect {
            if self.fleet_roe_editing {
                self.fleet_roe_input.pop();
            } else {
                self.fleet_roe_select_input.pop();
                self.sync_fleet_roe_cursor_to_input();
            }
            self.fleet_roe_status = None;
        }
    }

    pub fn backspace_fleet_detach_input(&mut self) {
        if self.current_screen != ScreenId::FleetDetach {
            return;
        }
        if self.fleet_detach_mode == FleetDetachMode::SelectingFleet {
            self.fleet_detach_select_input.pop();
            self.sync_fleet_detach_cursor_to_input();
        } else {
            self.fleet_detach_input.pop();
        }
        self.fleet_detach_status = None;
    }

    pub fn backspace_fleet_eta_input(&mut self) {
        if self.current_screen != ScreenId::FleetEta {
            return;
        }
        match self.fleet_eta_mode {
            FleetEtaMode::SelectingFleet => {
                self.fleet_eta_select_input.pop();
                self.sync_fleet_eta_cursor_to_input();
            }
            FleetEtaMode::EnteringDestination => {
                self.fleet_eta_destination_input.pop();
            }
            FleetEtaMode::ConfirmingSystemEntry => {
                self.fleet_eta_include_system_input.pop();
            }
            FleetEtaMode::ShowingResult => {}
        }
        self.fleet_eta_status = None;
    }

    pub fn submit_fleet_roe(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::FleetRoeSelect {
            return Ok(());
        }
        if !self.fleet_roe_editing {
            let rows = self.fleet_rows();
            if rows.is_empty() {
                self.current_screen = ScreenId::FleetMenu;
                return Ok(());
            }
            if self.fleet_roe_select_input.trim().is_empty() {
                self.fleet_roe_cursor = self.fleet_roe_cursor.min(rows.len() - 1);
            } else {
                let target_fleet_id = match self.fleet_roe_select_input.trim().parse::<u16>() {
                    Ok(value) => value,
                    Err(_) => {
                        self.fleet_roe_status =
                            Some("Enter a fleet number from the table.".to_string());
                        return Ok(());
                    }
                };
                let Some(index) = rows
                    .iter()
                    .position(|row| row.fleet_number == target_fleet_id)
                else {
                    self.fleet_roe_status = Some(format!(
                        "Fleet #{target_fleet_id} is not in your fleet list."
                    ));
                    return Ok(());
                };
                self.fleet_roe_cursor = index;
                sync_scroll_to_cursor(
                    &mut self.fleet_roe_scroll_offset,
                    self.fleet_roe_cursor,
                    crate::screen::FLEET_VISIBLE_ROWS,
                );
            }
            self.fleet_roe_select_input.clear();
            self.fleet_roe_input.clear();
            self.fleet_roe_status = None;
            self.fleet_roe_editing = true;
            return Ok(());
        }
        let rows = self.fleet_rows();
        let Some(selected_row) = rows.get(self.fleet_roe_cursor) else {
            self.current_screen = ScreenId::FleetMenu;
            return Ok(());
        };
        let parsed = if self.fleet_roe_input.trim().is_empty() {
            selected_row.rules_of_engagement
        } else {
            match self.fleet_roe_input.trim().parse::<u8>() {
                Ok(value) => value,
                Err(_) => {
                    self.fleet_roe_status = Some("Enter an ROE from 0 to 10.".to_string());
                    return Ok(());
                }
            }
        };
        if parsed > 10 {
            self.fleet_roe_status = Some("Enter an ROE from 0 to 10.".to_string());
            return Ok(());
        }
        let fleet = self
            .game_data
            .fleets
            .records
            .get_mut(selected_row.fleet_record_index_1_based - 1)
            .ok_or("fleet roe target missing")?;
        let has_combat_ships = fleet.destroyer_count() > 0
            || fleet.cruiser_count() > 0
            || fleet.battleship_count() > 0;
        if !has_combat_ships && parsed != 0 {
            self.fleet_roe_status = Some("Non-combat fleets must use ROE 0.".to_string());
            return Ok(());
        }
        fleet.set_rules_of_engagement(parsed);
        self.save_game_data()?;
        self.fleet_roe_input.clear();
        self.fleet_roe_status = None;
        self.fleet_roe_editing = false;
        Ok(())
    }

    pub fn submit_fleet_detach(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::FleetDetach {
            return Ok(());
        }
        let rows = self.fleet_rows();
        let Some(selected_row) = rows.get(self.fleet_detach_cursor) else {
            self.current_screen = ScreenId::FleetMenu;
            return Ok(());
        };

        if self.fleet_detach_mode == FleetDetachMode::SelectingFleet {
            if !self.fleet_detach_select_input.trim().is_empty() {
                let target_fleet_id = match self.fleet_detach_select_input.trim().parse::<u16>() {
                    Ok(value) => value,
                    Err(_) => {
                        self.fleet_detach_status =
                            Some("Enter a fleet number from the table.".to_string());
                        return Ok(());
                    }
                };
                let Some(index) = rows
                    .iter()
                    .position(|row| row.fleet_number == target_fleet_id)
                else {
                    self.fleet_detach_status = Some(format!(
                        "Fleet #{target_fleet_id} is not in your fleet list."
                    ));
                    return Ok(());
                };
                self.fleet_detach_cursor = index;
                sync_scroll_to_cursor(
                    &mut self.fleet_detach_scroll_offset,
                    self.fleet_detach_cursor,
                    crate::screen::FLEET_VISIBLE_ROWS,
                );
            }
            if self.current_fleet_detach_ship_total() <= 1 {
                self.fleet_detach_status =
                    Some("A fleet must contain at least two ships to detach.".to_string());
                return Ok(());
            }
            self.fleet_detach_select_input.clear();
            self.fleet_detach_input.clear();
            self.fleet_detach_status = None;
            self.fleet_detach_selection = FleetDetachSelection::default();
            self.fleet_detach_donor_speed = None;
            self.fleet_detach_mode = self
                .next_fleet_detach_prompt_mode(FleetDetachMode::SelectingFleet)
                .unwrap_or(FleetDetachMode::SettingNewFleetRoe);
            return Ok(());
        }

        let Some(record) = self
            .game_data
            .fleets
            .records
            .get(selected_row.fleet_record_index_1_based - 1)
            .cloned()
        else {
            self.current_screen = ScreenId::FleetMenu;
            return Ok(());
        };

        match self.fleet_detach_mode {
            FleetDetachMode::EnteringBattleships
            | FleetDetachMode::EnteringCruisers
            | FleetDetachMode::EnteringDestroyers
            | FleetDetachMode::EnteringFullTransports
            | FleetDetachMode::EnteringEmptyTransports
            | FleetDetachMode::EnteringScouts
            | FleetDetachMode::EnteringEtacs => {
                let value = self.resolve_fleet_detach_numeric_input(0)?;
                match self.fleet_detach_mode {
                    FleetDetachMode::EnteringBattleships => {
                        if value > record.battleship_count() {
                            self.fleet_detach_status =
                                Some("Enter a value from 0 to the table limit.".to_string());
                            return Ok(());
                        }
                        self.fleet_detach_selection.battleships = value;
                    }
                    FleetDetachMode::EnteringCruisers => {
                        if value > record.cruiser_count() {
                            self.fleet_detach_status =
                                Some("Enter a value from 0 to the table limit.".to_string());
                            return Ok(());
                        }
                        self.fleet_detach_selection.cruisers = value;
                    }
                    FleetDetachMode::EnteringDestroyers => {
                        if value > record.destroyer_count() {
                            self.fleet_detach_status =
                                Some("Enter a value from 0 to the table limit.".to_string());
                            return Ok(());
                        }
                        self.fleet_detach_selection.destroyers = value;
                    }
                    FleetDetachMode::EnteringFullTransports => {
                        if value > record.army_count() {
                            self.fleet_detach_status =
                                Some("Enter a value from 0 to the table limit.".to_string());
                            return Ok(());
                        }
                        self.fleet_detach_selection.full_transports = value;
                    }
                    FleetDetachMode::EnteringEmptyTransports => {
                        let available = record
                            .troop_transport_count()
                            .saturating_sub(record.army_count());
                        if value > available {
                            self.fleet_detach_status =
                                Some("Enter a value from 0 to the table limit.".to_string());
                            return Ok(());
                        }
                        self.fleet_detach_selection.empty_transports = value;
                    }
                    FleetDetachMode::EnteringScouts => {
                        if value > u16::from(record.scout_count()) {
                            self.fleet_detach_status =
                                Some("Enter a value from 0 to the table limit.".to_string());
                            return Ok(());
                        }
                        self.fleet_detach_selection.scouts = value as u8;
                    }
                    FleetDetachMode::EnteringEtacs => {
                        if value > record.etac_count() {
                            self.fleet_detach_status =
                                Some("Enter a value from 0 to the table limit.".to_string());
                            return Ok(());
                        }
                        self.fleet_detach_selection.etacs = value;
                    }
                    _ => {}
                }
                self.fleet_detach_input.clear();
                self.fleet_detach_status = None;
                if let Some(next_mode) = self.next_fleet_detach_prompt_mode(self.fleet_detach_mode)
                {
                    self.fleet_detach_mode = next_mode;
                } else if self.fleet_detach_selection.total_ships() == 0 {
                    self.fleet_detach_status = Some("Detach at least one ship.".to_string());
                } else if self.fleet_detach_requires_speed_prompt() {
                    self.fleet_detach_donor_speed = None;
                    self.fleet_detach_mode = FleetDetachMode::AdjustingDonorSpeed;
                } else {
                    self.fleet_detach_donor_speed = None;
                    self.fleet_detach_mode = FleetDetachMode::SettingNewFleetRoe;
                }
            }
            FleetDetachMode::AdjustingDonorSpeed => {
                let default_speed = self.fleet_detach_donor_default_speed().max(1);
                let speed = self.resolve_fleet_detach_numeric_input(default_speed as u16)? as u8;
                let max_speed = self.fleet_detach_donor_default_speed();
                if speed == 0 || speed > max_speed {
                    self.fleet_detach_status =
                        Some(format!("Enter a speed from 1 to {max_speed}."));
                    return Ok(());
                }
                self.fleet_detach_donor_speed = Some(speed);
                self.fleet_detach_input.clear();
                self.fleet_detach_status = None;
                self.fleet_detach_mode = FleetDetachMode::SettingNewFleetRoe;
            }
            FleetDetachMode::SettingNewFleetRoe => {
                let new_roe = self.resolve_fleet_detach_numeric_input(6)? as u8;
                if new_roe > 10 {
                    self.fleet_detach_status = Some("Enter an ROE from 0 to 10.".to_string());
                    return Ok(());
                }
                let detached_has_combat_ships = self.fleet_detach_selection.battleships > 0
                    || self.fleet_detach_selection.cruisers > 0
                    || self.fleet_detach_selection.destroyers > 0;
                if !detached_has_combat_ships && new_roe != 0 {
                    self.fleet_detach_status =
                        Some("Non-combat fleets must use ROE 0.".to_string());
                    return Ok(());
                }
                let donor_speed = if self.fleet_detach_requires_speed_prompt() {
                    Some(
                        self.fleet_detach_donor_speed
                            .unwrap_or(self.fleet_detach_donor_default_speed()),
                    )
                } else {
                    None
                };
                self.game_data.detach_ships_to_new_fleet(
                    self.player.record_index_1_based,
                    selected_row.fleet_record_index_1_based,
                    self.fleet_detach_selection,
                    donor_speed,
                    new_roe,
                )?;
                self.save_game_data()?;
                self.fleet_detach_mode = FleetDetachMode::SelectingFleet;
                self.fleet_detach_input.clear();
                self.fleet_detach_select_input.clear();
                self.fleet_detach_status = None;
                self.fleet_detach_selection = FleetDetachSelection::default();
                self.fleet_detach_donor_speed = None;
            }
            FleetDetachMode::SelectingFleet => {}
        }
        Ok(())
    }

    pub fn submit_fleet_eta(&mut self) {
        if self.current_screen != ScreenId::FleetEta {
            return;
        }
        let rows = self.fleet_rows();
        let Some(selected_row) = rows.get(self.fleet_eta_cursor) else {
            self.fleet_eta_status = Some("You have no active fleets.".to_string());
            self.fleet_eta_mode = FleetEtaMode::SelectingFleet;
            return;
        };
        match self.fleet_eta_mode {
            FleetEtaMode::SelectingFleet => {
                if !self.fleet_eta_select_input.trim().is_empty() {
                    let target_fleet_id = match self.fleet_eta_select_input.trim().parse::<u16>() {
                        Ok(value) => value,
                        Err(_) => {
                            self.fleet_eta_status =
                                Some("Enter a fleet number from the table.".to_string());
                            return;
                        }
                    };
                    let Some(index) = rows
                        .iter()
                        .position(|row| row.fleet_number == target_fleet_id)
                    else {
                        self.fleet_eta_status =
                            Some("Enter a fleet number from the table.".to_string());
                        return;
                    };
                    self.fleet_eta_cursor = index;
                    sync_scroll_to_cursor(
                        &mut self.fleet_eta_scroll_offset,
                        self.fleet_eta_cursor,
                        crate::screen::FLEET_VISIBLE_ROWS,
                    );
                }
                self.fleet_eta_select_input.clear();
                self.fleet_eta_destination_input.clear();
                self.fleet_eta_include_system_input.clear();
                self.fleet_eta_status = None;
                self.fleet_eta_mode = FleetEtaMode::EnteringDestination;
            }
            FleetEtaMode::EnteringDestination => {
                let default_destination = self.fleet_eta_default_destination();
                let Some(destination) = resolve_default_coords_input(
                    &self.fleet_eta_destination_input,
                    default_destination,
                ) else {
                    self.fleet_eta_status = Some("Enter coordinates like 10,13".to_string());
                    return;
                };
                let map_size =
                    ec_data::map_size_for_player_count(self.game_data.conquest.player_count());
                if destination[0] == 0
                    || destination[1] == 0
                    || destination[0] > map_size
                    || destination[1] > map_size
                {
                    self.fleet_eta_status = Some(format!("Enter coordinates within 1..{map_size}"));
                    return;
                }
                self.fleet_eta_destination_input = format!("{},{}", destination[0], destination[1]);
                self.fleet_eta_include_system_input.clear();
                self.fleet_eta_status = None;
                self.fleet_eta_mode = FleetEtaMode::ConfirmingSystemEntry;
            }
            FleetEtaMode::ConfirmingSystemEntry => {
                let include_system =
                    resolve_yes_no_input(&self.fleet_eta_include_system_input, false);
                let destination = resolve_default_coords_input(
                    &self.fleet_eta_destination_input,
                    self.fleet_eta_default_destination(),
                )
                .unwrap_or(self.fleet_eta_default_destination());
                let result =
                    self.calculate_fleet_eta_message(selected_row, destination, include_system);
                self.fleet_eta_status = Some(result);
                self.fleet_eta_include_system_input.clear();
                self.fleet_eta_mode = FleetEtaMode::ShowingResult;
            }
            FleetEtaMode::ShowingResult => {
                self.fleet_eta_status = None;
                self.fleet_eta_destination_input.clear();
                self.fleet_eta_include_system_input.clear();
                self.fleet_eta_mode = FleetEtaMode::SelectingFleet;
            }
        }
    }

    pub fn move_planet_database_list(&mut self, delta: i8) {
        if self.current_screen != ScreenId::PlanetDatabaseList {
            return;
        }
        let total = self.planet_database_rows().len();
        if total == 0 {
            self.planet_database_cursor = 0;
            return;
        }
        let next = self.planet_database_cursor as isize + delta as isize;
        self.planet_database_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.planet_database_scroll_offset,
            self.planet_database_cursor,
            crate::screen::PLANET_DATABASE_VISIBLE_ROWS,
        );
    }

    pub fn move_planet_database_detail(&mut self, delta: i8) {
        if self.current_screen != ScreenId::PlanetDatabaseDetail {
            return;
        }
        let total = self.planet_database_rows().len();
        if total == 0 {
            self.planet_database_detail_index = 0;
            return;
        }
        self.planet_database_detail_index = match delta {
            i8::MIN => 0,
            i8::MAX => total - 1,
            _ => self
                .planet_database_detail_index
                .saturating_add_signed(delta as isize)
                .min(total - 1),
        };
        self.planet_database_cursor = self.planet_database_detail_index;
        sync_scroll_to_cursor(
            &mut self.planet_database_scroll_offset,
            self.planet_database_cursor,
            crate::screen::PLANET_DATABASE_VISIBLE_ROWS,
        );
    }

    pub fn append_planet_database_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::PlanetDatabaseList {
            return;
        }
        if self.planet_database_input.len() < 16 && (ch.is_ascii_digit() || ch == ',' || ch == ' ')
        {
            self.planet_database_input.push(ch);
            self.planet_database_status = None;
        }
    }

    pub fn backspace_planet_database_input(&mut self) {
        if self.current_screen != ScreenId::PlanetDatabaseList {
            return;
        }
        self.planet_database_input.pop();
        self.planet_database_status = None;
    }

    pub fn submit_planet_database_lookup(&mut self) {
        if self.current_screen != ScreenId::PlanetDatabaseList {
            return;
        }
        let rows = self.planet_database_rows();
        if self.planet_database_input.trim().is_empty() {
            self.open_planet_database_detail();
            return;
        }
        let Some(coords) = resolve_default_coords_input(
            &self.planet_database_input,
            self.default_planet_prompt_coords(),
        ) else {
            self.planet_database_status = Some("Enter coordinates like 5,2".to_string());
            return;
        };
        let Some(index) = rows.iter().position(|row| row.coords == coords) else {
            self.planet_database_status =
                Some(format!("No world found at [{},{}]", coords[0], coords[1]));
            return;
        };
        self.planet_database_cursor = index;
        sync_scroll_to_cursor(
            &mut self.planet_database_scroll_offset,
            self.planet_database_cursor,
            crate::screen::PLANET_DATABASE_VISIBLE_ROWS,
        );
        self.planet_database_status = None;
        self.planet_database_input.clear();
        self.open_planet_database_detail();
    }

    pub fn move_planet_build(&mut self, delta: i8) {
        let total = self.build_planet_rows().len();
        if total == 0 {
            self.planet_build_index = 0;
            return;
        }
        // Wrap around so N on the last planet returns to the first.
        let next = self.planet_build_index as isize + delta as isize;
        self.planet_build_index = next.rem_euclid(total as isize) as usize;
        self.planet_build_status = None;
    }

    pub fn move_planet_commission_planet(&mut self, delta: i8) {
        if self.current_screen != ScreenId::PlanetCommissionMenu {
            return;
        }
        let total = self.commission_planet_rows().len();
        if total == 0 {
            self.planet_commission_index = 0;
            return;
        }
        let next = self.planet_commission_index as isize + delta as isize;
        self.planet_commission_index = next.rem_euclid(total as isize) as usize;
        self.planet_commission_cursor = 0;
        self.planet_commission_scroll_offset = 0;
        self.planet_commission_selected_slots.clear();
        self.planet_commission_status = None;
    }

    pub fn move_planet_commission_row(&mut self, delta: i8) {
        if self.current_screen != ScreenId::PlanetCommissionMenu {
            return;
        }
        let total = self.current_planet_commission_rows().len();
        if total == 0 {
            self.planet_commission_cursor = 0;
            return;
        }
        let next = self.planet_commission_cursor as isize + delta as isize;
        self.planet_commission_cursor = next.rem_euclid(total as isize) as usize;
        if self.planet_commission_cursor < self.planet_commission_scroll_offset {
            self.planet_commission_scroll_offset = self.planet_commission_cursor;
        } else if self.planet_commission_cursor
            >= self.planet_commission_scroll_offset + crate::screen::PLANET_COMMISSION_VISIBLE_ROWS
        {
            self.planet_commission_scroll_offset =
                self.planet_commission_cursor + 1 - crate::screen::PLANET_COMMISSION_VISIBLE_ROWS;
        }
    }

    pub fn commission_selected_stardock_row(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::PlanetCommissionMenu {
            return Ok(());
        }
        let rows = self.current_planet_commission_rows();
        let Some(current_row) = rows.get(self.planet_commission_cursor) else {
            self.planet_commission_status = Some("No stardock units are available.".to_string());
            return Ok(());
        };
        let selected_slots: Vec<usize> = if self.planet_commission_selected_slots.is_empty() {
            vec![current_row.slot_0_based]
        } else {
            rows.iter()
                .filter(|row| {
                    self.planet_commission_selected_slots
                        .contains(&row.slot_0_based)
                })
                .map(|row| row.slot_0_based)
                .collect()
        };
        let planet_record = self
            .current_commission_planet_row()?
            .planet_record_index_1_based;
        let result = match self.game_data.commission_planet_stardock_slots(
            self.player.record_index_1_based,
            planet_record,
            &selected_slots,
        ) {
            Ok(result) => result,
            Err(GameStateMutationError::InvalidCommissionSelection) => {
                self.planet_commission_status = Some(
                    "Select either ships for one fleet or one starbase by itself.".to_string(),
                );
                return Ok(());
            }
            Err(err) => return Err(err.into()),
        };
        self.save_game_data()?;
        match result {
            CommissionResult::Fleet {
                fleet_record_index_1_based,
            } => {
                let _ = self
                    .game_data
                    .fleets
                    .records
                    .get(fleet_record_index_1_based - 1)
                    .map(|fleet| fleet.local_slot_word_raw())
                    .ok_or("commissioned fleet record missing")?;
            }
            CommissionResult::Starbase {
                base_record_index_1_based: _,
            } => {}
        }
        self.planet_commission_status = None;

        let planet_rows = self.commission_planet_rows();
        if planet_rows.is_empty() {
            self.planet_commission_index = 0;
            self.planet_commission_cursor = 0;
            self.planet_commission_scroll_offset = 0;
        } else {
            self.planet_commission_index = self.planet_commission_index.min(planet_rows.len() - 1);
            let current_rows = self.current_planet_commission_rows();
            if current_rows.is_empty() {
                self.move_planet_commission_planet(1);
            } else {
                self.planet_commission_cursor =
                    self.planet_commission_cursor.min(current_rows.len() - 1);
            }
        }
        self.planet_commission_selected_slots.clear();
        Ok(())
    }

    pub fn confirm_planet_auto_commission(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::PlanetAutoCommissionConfirm {
            return Ok(());
        }
        let summary = self
            .game_data
            .auto_commission_all_stardock_units(self.player.record_index_1_based)?;
        self.save_game_data()?;
        self.planet_auto_commission_status = Some(format_auto_commission_status(summary));
        self.current_screen = ScreenId::PlanetAutoCommissionDone;
        Ok(())
    }

    pub fn move_planet_transport_planet(&mut self, delta: i8) {
        let ScreenId::PlanetTransportPlanetSelect(mode) = self.current_screen else {
            return;
        };
        let total = self.planet_transport_planet_rows(mode).len();
        if total == 0 {
            self.planet_transport_planet_cursor = 0;
            return;
        }
        let next = self.planet_transport_planet_cursor as isize + delta as isize;
        self.planet_transport_planet_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.planet_transport_planet_scroll_offset,
            self.planet_transport_planet_cursor,
            crate::screen::PLANET_TRANSPORT_VISIBLE_ROWS,
        );
    }

    pub fn confirm_planet_transport_planet(&mut self) {
        let ScreenId::PlanetTransportPlanetSelect(mode) = self.current_screen else {
            return;
        };
        let Some(selected_planet) = self
            .planet_transport_planet_rows(mode)
            .get(self.planet_transport_planet_cursor)
            .cloned()
        else {
            return;
        };
        self.planet_transport_selected_planet_record =
            Some(selected_planet.planet_record_index_1_based);
        self.planet_transport_fleet_cursor = 0;
        self.planet_transport_fleet_scroll_offset = 0;
        self.planet_transport_qty_input.clear();
        self.planet_transport_status = None;
        if self
            .current_planet_transport_fleet_rows(mode)
            .unwrap_or_default()
            .is_empty()
        {
            self.planet_transport_status = Some(match mode {
                PlanetTransportMode::Load => "No fleets here can take more armies.".to_string(),
                PlanetTransportMode::Unload => "No fleets here have loaded armies.".to_string(),
            });
            self.current_screen = ScreenId::PlanetTransportPlanetSelect(mode);
        } else {
            self.current_screen = ScreenId::PlanetTransportFleetSelect(mode);
        }
    }

    pub fn move_planet_transport_fleet(&mut self, delta: i8) {
        let ScreenId::PlanetTransportFleetSelect(mode) = self.current_screen else {
            return;
        };
        let total = self
            .current_planet_transport_fleet_rows(mode)
            .map(|rows| rows.len())
            .unwrap_or(0);
        if total == 0 {
            self.planet_transport_fleet_cursor = 0;
            return;
        }
        let next = self.planet_transport_fleet_cursor as isize + delta as isize;
        self.planet_transport_fleet_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.planet_transport_fleet_scroll_offset,
            self.planet_transport_fleet_cursor,
            crate::screen::PLANET_TRANSPORT_VISIBLE_ROWS,
        );
        self.planet_transport_qty_input.clear();
        self.planet_transport_status = None;
    }

    pub fn confirm_planet_transport_fleet(&mut self) {
        let ScreenId::PlanetTransportFleetSelect(mode) = self.current_screen else {
            return;
        };
        self.planet_transport_qty_input.clear();
        self.planet_transport_status = None;
        self.current_screen = ScreenId::PlanetTransportFleetSelect(mode);
    }

    pub fn append_planet_transport_qty_char(&mut self, ch: char) {
        if matches!(
            self.current_screen,
            ScreenId::PlanetTransportFleetSelect(_) | ScreenId::PlanetTransportQuantityPrompt(_)
        ) && self.planet_transport_qty_input.len() < 3
        {
            self.planet_transport_qty_input.push(ch);
            self.planet_transport_status = None;
        }
    }

    pub fn backspace_planet_transport_qty(&mut self) {
        if matches!(
            self.current_screen,
            ScreenId::PlanetTransportFleetSelect(_) | ScreenId::PlanetTransportQuantityPrompt(_)
        ) {
            self.planet_transport_qty_input.pop();
            self.planet_transport_status = None;
        }
    }

    pub fn submit_planet_transport_qty(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mode = match self.current_screen {
            ScreenId::PlanetTransportFleetSelect(mode)
            | ScreenId::PlanetTransportQuantityPrompt(mode) => mode,
            _ => return Ok(()),
        };
        let fleet = self.current_planet_transport_fleet_row(mode)?;
        let max_qty = fleet.available_qty;
        if max_qty == 0 {
            self.planet_transport_status = Some(match mode {
                PlanetTransportMode::Load => {
                    format!("Fleet {} cannot take any more armies.", fleet.fleet_number)
                }
                PlanetTransportMode::Unload => {
                    format!(
                        "Fleet {} has no loaded armies to unload.",
                        fleet.fleet_number
                    )
                }
            });
            self.current_screen = ScreenId::PlanetTransportFleetSelect(mode);
            return Ok(());
        }
        let qty = if self.planet_transport_qty_input.trim().is_empty() {
            max_qty
        } else {
            match self.planet_transport_qty_input.trim().parse::<u16>() {
                Ok(value) if value > 0 => value,
                _ => {
                    self.planet_transport_status = Some("Enter a positive army count.".to_string());
                    return Ok(());
                }
            }
        };
        if qty > max_qty {
            self.planet_transport_status = Some(format!("Enter a value from 1 to {max_qty}."));
            return Ok(());
        }
        let planet = self.current_planet_transport_planet_row(mode)?;
        let result = match mode {
            PlanetTransportMode::Load => self.game_data.load_planet_armies_onto_fleet(
                self.player.record_index_1_based,
                planet.planet_record_index_1_based,
                fleet.fleet_record_index_1_based,
                qty,
            ),
            PlanetTransportMode::Unload => self.game_data.unload_fleet_armies_to_planet(
                self.player.record_index_1_based,
                planet.planet_record_index_1_based,
                fleet.fleet_record_index_1_based,
                qty,
            ),
        };
        match result {
            Ok(()) => {}
            Err(GameStateMutationError::PlanetArmyCapacityExceeded { available, .. }) => {
                self.planet_transport_status = Some(if available == 0 {
                    "This planet is already at the maximum 255 armies.".to_string()
                } else {
                    format!("Planet can receive only {available} more armies.")
                });
                self.current_screen = ScreenId::PlanetTransportFleetSelect(mode);
                return Ok(());
            }
            Err(err) => return Err(err.into()),
        }
        self.save_game_data()?;
        self.planet_transport_status = None;
        self.planet_transport_qty_input.clear();
        let base_row = self
            .build_planet_rows()
            .into_iter()
            .find(|row| row.planet_record_index_1_based == planet.planet_record_index_1_based)
            .ok_or("transport planet row missing after submit")?;
        let eligible_fleets = self.planet_transport_eligible_fleet_rows_for_planet(mode, &base_row);
        if !eligible_fleets.is_empty() {
            self.planet_transport_fleet_cursor = self
                .planet_transport_fleet_cursor
                .min(eligible_fleets.len() - 1);
            self.current_screen = ScreenId::PlanetTransportFleetSelect(mode);
        } else {
            let planet_rows = self.planet_transport_planet_rows(mode);
            self.planet_transport_selected_planet_record = None;
            if !planet_rows.is_empty() {
                self.planet_transport_planet_cursor = self
                    .planet_transport_planet_cursor
                    .min(planet_rows.len() - 1);
                self.current_screen = ScreenId::PlanetTransportPlanetSelect(mode);
            } else {
                self.planet_transport_status = None;
                self.current_screen = ScreenId::PlanetMenu;
            }
        }
        Ok(())
    }

    pub fn toggle_planet_commission_selection(&mut self) {
        if self.current_screen != ScreenId::PlanetCommissionMenu {
            return;
        }
        let rows = self.current_planet_commission_rows();
        let Some(row) = rows.get(self.planet_commission_cursor) else {
            return;
        };
        if self
            .planet_commission_selected_slots
            .contains(&row.slot_0_based)
        {
            self.planet_commission_selected_slots
                .remove(&row.slot_0_based);
        } else {
            self.planet_commission_selected_slots
                .insert(row.slot_0_based);
        }
        self.planet_commission_status = None;
    }

    pub fn scroll_planet_build_list(&mut self, delta: i8) {
        if self.current_screen != ScreenId::PlanetBuildList {
            return;
        }
        let total = self.planet_build_list_rows().len();
        let max_offset = total.saturating_sub(crate::screen::PLANET_BUILD_LIST_VISIBLE_ROWS);
        self.planet_build_list_scroll_offset = self
            .planet_build_list_scroll_offset
            .saturating_add_signed(delta as isize)
            .min(max_offset);
    }

    pub fn move_planet_build_list_cursor(&mut self, delta: i8) {
        if self.current_screen != ScreenId::PlanetBuildList {
            return;
        }
        let total = self.planet_build_list_rows().len();
        if total == 0 {
            self.planet_build_list_cursor = 0;
            return;
        }
        let next = self.planet_build_list_cursor as isize + delta as isize;
        self.planet_build_list_cursor = next.rem_euclid(total as isize) as usize;
        // Keep scroll window in sync: ensure cursor is visible.
        if self.planet_build_list_cursor < self.planet_build_list_scroll_offset {
            self.planet_build_list_scroll_offset = self.planet_build_list_cursor;
        } else if self.planet_build_list_cursor
            >= self.planet_build_list_scroll_offset + crate::screen::PLANET_BUILD_LIST_VISIBLE_ROWS
        {
            self.planet_build_list_scroll_offset =
                self.planet_build_list_cursor + 1 - crate::screen::PLANET_BUILD_LIST_VISIBLE_ROWS;
        }
    }

    pub fn delete_planet_build_slot_request(&mut self) {
        if self.current_screen != ScreenId::PlanetBuildList {
            return;
        }
        let rows = self.planet_build_list_rows();
        let Some(row) = rows.get(self.planet_build_list_cursor) else {
            return;
        };
        if row.queue_qty == 0 {
            return;
        }
        self.planet_build_list_confirming = true;
    }

    pub fn confirm_delete_planet_build_slot(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.planet_build_list_confirming {
            return Ok(());
        }
        let rows = self.planet_build_list_rows();
        let Some(row) = rows.get(self.planet_build_list_cursor) else {
            self.planet_build_list_confirming = false;
            return Ok(());
        };
        let planet_record = self.current_build_planet_row()?.planet_record_index_1_based;
        let record = self
            .game_data
            .planets
            .records
            .get_mut(planet_record - 1)
            .ok_or("planet record missing")?;
        for slot in 0..10 {
            if ProductionItemKind::from_raw(record.build_kind_raw(slot)) == row.kind {
                record.set_build_count_raw(slot, 0);
                record.set_build_kind_raw(slot, 0);
            }
        }
        self.save_game_data()?;
        self.planet_build_list_confirming = false;
        // Clamp cursor after deletion.
        let new_total = self.planet_build_list_rows().len();
        if new_total == 0 {
            self.planet_build_list_cursor = 0;
        } else {
            self.planet_build_list_cursor = self.planet_build_list_cursor.min(new_total - 1);
        }
        Ok(())
    }

    pub fn cancel_delete_planet_build_slot(&mut self) {
        self.planet_build_list_confirming = false;
    }

    pub fn append_planet_build_unit_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::PlanetBuildSpecify
            && self.planet_build_unit_input.len() < 2
        {
            self.planet_build_unit_input.push(ch);
            self.planet_build_unit_status = None;
        }
    }

    pub fn backspace_planet_build_unit_input(&mut self) {
        if self.current_screen == ScreenId::PlanetBuildSpecify {
            self.planet_build_unit_input.pop();
            self.planet_build_unit_status = None;
        }
    }

    pub fn submit_planet_build_unit(&mut self) {
        let raw = self.planet_build_unit_input.trim();
        let number = if raw.is_empty() {
            0
        } else if let Ok(value) = raw.parse::<u8>() {
            value
        } else {
            self.planet_build_unit_status = Some("Enter a valid unit number.".to_string());
            return;
        };

        if number == 0 {
            self.current_screen = ScreenId::PlanetBuildMenu;
            return;
        }

        let Some(unit) = build_unit_spec(number) else {
            self.planet_build_unit_status = Some("That unit is not available.".to_string());
            return;
        };

        let Ok(max_qty) = self.current_planet_build_max_quantity_for(unit.kind) else {
            self.planet_build_unit_status = Some("No points are available to spend.".to_string());
            return;
        };
        if max_qty == 0 {
            self.planet_build_unit_status = Some(
                self.planet_build_unavailable_message(unit.kind)
                    .unwrap_or_else(|_| "No points are available to spend.".to_string()),
            );
            return;
        }

        self.planet_build_selected_kind = Some(unit.kind);
        self.planet_build_quantity_input.clear();
        self.planet_build_quantity_status = None;
        self.current_screen = ScreenId::PlanetBuildQuantity;
    }

    pub fn append_planet_build_quantity_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::PlanetBuildQuantity
            && self.planet_build_quantity_input.len() < 3
        {
            self.planet_build_quantity_input.push(ch);
            self.planet_build_quantity_status = None;
        }
    }

    pub fn backspace_planet_build_quantity_input(&mut self) {
        if self.current_screen == ScreenId::PlanetBuildQuantity {
            self.planet_build_quantity_input.pop();
            self.planet_build_quantity_status = None;
        }
    }

    pub fn submit_planet_build_quantity(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(kind) = self.planet_build_selected_kind else {
            self.current_screen = ScreenId::PlanetBuildSpecify;
            return Ok(());
        };
        let Some(unit) = build_unit_spec_by_kind(kind) else {
            self.current_screen = ScreenId::PlanetBuildSpecify;
            return Ok(());
        };
        let max_qty = self.current_planet_build_max_quantity_for(kind)?;
        if max_qty == 0 {
            self.planet_build_quantity_status = Some(self.planet_build_unavailable_message(kind)?);
            return Ok(());
        }

        let qty = if self.planet_build_quantity_input.trim().is_empty() {
            max_qty
        } else {
            match self.planet_build_quantity_input.trim().parse::<u32>() {
                Ok(value) => value,
                Err(_) => {
                    self.planet_build_quantity_status = Some("Enter a valid quantity.".to_string());
                    return Ok(());
                }
            }
        };

        if qty == 0 {
            self.current_screen = ScreenId::PlanetBuildSpecify;
            self.planet_build_quantity_input.clear();
            return Ok(());
        }
        if qty > max_qty {
            self.planet_build_quantity_status =
                Some(format!("Enter a quantity from 0 to {}.", max_qty));
            return Ok(());
        }

        let planet_record = self.current_build_planet_row()?.planet_record_index_1_based;

        // Armies and ground batteries go directly to the planet — no stardock needed.
        // For all other kinds (ships, starbases), each queued order will need one
        // stardock slot on completion. Warn and cap if the stardock is full.
        let needs_stardock = !matches!(
            kind,
            ProductionItemKind::Army | ProductionItemKind::GroundBattery
        );
        if needs_stardock {
            let free = self.game_data.planet_free_stardock_slots(planet_record)?;
            if free == 0 {
                self.planet_build_quantity_status =
                    Some("Stardock is full — commission ships first to free space.".to_string());
                return Ok(());
            }
        }

        let points = qty.saturating_mul(unit.cost);
        match self.game_data.append_planet_build_order(
            planet_record,
            points.min(u32::from(u8::MAX)) as u8,
            production_item_kind_raw(kind),
        ) {
            Ok(()) => {}
            Err(GameStateMutationError::PlanetBuildQueueFull { .. }) => {
                self.planet_build_quantity_status =
                    Some("Build queue is full (10 orders maximum).".to_string());
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        }
        self.save_game_data()?;
        self.planet_build_unit_input.clear();
        self.planet_build_unit_status = Some(format!("Queued {} {}.", qty, unit.label));
        self.planet_build_quantity_input.clear();
        self.planet_build_quantity_status = None;
        self.planet_build_selected_kind = None;
        self.current_screen = ScreenId::PlanetBuildSpecify;
        Ok(())
    }

    pub fn abort_current_planet_builds(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let row = self.current_build_planet_row()?;
        self.game_data
            .clear_planet_build_queue(row.planet_record_index_1_based)?;
        self.save_game_data()?;
        self.planet_build_status = Some("Build orders aborted.".to_string());
        self.current_screen = ScreenId::PlanetBuildMenu;
        Ok(())
    }

    pub fn append_planet_tax_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::PlanetTaxPrompt && self.planet_tax_input.len() < 3 {
            self.planet_tax_input.push(ch);
            self.planet_tax_status = None;
        }
    }

    pub fn backspace_planet_tax_input(&mut self) {
        if self.current_screen == ScreenId::PlanetTaxPrompt {
            self.planet_tax_input.pop();
            self.planet_tax_status = None;
        }
    }

    pub fn submit_planet_tax(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let raw = self.planet_tax_input.trim();
        let parsed = if raw.is_empty() {
            self.game_data.player.records[self.player.record_index_1_based - 1].tax_rate()
        } else {
            match raw.parse::<u8>() {
                Ok(value) => value,
                Err(_) => {
                    self.planet_tax_status =
                        Some("Enter an integer tax rate from 0 to 100.".to_string());
                    return Ok(());
                }
            }
        };
        if parsed > 100 {
            self.planet_tax_status = Some("Enter an integer tax rate from 0 to 100.".to_string());
            return Ok(());
        }
        self.game_data
            .set_player_tax_rate(self.player.record_index_1_based, parsed)?;
        self.save_game_data()?;
        self.planet_tax_input = parsed.to_string();
        self.planet_tax_status = Some(format!("Empire tax rate set to {parsed}%."));
        self.current_screen = ScreenId::PlanetTaxDone;
        Ok(())
    }

    pub fn handle_key(&self, key: crossterm::event::KeyEvent) -> crate::app::Action {
        match self.current_screen {
            ScreenId::Startup(StartupPhase::Splash)
                if self.startup_splash_page + 1 < STARTUP_SPLASH_PAGE_COUNT =>
            {
                match key.code {
                    crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Char('Q') => {
                        Action::Quit
                    }
                    _ => Action::AdvanceStartup,
                }
            }
            ScreenId::Startup(phase) => self.startup.handle_key(phase, key),
            ScreenId::FirstTimeMenu => self.first_time_menu.handle_key(key),
            ScreenId::FirstTimeHelp => self.first_time_help.handle_key(key),
            ScreenId::FirstTimeEmpires => self.first_time_empires.handle_key(key),
            ScreenId::FirstTimeIntro
                if self.first_time_intro_page + 1 < FIRST_TIME_INTRO_PAGE_COUNT =>
            {
                Action::AdvanceStartup
            }
            ScreenId::FirstTimeIntro => self.first_time_intro.handle_key(key),
            ScreenId::FirstTimeJoinEmpireName | ScreenId::FirstTimeHomeworldName => {
                match key.code {
                    crossterm::event::KeyCode::Char(ch) => Action::AppendFirstTimeInputChar(ch),
                    crossterm::event::KeyCode::Backspace => Action::BackspaceFirstTimeInput,
                    crossterm::event::KeyCode::Enter => Action::SubmitFirstTimeInput,
                    crossterm::event::KeyCode::Esc => Action::OpenFirstTimeMenu,
                    _ => Action::Noop,
                }
            }
            ScreenId::FirstTimeJoinEmpireConfirm => match key.code {
                crossterm::event::KeyCode::Enter
                | crossterm::event::KeyCode::Char('y')
                | crossterm::event::KeyCode::Char('Y') => Action::AcceptFirstTimePrompt,
                crossterm::event::KeyCode::Char('n')
                | crossterm::event::KeyCode::Char('N')
                | crossterm::event::KeyCode::Esc => Action::RejectFirstTimePrompt,
                _ => Action::Noop,
            },
            ScreenId::FirstTimeJoinSummary | ScreenId::FirstTimeJoinNoPending => match key.code {
                crossterm::event::KeyCode::Enter => Action::AcceptFirstTimePrompt,
                _ => Action::Noop,
            },
            ScreenId::FirstTimeHomeworldConfirm => match key.code {
                crossterm::event::KeyCode::Char('y') | crossterm::event::KeyCode::Char('Y') => {
                    Action::AcceptFirstTimePrompt
                }
                crossterm::event::KeyCode::Enter
                | crossterm::event::KeyCode::Char('n')
                | crossterm::event::KeyCode::Char('N')
                | crossterm::event::KeyCode::Esc => Action::RejectFirstTimePrompt,
                _ => Action::Noop,
            },
            ScreenId::MainMenu => self.main_menu.handle_key(key),
            ScreenId::GeneralMenu => self.general_menu.handle_key(key),
            ScreenId::GeneralHelp => self.general_help.handle_key(key),
            ScreenId::FleetHelp => self.fleet_help.handle_key(key),
            ScreenId::FleetMenu => self.fleet_menu.handle_key(key),
            ScreenId::FleetList(_) => self.fleet_list.handle_key(key),
            ScreenId::FleetReview => self.fleet_review.handle_key(key),
            ScreenId::FleetRoeSelect => self.handle_fleet_roe_key(key),
            ScreenId::FleetDetach => self.handle_fleet_detach_key(key),
            ScreenId::FleetEta => self.handle_fleet_eta_key(key),
            ScreenId::PlanetMenu => self.planet_menu.handle_key(key),
            ScreenId::PlanetHelp => self.planet_help.handle_key(key),
            ScreenId::PlanetAutoCommissionConfirm => self.planet_auto_commission.handle_key(key),
            ScreenId::PlanetAutoCommissionDone => Action::OpenPlanetMenu,
            ScreenId::PlanetCommissionMenu => self.planet_commission.handle_key(key),
            ScreenId::PlanetTransportPlanetSelect(_) => {
                self.planet_transport.handle_planet_key(key)
            }
            ScreenId::PlanetTransportFleetSelect(_) => self.planet_transport.handle_fleet_key(key),
            ScreenId::PlanetTransportQuantityPrompt(_) => {
                self.planet_transport.handle_quantity_key(key)
            }
            ScreenId::PlanetTransportDone(_) => Action::OpenPlanetMenu,
            ScreenId::PlanetBuildHelp => self.build_help.handle_key(key),
            ScreenId::PlanetBuildMenu => self.planet_build.handle_menu_key(key),
            ScreenId::PlanetBuildReview => self.planet_build.handle_review_key(key),
            ScreenId::PlanetBuildList => self
                .planet_build
                .handle_list_key(key, self.planet_build_list_confirming),
            ScreenId::PlanetBuildChange => self.planet_build.handle_change_key(key),
            ScreenId::PlanetBuildAbortConfirm => self.planet_build.handle_abort_key(key),
            ScreenId::PlanetBuildSpecify => self.planet_build.handle_specify_key(key),
            ScreenId::PlanetBuildQuantity => self.planet_build.handle_quantity_key(key),
            ScreenId::PlanetListSortPrompt(PlanetListMode::Stub(_)) => Action::OpenPlanetMenu,
            ScreenId::PlanetListSortPrompt(_) => self.planet_list.handle_sort_prompt_key(key),
            ScreenId::PlanetBriefList(_) => self.planet_list.handle_brief_key(key),
            ScreenId::PlanetDetailList(_) => self.planet_list.handle_detail_key(key),
            ScreenId::PlanetTaxPrompt => self.planet_tax.handle_prompt_key(key),
            ScreenId::PlanetTaxDone => self.planet_tax.handle_done_key(key),
            ScreenId::Starmap if self.starmap_capture_complete => {
                self.starmap.handle_complete_key(key)
            }
            ScreenId::Starmap if self.starmap_dump_active => self.starmap.handle_dump_key(key),
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

    pub fn open_planet_info_prompt(&mut self, menu: CommandMenu) {
        self.command_return_menu = menu;
        self.planet_info_input.clear();
        self.planet_info_error = None;
        self.planet_info_selected = None;
        self.current_screen = ScreenId::PlanetInfoPrompt;
    }

    pub fn open_enemies(&mut self) {
        self.enemies_input.clear();
        self.enemies_status = None;
        self.enemies_scroll_offset = 0;
        self.enemies_cursor = 0;
        self.current_screen = ScreenId::Enemies;
    }

    pub fn open_delete_reviewables(&mut self) {
        self.delete_reviewables_status = None;
        self.current_screen = ScreenId::DeleteReviewables;
    }

    pub fn open_empire_status(&mut self) {
        self.command_return_menu = self.origin_command_menu();
        self.current_screen = ScreenId::EmpireStatus;
    }

    pub fn open_empire_profile(&mut self) {
        self.command_return_menu = self.origin_command_menu();
        self.current_screen = ScreenId::EmpireProfile;
    }

    pub fn open_rankings_table(&mut self, sort: ec_data::EmpireProductionRankingSort) {
        self.command_return_menu = self.origin_command_menu();
        self.current_screen = ScreenId::Rankings(sort);
    }

    pub fn open_reports(&mut self) {
        self.command_return_menu = self.origin_command_menu();
        self.current_screen = ScreenId::Reports;
    }

    pub fn open_compose_message_recipient(&mut self) {
        self.compose_recipient_input.clear();
        self.compose_recipient_status = None;
        self.compose_recipient_scroll_offset = 0;
        self.compose_recipient_cursor = 0;
        self.compose_recipient_empire = None;
        self.compose_subject.clear();
        self.compose_subject_status = None;
        self.compose_body.clear();
        self.compose_body_cursor = 0;
        self.compose_body_status = None;
        self.compose_outbox_input.clear();
        self.compose_outbox_status = None;
        self.compose_outbox_scroll_offset = 0;
        self.compose_sent_status = None;
        self.current_screen = ScreenId::ComposeMessageRecipient;
    }

    pub fn open_compose_message_subject(&mut self) {
        if self.compose_recipient_empire.is_none() {
            self.open_compose_message_recipient();
            return;
        }
        self.compose_subject_status = None;
        self.current_screen = ScreenId::ComposeMessageSubject;
    }

    pub fn open_compose_message_body(&mut self) {
        if self.compose_recipient_empire.is_none() {
            self.open_compose_message_recipient();
            return;
        }
        self.compose_body_status = None;
        self.current_screen = ScreenId::ComposeMessageBody;
    }

    pub fn open_compose_message_outbox(&mut self) {
        self.compose_outbox_input.clear();
        self.compose_outbox_status = None;
        self.compose_outbox_scroll_offset = 0;
        self.compose_outbox_cursor = 0;
        self.current_screen = ScreenId::ComposeMessageOutbox;
    }

    pub fn open_compose_message_discard_confirm(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageBody {
            self.current_screen = ScreenId::ComposeMessageDiscardConfirm;
        }
    }

    pub fn open_compose_message_send_confirm(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageBody {
            let body = self.compose_body.trim();
            if body.is_empty() {
                self.compose_body_status = Some("Message body cannot be empty.".to_string());
                return;
            }
            self.current_screen = ScreenId::ComposeMessageSendConfirm;
        }
    }

    pub fn scroll_enemies(&mut self, delta: i8) {
        if self.current_screen != ScreenId::Enemies {
            return;
        }
        let total = self.game_data.player.records.len().saturating_sub(1);
        let max_offset = total.saturating_sub(crate::screen::ENEMIES_VISIBLE_ROWS);
        self.enemies_scroll_offset = self
            .enemies_scroll_offset
            .saturating_add_signed(delta as isize)
            .min(max_offset);
    }

    pub fn move_enemies_cursor(&mut self, delta: i8) {
        if self.current_screen != ScreenId::Enemies {
            return;
        }
        // Total rows = all empires minus self.
        let total = self.game_data.player.records.len().saturating_sub(1);
        if total == 0 {
            return;
        }
        let next = self.enemies_cursor as isize + delta as isize;
        self.enemies_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.enemies_scroll_offset,
            self.enemies_cursor,
            crate::screen::ENEMIES_VISIBLE_ROWS,
        );
    }

    pub fn open_partial_starmap_prompt(&mut self, menu: CommandMenu) {
        self.command_return_menu = menu;
        let default = self.default_planet_prompt_coords();
        self.partial_starmap_input.clear();
        self.partial_starmap_error = None;
        self.partial_starmap_center = default;
        self.current_screen = ScreenId::PartialStarmapPrompt;
    }

    pub fn return_to_command_menu(&mut self) {
        self.current_screen = match self.command_return_menu {
            CommandMenu::Main => ScreenId::MainMenu,
            CommandMenu::General => ScreenId::GeneralMenu,
            CommandMenu::Fleet => ScreenId::FleetMenu,
            CommandMenu::Planet => ScreenId::PlanetMenu,
            CommandMenu::PlanetBuild => ScreenId::PlanetBuildMenu,
        };
    }

    fn origin_command_menu(&self) -> CommandMenu {
        match self.current_screen {
            ScreenId::MainMenu | ScreenId::PlanetDatabaseList | ScreenId::PlanetDatabaseDetail => {
                CommandMenu::Main
            }
            ScreenId::FleetHelp
            | ScreenId::FleetMenu
            | ScreenId::FleetList(_)
            | ScreenId::FleetReview
            | ScreenId::FleetRoeSelect
            | ScreenId::FleetDetach
            | ScreenId::FleetEta => CommandMenu::Fleet,
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
            | ScreenId::FirstTimeJoinEmpireName
            | ScreenId::FirstTimeJoinEmpireConfirm
            | ScreenId::FirstTimeJoinSummary
            | ScreenId::FirstTimeJoinNoPending
            | ScreenId::FirstTimeHomeworldName
            | ScreenId::FirstTimeHomeworldConfirm
            | ScreenId::PartialStarmapPrompt
            | ScreenId::PartialStarmapView
            | ScreenId::PlanetInfoPrompt
            | ScreenId::PlanetInfoDetail => self.command_return_menu,
        }
    }

    pub fn append_partial_starmap_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::PartialStarmapPrompt
            && self.partial_starmap_input.len() < 16
        {
            self.partial_starmap_input.push(ch);
            self.partial_starmap_error = None;
        }
    }

    pub fn backspace_partial_starmap_input(&mut self) {
        if self.current_screen == ScreenId::PartialStarmapPrompt {
            self.partial_starmap_input.pop();
            self.partial_starmap_error = None;
        }
    }

    pub fn submit_partial_starmap_prompt(&mut self) {
        let Some(coords) =
            resolve_default_coords_input(&self.partial_starmap_input, self.partial_starmap_center)
        else {
            self.partial_starmap_error = Some("Enter coordinates like 5,2".to_string());
            return;
        };
        let map_size = ec_data::map_size_for_player_count(self.game_data.conquest.player_count());
        if coords[0] == 0 || coords[1] == 0 || coords[0] > map_size || coords[1] > map_size {
            self.partial_starmap_error = Some(format!("Enter coordinates within 1..{map_size}"));
            return;
        }
        self.partial_starmap_center = coords;
        self.partial_starmap_error = None;
        self.current_screen = ScreenId::PartialStarmapView;
    }

    pub fn move_partial_starmap(&mut self, dx: i8, dy: i8) {
        let map_size = ec_data::map_size_for_player_count(self.game_data.conquest.player_count());
        self.partial_starmap_center[0] = self.partial_starmap_center[0]
            .saturating_add_signed(dx)
            .clamp(1, map_size);
        self.partial_starmap_center[1] = self.partial_starmap_center[1]
            .saturating_add_signed(dy)
            .clamp(1, map_size);
    }

    pub fn toggle_autopilot(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let player = &mut self.game_data.player.records[self.player.record_index_1_based - 1];
        let next = if player.autopilot_flag() == 0 { 1 } else { 0 };
        player.set_autopilot_flag(next);
        self.save_game_data()?;
        Ok(())
    }

    pub fn append_enemies_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::Enemies && self.enemies_input.len() < 2 {
            self.enemies_input.push(ch);
            self.enemies_status = None;
        }
    }

    pub fn backspace_enemies_input(&mut self) {
        if self.current_screen == ScreenId::Enemies {
            self.enemies_input.pop();
            self.enemies_status = None;
        }
    }

    pub fn submit_enemies_input(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // If the input box is empty, derive the empire id from the cursor row.
        let empire_id = if self.enemies_input.trim().is_empty() {
            let mut ids: Vec<u8> = self
                .game_data
                .player
                .records
                .iter()
                .enumerate()
                .filter(|(idx, _)| *idx + 1 != self.player.record_index_1_based)
                .map(|(idx, _)| (idx + 1) as u8)
                .collect();
            ids.sort_unstable();
            match ids.get(self.enemies_cursor) {
                Some(&id) => id,
                None => {
                    self.enemies_status = Some("No empire selected.".to_string());
                    return Ok(());
                }
            }
        } else {
            match self.enemies_input.parse::<u8>() {
                Ok(id) => id,
                Err(_) => {
                    self.enemies_status = Some("Enter an empire number.".to_string());
                    return Ok(());
                }
            }
        };
        let max_empire = self.game_data.conquest.player_count();
        if !(1..=max_empire).contains(&empire_id) {
            self.enemies_status = Some(format!("Enter an empire number in 1..={max_empire}."));
            return Ok(());
        }
        if empire_id as usize == self.player.record_index_1_based {
            self.enemies_status = Some("You cannot target your own empire.".to_string());
            return Ok(());
        }
        let current = self
            .game_data
            .stored_diplomatic_relation(self.player.record_index_1_based as u8, empire_id)
            .unwrap_or(ec_data::DiplomaticRelation::Neutral);
        let next = match current {
            ec_data::DiplomaticRelation::Neutral => ec_data::DiplomaticRelation::Enemy,
            ec_data::DiplomaticRelation::Enemy => ec_data::DiplomaticRelation::Neutral,
        };
        self.game_data.set_stored_diplomatic_relation(
            self.player.record_index_1_based as u8,
            empire_id,
            next,
        )?;
        self.save_game_data()?;
        self.enemies_status = None;
        self.enemies_input.clear();
        Ok(())
    }

    pub fn append_compose_recipient_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::ComposeMessageRecipient
            && self.compose_recipient_input.len() < 2
        {
            self.compose_recipient_input.push(ch);
            self.compose_recipient_status = None;
        }
    }

    pub fn scroll_compose_recipients(&mut self, delta: i8) {
        if self.current_screen != ScreenId::ComposeMessageRecipient {
            return;
        }
        let total = self.game_data.player.records.len().saturating_sub(1);
        let max_offset = total.saturating_sub(crate::screen::RECIPIENT_VISIBLE_ROWS);
        self.compose_recipient_scroll_offset = self
            .compose_recipient_scroll_offset
            .saturating_add_signed(delta as isize)
            .min(max_offset);
    }

    pub fn move_compose_recipient_cursor(&mut self, delta: i8) {
        if self.current_screen != ScreenId::ComposeMessageRecipient {
            return;
        }
        let total = self.game_data.player.records.len().saturating_sub(1);
        if total == 0 {
            return;
        }
        let next = self.compose_recipient_cursor as isize + delta as isize;
        self.compose_recipient_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.compose_recipient_scroll_offset,
            self.compose_recipient_cursor,
            crate::screen::RECIPIENT_VISIBLE_ROWS,
        );
    }

    pub fn backspace_compose_recipient(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageRecipient {
            self.compose_recipient_input.pop();
            self.compose_recipient_status = None;
        }
    }

    pub fn submit_compose_recipient(&mut self) {
        // If the input box is empty, derive the empire id from the cursor row.
        let empire_id = if self.compose_recipient_input.trim().is_empty() {
            let ids: Vec<u8> = self
                .game_data
                .player
                .records
                .iter()
                .enumerate()
                .filter(|(idx, _)| *idx + 1 != self.player.record_index_1_based)
                .map(|(idx, _)| (idx + 1) as u8)
                .collect();
            match ids.get(self.compose_recipient_cursor) {
                Some(&id) => id,
                None => {
                    self.compose_recipient_status = Some("No empire selected.".to_string());
                    return;
                }
            }
        } else {
            match self.compose_recipient_input.parse::<u8>() {
                Ok(id) => id,
                Err(_) => {
                    self.compose_recipient_status = Some("Enter an empire number.".to_string());
                    return;
                }
            }
        };
        let max_empire = self.game_data.conquest.player_count();
        if !(1..=max_empire).contains(&empire_id) {
            self.compose_recipient_status =
                Some(format!("Enter an empire number in 1..={max_empire}."));
            return;
        }
        if empire_id as usize == self.player.record_index_1_based {
            self.compose_recipient_status = Some("You cannot message your own empire.".to_string());
            return;
        }
        self.compose_recipient_empire = Some(empire_id);
        self.compose_subject.clear();
        self.compose_subject_status = None;
        self.compose_body.clear();
        self.compose_body_cursor = 0;
        self.compose_body_status = None;
        self.current_screen = ScreenId::ComposeMessageSubject;
    }

    pub fn append_compose_subject_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::ComposeMessageSubject
            && self.compose_subject.chars().count() < crate::screen::COMPOSE_SUBJECT_LIMIT
        {
            self.compose_subject.push(ch);
            self.compose_subject_status = None;
        }
    }

    pub fn backspace_compose_subject(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageSubject {
            self.compose_subject.pop();
            self.compose_subject_status = None;
        }
    }

    pub fn submit_compose_subject(&mut self) {
        if self.current_screen != ScreenId::ComposeMessageSubject {
            return;
        }
        self.compose_body_cursor = self.compose_body.chars().count();
        self.compose_body_status = None;
        self.current_screen = ScreenId::ComposeMessageBody;
    }

    pub fn confirm_discard_composed_message(&mut self) {
        self.open_compose_message_recipient();
    }

    pub fn append_compose_body_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::ComposeMessageBody
            && self.compose_body.chars().count() < crate::screen::COMPOSE_BODY_LIMIT
        {
            insert_char_at(&mut self.compose_body, self.compose_body_cursor, ch);
            self.compose_body_cursor += 1;
            self.compose_body_status = None;
        } else if self.current_screen == ScreenId::ComposeMessageBody {
            self.compose_body_status = Some(format!(
                "Message length limit is {} characters.",
                crate::screen::COMPOSE_BODY_LIMIT
            ));
        }
    }

    pub fn backspace_compose_body(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageBody {
            if self.compose_body_cursor > 0 {
                remove_char_before(&mut self.compose_body, self.compose_body_cursor);
                self.compose_body_cursor -= 1;
            }
            self.compose_body_status = None;
        }
    }

    pub fn delete_compose_body_char(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageBody {
            remove_char_at(&mut self.compose_body, self.compose_body_cursor);
            self.compose_body_status = None;
        }
    }

    pub fn insert_compose_newline(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageBody
            && self.compose_body.chars().count() < crate::screen::COMPOSE_BODY_LIMIT
        {
            insert_char_at(&mut self.compose_body, self.compose_body_cursor, '\n');
            self.compose_body_cursor += 1;
            self.compose_body_status = None;
        } else if self.current_screen == ScreenId::ComposeMessageBody {
            self.compose_body_status = Some(format!(
                "Message length limit is {} characters.",
                crate::screen::COMPOSE_BODY_LIMIT
            ));
        }
    }

    pub fn move_compose_body_cursor_left(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageBody {
            self.compose_body_cursor = self.compose_body_cursor.saturating_sub(1);
        }
    }

    pub fn move_compose_body_cursor_right(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageBody {
            self.compose_body_cursor =
                (self.compose_body_cursor + 1).min(self.compose_body.chars().count());
        }
    }

    pub fn move_compose_body_cursor_home(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageBody {
            self.compose_body_cursor =
                line_start_index(&self.compose_body, self.compose_body_cursor);
        }
    }

    pub fn move_compose_body_cursor_end(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageBody {
            self.compose_body_cursor = line_end_index(&self.compose_body, self.compose_body_cursor);
        }
    }

    pub fn move_compose_body_cursor_up(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageBody {
            self.compose_body_cursor =
                vertical_cursor_target(&self.compose_body, self.compose_body_cursor, -1);
        }
    }

    pub fn move_compose_body_cursor_down(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageBody {
            self.compose_body_cursor =
                vertical_cursor_target(&self.compose_body, self.compose_body_cursor, 1);
        }
    }

    pub fn send_composed_message(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::ComposeMessageSendConfirm {
            return Ok(());
        }
        let Some(recipient_empire_id) = self.compose_recipient_empire else {
            self.compose_body_status = Some("Choose a recipient first.".to_string());
            return Ok(());
        };
        let body = self.compose_body.trim();
        if body.is_empty() {
            self.compose_body_status = Some("Message body cannot be empty.".to_string());
            return Ok(());
        }
        self.queued_mail.push(QueuedPlayerMail {
            sender_empire_id: self.player.record_index_1_based as u8,
            recipient_empire_id,
            year: self.game_data.conquest.game_year(),
            subject: self.compose_subject.trim().to_string(),
            body: body.to_string(),
        });
        self.save_game_data()?;
        self.compose_sent_status = Some(format!(
            "Message queued for Empire {recipient_empire_id}. It will be delivered after turn maintenance."
        ));
        self.current_screen = ScreenId::ComposeMessageSent;
        Ok(())
    }

    pub fn scroll_compose_outbox(&mut self, delta: i8) {
        if self.current_screen != ScreenId::ComposeMessageOutbox {
            return;
        }
        let total = self.compose_outbox_queue_len();
        let max_offset = total.saturating_sub(crate::screen::OUTBOX_VISIBLE_ROWS);
        self.compose_outbox_scroll_offset = self
            .compose_outbox_scroll_offset
            .saturating_add_signed(delta as isize)
            .min(max_offset);
    }

    pub fn move_compose_outbox_cursor(&mut self, delta: i8) {
        if self.current_screen != ScreenId::ComposeMessageOutbox {
            return;
        }
        let total = self.compose_outbox_queue_len();
        if total == 0 {
            return;
        }
        let next = self.compose_outbox_cursor as isize + delta as isize;
        self.compose_outbox_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.compose_outbox_scroll_offset,
            self.compose_outbox_cursor,
            crate::screen::OUTBOX_VISIBLE_ROWS,
        );
    }

    pub fn append_compose_outbox_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::ComposeMessageOutbox
            && self.compose_outbox_input.len() < 2
        {
            self.compose_outbox_input.push(ch);
            self.compose_outbox_status = None;
        }
    }

    pub fn backspace_compose_outbox_input(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageOutbox {
            self.compose_outbox_input.pop();
            self.compose_outbox_status = None;
        }
    }

    pub fn delete_queued_compose_message(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::ComposeMessageOutbox {
            return Ok(());
        }
        // If the input box is empty, use the cursor row (1-based queue_no).
        let queue_no = if self.compose_outbox_input.trim().is_empty() {
            self.compose_outbox_cursor + 1
        } else {
            let Ok(n) = self.compose_outbox_input.parse::<usize>() else {
                self.compose_outbox_status = Some("Enter a queued message number.".to_string());
                return Ok(());
            };
            if n == 0 {
                self.compose_outbox_status = Some("Enter a queued message number.".to_string());
                return Ok(());
            }
            n
        };

        let sender_empire_id = self.player.record_index_1_based as u8;
        let mut queue = self.queued_mail.clone();
        let own_indexes = queue
            .iter()
            .enumerate()
            .filter_map(|(idx, mail)| (mail.sender_empire_id == sender_empire_id).then_some(idx))
            .collect::<Vec<_>>();
        let Some(queue_index) = own_indexes.get(queue_no - 1).copied() else {
            self.compose_outbox_status = Some(format!(
                "Enter a queued message number in 1..={}.",
                own_indexes.len()
            ));
            return Ok(());
        };

        queue.remove(queue_index);
        self.queued_mail = queue;
        self.save_game_data()?;
        self.compose_outbox_input.clear();
        self.compose_outbox_status = None;

        // Clamp cursor and scroll offset to the new (smaller) queue.
        let new_len = own_indexes.len().saturating_sub(1);
        self.compose_outbox_cursor = self.compose_outbox_cursor.min(new_len.saturating_sub(1));
        let max_offset = new_len.saturating_sub(crate::screen::OUTBOX_VISIBLE_ROWS);
        self.compose_outbox_scroll_offset = self.compose_outbox_scroll_offset.min(max_offset);
        Ok(())
    }

    pub fn delete_reviewables(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        clear_report_bytes(&mut self.results_bytes, &mut self.messages_bytes);
        if let Some(player) = self
            .game_data
            .player
            .records
            .get_mut(self.player.record_index_1_based - 1)
        {
            player.raw[0x30] = 0;
            player.raw[0x34] = 0;
        }
        self.save_game_data()?;
        let refreshed = ReportsPreview::from_bytes(&self.results_bytes, &self.messages_bytes);
        let summary = MainMenuSummary::from_game_data(
            &self.game_data,
            self.player.record_index_1_based,
            !self.results_bytes.is_empty(),
        );
        self.reports
            .replace(refreshed, ReviewSummary::from_main_menu(&summary));
        self.delete_reviewables_status = Some("Messages and results deleted.".to_string());
        Ok(())
    }

    pub fn open_starmap(&mut self) {
        self.starmap_view_x = 1;
        self.starmap_view_y = 1;
        self.starmap_status = None;
        self.starmap_dump_lines.clear();
        self.starmap_dump_offset = 0;
        self.starmap_dump_active = false;
        self.starmap_capture_complete = false;
        self.current_screen = ScreenId::Starmap;
    }

    pub fn export_starmap(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let projection = build_player_starmap_projection(
            &self.game_data,
            &self.database,
            self.player.record_index_1_based as u8,
        );
        std::fs::create_dir_all(&self.export_root)?;
        let filename = format!(
            "ECMAP-P{}-Y{}.TXT",
            self.player.record_index_1_based,
            self.game_data.conquest.game_year()
        );
        let export_path = self.export_root.join(&filename);
        let csv_path = self.export_root.join(filename.replace(".TXT", ".CSV"));
        let details_csv_path = self
            .export_root
            .join(filename.replace(".TXT", "-DETAILS.CSV"));
        std::fs::write(&export_path, projection.render_ascii_export())?;
        std::fs::write(&csv_path, projection.render_csv_export())?;
        std::fs::write(&details_csv_path, projection.render_csv_details_export())?;
        if let Some(queue_dir) = &self.queue_dir {
            std::fs::create_dir_all(queue_dir)?;
            std::fs::copy(&export_path, queue_dir.join(&filename))?;
            std::fs::copy(&csv_path, queue_dir.join(csv_path.file_name().unwrap()))?;
            std::fs::copy(
                &details_csv_path,
                queue_dir.join(details_csv_path.file_name().unwrap()),
            )?;
            self.starmap_status = Some(format!(
                "Exported TXT + grid CSV + details CSV and queued copies in {}",
                queue_dir.display()
            ));
        } else {
            self.starmap_status = Some(format!(
                "Exported {}, {}, and {}",
                export_path.display(),
                csv_path.display(),
                details_csv_path.display()
            ));
        }
        Ok(())
    }

    pub fn starmap_dump_text(&self) -> String {
        build_player_starmap_projection(
            &self.game_data,
            &self.database,
            self.player.record_index_1_based as u8,
        )
        .render_ascii_map()
    }

    pub fn begin_starmap_dump(&mut self) {
        self.starmap_dump_lines = self
            .starmap_dump_text()
            .lines()
            .map(|line| line.to_string())
            .collect();
        self.starmap_dump_offset = 0;
        self.starmap_dump_active = true;
        self.starmap_capture_complete = false;
    }

    pub fn advance_starmap_page(&mut self) {
        const PAGE_LINES: usize = 16;
        if !self.starmap_dump_active {
            return;
        }
        let next_offset = self.starmap_dump_offset.saturating_add(PAGE_LINES);
        if next_offset >= self.starmap_dump_lines.len() {
            self.starmap_dump_active = false;
            self.starmap_capture_complete = true;
        } else {
            self.starmap_dump_offset = next_offset;
        }
    }

    pub fn append_planet_info_char(&mut self, ch: char) {
        if self.planet_info_input.len() < 16 {
            self.planet_info_input.push(ch);
            self.planet_info_error = None;
        }
    }

    pub fn backspace_planet_info_input(&mut self) {
        self.planet_info_input.pop();
        self.planet_info_error = None;
    }

    pub fn submit_planet_info_prompt(&mut self) {
        let Some(coords) = resolve_default_coords_input(
            &self.planet_info_input,
            self.default_planet_prompt_coords(),
        ) else {
            self.planet_info_error = Some("Enter coordinates like 5,2".to_string());
            return;
        };

        let Some(planet_idx) = self.game_data.planet_record_index_at_coords(coords) else {
            self.planet_info_error =
                Some(format!("No world found at [{},{}]", coords[0], coords[1]));
            return;
        };

        self.planet_info_selected = Some(planet_idx);
        self.planet_info_error = None;
        self.current_screen = ScreenId::PlanetInfoDetail;
    }

    pub fn planet_info_input(&self) -> &str {
        &self.planet_info_input
    }

    pub fn selected_planet_info(&self) -> Option<usize> {
        self.planet_info_selected
    }

    pub fn current_autopilot_flag(&self) -> u8 {
        self.game_data.player.records[self.player.record_index_1_based - 1].autopilot_flag()
    }

    pub fn current_fleet_roe_by_id(&self, fleet_id: u16) -> Option<u8> {
        self.game_data
            .fleets
            .records
            .iter()
            .find(|fleet| {
                fleet.owner_empire_raw() as usize == self.player.record_index_1_based
                    && fleet.local_slot_word_raw() == fleet_id
            })
            .map(|fleet| fleet.rules_of_engagement())
    }

    pub fn selected_fleet_roe_id(&self) -> Option<u16> {
        let rows = self.fleet_rows();
        rows.get(self.fleet_roe_cursor).map(|row| row.fleet_number)
    }

    pub fn selected_fleet_eta_id(&self) -> Option<u16> {
        let rows = self.fleet_rows();
        rows.get(self.fleet_eta_cursor).map(|row| row.fleet_number)
    }

    pub fn current_relation_to(&self, empire_id: u8) -> Option<ec_data::DiplomaticRelation> {
        self.game_data
            .stored_diplomatic_relation(self.player.record_index_1_based as u8, empire_id)
    }

    pub fn enemies_scroll_offset(&self) -> usize {
        self.enemies_scroll_offset
    }

    fn sorted_planet_rows(&self, sort: PlanetListSort) -> Vec<ec_data::EmpirePlanetEconomyRow> {
        let mut rows = self
            .game_data
            .empire_planet_economy_rows(self.player.record_index_1_based);
        rows.sort_by(|left, right| match sort {
            PlanetListSort::CurrentProduction => right
                .present_production
                .cmp(&left.present_production)
                .then_with(|| left.coords.cmp(&right.coords)),
            PlanetListSort::Location => left.coords.cmp(&right.coords),
            PlanetListSort::PotentialProduction => right
                .potential_production
                .cmp(&left.potential_production)
                .then_with(|| left.coords.cmp(&right.coords)),
        });
        rows
    }

    fn fleet_rows(&self) -> Vec<FleetRow> {
        let mut rows = self
            .game_data
            .fleets
            .records
            .iter()
            .enumerate()
            .filter(|(_, fleet)| {
                fleet.owner_empire_raw() as usize == self.player.record_index_1_based
            })
            .map(|(idx, fleet)| FleetRow {
                fleet_record_index_1_based: idx + 1,
                fleet_number: fleet.local_slot_word_raw(),
                coords: fleet.current_location_coords_raw(),
                target_coords: fleet.standing_order_target_coords_raw(),
                current_speed: fleet.current_speed(),
                max_speed: fleet.max_speed(),
                eta_label: fleet_eta_label(&self.game_data, idx),
                rules_of_engagement: fleet.rules_of_engagement(),
                order_label: fleet.standing_order_summary(),
                composition_label: fleet.ship_composition_summary(),
            })
            .collect::<Vec<_>>();
        rows.sort_by_key(|row| row.fleet_number);
        rows
    }

    fn planet_database_rows(&self) -> Vec<PlanetDatabaseRow> {
        let mut rows = build_player_starmap_projection(
            &self.game_data,
            &self.database,
            self.player.record_index_1_based as u8,
        )
        .worlds
        .into_iter()
        .map(|world| {
            let intel_snapshot = self
                .planet_intel_snapshots
                .get(&world.planet_record_index_1_based);
            let intel_label = planet_database_intel_label(intel_snapshot, &world);
            let owner_label = world
                .known_owner_empire_name
                .as_deref()
                .filter(|name| !name.is_empty())
                .map(str::to_string)
                .or_else(|| {
                    world
                        .known_owner_empire_id
                        .map(|empire_id| format!("Empire {:02}", empire_id))
                })
                .unwrap_or_else(|| "?".to_string());
            PlanetDatabaseRow {
                planet_record_index_1_based: world.planet_record_index_1_based,
                coords: world.coords,
                name_label: world.known_name.unwrap_or_else(|| "?".to_string()),
                owner_label,
                potential_label: world
                    .known_potential_production
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "?".to_string()),
                armies_label: world
                    .known_armies
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "?".to_string()),
                batteries_label: world
                    .known_ground_batteries
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "?".to_string()),
                last_intel_year_label: intel_snapshot
                    .and_then(|snapshot| snapshot.last_intel_year)
                    .map(|year| year.to_string())
                    .unwrap_or_else(|| "?".to_string()),
                intel_label,
            }
        })
        .collect::<Vec<_>>();
        rows.sort_by_key(|row| row.coords);
        rows
    }

    fn build_planet_rows(&self) -> Vec<ec_data::EmpirePlanetEconomyRow> {
        self.sorted_planet_rows(PlanetListSort::CurrentProduction)
    }

    fn commission_planet_rows(&self) -> Vec<ec_data::EmpirePlanetEconomyRow> {
        self.build_planet_rows()
            .into_iter()
            .filter(|row| {
                self.game_data
                    .planets
                    .records
                    .get(row.planet_record_index_1_based - 1)
                    .map(|record| (0..10).any(|slot| record.stardock_kind_raw(slot) != 0))
                    .unwrap_or(false)
            })
            .collect()
    }

    fn planet_transport_planet_rows(
        &self,
        mode: PlanetTransportMode,
    ) -> Vec<PlanetTransportPlanetRow> {
        self.build_planet_rows()
            .into_iter()
            .filter_map(|row| {
                if mode == PlanetTransportMode::Load && row.armies == 0 {
                    return None;
                }
                let fleets = self.planet_transport_eligible_fleet_rows_for_planet(mode, &row);
                if fleets.is_empty() {
                    return None;
                }
                Some(PlanetTransportPlanetRow {
                    planet_record_index_1_based: row.planet_record_index_1_based,
                    planet_name: row.planet_name,
                    coords: row.coords,
                    planet_armies: row.armies,
                    transport_capacity: fleets.iter().map(|fleet| fleet.available_qty).sum(),
                })
            })
            .collect()
    }

    fn current_planet_transport_planet_row(
        &self,
        mode: PlanetTransportMode,
    ) -> Result<PlanetTransportPlanetRow, Box<dyn std::error::Error>> {
        if matches!(self.current_screen, ScreenId::PlanetTransportFleetSelect(_)) {
            let selected_record = self
                .planet_transport_selected_planet_record
                .ok_or_else(|| "current transport planet missing".to_string())?;
            let base_row = self
                .build_planet_rows()
                .into_iter()
                .find(|row| row.planet_record_index_1_based == selected_record)
                .ok_or_else(|| "current transport planet missing".to_string())?;
            let transport_capacity = self
                .planet_transport_fleet_rows_for_planet(mode, &base_row)
                .iter()
                .map(|fleet| fleet.available_qty)
                .sum();
            return Ok(PlanetTransportPlanetRow {
                planet_record_index_1_based: base_row.planet_record_index_1_based,
                planet_name: base_row.planet_name,
                coords: base_row.coords,
                planet_armies: base_row.armies,
                transport_capacity,
            });
        }

        self.planet_transport_planet_rows(mode)
            .get(self.planet_transport_planet_cursor)
            .cloned()
            .ok_or_else(|| "current transport planet missing".into())
    }

    fn current_planet_transport_fleet_rows(
        &self,
        mode: PlanetTransportMode,
    ) -> Result<Vec<PlanetTransportFleetRow>, Box<dyn std::error::Error>> {
        let planet = self.current_planet_transport_planet_row(mode)?;
        let base_row = self
            .build_planet_rows()
            .into_iter()
            .find(|row| row.planet_record_index_1_based == planet.planet_record_index_1_based)
            .ok_or("transport planet row missing")?;
        Ok(self.planet_transport_fleet_rows_for_planet(mode, &base_row))
    }

    fn current_planet_transport_fleet_row(
        &self,
        mode: PlanetTransportMode,
    ) -> Result<PlanetTransportFleetRow, Box<dyn std::error::Error>> {
        self.current_planet_transport_fleet_rows(mode)?
            .get(self.planet_transport_fleet_cursor)
            .cloned()
            .ok_or_else(|| "current transport fleet missing".into())
    }

    fn planet_transport_fleet_rows_for_planet(
        &self,
        mode: PlanetTransportMode,
        row: &ec_data::EmpirePlanetEconomyRow,
    ) -> Vec<PlanetTransportFleetRow> {
        self.game_data
            .fleets
            .records
            .iter()
            .enumerate()
            .filter(|(_, fleet)| {
                fleet.owner_empire_raw() as usize == self.player.record_index_1_based
                    && fleet.current_location_coords_raw() == row.coords
                    && fleet.troop_transport_count() > 0
            })
            .map(|(idx, fleet)| {
                let available_qty = match mode {
                    PlanetTransportMode::Load => fleet
                        .troop_transport_count()
                        .saturating_sub(fleet.army_count()),
                    PlanetTransportMode::Unload => fleet
                        .army_count()
                        .min(u16::from(u8::MAX.saturating_sub(row.armies))),
                };
                PlanetTransportFleetRow {
                    fleet_record_index_1_based: idx + 1,
                    fleet_number: fleet.local_slot_word_raw(),
                    troop_transports: fleet.troop_transport_count(),
                    loaded_armies: fleet.army_count(),
                    available_qty,
                }
            })
            .collect()
    }

    fn planet_transport_eligible_fleet_rows_for_planet(
        &self,
        mode: PlanetTransportMode,
        row: &ec_data::EmpirePlanetEconomyRow,
    ) -> Vec<PlanetTransportFleetRow> {
        self.planet_transport_fleet_rows_for_planet(mode, row)
            .into_iter()
            .filter(|fleet| fleet.available_qty > 0)
            .collect()
    }

    fn current_commission_planet_row(
        &self,
    ) -> Result<ec_data::EmpirePlanetEconomyRow, Box<dyn std::error::Error>> {
        self.commission_planet_rows()
            .get(self.planet_commission_index)
            .cloned()
            .ok_or_else(|| "current commission planet missing".into())
    }

    fn current_planet_commission_view(
        &self,
    ) -> Result<PlanetCommissionView, Box<dyn std::error::Error>> {
        let row = match self.current_commission_planet_row() {
            Ok(row) => row,
            Err(_) => {
                return Ok(PlanetCommissionView {
                    planet_name: "No commissionable planets".to_string(),
                    coords: self.default_planet_prompt_coords(),
                    rows: vec![],
                });
            }
        };
        Ok(PlanetCommissionView {
            planet_name: row.planet_name,
            coords: row.coords,
            rows: self.current_planet_commission_rows(),
        })
    }

    fn current_planet_commission_rows(&self) -> Vec<PlanetCommissionRow> {
        let Ok(row) = self.current_commission_planet_row() else {
            return vec![];
        };
        let Some(record) = self
            .game_data
            .planets
            .records
            .get(row.planet_record_index_1_based - 1)
        else {
            return vec![];
        };
        (0..10)
            .filter_map(|slot| {
                let kind_raw = record.stardock_kind_raw(slot);
                let qty = u32::from(record.stardock_count_raw(slot));
                if kind_raw == 0 || qty == 0 {
                    return None;
                }
                let kind = ProductionItemKind::from_raw(kind_raw);
                let unit_label = build_unit_spec_by_kind(kind)
                    .map(|spec| spec.label.to_string())
                    .unwrap_or_else(|| format!("Unknown (kind {})", kind_raw));
                Some(PlanetCommissionRow {
                    slot_0_based: slot,
                    unit_label,
                    qty,
                })
            })
            .collect()
    }

    fn build_change_rows(&self) -> Vec<PlanetBuildChangeRow> {
        self.build_planet_rows()
            .into_iter()
            .map(|row| {
                let available_points = u32::from(row.build_capacity)
                    .min(row.stored_production_points.min(u32::from(u16::MAX)));
                let committed_points = self
                    .current_build_committed_points(row.planet_record_index_1_based)
                    .unwrap_or(0);
                PlanetBuildChangeRow {
                    planet_name: row.planet_name,
                    coords: row.coords,
                    present_production: row.present_production,
                    potential_production: row.potential_production,
                    available_points,
                    committed_points,
                }
            })
            .collect()
    }

    fn current_build_planet_row(
        &self,
    ) -> Result<ec_data::EmpirePlanetEconomyRow, Box<dyn std::error::Error>> {
        self.build_planet_rows()
            .get(self.planet_build_index)
            .cloned()
            .ok_or_else(|| "current build planet missing".into())
    }

    fn current_planet_build_orders(&self) -> Vec<PlanetBuildOrder> {
        let Ok(row) = self.current_build_planet_row() else {
            return vec![];
        };
        let Some(record) = self
            .game_data
            .planets
            .records
            .get(row.planet_record_index_1_based - 1)
        else {
            return vec![];
        };
        (0..10)
            .filter_map(|slot| {
                let points = record.build_count_raw(slot);
                let kind_raw = record.build_kind_raw(slot);
                if points == 0 || kind_raw == 0 {
                    None
                } else {
                    Some(PlanetBuildOrder {
                        kind: ProductionItemKind::from_raw(kind_raw),
                        points_remaining: points,
                    })
                }
            })
            .collect()
    }

    fn current_planet_build_view(&self) -> Result<PlanetBuildMenuView, Box<dyn std::error::Error>> {
        let row = match self.current_build_planet_row() {
            Ok(row) => row,
            Err(_) => {
                return Ok(PlanetBuildMenuView {
                    row: ec_data::EmpirePlanetEconomyRow {
                        planet_record_index_1_based: 0,
                        coords: self.default_planet_prompt_coords(),
                        planet_name: "No owned planets".to_string(),
                        present_production: 0,
                        potential_production: 0,
                        stored_production_points: 0,
                        yearly_tax_revenue: 0,
                        yearly_growth_delta: 0,
                        build_capacity: 0,
                        has_friendly_starbase: false,
                        armies: 0,
                        ground_batteries: 0,
                        is_homeworld_seed: false,
                    },
                    committed_points: 0,
                    available_points: 0,
                    points_left: 0,
                    queue_used: 0,
                    queue_capacity: 10,
                    stardock_used: 0,
                    stardock_capacity: 10,
                });
            }
        };
        let committed_points =
            self.current_build_committed_points(row.planet_record_index_1_based)?;
        let available_points = u32::from(row.build_capacity)
            .min(row.stored_production_points.min(u32::from(u16::MAX)));
        let points_left = available_points.saturating_sub(committed_points);
        let record = self
            .game_data
            .planets
            .records
            .get(row.planet_record_index_1_based - 1)
            .ok_or("planet record missing")?;
        let queue_capacity: usize = 10;
        let queue_used = (0..queue_capacity)
            .filter(|&s| record.build_count_raw(s) != 0 || record.build_kind_raw(s) != 0)
            .count();
        let stardock_capacity: usize = 10;
        let stardock_open_now = self
            .game_data
            .planet_open_stardock_slots_now(row.planet_record_index_1_based)?;
        let stardock_used = stardock_capacity.saturating_sub(stardock_open_now);
        Ok(PlanetBuildMenuView {
            row,
            committed_points,
            available_points,
            points_left,
            queue_used,
            queue_capacity,
            stardock_used,
            stardock_capacity,
        })
    }

    fn current_build_committed_points(
        &self,
        planet_record_index_1_based: usize,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        let record = self
            .game_data
            .planets
            .records
            .get(planet_record_index_1_based - 1)
            .ok_or("planet record missing")?;
        Ok((0..10)
            .map(|slot| u32::from(record.build_count_raw(slot)))
            .sum::<u32>())
    }

    fn current_planet_build_max_quantity(&self) -> Result<u32, Box<dyn std::error::Error>> {
        let kind = self
            .planet_build_selected_kind
            .ok_or("planet build kind missing")?;
        self.current_planet_build_max_quantity_for(kind)
    }

    fn current_planet_build_max_quantity_for(
        &self,
        kind: ProductionItemKind,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        let view = self.current_planet_build_view()?;
        let unit = build_unit_spec_by_kind(kind).ok_or("unit spec missing")?;
        let mut max_qty = max_quantity(view.points_left, unit.cost);
        match kind {
            ProductionItemKind::Army => {
                let free = self
                    .game_data
                    .planet_free_army_capacity(view.row.planet_record_index_1_based)?;
                max_qty = max_qty.min(u32::from(free));
            }
            ProductionItemKind::GroundBattery => {
                let free = self
                    .game_data
                    .planet_free_ground_battery_capacity(view.row.planet_record_index_1_based)?;
                max_qty = max_qty.min(u32::from(free));
            }
            _ => {}
        }
        Ok(max_qty)
    }

    fn planet_build_unavailable_message(
        &self,
        kind: ProductionItemKind,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let view = self.current_planet_build_view()?;
        if view.points_left == 0 {
            return Ok("No points are available to spend.".to_string());
        }
        Ok(match kind {
            ProductionItemKind::Army => "Planet already has the maximum 255 armies.",
            ProductionItemKind::GroundBattery => {
                "Planet already has the maximum 255 ground batteries."
            }
            _ => "No points are available to spend.",
        }
        .to_string())
    }

    fn planet_build_list_rows(&self) -> Vec<PlanetBuildListRow> {
        let Ok(row) = self.current_build_planet_row() else {
            return vec![];
        };
        let Some(record) = self
            .game_data
            .planets
            .records
            .get(row.planet_record_index_1_based - 1)
        else {
            return vec![];
        };
        let mut queue_qty_by_kind: BTreeMap<u8, u32> = BTreeMap::new();
        let mut stardock_qty_by_kind: BTreeMap<u8, u32> = BTreeMap::new();

        for slot in 0..10 {
            let points = u32::from(record.build_count_raw(slot));
            let kind_raw = record.build_kind_raw(slot);
            if points == 0 || kind_raw == 0 {
                continue;
            }
            let kind = ProductionItemKind::from_raw(kind_raw);
            let cost = u32::from(build_unit_spec_by_kind(kind).map(|u| u.cost).unwrap_or(1));
            let qty = if cost > 0 { points / cost } else { 0 };
            *queue_qty_by_kind.entry(kind_raw).or_default() += qty.max(1);
        }

        for slot in 0..10 {
            let qty = u32::from(record.stardock_count_raw(slot));
            let kind_raw = record.stardock_kind_raw(slot);
            if qty == 0 || kind_raw == 0 {
                continue;
            }
            *stardock_qty_by_kind.entry(kind_raw).or_default() += qty;
        }

        let mut ordered_kind_raws = vec![1, 2, 3, 4, 5, 6, 9, 8, 7];
        for kind_raw in queue_qty_by_kind.keys().chain(stardock_qty_by_kind.keys()) {
            if !ordered_kind_raws.contains(kind_raw) {
                ordered_kind_raws.push(*kind_raw);
            }
        }

        ordered_kind_raws
            .into_iter()
            .filter_map(|kind_raw| {
                let queue_qty = queue_qty_by_kind.get(&kind_raw).copied().unwrap_or(0);
                let stardock_qty = stardock_qty_by_kind.get(&kind_raw).copied().unwrap_or(0);
                if queue_qty == 0 && stardock_qty == 0 {
                    return None;
                }
                let kind = ProductionItemKind::from_raw(kind_raw);
                let (unit_label, cost) = build_unit_spec_by_kind(kind)
                    .map(|u| (u.label.to_string(), u.cost))
                    .unwrap_or_else(|| (format!("Unknown (kind {})", kind_raw), 0));
                Some(PlanetBuildListRow {
                    kind,
                    unit_label,
                    points: u32::from(cost),
                    queue_qty,
                    stardock_qty: if kind.requires_stardock() {
                        Some(stardock_qty)
                    } else {
                        None
                    },
                })
            })
            .collect()
    }

    fn compose_outbox_queue(&self) -> Result<Vec<QueuedPlayerMail>, Box<dyn std::error::Error>> {
        let sender_empire_id = self.player.record_index_1_based as u8;
        Ok(self
            .queued_mail
            .clone()
            .into_iter()
            .filter(|mail| mail.sender_empire_id == sender_empire_id)
            .collect())
    }

    fn compose_outbox_queue_len(&self) -> usize {
        self.compose_outbox_queue()
            .map(|queue| queue.len())
            .unwrap_or(0)
    }

    fn handle_planet_info_prompt_key(&self, key: crossterm::event::KeyEvent) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                crate::app::Action::ReturnToCommandMenu
            }
            KeyCode::Enter => crate::app::Action::SubmitPlanetInfoPrompt,
            KeyCode::Backspace => crate::app::Action::BackspacePlanetInfoInput,
            KeyCode::Char(ch)
                if ch.is_ascii_digit()
                    || matches!(ch, ',' | ' ' | ':' | '/' | '-' | '(' | ')' | '[' | ']') =>
            {
                crate::app::Action::AppendPlanetInfoChar(ch)
            }
            _ => crate::app::Action::Noop,
        }
    }

    fn handle_fleet_roe_key(&self, key: crossterm::event::KeyEvent) -> crate::app::Action {
        use crossterm::event::KeyCode;

        if self.fleet_roe_editing {
            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::OpenFleetRoeSelect
                }
                KeyCode::Enter => crate::app::Action::SubmitFleetRoe,
                KeyCode::Backspace => crate::app::Action::BackspaceFleetRoeInput,
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    crate::app::Action::AppendFleetRoeChar(ch)
                }
                _ => crate::app::Action::Noop,
            }
        } else {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    crate::app::Action::MoveFleetRoeSelect(-1)
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    crate::app::Action::MoveFleetRoeSelect(1)
                }
                KeyCode::PageUp => crate::app::Action::MoveFleetRoeSelect(-8),
                KeyCode::PageDown => crate::app::Action::MoveFleetRoeSelect(8),
                KeyCode::Enter => crate::app::Action::SubmitFleetRoe,
                KeyCode::Backspace => crate::app::Action::BackspaceFleetRoeInput,
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    crate::app::Action::AppendFleetRoeChar(ch)
                }
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::OpenFleetMenu
                }
                _ => crate::app::Action::Noop,
            }
        }
    }

    fn handle_fleet_detach_key(&self, key: crossterm::event::KeyEvent) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match self.fleet_detach_mode {
            FleetDetachMode::SelectingFleet => match key.code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    crate::app::Action::MoveFleetDetachSelect(-1)
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    crate::app::Action::MoveFleetDetachSelect(1)
                }
                KeyCode::PageUp => crate::app::Action::MoveFleetDetachSelect(-8),
                KeyCode::PageDown => crate::app::Action::MoveFleetDetachSelect(8),
                KeyCode::Enter => crate::app::Action::SubmitFleetDetach,
                KeyCode::Backspace => crate::app::Action::BackspaceFleetDetachInput,
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    crate::app::Action::AppendFleetDetachChar(ch)
                }
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::OpenFleetMenu
                }
                _ => crate::app::Action::Noop,
            },
            _ => match key.code {
                KeyCode::Enter => crate::app::Action::SubmitFleetDetach,
                KeyCode::Backspace => crate::app::Action::BackspaceFleetDetachInput,
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::OpenFleetDetach
                }
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    crate::app::Action::AppendFleetDetachChar(ch)
                }
                _ => crate::app::Action::Noop,
            },
        }
    }

    fn sync_fleet_roe_cursor_to_input(&mut self) {
        if self.current_screen != ScreenId::FleetRoeSelect || self.fleet_roe_editing {
            return;
        }
        let Ok(target_fleet_id) = self.fleet_roe_select_input.trim().parse::<u16>() else {
            return;
        };
        let rows = self.fleet_rows();
        let Some(index) = rows
            .iter()
            .position(|row| row.fleet_number == target_fleet_id)
        else {
            return;
        };
        self.fleet_roe_cursor = index;
        sync_scroll_to_cursor(
            &mut self.fleet_roe_scroll_offset,
            self.fleet_roe_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
    }

    fn sync_fleet_detach_cursor_to_input(&mut self) {
        if self.current_screen != ScreenId::FleetDetach
            || self.fleet_detach_mode != FleetDetachMode::SelectingFleet
        {
            return;
        }
        let Ok(target_fleet_id) = self.fleet_detach_select_input.trim().parse::<u16>() else {
            return;
        };
        let rows = self.fleet_rows();
        let Some(index) = rows
            .iter()
            .position(|row| row.fleet_number == target_fleet_id)
        else {
            return;
        };
        self.fleet_detach_cursor = index;
        sync_scroll_to_cursor(
            &mut self.fleet_detach_scroll_offset,
            self.fleet_detach_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
    }

    fn fleet_detach_prompt_and_default(&self, rows: &[FleetRow]) -> (String, String) {
        let fleet_number = rows
            .get(self.fleet_detach_cursor)
            .map(|row| row.fleet_number)
            .unwrap_or(1);
        match self.fleet_detach_mode {
            FleetDetachMode::SelectingFleet => (
                "Detach ships from fleet # ".to_string(),
                format_fleet_number_for_rows(fleet_number, rows),
            ),
            FleetDetachMode::EnteringBattleships => {
                ("Battleships to detach ".to_string(), "0".to_string())
            }
            FleetDetachMode::EnteringCruisers => {
                ("Cruisers to detach ".to_string(), "0".to_string())
            }
            FleetDetachMode::EnteringDestroyers => {
                ("Destroyers to detach ".to_string(), "0".to_string())
            }
            FleetDetachMode::EnteringFullTransports => {
                ("FULL transports to detach ".to_string(), "0".to_string())
            }
            FleetDetachMode::EnteringEmptyTransports => {
                ("EMPTY transports to detach ".to_string(), "0".to_string())
            }
            FleetDetachMode::EnteringScouts => {
                ("Scout ships to detach ".to_string(), "0".to_string())
            }
            FleetDetachMode::EnteringEtacs => {
                ("ETAC ships to detach ".to_string(), "0".to_string())
            }
            FleetDetachMode::AdjustingDonorSpeed => (
                format!(
                    "Fleet #{} new speed ",
                    format_fleet_number_for_rows(fleet_number, rows)
                ),
                self.fleet_detach_donor_default_speed().to_string(),
            ),
            FleetDetachMode::SettingNewFleetRoe => ("New fleet ROE ".to_string(), "6".to_string()),
        }
    }

    fn fleet_detach_current_input(&self) -> &str {
        if self.fleet_detach_mode == FleetDetachMode::SelectingFleet {
            &self.fleet_detach_select_input
        } else {
            &self.fleet_detach_input
        }
    }

    fn current_fleet_detach_ship_total(&self) -> u32 {
        let rows = self.fleet_rows();
        let Some(selected_row) = rows.get(self.fleet_detach_cursor) else {
            return 0;
        };
        self.game_data
            .fleets
            .records
            .get(selected_row.fleet_record_index_1_based - 1)
            .map(|fleet| {
                u32::from(fleet.battleship_count())
                    + u32::from(fleet.cruiser_count())
                    + u32::from(fleet.destroyer_count())
                    + u32::from(fleet.troop_transport_count())
                    + u32::from(fleet.scout_count())
                    + u32::from(fleet.etac_count())
            })
            .unwrap_or(0)
    }

    fn next_fleet_detach_prompt_mode(&self, current: FleetDetachMode) -> Option<FleetDetachMode> {
        let rows = self.fleet_rows();
        let selected_row = rows.get(self.fleet_detach_cursor)?;
        let fleet = self
            .game_data
            .fleets
            .records
            .get(selected_row.fleet_record_index_1_based - 1)?;
        let modes = [
            (
                FleetDetachMode::EnteringBattleships,
                fleet.battleship_count() > 0,
            ),
            (FleetDetachMode::EnteringCruisers, fleet.cruiser_count() > 0),
            (
                FleetDetachMode::EnteringDestroyers,
                fleet.destroyer_count() > 0,
            ),
            (
                FleetDetachMode::EnteringFullTransports,
                fleet.army_count() > 0,
            ),
            (
                FleetDetachMode::EnteringEmptyTransports,
                fleet.troop_transport_count() > fleet.army_count(),
            ),
            (FleetDetachMode::EnteringScouts, fleet.scout_count() > 0),
            (FleetDetachMode::EnteringEtacs, fleet.etac_count() > 0),
        ];
        let start_idx = match current {
            FleetDetachMode::SelectingFleet => 0,
            FleetDetachMode::EnteringBattleships => 1,
            FleetDetachMode::EnteringCruisers => 2,
            FleetDetachMode::EnteringDestroyers => 3,
            FleetDetachMode::EnteringFullTransports => 4,
            FleetDetachMode::EnteringEmptyTransports => 5,
            FleetDetachMode::EnteringScouts => 6,
            FleetDetachMode::EnteringEtacs
            | FleetDetachMode::AdjustingDonorSpeed
            | FleetDetachMode::SettingNewFleetRoe => modes.len(),
        };
        modes
            .iter()
            .skip(start_idx)
            .find_map(|(mode, include)| (*include).then_some(*mode))
    }

    fn fleet_detach_requires_speed_prompt(&self) -> bool {
        let rows = self.fleet_rows();
        let Some(selected_row) = rows.get(self.fleet_detach_cursor) else {
            return false;
        };
        let Some(fleet) = self
            .game_data
            .fleets
            .records
            .get(selected_row.fleet_record_index_1_based - 1)
        else {
            return false;
        };
        let mut donor_after = fleet.clone();
        donor_after.set_battleship_count(
            donor_after
                .battleship_count()
                .saturating_sub(self.fleet_detach_selection.battleships),
        );
        donor_after.set_cruiser_count(
            donor_after
                .cruiser_count()
                .saturating_sub(self.fleet_detach_selection.cruisers),
        );
        donor_after.set_destroyer_count(
            donor_after
                .destroyer_count()
                .saturating_sub(self.fleet_detach_selection.destroyers),
        );
        donor_after.set_troop_transport_count(donor_after.troop_transport_count().saturating_sub(
            self.fleet_detach_selection.full_transports
                + self.fleet_detach_selection.empty_transports,
        ));
        donor_after.set_army_count(
            donor_after
                .army_count()
                .saturating_sub(self.fleet_detach_selection.full_transports),
        );
        donor_after.set_scout_count(
            donor_after
                .scout_count()
                .saturating_sub(self.fleet_detach_selection.scouts),
        );
        donor_after.set_etac_count(
            donor_after
                .etac_count()
                .saturating_sub(self.fleet_detach_selection.etacs),
        );
        donor_after.recompute_max_speed_from_composition();
        donor_after.max_speed() > 0 && fleet.current_speed() > donor_after.max_speed()
    }

    fn fleet_detach_donor_default_speed(&self) -> u8 {
        let rows = self.fleet_rows();
        let Some(selected_row) = rows.get(self.fleet_detach_cursor) else {
            return 1;
        };
        let Some(fleet) = self
            .game_data
            .fleets
            .records
            .get(selected_row.fleet_record_index_1_based - 1)
        else {
            return 1;
        };
        let mut donor_after = fleet.clone();
        donor_after.set_battleship_count(
            donor_after
                .battleship_count()
                .saturating_sub(self.fleet_detach_selection.battleships),
        );
        donor_after.set_cruiser_count(
            donor_after
                .cruiser_count()
                .saturating_sub(self.fleet_detach_selection.cruisers),
        );
        donor_after.set_destroyer_count(
            donor_after
                .destroyer_count()
                .saturating_sub(self.fleet_detach_selection.destroyers),
        );
        donor_after.set_troop_transport_count(donor_after.troop_transport_count().saturating_sub(
            self.fleet_detach_selection.full_transports
                + self.fleet_detach_selection.empty_transports,
        ));
        donor_after.set_army_count(
            donor_after
                .army_count()
                .saturating_sub(self.fleet_detach_selection.full_transports),
        );
        donor_after.set_scout_count(
            donor_after
                .scout_count()
                .saturating_sub(self.fleet_detach_selection.scouts),
        );
        donor_after.set_etac_count(
            donor_after
                .etac_count()
                .saturating_sub(self.fleet_detach_selection.etacs),
        );
        donor_after.recompute_max_speed_from_composition();
        donor_after.max_speed().max(1)
    }

    fn resolve_fleet_detach_numeric_input(
        &mut self,
        default: u16,
    ) -> Result<u16, Box<dyn std::error::Error>> {
        let raw = self.fleet_detach_input.trim();
        if raw.is_empty() {
            return Ok(default);
        }
        match raw.parse::<u16>() {
            Ok(value) => Ok(value),
            Err(_) => {
                self.fleet_detach_status = Some("Enter an integer value.".to_string());
                Err("invalid detach numeric input".into())
            }
        }
    }

    fn handle_fleet_eta_key(&self, key: crossterm::event::KeyEvent) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match self.fleet_eta_mode {
            FleetEtaMode::SelectingFleet => match key.code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    crate::app::Action::MoveFleetEtaSelect(-1)
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    crate::app::Action::MoveFleetEtaSelect(1)
                }
                KeyCode::PageUp => crate::app::Action::MoveFleetEtaSelect(-8),
                KeyCode::PageDown => crate::app::Action::MoveFleetEtaSelect(8),
                KeyCode::Enter => crate::app::Action::SubmitFleetEta,
                KeyCode::Backspace => crate::app::Action::BackspaceFleetEtaInput,
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    crate::app::Action::AppendFleetEtaChar(ch)
                }
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::OpenFleetMenu
                }
                _ => crate::app::Action::Noop,
            },
            FleetEtaMode::EnteringDestination => match key.code {
                KeyCode::Enter => crate::app::Action::SubmitFleetEta,
                KeyCode::Backspace => crate::app::Action::BackspaceFleetEtaInput,
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::OpenFleetEta
                }
                KeyCode::Char(ch)
                    if ch.is_ascii_digit() || matches!(ch, ',' | ' ' | '(' | ')' | '[' | ']') =>
                {
                    crate::app::Action::AppendFleetEtaChar(ch)
                }
                _ => crate::app::Action::Noop,
            },
            FleetEtaMode::ConfirmingSystemEntry => match key.code {
                KeyCode::Enter => crate::app::Action::SubmitFleetEta,
                KeyCode::Backspace => crate::app::Action::BackspaceFleetEtaInput,
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::OpenFleetEta
                }
                KeyCode::Char(ch) if matches!(ch, 'y' | 'Y' | 'n' | 'N') => {
                    crate::app::Action::AppendFleetEtaChar(ch)
                }
                _ => crate::app::Action::Noop,
            },
            FleetEtaMode::ShowingResult => match key.code {
                KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::SubmitFleetEta
                }
                _ => crate::app::Action::Noop,
            },
        }
    }

    fn sync_fleet_eta_cursor_to_input(&mut self) {
        if self.current_screen != ScreenId::FleetEta
            || self.fleet_eta_mode != FleetEtaMode::SelectingFleet
        {
            return;
        }
        let Ok(target_fleet_id) = self.fleet_eta_select_input.trim().parse::<u16>() else {
            return;
        };
        let rows = self.fleet_rows();
        let Some(index) = rows
            .iter()
            .position(|row| row.fleet_number == target_fleet_id)
        else {
            return;
        };
        self.fleet_eta_cursor = index;
        sync_scroll_to_cursor(
            &mut self.fleet_eta_scroll_offset,
            self.fleet_eta_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
    }

    fn default_planet_prompt_coords(&self) -> [u8; 2] {
        let homeworld_index = self
            .game_data
            .player
            .records
            .get(self.player.record_index_1_based - 1)
            .map(|player| player.homeworld_planet_index_1_based_raw() as usize)
            .unwrap_or(0);
        if homeworld_index != 0 {
            if let Some(planet) = self.game_data.planets.records.get(homeworld_index - 1) {
                return planet.coords_raw();
            }
        }
        self.game_data
            .planets
            .records
            .iter()
            .find(|planet| {
                planet.owner_empire_slot_raw() as usize == self.player.record_index_1_based
                    && planet.is_homeworld_seed_ignoring_name()
            })
            .map(|planet| planet.coords_raw())
            .unwrap_or([8, 2])
    }

    fn save_game_data(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.campaign_store.save_runtime_state(
            &self.game_data,
            &self.database,
            &self.results_bytes,
            &self.messages_bytes,
            &self.queued_mail,
        )?;
        self.planet_intel_snapshots = self
            .campaign_store
            .latest_planet_intel_for_viewer(self.player.record_index_1_based as u8)?
            .into_iter()
            .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
            .collect::<BTreeMap<_, _>>();
        Ok(())
    }

    fn fleet_eta_default_destination(&self) -> [u8; 2] {
        let rows = self.fleet_rows();
        let Some(row) = rows.get(self.fleet_eta_cursor) else {
            return [8, 2];
        };
        if row.target_coords[0] > 0 && row.target_coords[1] > 0 {
            row.target_coords
        } else {
            row.coords
        }
    }

    fn calculate_fleet_eta_message(
        &self,
        row: &FleetRow,
        destination: [u8; 2],
        include_system: bool,
    ) -> String {
        if row.current_speed == 0 {
            return format!(
                "Fleet {} is stopped at [{},{}].",
                row.fleet_number, row.coords[0], row.coords[1]
            );
        }
        let mut game_data = self.game_data.clone();
        let fleet_index = row.fleet_record_index_1_based - 1;
        let fleet = &mut game_data.fleets.records[fleet_index];
        fleet.set_standing_order_target_coords_raw(destination);
        let Some(route) = plan_route(&game_data, fleet_index) else {
            return format!("No route found to [{},{}].", destination[0], destination[1]);
        };
        let mut steps = route.steps.len().saturating_sub(1);
        if include_system && destination != row.coords {
            steps += 1;
        }
        let years = steps.div_ceil(row.current_speed as usize);
        let arrival_year = self.game_data.conquest.game_year() + years as u16;
        format!(
            "Fleet {} reaches [{},{}] in {} year(s), arriving in {}.",
            row.fleet_number, destination[0], destination[1], years, arrival_year
        )
    }

    fn first_time_empire_rows(&self) -> Vec<String> {
        self.game_data
            .player
            .records
            .iter()
            .enumerate()
            .map(|(idx, player)| {
                let slot = idx + 1;
                if player.occupied_flag() != 0 {
                    let handle = player.assigned_player_handle_summary();
                    let empire = player.controlled_empire_name_summary();
                    format!(
                        "Empire {:>2}: JOINED  {}{}",
                        slot,
                        if empire.is_empty() {
                            "Empire".to_string()
                        } else {
                            empire
                        },
                        if handle.is_empty() {
                            String::new()
                        } else {
                            format!(" [{handle}]")
                        }
                    )
                } else {
                    format!(
                        "Empire {:>2}: OPEN    Available for a new Star Master",
                        slot
                    )
                }
            })
            .collect()
    }

    fn first_time_homeworld_summary(
        &self,
    ) -> Result<([u8; 2], u16, u16), Box<dyn std::error::Error>> {
        let planet_index = self
            .game_data
            .player
            .records
            .get(self.player.record_index_1_based - 1)
            .ok_or("player record missing for homeworld prompt")?
            .homeworld_planet_index_1_based_raw() as usize;
        let planet = self
            .game_data
            .planets
            .records
            .get(planet_index.saturating_sub(1))
            .ok_or("homeworld planet missing for first-time prompt")?;
        Ok((
            planet.coords_raw(),
            planet
                .present_production_points()
                .unwrap_or(planet.potential_production_points()),
            planet.potential_production_points(),
        ))
    }

    fn complete_first_time_join(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.game_data.join_player(
            self.player.record_index_1_based,
            &self.first_time_empire_name,
        )?;
        self.save_game_data()?;
        self.refresh_player_context()?;
        Ok(())
    }

    fn complete_first_time_homeworld_name(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.game_data.rename_player_homeworld(
            self.player.record_index_1_based,
            &self.first_time_homeworld_name,
        )?;
        self.save_game_data()?;
        self.refresh_player_context()?;
        Ok(())
    }

    fn refresh_player_context(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.player =
            PlayerContext::from_game_data(&self.game_data, self.player.record_index_1_based)?;
        let refreshed = ReportsPreview::from_bytes(&self.results_bytes, &self.messages_bytes);
        let summary = MainMenuSummary::from_game_data(
            &self.game_data,
            self.player.record_index_1_based,
            !self.results_bytes.is_empty(),
        );
        self.reports
            .replace(refreshed, ReviewSummary::from_main_menu(&summary));
        Ok(())
    }

    fn startup_target_screen(&self, phase: StartupPhase) -> ScreenId {
        match phase {
            StartupPhase::Complete => {
                if self.player.is_joined {
                    self.pending_naming_screen().unwrap_or(ScreenId::MainMenu)
                } else {
                    ScreenId::FirstTimeMenu
                }
            }
            other => ScreenId::Startup(other),
        }
    }

    fn pending_naming_screen(&self) -> Option<ScreenId> {
        let Some(player) = self
            .game_data
            .player
            .records
            .get(self.player.record_index_1_based - 1)
        else {
            return None;
        };
        let planet_index = player.homeworld_planet_index_1_based_raw() as usize;
        if planet_index == 0 {
            return None;
        }
        self.game_data
            .planets
            .records
            .get(planet_index - 1)
            .filter(|planet| planet.is_named_homeworld_seed())
            .map(|_| ScreenId::FirstTimeHomeworldName)
    }
}

fn planet_database_intel_label(
    snapshot: Option<&PlanetIntelSnapshot>,
    world: &PlayerStarmapWorld,
) -> String {
    if let Some(snapshot) = snapshot {
        return match snapshot.intel_tier {
            ec_data::IntelTier::Owned => "owned".to_string(),
            ec_data::IntelTier::Full => "full".to_string(),
            ec_data::IntelTier::Partial => "partial".to_string(),
            ec_data::IntelTier::Unknown => "unknown".to_string(),
        };
    }
    if world.known_owner_empire_id == Some(0) {
        return "unknown".to_string();
    }
    if world.known_armies.is_some() || world.known_ground_batteries.is_some() {
        "full".to_string()
    } else if world.known_name.is_some()
        || world.known_owner_empire_id.is_some()
        || world.known_owner_empire_name.is_some()
        || world.known_potential_production.is_some()
    {
        "partial".to_string()
    } else {
        "unknown".to_string()
    }
}

/// Keep `scroll_offset` in sync with `cursor` so the highlighted row is always visible.
fn sync_scroll_to_cursor(scroll_offset: &mut usize, cursor: usize, visible: usize) {
    if cursor < *scroll_offset {
        *scroll_offset = cursor;
    } else if cursor >= *scroll_offset + visible {
        *scroll_offset = cursor + 1 - visible;
    }
}

fn compose_recipient_label(game_data: &CoreGameData, empire_id: Option<u8>) -> String {
    let Some(empire_id) = empire_id else {
        return "<unknown>".to_string();
    };
    let Some(player) = game_data
        .player
        .records
        .get(empire_id.saturating_sub(1) as usize)
    else {
        return format!("Empire {empire_id}");
    };
    let name = player.controlled_empire_name_summary();
    let fallback = player.legacy_status_name_summary();
    let display = if !name.is_empty() { name } else { fallback };
    format!("Empire {empire_id} ({display})")
}

fn production_item_kind_raw(kind: ProductionItemKind) -> u8 {
    match kind {
        ProductionItemKind::Destroyer => 1,
        ProductionItemKind::Cruiser => 2,
        ProductionItemKind::Battleship => 3,
        ProductionItemKind::Scout => 4,
        ProductionItemKind::Transport => 5,
        ProductionItemKind::Etac => 6,
        ProductionItemKind::GroundBattery => 7,
        ProductionItemKind::Army => 8,
        ProductionItemKind::Starbase => 9,
        ProductionItemKind::Unknown(raw) => raw,
    }
}


fn char_to_byte_index(body: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }
    body.char_indices()
        .nth(char_index)
        .map(|(idx, _)| idx)
        .unwrap_or(body.len())
}

fn format_auto_commission_status(summary: AutoCommissionSummary) -> String {
    format!(
        "Commissioned {} ships into {} new fleets and {} starbases from {} planets.",
        summary.ships_commissioned,
        summary.fleets_created,
        summary.starbases_commissioned,
        summary.planets_used
    )
}

fn fleet_eta_label(game_data: &CoreGameData, fleet_idx: usize) -> String {
    let Some(fleet) = game_data.fleets.records.get(fleet_idx) else {
        return "?".to_string();
    };

    if fleet.current_location_coords_raw() == fleet.standing_order_target_coords_raw() {
        return "0".to_string();
    }

    let Some(route) = plan_route(game_data, fleet_idx) else {
        return "N/A".to_string();
    };
    let steps_remaining = route.steps.len().saturating_sub(1);
    if steps_remaining == 0 {
        return "0".to_string();
    }

    let speed = usize::from(fleet.current_speed());
    if speed == 0 {
        return "STOP".to_string();
    }

    steps_remaining.div_ceil(speed).to_string()
}

fn resolve_default_coords_input(input: &str, default: [u8; 2]) -> Option<[u8; 2]> {
    if input.trim().is_empty() {
        Some(default)
    } else {
        crate::screen::parse_planet_coords(input)
    }
}

fn resolve_yes_no_input(input: &str, default: bool) -> bool {
    match input.trim().to_ascii_uppercase().as_str() {
        "" => default,
        "Y" | "YES" => true,
        "N" | "NO" => false,
        _ => default,
    }
}

fn format_fleet_number_for_rows(fleet_number: u16, rows: &[FleetRow]) -> String {
    let max_fleet_number = rows.iter().map(|row| row.fleet_number).max().unwrap_or(1);
    crate::screen::format_fleet_number(fleet_number, max_fleet_number)
}

fn insert_char_at(body: &mut String, cursor_index: usize, ch: char) {
    let byte_index = char_to_byte_index(body, cursor_index);
    body.insert(byte_index, ch);
}

fn remove_char_before(body: &mut String, cursor_index: usize) {
    if cursor_index == 0 {
        return;
    }
    let start = char_to_byte_index(body, cursor_index - 1);
    let end = char_to_byte_index(body, cursor_index);
    body.replace_range(start..end, "");
}

fn remove_char_at(body: &mut String, cursor_index: usize) {
    let char_count = body.chars().count();
    if cursor_index >= char_count {
        return;
    }
    let start = char_to_byte_index(body, cursor_index);
    let end = char_to_byte_index(body, cursor_index + 1);
    body.replace_range(start..end, "");
}

fn line_start_index(body: &str, cursor_index: usize) -> usize {
    let chars = body.chars().collect::<Vec<_>>();
    let mut start = cursor_index.min(chars.len());
    while start > 0 && chars[start - 1] != '\n' {
        start -= 1;
    }
    start
}

fn line_end_index(body: &str, cursor_index: usize) -> usize {
    let chars = body.chars().collect::<Vec<_>>();
    let mut end = cursor_index.min(chars.len());
    while end < chars.len() && chars[end] != '\n' {
        end += 1;
    }
    end
}

fn vertical_cursor_target(body: &str, cursor_index: usize, delta: isize) -> usize {
    let chars = body.chars().collect::<Vec<_>>();
    let cursor = cursor_index.min(chars.len());
    let line_start = line_start_index(body, cursor);
    let line_end = line_end_index(body, cursor);
    let column = cursor.saturating_sub(line_start);

    let target_line_start = if delta < 0 {
        if line_start == 0 {
            return cursor;
        }
        let prev_end = line_start - 1;
        let mut prev_start = prev_end;
        while prev_start > 0 && chars[prev_start - 1] != '\n' {
            prev_start -= 1;
        }
        prev_start
    } else {
        if line_end == chars.len() {
            return cursor;
        }
        line_end + 1
    };

    let mut target_line_end = target_line_start;
    while target_line_end < chars.len() && chars[target_line_end] != '\n' {
        target_line_end += 1;
    }
    let target_len = target_line_end.saturating_sub(target_line_start);
    target_line_start + column.min(target_len)
}
