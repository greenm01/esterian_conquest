use std::fs;
use std::path::PathBuf;

use ec_data::CoreGameData;

use crate::model::{GeneralMenuSummary, MainMenuSummary, PlayerContext};
use crate::reports::ReportsPreview;
use crate::screen::{
    GeneralMenuScreen, MainMenuScreen, ReportsScreen, Screen, ScreenFrame, ScreenId,
};
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
    main_menu: MainMenuScreen,
    general_menu: GeneralMenuScreen,
    reports: ReportsScreen,
}

impl App {
    pub fn load(config: AppConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let game_data = CoreGameData::load(&config.game_dir)?;
        let player = PlayerContext::from_game_data(&game_data, config.player_record_index_1_based)?;
        let pending_results = file_nonempty(config.game_dir.join("RESULTS.DAT"));
        let main_menu_summary = MainMenuSummary::from_game_data(
            &game_data,
            config.player_record_index_1_based,
            pending_results,
        );
        let general_menu_summary = GeneralMenuSummary::from_main_menu(&main_menu_summary);
        let reports = ReportsPreview::load(&config.game_dir)?;

        Ok(Self {
            game_dir: config.game_dir,
            game_data,
            player,
            current_screen: ScreenId::MainMenu,
            main_menu: MainMenuScreen::new(main_menu_summary),
            general_menu: GeneralMenuScreen::new(general_menu_summary),
            reports: ReportsScreen::new(reports),
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

        match self.current_screen {
            ScreenId::MainMenu => self.main_menu.render(terminal, &frame),
            ScreenId::GeneralMenu => self.general_menu.render(terminal, &frame),
            ScreenId::Reports => self.reports.render(terminal, &frame),
        }
    }

    pub fn current_screen(&self) -> ScreenId {
        self.current_screen
    }

    pub fn current_screen_mut(&mut self) -> &mut ScreenId {
        &mut self.current_screen
    }

    pub fn handle_key(&self, key: crossterm::event::KeyEvent) -> crate::app::Action {
        match self.current_screen {
            ScreenId::MainMenu => self.main_menu.handle_key(key),
            ScreenId::GeneralMenu => self.general_menu.handle_key(key),
            ScreenId::Reports => self.reports.handle_key(key),
        }
    }
}

fn file_nonempty(path: PathBuf) -> bool {
    fs::metadata(path)
        .map(|meta| meta.len() > 0)
        .unwrap_or(false)
}
