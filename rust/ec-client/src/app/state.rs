use std::fs;
use std::path::PathBuf;

use ec_data::CoreGameData;

use crate::model::{MainMenuSummary, PlayerContext, ReviewSummary};
use crate::reports::ReportsPreview;
use crate::screen::{
    EmpireProfileScreen, EmpireStatusScreen, GeneralMenuScreen, MainMenuScreen, RankingsScreen,
    RankingsView, ReportsScreen, Screen, ScreenFrame, ScreenId, StartupScreen,
};
use crate::startup::{StartupPhase, StartupSequence, StartupSummary};
use crate::terminal::Terminal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub game_dir: PathBuf,
    pub player_record_index_1_based: usize,
}

pub struct App {
    game_dir: PathBuf,
    game_data: CoreGameData,
    player: PlayerContext,
    current_screen: ScreenId,
    startup_sequence: StartupSequence,
    startup: StartupScreen,
    main_menu: MainMenuScreen,
    general_menu: GeneralMenuScreen,
    empire_status: EmpireStatusScreen,
    empire_profile: EmpireProfileScreen,
    rankings: RankingsScreen,
    reports: ReportsScreen,
}

impl App {
    pub fn load(config: AppConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let game_data = CoreGameData::load(&config.game_dir)?;
        let player = PlayerContext::from_game_data(&game_data, config.player_record_index_1_based)?;
        let pending_results = file_nonempty(config.game_dir.join("RESULTS.DAT"));
        let reports = ReportsPreview::load(&config.game_dir)?;
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
            game_dir: config.game_dir,
            game_data,
            player,
            current_screen: ScreenId::Startup(startup_sequence.current()),
            startup_sequence,
            startup: StartupScreen::new(startup_summary, reports.clone()),
            main_menu: MainMenuScreen::new(),
            general_menu: GeneralMenuScreen::new(),
            empire_status: EmpireStatusScreen::new(),
            empire_profile: EmpireProfileScreen::new(),
            rankings: RankingsScreen::new(),
            reports: ReportsScreen::new(reports, review_summary),
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
            ScreenId::EmpireStatus => self.empire_status.handle_key(key),
            ScreenId::EmpireProfile => self.empire_profile.handle_key(key),
            ScreenId::Rankings(RankingsView::Prompt) => self.rankings.handle_prompt_key(key),
            ScreenId::Rankings(RankingsView::Table(_)) => self.rankings.handle_table_key(key),
            ScreenId::Reports => self.reports.handle_key(key),
        }
    }
}

fn file_nonempty(path: PathBuf) -> bool {
    fs::metadata(path)
        .map(|meta| meta.len() > 0)
        .unwrap_or(false)
}
