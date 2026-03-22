use crate::app::action::Action;
use crate::app::state::App;
use crate::domains::startup::StartupAction;
use crate::model::{MainMenuSummary, PlayerContext, ReviewSummary};
use crate::reports::{ReportsPreview, has_visible_runtime_messages};
use crate::screen::{
    FIRST_TIME_INTRO_PAGE_COUNT, STARTUP_SPLASH_PAGE_COUNT, ScreenId, StartupReviewMode,
};
use crate::startup::{StartupPhase, StartupSummary};

impl App {
    fn startup_review_row_count(&self, is_results: bool) -> usize {
        if is_results {
            self.startup
                .results_block_row_count(self.startup_state.results_block)
        } else {
            self.startup
                .messages_block_row_count(self.startup_state.messages_block)
        }
    }

    fn startup_review_scroll_offset(&self, is_results: bool) -> usize {
        if is_results {
            self.startup_state.results_page
        } else {
            self.startup_state.messages_page
        }
    }

    fn startup_review_max_scroll_offset(&self, is_results: bool) -> usize {
        self.startup_review_row_count(is_results)
            .saturating_sub(crate::domains::startup::screens::startup::STARTUP_REVIEW_VISIBLE_LINES)
    }

    fn startup_review_is_at_end(&self, is_results: bool) -> bool {
        self.startup_review_scroll_offset(is_results)
            >= self.startup_review_max_scroll_offset(is_results)
    }

    fn startup_review_nonstop(&self, is_results: bool) -> bool {
        if is_results {
            self.startup_state.results_nonstop
        } else {
            self.startup_state.messages_nonstop
        }
    }

    fn delete_current_startup_review_block(
        &mut self,
        is_results: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if is_results {
            let block_idx = self.startup_state.results_block;
            // Find the actual ReportBlockRow for this display index (among
            // the non-deleted blocks) and soft-delete it.
            let active_indices: Vec<usize> = self
                .report_block_rows
                .iter()
                .enumerate()
                .filter(|(_, r)| !r.recipient_deleted)
                .map(|(i, _)| i)
                .collect();
            if let Some(&row_idx) = active_indices.get(block_idx) {
                let bi = self.report_block_rows[row_idx].block_index;
                self.planet
                    .campaign_store
                    .mark_report_block_deleted(self.snapshot_id, bi)?;
                self.report_block_rows[row_idx].recipient_deleted = true;
            }
            self.sync_player_review_flags();
            self.save_game_data()?;
            self.refresh_review_context()?;
            self.startup_state.results_deleted_any = true;
            self.startup_state.results_page = 0;
            if self.startup_state.results_block < self.startup.result_block_count() {
                self.startup_state.results_mode = StartupReviewMode::ContinuePrompt;
            } else {
                self.startup_state.results_mode = StartupReviewMode::EndStatus;
            }
        } else {
            let block_idx = self.startup_state.messages_block;
            let mail_index = self
                .startup
                .message_blocks()
                .get(block_idx)
                .and_then(|block| block.runtime_mail_index);
            if let Some(mail) = mail_index.and_then(|idx| self.queued_mail.get_mut(idx)) {
                mail.mark_deleted_by_recipient();
            }
            self.sync_player_review_flags();
            self.save_game_data()?;
            self.refresh_review_context()?;
            self.startup_state.messages_deleted_any = true;
            self.startup_state.messages_page = 0;
            if self.startup_state.messages_block < self.startup.message_block_count() {
                self.startup_state.messages_mode = StartupReviewMode::ContinuePrompt;
            } else {
                self.startup_state.messages_mode = StartupReviewMode::EndStatus;
            }
        }
        Ok(())
    }

    fn skip_current_startup_review_block(&mut self, is_results: bool) {
        if is_results {
            let next_block = self.startup_state.results_block + 1;
            self.startup_state.results_block = next_block;
            self.startup_state.results_page = 0;
            if next_block < self.startup.result_block_count() {
                self.startup_state.results_mode = StartupReviewMode::ContinuePrompt;
            } else {
                self.startup_state.results_mode = StartupReviewMode::EndStatus;
            }
        } else {
            let next_block = self.startup_state.messages_block + 1;
            self.startup_state.messages_block = next_block;
            self.startup_state.messages_page = 0;
            if next_block < self.startup.message_block_count() {
                self.startup_state.messages_mode = StartupReviewMode::ContinuePrompt;
            } else {
                self.startup_state.messages_mode = StartupReviewMode::EndStatus;
            }
        }
    }

    pub fn scroll_startup_review(&mut self, delta: isize) {
        let is_results = match self.current_screen {
            ScreenId::Startup(StartupPhase::Results) => true,
            ScreenId::Startup(StartupPhase::Messages) => false,
            _ => return,
        };
        let mode = if is_results {
            self.startup_state.results_mode
        } else {
            self.startup_state.messages_mode
        };
        if mode != StartupReviewMode::ItemBody {
            return;
        }
        if self.startup_review_is_at_end(is_results) && !self.startup_review_nonstop(is_results) {
            return;
        }

        let current = self.startup_review_scroll_offset(is_results) as isize;
        let max = self.startup_review_max_scroll_offset(is_results) as isize;
        let next = (current + delta).clamp(0, max) as usize;
        self.set_startup_review_page(is_results, next);
    }

    pub fn advance_startup(&mut self) {
        if self.current_screen == ScreenId::FirstTimeIntro {
            if self.startup_state.first_time_intro_page + 1 < FIRST_TIME_INTRO_PAGE_COUNT {
                self.startup_state.first_time_intro_page += 1;
            } else {
                self.current_screen = ScreenId::FirstTimeMenu;
            }
            return;
        }
        if self.current_screen == ScreenId::Startup(StartupPhase::Splash)
            && self.startup_state.splash_page + 1 < STARTUP_SPLASH_PAGE_COUNT
        {
            self.startup_state.splash_page += 1;
            return;
        }
        if self.current_screen == ScreenId::Startup(StartupPhase::Splash) {
            let next = self.startup_sequence.advance();
            self.current_screen = self.startup_target_screen(next);
            return;
        }
        if self.current_screen == ScreenId::Startup(StartupPhase::Intro)
            && self.startup_state.intro_page + 1 < crate::screen::STARTUP_INTRO_PAGE_COUNT
        {
            self.startup_state.intro_page += 1;
            return;
        }
        if self.current_screen == ScreenId::Startup(StartupPhase::Results) {
            self.advance_startup_review_phase(true);
            return;
        }
        if self.current_screen == ScreenId::Startup(StartupPhase::Messages) {
            self.advance_startup_review_phase(false);
            return;
        }
        self.reset_startup_review_cursors_for_phase_exit();
        let next = self.startup_sequence.advance();
        self.current_screen = self.startup_target_screen(next);
    }

    fn advance_startup_review_phase(&mut self, is_results: bool) {
        let mode = if is_results {
            self.startup_state.results_mode
        } else {
            self.startup_state.messages_mode
        };
        let block = if is_results {
            self.startup_state.results_block
        } else {
            self.startup_state.messages_block
        };
        let page = if is_results {
            self.startup_state.results_page
        } else {
            self.startup_state.messages_page
        };
        let nonstop = if is_results {
            self.startup_state.results_nonstop
        } else {
            self.startup_state.messages_nonstop
        };
        let block_count = if is_results {
            self.startup.result_block_count()
        } else {
            self.startup.message_block_count()
        };
        let max_scroll = self.startup_review_max_scroll_offset(is_results);

        match mode {
            StartupReviewMode::ViewPrompt => {
                if block_count == 0 {
                    self.advance_startup_phase(is_results);
                } else {
                    self.set_startup_review_mode(is_results, StartupReviewMode::ItemBody);
                    self.set_startup_review_page(is_results, 0);
                }
            }
            StartupReviewMode::ItemBody => {
                if page < max_scroll {
                    self.set_startup_review_page(is_results, page + 1);
                } else if nonstop {
                    let next_block = block + 1;
                    if next_block < block_count {
                        self.set_startup_review_block(is_results, next_block);
                        self.set_startup_review_page(is_results, 0);
                    } else {
                        self.set_startup_review_mode(is_results, StartupReviewMode::EndStatus);
                    }
                } else {
                    self.skip_current_startup_review_block(is_results);
                }
            }
            StartupReviewMode::DeletePrompt => {
                let next_block = block + 1;
                self.set_startup_review_block(is_results, next_block);
                self.set_startup_review_page(is_results, 0);
                if next_block < block_count {
                    self.set_startup_review_mode(is_results, StartupReviewMode::ContinuePrompt);
                } else {
                    self.set_startup_review_mode(is_results, StartupReviewMode::EndStatus);
                }
            }
            StartupReviewMode::ContinuePrompt => {
                self.set_startup_review_mode(is_results, StartupReviewMode::ItemBody);
                self.set_startup_review_page(is_results, 0);
            }
            StartupReviewMode::EndStatus => {
                self.advance_startup_phase(is_results);
            }
        }
    }

    fn advance_startup_phase(&mut self, is_results: bool) {
        if is_results {
            self.startup_state.results_block = 0;
            self.startup_state.results_page = 0;
            self.startup_state.results_mode = StartupReviewMode::ViewPrompt;
            self.startup_state.results_nonstop = false;
        } else {
            self.startup_state.messages_block = 0;
            self.startup_state.messages_page = 0;
            self.startup_state.messages_mode = StartupReviewMode::ViewPrompt;
            self.startup_state.messages_nonstop = false;
        }
        let next = self.startup_sequence.advance();
        self.current_screen = self.startup_target_screen(next);
    }

    fn set_startup_review_mode(&mut self, is_results: bool, mode: StartupReviewMode) {
        if is_results {
            self.startup_state.results_mode = mode;
        } else {
            self.startup_state.messages_mode = mode;
        }
    }

    fn set_startup_review_block(&mut self, is_results: bool, block: usize) {
        if is_results {
            self.startup_state.results_block = block;
        } else {
            self.startup_state.messages_block = block;
        }
    }

    fn set_startup_review_page(&mut self, is_results: bool, page: usize) {
        if is_results {
            self.startup_state.results_page = page;
        } else {
            self.startup_state.messages_page = page;
        }
    }

    pub fn skip_startup_intro(&mut self) {
        let next = self.startup_sequence.skip_intro();
        self.current_screen = self.startup_target_screen(next);
    }

    pub fn startup_accept_default(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match self.current_screen {
            ScreenId::Startup(StartupPhase::Results) => match self.startup_state.results_mode {
                StartupReviewMode::ViewPrompt => {
                    self.startup_state.results_mode = StartupReviewMode::ItemBody;
                    self.startup_state.results_page = 0;
                }
                StartupReviewMode::ItemBody
                    if self.startup_review_is_at_end(true)
                        && !self.startup_review_nonstop(true) =>
                {
                    self.delete_current_startup_review_block(true)?;
                }
                StartupReviewMode::DeletePrompt => {
                    self.delete_current_startup_review_block(true)?;
                }
                StartupReviewMode::ContinuePrompt => {
                    self.startup_state.results_mode = StartupReviewMode::ItemBody;
                    self.startup_state.results_page = 0;
                }
                _ => {}
            },
            ScreenId::Startup(StartupPhase::Messages) => match self.startup_state.messages_mode {
                StartupReviewMode::ViewPrompt => {
                    self.startup_state.messages_mode = StartupReviewMode::ItemBody;
                    self.startup_state.messages_page = 0;
                }
                StartupReviewMode::ItemBody
                    if self.startup_review_is_at_end(false)
                        && !self.startup_review_nonstop(false) =>
                {
                    self.delete_current_startup_review_block(false)?;
                }
                StartupReviewMode::DeletePrompt => {
                    self.delete_current_startup_review_block(false)?;
                }
                StartupReviewMode::ContinuePrompt => {
                    self.startup_state.messages_mode = StartupReviewMode::ItemBody;
                    self.startup_state.messages_page = 0;
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    pub fn startup_reject_choice(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match self.current_screen {
            ScreenId::Startup(StartupPhase::Results) => match self.startup_state.results_mode {
                StartupReviewMode::ViewPrompt => {
                    self.advance_startup_phase(true);
                }
                StartupReviewMode::ItemBody
                    if self.startup_review_is_at_end(true)
                        && !self.startup_review_nonstop(true) =>
                {
                    self.skip_current_startup_review_block(true);
                }
                StartupReviewMode::DeletePrompt => {
                    self.skip_current_startup_review_block(true);
                }
                StartupReviewMode::ContinuePrompt => {
                    self.startup_state.results_mode = StartupReviewMode::EndStatus;
                }
                _ => {}
            },
            ScreenId::Startup(StartupPhase::Messages) => match self.startup_state.messages_mode {
                StartupReviewMode::ViewPrompt => {
                    self.advance_startup_phase(false);
                }
                StartupReviewMode::ItemBody
                    if self.startup_review_is_at_end(false)
                        && !self.startup_review_nonstop(false) =>
                {
                    self.skip_current_startup_review_block(false);
                }
                StartupReviewMode::DeletePrompt => {
                    self.skip_current_startup_review_block(false);
                }
                StartupReviewMode::ContinuePrompt => {
                    self.startup_state.messages_mode = StartupReviewMode::EndStatus;
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    pub fn startup_enable_nonstop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match self.current_screen {
            ScreenId::Startup(StartupPhase::Results) => match self.startup_state.results_mode {
                StartupReviewMode::ViewPrompt | StartupReviewMode::ContinuePrompt => {
                    self.startup_state.results_nonstop = true;
                    self.startup_state.results_mode = StartupReviewMode::ItemBody;
                    self.startup_state.results_page = 0;
                }
                _ => {}
            },
            ScreenId::Startup(StartupPhase::Messages) => match self.startup_state.messages_mode {
                StartupReviewMode::ViewPrompt | StartupReviewMode::ContinuePrompt => {
                    self.startup_state.messages_nonstop = true;
                    self.startup_state.messages_mode = StartupReviewMode::ItemBody;
                    self.startup_state.messages_page = 0;
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    pub fn open_first_time_menu(&mut self) {
        self.startup_state.first_time_status = None;
        self.startup_state.first_time_input.clear();
        self.current_screen = ScreenId::FirstTimeMenu;
    }

    pub fn open_first_time_help(&mut self) {
        self.startup_state.first_time_status = None;
        self.current_screen = ScreenId::FirstTimeHelp;
    }

    pub fn open_first_time_empires(&mut self) {
        self.startup_state.first_time_status = None;
        self.current_screen = ScreenId::FirstTimeEmpires;
    }

    pub fn open_first_time_intro(&mut self) {
        self.startup_state.first_time_status = None;
        self.startup_state.first_time_intro_page = 0;
        self.current_screen = ScreenId::FirstTimeIntro;
    }

    pub fn open_first_time_join_name(&mut self) {
        self.startup_state.first_time_status = None;
        self.startup_state.first_time_input.clear();
        self.startup_state.first_time_rename_preloaded_empire = false;
        self.current_screen = ScreenId::FirstTimeJoinEmpireName;
    }

    pub fn show_first_time_ansi_notice(&mut self) {
        self.startup_state.first_time_status =
            Some("ANSI stays on. The stars look better in color.".to_string());
        self.current_screen = ScreenId::FirstTimeMenu;
    }

    pub fn append_first_time_input_char(&mut self, ch: char) {
        if !matches!(
            self.current_screen,
            ScreenId::FirstTimeJoinEmpireName
                | ScreenId::FirstTimeHomeworldName
                | ScreenId::ColonyWorldName
        ) {
            return;
        }
        if !ch.is_ascii_graphic() && ch != ' ' {
            return;
        }
        if self.startup_state.first_time_input.chars().count() >= 20 {
            return;
        }
        self.startup_state.first_time_input.push(ch);
    }

    pub fn backspace_first_time_input(&mut self) {
        if !matches!(
            self.current_screen,
            ScreenId::FirstTimeJoinEmpireName
                | ScreenId::FirstTimeHomeworldName
                | ScreenId::ColonyWorldName
        ) {
            return;
        }
        self.startup_state.first_time_input.pop();
    }

    pub fn submit_first_time_input(&mut self) {
        match self.current_screen {
            ScreenId::FirstTimeJoinEmpireName => {
                let value = self.startup_state.first_time_input.trim();
                if value.is_empty() {
                    self.startup_state.first_time_status =
                        Some("Empire names need at least one visible character.".to_string());
                    return;
                }
                self.startup_state.first_time_status = None;
                self.startup_state.first_time_empire_name = value.to_string();
                self.startup_state.first_time_input.clear();
                self.current_screen = ScreenId::FirstTimeJoinEmpireConfirm;
            }
            ScreenId::FirstTimeHomeworldName => {
                let value = self.startup_state.first_time_input.trim();
                if value.is_empty() {
                    self.startup_state.first_time_status =
                        Some("Homeworld names need at least one visible character.".to_string());
                    return;
                }
                self.startup_state.first_time_status = None;
                self.startup_state.first_time_homeworld_name = value.to_string();
                self.startup_state.first_time_input.clear();
                self.current_screen = ScreenId::FirstTimeHomeworldConfirm;
            }
            ScreenId::ColonyWorldName => {
                let value = self.startup_state.first_time_input.trim();
                if value.is_empty() {
                    self.startup_state.first_time_status =
                        Some("World names need at least one visible character.".to_string());
                    return;
                }
                self.startup_state.first_time_status = None;
                self.startup_state.colony_world_planet_record_index_1_based =
                    self.colony_world_target_planet_index();
                self.startup_state.colony_world_name = value.to_string();
                self.startup_state.first_time_input.clear();
                self.current_screen = ScreenId::ColonyWorldConfirm;
            }
            _ => {}
        }
    }

    pub fn accept_first_time_prompt(&mut self) {
        match self.current_screen {
            ScreenId::FirstTimePreloadedRenamePrompt => {
                self.startup_state.first_time_status = None;
                self.startup_state.first_time_input = self.player.empire_name.clone();
                self.startup_state.first_time_rename_preloaded_empire = true;
                self.current_screen = ScreenId::FirstTimeJoinEmpireName;
            }
            ScreenId::FirstTimeJoinEmpireConfirm => {
                if self.startup_state.first_time_rename_preloaded_empire {
                    if self.complete_preloaded_empire_rename().is_ok() {
                        self.startup_state.first_time_rename_preloaded_empire = false;
                        self.current_screen = ScreenId::FirstTimeJoinSummary;
                    }
                } else if self.complete_first_time_join().is_ok() {
                    self.current_screen = ScreenId::FirstTimeJoinSummary;
                }
            }
            ScreenId::FirstTimeJoinSummary => {
                self.current_screen = ScreenId::FirstTimeJoinNoPending;
            }
            ScreenId::FirstTimeJoinNoPending => {
                self.startup_state.first_time_status = None;
                self.startup_state.first_time_input.clear();
                self.current_screen = self.pending_naming_screen().unwrap_or(ScreenId::MainMenu);
            }
            ScreenId::FirstTimeHomeworldConfirm => {
                if self.complete_first_time_homeworld_name().is_ok() {
                    self.current_screen =
                        self.pending_naming_screen().unwrap_or(ScreenId::MainMenu);
                }
            }
            ScreenId::ColonyWorldConfirm => {
                if self.complete_colony_world_name().is_ok() {
                    self.current_screen =
                        self.pending_naming_screen().unwrap_or(ScreenId::MainMenu);
                }
            }
            _ => {}
        }
    }

    pub fn reject_first_time_prompt(&mut self) {
        match self.current_screen {
            ScreenId::FirstTimePreloadedRenamePrompt => {
                self.startup_state.first_time_status = None;
                self.startup_state.first_time_input.clear();
                self.startup_state.first_time_rename_preloaded_empire = false;
                self.current_screen = ScreenId::FirstTimeJoinSummary;
            }
            ScreenId::FirstTimeJoinEmpireName
                if self.startup_state.first_time_rename_preloaded_empire =>
            {
                self.startup_state.first_time_status = None;
                self.startup_state.first_time_input.clear();
                self.current_screen = ScreenId::FirstTimePreloadedRenamePrompt;
            }
            ScreenId::FirstTimeJoinEmpireConfirm => {
                self.startup_state.first_time_input =
                    self.startup_state.first_time_empire_name.clone();
                self.current_screen = ScreenId::FirstTimeJoinEmpireName;
            }
            ScreenId::FirstTimeHomeworldConfirm => {
                self.startup_state.first_time_input =
                    self.startup_state.first_time_homeworld_name.clone();
                self.current_screen = ScreenId::FirstTimeHomeworldName;
            }
            ScreenId::ColonyWorldName => {
                self.startup_state.first_time_status =
                    Some("You must name this newly colonized world before continuing.".to_string());
                self.current_screen = ScreenId::ColonyWorldName;
            }
            ScreenId::ColonyWorldConfirm => {
                self.startup_state.first_time_input = self.startup_state.colony_world_name.clone();
                self.current_screen = ScreenId::ColonyWorldName;
            }
            _ => {}
        }
    }

    pub(crate) fn handle_startup_key(
        &self,
        phase: StartupPhase,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match phase {
            StartupPhase::Splash => {
                if self.startup_state.splash_page == 0 {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            Action::Startup(StartupAction::Advance)
                        }
                        _ => Action::Startup(StartupAction::SkipIntro),
                    }
                } else {
                    Action::Startup(StartupAction::Advance)
                }
            }
            StartupPhase::Intro | StartupPhase::LoginSummary => {
                Action::Startup(StartupAction::Advance)
            }
            StartupPhase::Results => {
                if self.startup_state.results_mode == StartupReviewMode::ItemBody {
                    if !self.startup_review_is_at_end(true) {
                        return match key.code {
                            KeyCode::Char('q') | KeyCode::Char('Q') => Action::Quit,
                            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                                Action::Startup(StartupAction::ScrollReview(-1))
                            }
                            KeyCode::PageUp => Action::Startup(StartupAction::ScrollReview(
                                -(crate::domains::startup::screens::startup::STARTUP_REVIEW_VISIBLE_LINES as isize),
                            )),
                            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                                Action::Startup(StartupAction::ScrollReview(1))
                            }
                            KeyCode::PageDown => Action::Startup(StartupAction::ScrollReview(
                                crate::domains::startup::screens::startup::STARTUP_REVIEW_VISIBLE_LINES as isize,
                            )),
                            _ => Action::Startup(StartupAction::Advance),
                        };
                    }
                }
                match self.startup_state.results_mode {
                    StartupReviewMode::ViewPrompt | StartupReviewMode::ContinuePrompt => {
                        match key.code {
                            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                                Action::Startup(StartupAction::AcceptDefault)
                            }
                            KeyCode::Char('n') | KeyCode::Char('N') => {
                                Action::Startup(StartupAction::RejectChoice)
                            }
                            KeyCode::Char('s') | KeyCode::Char('S') => {
                                Action::Startup(StartupAction::EnableNonstop)
                            }
                            KeyCode::Char('q') | KeyCode::Char('Q') => Action::Quit,
                            _ => Action::Noop,
                        }
                    }
                    StartupReviewMode::ItemBody => match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => Action::Quit,
                        KeyCode::Char('y') | KeyCode::Char('Y')
                            if self.startup_review_is_at_end(true)
                                && !self.startup_review_nonstop(true) =>
                        {
                            Action::Startup(StartupAction::AcceptDefault)
                        }
                        KeyCode::Enter | KeyCode::Char('n') | KeyCode::Char('N')
                            if self.startup_review_is_at_end(true)
                                && !self.startup_review_nonstop(true) =>
                        {
                            Action::Startup(StartupAction::RejectChoice)
                        }
                        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                            Action::Startup(StartupAction::ScrollReview(-1))
                        }
                        KeyCode::PageUp => Action::Startup(StartupAction::ScrollReview(
                            -(crate::domains::startup::screens::startup::STARTUP_REVIEW_VISIBLE_LINES as isize),
                        )),
                        _ => Action::Startup(StartupAction::Advance),
                    },
                    StartupReviewMode::DeletePrompt => match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            Action::Startup(StartupAction::AcceptDefault)
                        }
                        KeyCode::Enter | KeyCode::Char('n') | KeyCode::Char('N') => {
                            Action::Startup(StartupAction::RejectChoice)
                        }
                        KeyCode::Char('q') | KeyCode::Char('Q') => Action::Quit,
                        _ => Action::Noop,
                    },
                    StartupReviewMode::EndStatus => match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => Action::Quit,
                        _ => Action::Startup(StartupAction::Advance),
                    },
                }
            }
            StartupPhase::Messages => {
                if self.startup_state.messages_mode == StartupReviewMode::ItemBody {
                    if !self.startup_review_is_at_end(false) {
                        return match key.code {
                            KeyCode::Char('q') | KeyCode::Char('Q') => Action::Quit,
                            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                                Action::Startup(StartupAction::ScrollReview(-1))
                            }
                            KeyCode::PageUp => Action::Startup(StartupAction::ScrollReview(
                                -(crate::domains::startup::screens::startup::STARTUP_REVIEW_VISIBLE_LINES as isize),
                            )),
                            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                                Action::Startup(StartupAction::ScrollReview(1))
                            }
                            KeyCode::PageDown => Action::Startup(StartupAction::ScrollReview(
                                crate::domains::startup::screens::startup::STARTUP_REVIEW_VISIBLE_LINES as isize,
                            )),
                            _ => Action::Startup(StartupAction::Advance),
                        };
                    }
                }
                match self.startup_state.messages_mode {
                    StartupReviewMode::ViewPrompt | StartupReviewMode::ContinuePrompt => {
                        match key.code {
                            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                                Action::Startup(StartupAction::AcceptDefault)
                            }
                            KeyCode::Char('n') | KeyCode::Char('N') => {
                                Action::Startup(StartupAction::RejectChoice)
                            }
                            KeyCode::Char('s') | KeyCode::Char('S') => {
                                Action::Startup(StartupAction::EnableNonstop)
                            }
                            KeyCode::Char('q') | KeyCode::Char('Q') => Action::Quit,
                            _ => Action::Noop,
                        }
                    }
                    StartupReviewMode::ItemBody => match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => Action::Quit,
                        KeyCode::Char('y') | KeyCode::Char('Y')
                            if self.startup_review_is_at_end(false)
                                && !self.startup_review_nonstop(false) =>
                        {
                            Action::Startup(StartupAction::AcceptDefault)
                        }
                        KeyCode::Enter | KeyCode::Char('n') | KeyCode::Char('N')
                            if self.startup_review_is_at_end(false)
                                && !self.startup_review_nonstop(false) =>
                        {
                            Action::Startup(StartupAction::RejectChoice)
                        }
                        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                            Action::Startup(StartupAction::ScrollReview(-1))
                        }
                        KeyCode::PageUp => Action::Startup(StartupAction::ScrollReview(
                            -(crate::domains::startup::screens::startup::STARTUP_REVIEW_VISIBLE_LINES as isize),
                        )),
                        _ => Action::Startup(StartupAction::Advance),
                    },
                    StartupReviewMode::DeletePrompt => match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            Action::Startup(StartupAction::AcceptDefault)
                        }
                        KeyCode::Enter | KeyCode::Char('n') | KeyCode::Char('N') => {
                            Action::Startup(StartupAction::RejectChoice)
                        }
                        KeyCode::Char('q') | KeyCode::Char('Q') => Action::Quit,
                        _ => Action::Noop,
                    },
                    StartupReviewMode::EndStatus => match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => Action::Quit,
                        _ => Action::Startup(StartupAction::Advance),
                    },
                }
            }
            StartupPhase::Complete => Action::Noop,
        }
    }

    pub(crate) fn first_time_empire_rows(&self) -> Vec<String> {
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

    pub(crate) fn first_time_homeworld_summary(
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

    pub(crate) fn colony_world_summary(
        &self,
    ) -> Result<([u8; 2], u16, u16), Box<dyn std::error::Error>> {
        let planet_index = self
            .startup_state
            .colony_world_planet_record_index_1_based
            .or_else(|| self.colony_world_target_planet_index())
            .ok_or("colony world prompt missing target planet")?;
        let planet = self
            .game_data
            .planets
            .records
            .get(planet_index - 1)
            .ok_or("colony world missing for naming prompt")?;
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
            &self.startup_state.first_time_empire_name,
        )?;
        self.save_game_data()?;
        self.refresh_player_context()?;
        Ok(())
    }

    fn complete_first_time_homeworld_name(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.game_data.rename_player_homeworld(
            self.player.record_index_1_based,
            &self.startup_state.first_time_homeworld_name,
        )?;
        self.save_game_data()?;
        self.refresh_player_context()?;
        Ok(())
    }

    fn complete_colony_world_name(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let planet_index = self
            .startup_state
            .colony_world_planet_record_index_1_based
            .or_else(|| self.colony_world_target_planet_index())
            .ok_or("colony world prompt missing target planet")?;
        self.game_data.rename_owned_planet(
            self.player.record_index_1_based,
            planet_index,
            &self.startup_state.colony_world_name,
        )?;
        self.save_game_data()?;
        self.refresh_player_context()?;
        self.startup_state.colony_world_planet_record_index_1_based = None;
        self.startup_state.colony_world_name.clear();
        Ok(())
    }

    fn complete_preloaded_empire_rename(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let player = self
            .game_data
            .player
            .records
            .get_mut(self.player.record_index_1_based - 1)
            .ok_or("player record missing for pre-loaded rename")?;
        player.set_controlled_empire_name_raw(&self.startup_state.first_time_empire_name);
        self.save_game_data()?;
        self.refresh_player_context()?;
        Ok(())
    }

    fn refresh_player_context(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.player =
            PlayerContext::from_game_data(&self.game_data, self.player.record_index_1_based)?;
        self.refresh_review_context()?;
        Ok(())
    }

    fn refresh_review_context(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let refreshed = ReportsPreview::from_block_rows(
            &self.game_data,
            self.player.record_index_1_based as u8,
            &self.report_block_rows,
            &self.queued_mail,
        );
        let has_results = self.has_active_report_blocks();
        let summary = MainMenuSummary::from_game_data(
            &self.game_data,
            self.player.record_index_1_based,
            has_results,
            has_visible_runtime_messages(self.player.record_index_1_based as u8, &self.queued_mail),
        );
        let startup_summary = StartupSummary::from_reports(
            summary.game_year,
            self.player.classic_login_state,
            summary.pending_results,
            summary.pending_messages,
            &refreshed,
        );
        self.startup.replace(startup_summary, refreshed.clone());
        self.reports
            .replace(refreshed, ReviewSummary::from_main_menu(&summary));
        Ok(())
    }

    fn reset_startup_review_cursors_for_phase_exit(&mut self) {
        if self.current_screen != ScreenId::Startup(StartupPhase::Results) {
            self.startup_state.results_block = 0;
            self.startup_state.results_page = 0;
            self.startup_state.results_mode = StartupReviewMode::ViewPrompt;
            self.startup_state.results_nonstop = false;
            self.startup_state.results_deleted_any = false;
        }
        if self.current_screen != ScreenId::Startup(StartupPhase::Messages) {
            self.startup_state.messages_block = 0;
            self.startup_state.messages_page = 0;
            self.startup_state.messages_mode = StartupReviewMode::ViewPrompt;
            self.startup_state.messages_nonstop = false;
            self.startup_state.messages_deleted_any = false;
        }
    }

    fn sync_player_review_flags(&mut self) {
        let has_results = self.has_active_report_blocks();
        if let Some(player) = self
            .game_data
            .player
            .records
            .get_mut(self.player.record_index_1_based - 1)
        {
            player.set_classic_login_reviewables_present(
                has_results
                    || has_visible_runtime_messages(
                        self.player.record_index_1_based as u8,
                        &self.queued_mail,
                    ),
            );
            player.set_classic_results_chain_state(has_results, if has_results { 1 } else { 0 });
        }
    }

    fn startup_target_screen(&self, phase: StartupPhase) -> ScreenId {
        match phase {
            StartupPhase::Complete => match self.player.classic_login_state {
                crate::model::ClassicLoginState::FirstTimeMenu => ScreenId::FirstTimeMenu,
                crate::model::ClassicLoginState::MatchedPreloadedFirstLogin => {
                    ScreenId::FirstTimePreloadedRenamePrompt
                }
                crate::model::ClassicLoginState::ReturningPlayer => {
                    self.pending_naming_screen().unwrap_or(ScreenId::MainMenu)
                }
            },
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
            return self.pending_colony_world_naming_screen();
        }
        if self
            .game_data
            .planets
            .records
            .get(planet_index - 1)
            .filter(|planet| planet.is_named_homeworld_seed())
            .is_some()
        {
            return Some(ScreenId::FirstTimeHomeworldName);
        }
        self.pending_colony_world_naming_screen()
    }

    fn pending_colony_world_naming_screen(&self) -> Option<ScreenId> {
        self.colony_world_target_planet_index()
            .map(|_| ScreenId::ColonyWorldName)
    }

    fn colony_world_target_planet_index(&self) -> Option<usize> {
        let player_empire = self.player.record_index_1_based as u8;
        let homeworld_index = self
            .game_data
            .player
            .records
            .get(self.player.record_index_1_based - 1)
            .map(|player| player.homeworld_planet_index_1_based_raw() as usize)
            .unwrap_or(0);
        self.game_data
            .planets
            .records
            .iter()
            .enumerate()
            .find(|(idx, planet)| {
                planet.owner_empire_slot_raw() == player_empire
                    && planet.planet_name() == "Not Named Yet"
                    && *idx + 1 != homeworld_index
            })
            .map(|(idx, _)| idx + 1)
    }
}
