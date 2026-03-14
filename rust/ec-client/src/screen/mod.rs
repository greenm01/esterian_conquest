mod buffer;
mod empire_profile;
mod empire_status;
mod delete_reviewables;
mod enemies;
mod general_menu;
mod general_help;
mod layout;
mod main_menu;
mod message_compose;
mod planet_help;
mod planet_info;
mod planet_list;
mod planet_menu;
mod planet_tax;
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
pub(crate) use enemies::ENEMIES_VISIBLE_ROWS;
pub use enemies::EnemiesScreen;
pub use general_menu::GeneralMenuScreen;
pub use general_help::GeneralHelpScreen;
pub use layout::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};
pub use main_menu::MainMenuScreen;
pub(crate) use message_compose::{
    COMPOSE_BODY_LIMIT, COMPOSE_SUBJECT_LIMIT, OUTBOX_VISIBLE_ROWS, RECIPIENT_VISIBLE_ROWS,
};
pub use message_compose::MessageComposeScreen;
pub use planet_help::PlanetHelpScreen;
pub use planet_info::{PlanetInfoScreen, parse_planet_coords};
pub(crate) use planet_list::PLANET_BRIEF_VISIBLE_ROWS;
pub use planet_list::{PlanetListMode, PlanetListScreen, PlanetListSort};
pub use planet_menu::PlanetMenuScreen;
pub use planet_tax::PlanetTaxScreen;
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
    GeneralHelp,
    PlanetMenu,
    PlanetHelp,
    PlanetListSortPrompt(PlanetListMode),
    PlanetBriefList(PlanetListSort),
    PlanetDetailList(PlanetListSort),
    PlanetTaxPrompt,
    PlanetTaxDone,
    Starmap,
    PartialStarmapPrompt,
    PartialStarmapView,
    PlanetInfoPrompt,
    PlanetInfoDetail,
    Enemies,
    DeleteReviewables,
    ComposeMessageRecipient,
    ComposeMessageSubject,
    ComposeMessageBody,
    ComposeMessageOutbox,
    ComposeMessageDiscardConfirm,
    ComposeMessageSendConfirm,
    ComposeMessageSent,
    EmpireStatus,
    EmpireProfile,
    Rankings(RankingsView),
    Reports,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandMenu {
    General,
    Planet,
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
