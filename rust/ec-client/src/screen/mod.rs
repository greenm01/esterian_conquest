mod general_menu;
mod main_menu;
mod reports;

pub use general_menu::GeneralMenuScreen;
pub use main_menu::MainMenuScreen;
pub use reports::ReportsScreen;

use std::path::Path;

use crossterm::event::KeyEvent;
use ec_data::CoreGameData;

use crate::app::Action;
use crate::model::PlayerContext;
use crate::terminal::Terminal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenId {
    MainMenu,
    GeneralMenu,
    Reports,
}

pub struct ScreenFrame<'a> {
    pub game_dir: &'a Path,
    pub game_data: &'a CoreGameData,
    pub player: &'a PlayerContext,
}

pub trait Screen {
    fn render(
        &mut self,
        terminal: &mut dyn Terminal,
        frame: &ScreenFrame<'_>,
    ) -> Result<(), Box<dyn std::error::Error>>;

    fn handle_key(&self, key: KeyEvent) -> Action;
}
