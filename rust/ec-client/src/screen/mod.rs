mod buffer;
mod empire_profile;
mod empire_status;
mod delete_reviewables;
mod enemies;
mod general_menu;
mod layout;
mod main_menu;
mod message_compose;
mod planet_info;
mod partial_starmap;
mod reports;
mod rankings;
mod startup;
mod starmap;
mod table;

pub use buffer::{Cell, CellStyle, PlayfieldBuffer, RgbColor, StyledSpan};
pub use empire_profile::EmpireProfileScreen;
pub use empire_status::EmpireStatusScreen;
pub use delete_reviewables::DeleteReviewablesScreen;
pub use enemies::EnemiesScreen;
pub use general_menu::GeneralMenuScreen;
pub use layout::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};
pub use main_menu::MainMenuScreen;
pub use message_compose::MessageComposeScreen;
pub use planet_info::{PlanetInfoScreen, parse_planet_coords};
pub use partial_starmap::PartialStarmapScreen;
pub use rankings::{RankingsScreen, RankingsView};
pub use reports::ReportsScreen;
pub use startup::StartupScreen;
pub use starmap::StarmapScreen;

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
    Starmap,
    PartialStarmapPrompt,
    PartialStarmapView,
    PlanetInfoPrompt,
    PlanetInfoDetail,
    Enemies,
    DeleteReviewables,
    ComposeMessageRecipient,
    ComposeMessageBody,
    ComposeMessageSent,
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
