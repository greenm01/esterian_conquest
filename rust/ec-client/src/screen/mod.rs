mod buffer;
mod empire_profile;
mod empire_status;
mod general_menu;
mod layout;
mod main_menu;
mod planet_info;
mod reports;
mod rankings;
mod startup;
mod table;

pub use buffer::{Cell, CellStyle, PlayfieldBuffer, RgbColor, StyledSpan};
pub use empire_profile::EmpireProfileScreen;
pub use empire_status::EmpireStatusScreen;
pub use general_menu::GeneralMenuScreen;
pub use layout::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};
pub use main_menu::MainMenuScreen;
pub use planet_info::{PlanetInfoScreen, parse_planet_coords};
pub use rankings::{RankingsScreen, RankingsView};
pub use reports::ReportsScreen;
pub use startup::StartupScreen;

use std::path::Path;

use crossterm::event::KeyEvent;
use ec_data::CoreGameData;

use crate::app::Action;
use crate::model::PlayerContext;
use crate::startup::StartupPhase;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenId {
    Startup(StartupPhase),
    MainMenu,
    GeneralMenu,
    PlanetInfoPrompt,
    PlanetInfoDetail,
    EmpireStatus,
    EmpireProfile,
    Rankings(RankingsView),
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
        frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>>;

    fn handle_key(&self, key: KeyEvent) -> Action;
}
