mod buffer;
mod delete_reviewables;
mod empire_profile;
mod empire_status;
mod enemies;
mod general_help;
mod general_menu;
mod layout;
mod main_menu;
mod message_compose;
mod partial_starmap;
mod planet_build;
mod planet_help;
mod planet_info;
mod planet_list;
mod planet_menu;
mod planet_tax;
mod rankings;
mod reports;
mod starmap;
mod startup;
mod table;

pub use buffer::{Cell, CellStyle, PlayfieldBuffer, RgbColor, StyledSpan};
pub use delete_reviewables::DeleteReviewablesScreen;
pub use empire_profile::EmpireProfileScreen;
pub use empire_status::EmpireStatusScreen;
pub use enemies::EnemiesScreen;
pub(crate) use enemies::ENEMIES_VISIBLE_ROWS;
pub use general_help::GeneralHelpScreen;
pub use general_menu::GeneralMenuScreen;
pub use layout::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};
pub use main_menu::MainMenuScreen;
pub use message_compose::MessageComposeScreen;
pub(crate) use message_compose::{
    COMPOSE_BODY_LIMIT, COMPOSE_SUBJECT_LIMIT, OUTBOX_VISIBLE_ROWS, RECIPIENT_VISIBLE_ROWS,
};
pub use partial_starmap::PartialStarmapScreen;
pub use planet_build::{
    build_kind_name, build_order_summary, build_unit_spec, build_unit_spec_by_kind, infer_quantity,
    max_quantity, BuildUnitSpec, PlanetBuildChangeRow, PlanetBuildListRow, PlanetBuildMenuView,
    PlanetBuildOrder, PlanetBuildScreen,
};
pub(crate) use planet_build::{PLANET_BUILD_CHANGE_VISIBLE_ROWS, PLANET_BUILD_LIST_VISIBLE_ROWS};
pub use planet_help::PlanetHelpScreen;
pub use planet_info::{parse_planet_coords, PlanetInfoScreen};
pub(crate) use planet_list::PLANET_BRIEF_VISIBLE_ROWS;
pub use planet_list::{PlanetListMode, PlanetListScreen, PlanetListSort};
pub use planet_menu::PlanetMenuScreen;
pub use planet_tax::PlanetTaxScreen;
pub use rankings::{RankingsScreen, RankingsView};
pub use reports::ReportsScreen;
pub use starmap::StarmapScreen;
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
    GeneralHelp,
    PlanetMenu,
    PlanetHelp,
    PlanetBuildMenu,
    PlanetBuildReview,
    PlanetBuildList,
    PlanetBuildChange,
    PlanetBuildAbortConfirm,
    PlanetBuildSpecify,
    PlanetBuildQuantity,
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
    PlanetBuild,
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
