use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use ec_data::{
    append_mail_queue, build_player_starmap_projection, load_mail_queue, save_mail_queue,
    CommissionResult, CoreGameData, DatabaseDat, GameStateMutationError, ProductionItemKind,
    QueuedPlayerMail,
};

use crate::app::Action;
use crate::model::{MainMenuSummary, PlayerContext, ReviewSummary};
use crate::reports::{clear_report_files, ReportsPreview};
use crate::screen::{
    build_unit_spec, build_unit_spec_by_kind, max_quantity, BuildHelpScreen, CommandMenu,
    DeleteReviewablesScreen, EmpireProfileScreen, EmpireStatusScreen, EnemiesScreen,
    GeneralHelpScreen, GeneralMenuScreen, MainMenuScreen, MessageComposeScreen,
    PartialStarmapScreen, PlanetBuildChangeRow, PlanetBuildListRow, PlanetBuildMenuView,
    PlanetBuildOrder, PlanetBuildScreen, PlanetCommissionRow, PlanetCommissionScreen,
    PlanetCommissionView, PlanetHelpScreen, PlanetInfoScreen, PlanetListMode, PlanetListScreen,
    PlanetListSort, PlanetMenuScreen, PlanetTaxScreen, RankingsScreen, RankingsView,
    ReportsScreen, Screen, ScreenFrame, ScreenId, StarmapScreen, StartupScreen,
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
    main_menu: MainMenuScreen,
    general_menu: GeneralMenuScreen,
    general_help: GeneralHelpScreen,
    planet_menu: PlanetMenuScreen,
    planet_help: PlanetHelpScreen,
    planet_commission: PlanetCommissionScreen,
    build_help: BuildHelpScreen,
    planet_build: PlanetBuildScreen,
    planet_list: PlanetListScreen,
    planet_tax: PlanetTaxScreen,
    starmap: StarmapScreen,
    partial_starmap: PartialStarmapScreen,
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
    planet_brief_scroll_offset: usize,
    planet_brief_cursor: usize,
    planet_detail_index: usize,
    planet_commission_index: usize,
    planet_commission_cursor: usize,
    planet_commission_scroll_offset: usize,
    planet_commission_selected_slots: BTreeSet<usize>,
    planet_commission_status: Option<String>,
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
}

impl App {
    pub fn load(config: AppConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let game_dir = config.game_dir.clone();
        let export_root = config
            .export_root
            .clone()
            .unwrap_or_else(|| game_dir.join("exports"));
        let game_data = CoreGameData::load(&game_dir)?;
        let database = DatabaseDat::parse(&fs::read(game_dir.join("DATABASE.DAT"))?)?;
        let player = PlayerContext::from_game_data(&game_data, config.player_record_index_1_based)?;
        let pending_results = file_nonempty(game_dir.join("RESULTS.DAT"));
        let reports = ReportsPreview::load(&game_dir)?;
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
        let startup_sequence = StartupSequence::new(&startup_summary);

        Ok(Self {
            game_dir,
            game_data,
            database,
            player,
            current_screen: ScreenId::Startup(startup_sequence.current()),
            startup_sequence,
            startup: StartupScreen::new(startup_summary, reports.clone()),
            main_menu: MainMenuScreen::new(),
            general_menu: GeneralMenuScreen::new(),
            general_help: GeneralHelpScreen::new(),
            planet_menu: PlanetMenuScreen::new(),
            planet_help: PlanetHelpScreen::new(),
            planet_commission: PlanetCommissionScreen::new(),
            build_help: BuildHelpScreen::new(),
            planet_build: PlanetBuildScreen::new(),
            planet_list: PlanetListScreen::new(),
            planet_tax: PlanetTaxScreen::new(),
            starmap: StarmapScreen::new(),
            partial_starmap: PartialStarmapScreen::new(),
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
            planet_brief_scroll_offset: 0,
            planet_brief_cursor: 0,
            planet_detail_index: 0,
            planet_commission_index: 0,
            planet_commission_cursor: 0,
            planet_commission_scroll_offset: 0,
            planet_commission_selected_slots: BTreeSet::new(),
            planet_commission_status: None,
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
        })
    }

    pub fn render(
        &mut self,
        terminal: &mut dyn Terminal,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let frame = ScreenFrame {
            game_dir: &self.game_dir,
            game_data: &self.game_data,
            player: &self.player,
        };

        let playfield = match self.current_screen {
            ScreenId::Startup(phase) => self.startup.render_phase(&frame, phase)?,
            ScreenId::MainMenu => self.main_menu.render(&frame)?,
            ScreenId::GeneralMenu => self.general_menu.render(&frame)?,
            ScreenId::GeneralHelp => self.general_help.render(&frame)?,
            ScreenId::PlanetMenu => self.planet_menu.render(&frame)?,
            ScreenId::PlanetHelp => self.planet_help.render(&frame)?,
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
                &self.partial_starmap_input,
                self.partial_starmap_error.as_deref(),
                self.command_return_menu,
            )?,
            ScreenId::PartialStarmapView => self.partial_starmap.render_view(
                &frame,
                &self.database,
                self.partial_starmap_center,
            )?,
            ScreenId::PlanetInfoPrompt => self.planet_info.render_prompt(
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
            ScreenId::EmpireStatus => self.empire_status.render(&frame)?,
            ScreenId::EmpireProfile => self.empire_profile.render(&frame)?,
            ScreenId::Rankings(RankingsView::Prompt) => self.rankings.render_prompt(&frame)?,
            ScreenId::Rankings(RankingsView::Table(sort)) => {
                self.rankings.render_table(&frame, sort)?
            }
            ScreenId::Reports => self.reports.render(&frame)?,
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
        let next = self.startup_sequence.advance();
        self.current_screen = match next {
            StartupPhase::Complete => ScreenId::MainMenu,
            phase => ScreenId::Startup(phase),
        };
    }

    pub fn open_startup_intro(&mut self) {
        let next = self.startup_sequence.open_intro();
        self.current_screen = ScreenId::Startup(next);
    }

    pub fn open_planet_menu(&mut self) {
        self.command_return_menu = CommandMenu::Planet;
        self.current_screen = ScreenId::PlanetMenu;
    }

    pub fn open_planet_help(&mut self) {
        self.current_screen = ScreenId::PlanetHelp;
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
            self.planet_commission_status =
                Some("No owned planets have units waiting in stardock.".to_string());
        } else {
            self.planet_commission_index = self.planet_commission_index.min(total - 1);
        }
        self.current_screen = ScreenId::PlanetCommissionMenu;
    }

    pub fn open_planet_build_help(&mut self) {
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
            self.planet_build_status = Some("No owned planets available for building.".to_string());
        } else {
            self.planet_build_index = self.planet_build_index.min(total - 1);
        }
        self.current_screen = ScreenId::PlanetBuildMenu;
    }

    pub fn open_planet_build_review(&mut self) {
        self.current_screen = ScreenId::PlanetBuildReview;
    }

    pub fn open_planet_build_list(&mut self) {
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
        self.current_screen = ScreenId::PlanetBuildAbortConfirm;
    }

    pub fn open_planet_build_specify(&mut self) {
        self.planet_build_unit_input.clear();
        self.planet_build_unit_status = None;
        self.planet_build_quantity_input.clear();
        self.planet_build_quantity_status = None;
        self.planet_build_selected_kind = None;
        self.current_screen = ScreenId::PlanetBuildSpecify;
    }

    pub fn open_planet_tax_prompt(&mut self) {
        self.planet_tax_input = String::new();
        self.planet_tax_status = None;
        self.current_screen = ScreenId::PlanetTaxPrompt;
    }

    pub fn open_planet_list_sort_prompt(&mut self, mode: PlanetListMode) {
        self.planet_list_sort_status = None;
        self.current_screen = ScreenId::PlanetListSortPrompt(mode);
    }

    pub fn submit_planet_list_sort(&mut self, mode: PlanetListMode, sort: PlanetListSort) {
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
            self.planet_commission_scroll_offset = self.planet_commission_cursor + 1
                - crate::screen::PLANET_COMMISSION_VISIBLE_ROWS;
        }
    }

    pub fn commission_selected_stardock_row(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
                .filter(|row| self.planet_commission_selected_slots.contains(&row.slot_0_based))
                .map(|row| row.slot_0_based)
                .collect()
        };
        let planet_record = self.current_commission_planet_row()?.planet_record_index_1_based;
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
        self.game_data.save(&self.game_dir)?;
        self.planet_commission_status = Some(match result {
            CommissionResult::Fleet {
                fleet_record_index_1_based,
            } => format!("Commissioned a new fleet as Fleet #{fleet_record_index_1_based}."),
            CommissionResult::Starbase {
                base_record_index_1_based,
            } => format!("Commissioned a new starbase as Base #{base_record_index_1_based}."),
        });

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
            self.planet_commission_selected_slots.remove(&row.slot_0_based);
        } else {
            self.planet_commission_selected_slots.insert(row.slot_0_based);
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
        self.game_data.save(&self.game_dir)?;
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
            self.planet_build_unit_status = Some("No points are available to spend.".to_string());
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
            self.planet_build_quantity_status =
                Some("No points are available to spend.".to_string());
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
        self.game_data.save(&self.game_dir)?;
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
        self.game_data.save(&self.game_dir)?;
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
        self.game_data.save(&self.game_dir)?;
        self.planet_tax_input = parsed.to_string();
        self.planet_tax_status = Some(format!("Empire tax rate set to {parsed}%."));
        self.current_screen = ScreenId::PlanetTaxDone;
        Ok(())
    }

    pub fn handle_key(&self, key: crossterm::event::KeyEvent) -> crate::app::Action {
        match self.current_screen {
            ScreenId::Startup(phase) => self.startup.handle_key(phase, key),
            ScreenId::MainMenu => self.main_menu.handle_key(key),
            ScreenId::GeneralMenu => self.general_menu.handle_key(key),
            ScreenId::GeneralHelp => self.general_help.handle_key(key),
            ScreenId::PlanetMenu => self.planet_menu.handle_key(key),
            ScreenId::PlanetHelp => self.planet_help.handle_key(key),
            ScreenId::PlanetCommissionMenu => self.planet_commission.handle_key(key),
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
            ScreenId::Rankings(RankingsView::Prompt) => self.rankings.handle_prompt_key(key),
            ScreenId::Rankings(RankingsView::Table(_)) => self.rankings.handle_table_key(key),
            ScreenId::Reports => self.reports.handle_key(key),
        }
    }

    pub fn open_planet_info_prompt(&mut self, menu: CommandMenu) {
        self.command_return_menu = menu;
        self.planet_info_input = self
            .game_data
            .planets
            .records
            .iter()
            .find(|planet| {
                planet.owner_empire_slot_raw() as usize == self.player.record_index_1_based
                    && planet.is_homeworld_seed_ignoring_name()
            })
            .map(|planet| {
                let [x, y] = planet.coords_raw();
                format!("{x},{y}")
            })
            .unwrap_or_default();
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
        let default = self
            .game_data
            .planets
            .records
            .iter()
            .find(|planet| {
                planet.owner_empire_slot_raw() as usize == self.player.record_index_1_based
                    && planet.is_homeworld_seed_ignoring_name()
            })
            .map(|planet| planet.coords_raw())
            .unwrap_or([8, 2]);
        self.partial_starmap_input = format!("{},{}", default[0], default[1]);
        self.partial_starmap_error = None;
        self.partial_starmap_center = default;
        self.current_screen = ScreenId::PartialStarmapPrompt;
    }

    pub fn return_to_command_menu(&mut self) {
        self.current_screen = match self.command_return_menu {
            CommandMenu::General => ScreenId::GeneralMenu,
            CommandMenu::Planet => ScreenId::PlanetMenu,
            CommandMenu::PlanetBuild => ScreenId::PlanetBuildMenu,
        };
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
        let Some(coords) = crate::screen::parse_planet_coords(&self.partial_starmap_input) else {
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
        self.game_data.save(&self.game_dir)?;
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
        self.game_data.save(&self.game_dir)?;
        self.enemies_status = Some(format!(
            "Empire {empire_id} is now {}.",
            match next {
                ec_data::DiplomaticRelation::Enemy => "ENEMY",
                ec_data::DiplomaticRelation::Neutral => "NEUTRAL",
            }
        ));
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
        append_mail_queue(
            &self.game_dir,
            &QueuedPlayerMail {
                sender_empire_id: self.player.record_index_1_based as u8,
                recipient_empire_id,
                year: self.game_data.conquest.game_year(),
                subject: self.compose_subject.trim().to_string(),
                body: body.to_string(),
            },
        )?;
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
        let mut queue = load_mail_queue(&self.game_dir)?;
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
        save_mail_queue(&self.game_dir, &queue)?;
        self.compose_outbox_input.clear();
        self.compose_outbox_status = Some(format!("Queued message {:02} deleted.", queue_no));

        // Clamp cursor and scroll offset to the new (smaller) queue.
        let new_len = own_indexes.len().saturating_sub(1);
        self.compose_outbox_cursor = self.compose_outbox_cursor.min(new_len.saturating_sub(1));
        let max_offset = new_len.saturating_sub(crate::screen::OUTBOX_VISIBLE_ROWS);
        self.compose_outbox_scroll_offset = self.compose_outbox_scroll_offset.min(max_offset);
        Ok(())
    }

    pub fn delete_reviewables(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        clear_report_files(&self.game_dir)?;
        if let Some(player) = self
            .game_data
            .player
            .records
            .get_mut(self.player.record_index_1_based - 1)
        {
            player.raw[0x30] = 0;
            player.raw[0x34] = 0;
        }
        self.game_data.save(&self.game_dir)?;
        let refreshed = ReportsPreview::load(&self.game_dir)?;
        let summary = MainMenuSummary::from_game_data(
            &self.game_data,
            self.player.record_index_1_based,
            false,
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
        let Some(coords) = crate::screen::parse_planet_coords(&self.planet_info_input) else {
            self.planet_info_error = Some("Enter coordinates like 5,2".to_string());
            return;
        };

        let Some(planet_idx) = self.game_data.planet_record_index_at_coords(coords) else {
            self.planet_info_error = Some(format!(
                "No world found at X={}, Y={}",
                coords[0], coords[1]
            ));
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
        let row = self.current_commission_planet_row()?;
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
                let available_points =
                    u32::from(row.build_capacity).min(row.stored_production_points.min(u32::from(u16::MAX)));
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
        let row = self.current_build_planet_row()?;
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
        Ok(max_quantity(view.points_left, unit.cost))
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
        Ok(load_mail_queue(&self.game_dir)?
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
                crate::app::Action::OpenGeneralMenu
            }
            KeyCode::Enter => crate::app::Action::SubmitPlanetInfoPrompt,
            KeyCode::Backspace => crate::app::Action::BackspacePlanetInfoInput,
            KeyCode::Char(ch)
                if ch.is_ascii_digit() || matches!(ch, ',' | ' ' | ':' | '/' | '-') =>
            {
                crate::app::Action::AppendPlanetInfoChar(ch)
            }
            _ => crate::app::Action::Noop,
        }
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

fn file_nonempty(path: PathBuf) -> bool {
    fs::metadata(path)
        .map(|meta| meta.len() > 0)
        .unwrap_or(false)
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
