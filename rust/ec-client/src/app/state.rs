use std::fs;
use std::path::PathBuf;

use ec_data::{
    CoreGameData, DatabaseDat, QueuedPlayerMail, append_mail_queue, load_mail_queue,
    build_player_starmap_projection,
    save_mail_queue,
};

use crate::model::{MainMenuSummary, PlayerContext, ReviewSummary};
use crate::reports::{ReportsPreview, clear_report_files};
use crate::screen::{
    DeleteReviewablesScreen, EmpireProfileScreen, EmpireStatusScreen, EnemiesScreen,
    GeneralHelpScreen,
    GeneralMenuScreen, MainMenuScreen, MessageComposeScreen, PlanetInfoScreen,
    PartialStarmapScreen, RankingsScreen, RankingsView, ReportsScreen, Screen, ScreenFrame,
    ScreenId, StartupScreen, StarmapScreen,
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
    enemies_input: String,
    enemies_status: Option<String>,
    enemies_scroll_offset: usize,
    delete_reviewables_status: Option<String>,
    compose_recipient_input: String,
    compose_recipient_status: Option<String>,
    compose_recipient_scroll_offset: usize,
    compose_recipient_empire: Option<u8>,
    compose_subject: String,
    compose_subject_status: Option<String>,
    compose_body: String,
    compose_body_cursor: usize,
    compose_body_status: Option<String>,
    compose_outbox_input: String,
    compose_outbox_status: Option<String>,
    compose_outbox_scroll_offset: usize,
    compose_sent_status: Option<String>,
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
            enemies_input: String::new(),
            enemies_status: None,
            enemies_scroll_offset: 0,
            delete_reviewables_status: None,
            compose_recipient_input: String::new(),
            compose_recipient_status: None,
            compose_recipient_scroll_offset: 0,
            compose_recipient_empire: None,
            compose_subject: String::new(),
            compose_subject_status: None,
            compose_body: String::new(),
            compose_body_cursor: 0,
            compose_body_status: None,
            compose_outbox_input: String::new(),
            compose_outbox_status: None,
            compose_outbox_scroll_offset: 0,
            compose_sent_status: None,
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
            ScreenId::Starmap if self.starmap_capture_complete => self.starmap.render_complete()?,
            ScreenId::Starmap if self.starmap_dump_active => self.starmap.render_dump_page(
                &self.starmap_dump_lines,
                self.starmap_dump_offset,
            )?,
            ScreenId::Starmap => self.starmap.render_prompt(self.starmap_status.as_deref())?,
            ScreenId::PartialStarmapPrompt => self
                .partial_starmap
                .render_prompt(&self.partial_starmap_input, self.partial_starmap_error.as_deref())?,
            ScreenId::PartialStarmapView => self.partial_starmap.render_view(
                &frame,
                &self.database,
                self.partial_starmap_center,
            )?,
            ScreenId::PlanetInfoPrompt => self
                .planet_info
                .render_prompt(&self.planet_info_input, self.planet_info_error.as_deref())?,
            ScreenId::PlanetInfoDetail => self.planet_info.render_detail(
                &frame,
                self.planet_info_selected.ok_or("planet info detail not selected")?,
            )?,
            ScreenId::Enemies => self
                .enemies
                .render(
                    &frame,
                    &self.enemies_input,
                    self.enemies_status.as_deref(),
                    self.enemies_scroll_offset,
                )?,
            ScreenId::DeleteReviewables => self
                .delete_reviewables
                .render(self.delete_reviewables_status.as_deref())?,
            ScreenId::ComposeMessageRecipient => self.message_compose.render_recipient(
                &frame,
                &self.compose_recipient_input,
                self.compose_recipient_status.as_deref(),
                self.compose_recipient_scroll_offset,
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

    pub fn handle_key(&self, key: crossterm::event::KeyEvent) -> crate::app::Action {
        match self.current_screen {
            ScreenId::Startup(phase) => self.startup.handle_key(phase, key),
            ScreenId::MainMenu => self.main_menu.handle_key(key),
            ScreenId::GeneralMenu => self.general_menu.handle_key(key),
            ScreenId::GeneralHelp => self.general_help.handle_key(key),
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

    pub fn open_planet_info_prompt(&mut self) {
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
        let total = self
            .game_data
            .player
            .records
            .len()
            .saturating_sub(1);
        let max_offset = total.saturating_sub(crate::screen::ENEMIES_VISIBLE_ROWS);
        self.enemies_scroll_offset = self
            .enemies_scroll_offset
            .saturating_add_signed(delta as isize)
            .min(max_offset);
    }

    pub fn open_partial_starmap_prompt(&mut self) {
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

    pub fn append_partial_starmap_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::PartialStarmapPrompt && self.partial_starmap_input.len() < 16 {
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
            self.partial_starmap_error =
                Some(format!("Enter coordinates within 1..{map_size}"));
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
        let Ok(empire_id) = self.enemies_input.parse::<u8>() else {
            self.enemies_status = Some("Enter an empire number.".to_string());
            return Ok(());
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
        let total = self
            .game_data
            .player
            .records
            .len()
            .saturating_sub(1);
        let max_offset = total.saturating_sub(crate::screen::RECIPIENT_VISIBLE_ROWS);
        self.compose_recipient_scroll_offset = self
            .compose_recipient_scroll_offset
            .saturating_add_signed(delta as isize)
            .min(max_offset);
    }

    pub fn backspace_compose_recipient(&mut self) {
        if self.current_screen == ScreenId::ComposeMessageRecipient {
            self.compose_recipient_input.pop();
            self.compose_recipient_status = None;
        }
    }

    pub fn submit_compose_recipient(&mut self) {
        let Ok(empire_id) = self.compose_recipient_input.parse::<u8>() else {
            self.compose_recipient_status = Some("Enter an empire number.".to_string());
            return;
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
            self.compose_body_cursor = line_start_index(&self.compose_body, self.compose_body_cursor);
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

    pub fn append_compose_outbox_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::ComposeMessageOutbox && self.compose_outbox_input.len() < 2
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
        let Ok(queue_no) = self.compose_outbox_input.parse::<usize>() else {
            self.compose_outbox_status = Some("Enter a queued message number.".to_string());
            return Ok(());
        };
        if queue_no == 0 {
            self.compose_outbox_status = Some("Enter a queued message number.".to_string());
            return Ok(());
        }

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

        let max_offset = own_indexes.len().saturating_sub(1).saturating_sub(crate::screen::OUTBOX_VISIBLE_ROWS);
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

    fn compose_outbox_queue(&self) -> Result<Vec<QueuedPlayerMail>, Box<dyn std::error::Error>> {
        let sender_empire_id = self.player.record_index_1_based as u8;
        Ok(load_mail_queue(&self.game_dir)?
            .into_iter()
            .filter(|mail| mail.sender_empire_id == sender_empire_id)
            .collect())
    }

    fn compose_outbox_queue_len(&self) -> usize {
        self.compose_outbox_queue().map(|queue| queue.len()).unwrap_or(0)
    }

    fn handle_planet_info_prompt_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
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

fn compose_recipient_label(game_data: &CoreGameData, empire_id: Option<u8>) -> String {
    let Some(empire_id) = empire_id else {
        return "<unknown>".to_string();
    };
    let Some(player) = game_data.player.records.get(empire_id.saturating_sub(1) as usize) else {
        return format!("Empire {empire_id}");
    };
    let name = player.controlled_empire_name_summary();
    let fallback = player.legacy_status_name_summary();
    let display = if !name.is_empty() { name } else { fallback };
    format!("Empire {empire_id} ({display})")
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
