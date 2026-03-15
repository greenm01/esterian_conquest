mod build_help;
mod buffer;
mod delete_reviewables;
mod empire_profile;
mod empire_status;
mod enemies;
mod fleet;
mod fleet_help;
mod first_time;
mod general_help;
mod general_menu;
mod layout;
mod main_menu;
mod message_compose;
mod partial_starmap;
mod planet_database;
mod planet_auto_commission;
mod planet_build;
mod planet_commission;
mod planet_help;
mod planet_info;
mod planet_list;
mod planet_menu;
mod planet_tax;
mod planet_transport;
mod rankings;
mod reports;
mod starmap;
mod startup;
mod table;

pub use buffer::{Cell, CellStyle, PlayfieldBuffer, RgbColor, StyledSpan};
pub use build_help::BuildHelpScreen;
pub use delete_reviewables::DeleteReviewablesScreen;
pub use empire_profile::EmpireProfileScreen;
pub use empire_status::EmpireStatusScreen;
pub use enemies::EnemiesScreen;
pub use fleet::{
    FleetDetachMode, FleetDetachScreen, FleetEtaMode, FleetEtaScreen, FleetListMode,
    FleetListScreen, FleetMenuScreen, FleetReviewScreen, FleetRoeScreen, FleetRow,
};
pub use first_time::{
    render_first_time_homeworld_confirm, render_first_time_homeworld_name,
    render_first_time_join_confirm, render_first_time_join_name,
    render_first_time_join_name_confirm, render_first_time_join_no_pending,
    render_first_time_join_summary, FirstTimeEmpiresScreen, FirstTimeHelpScreen,
    FirstTimeIntroScreen, FirstTimeMenuScreen,
};
pub(crate) use first_time::FIRST_TIME_INTRO_PAGE_COUNT;
pub use fleet_help::FleetHelpScreen;
pub(crate) use enemies::ENEMIES_VISIBLE_ROWS;
pub(crate) use fleet::FLEET_VISIBLE_ROWS;
pub(crate) use table::format_fleet_number;
pub use general_help::GeneralHelpScreen;
pub use general_menu::GeneralMenuScreen;
pub use layout::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};
pub use main_menu::MainMenuScreen;
pub use message_compose::MessageComposeScreen;
pub(crate) use message_compose::{
    COMPOSE_BODY_LIMIT, COMPOSE_SUBJECT_LIMIT, OUTBOX_VISIBLE_ROWS, RECIPIENT_VISIBLE_ROWS,
};
pub use partial_starmap::PartialStarmapScreen;
pub use planet_database::{PlanetDatabaseRow, PlanetDatabaseScreen};
pub use planet_auto_commission::PlanetAutoCommissionScreen;
pub use planet_build::{
    build_kind_name, build_order_summary, build_unit_spec, build_unit_spec_by_kind, infer_quantity,
    max_quantity, BuildUnitSpec, PlanetBuildChangeRow, PlanetBuildListRow, PlanetBuildMenuView,
    PlanetBuildOrder, PlanetBuildScreen,
};
pub use planet_commission::{PlanetCommissionRow, PlanetCommissionScreen, PlanetCommissionView};
pub(crate) use planet_build::{PLANET_BUILD_CHANGE_VISIBLE_ROWS, PLANET_BUILD_LIST_VISIBLE_ROWS};
pub(crate) use planet_commission::PLANET_COMMISSION_VISIBLE_ROWS;
pub(crate) use planet_database::PLANET_DATABASE_VISIBLE_ROWS;
pub(crate) use planet_transport::PLANET_TRANSPORT_VISIBLE_ROWS;
pub use planet_help::PlanetHelpScreen;
pub use planet_info::{parse_planet_coords, PlanetInfoScreen};
pub(crate) use planet_list::PLANET_BRIEF_VISIBLE_ROWS;
pub use planet_list::{PlanetListMode, PlanetListScreen, PlanetListSort};
pub use planet_menu::PlanetMenuScreen;
pub use planet_tax::PlanetTaxScreen;
pub use planet_transport::{PlanetTransportFleetRow, PlanetTransportMode, PlanetTransportPlanetRow, PlanetTransportScreen};
pub use rankings::RankingsScreen;
pub use reports::ReportsScreen;
pub use starmap::StarmapScreen;
pub use startup::StartupScreen;
pub(crate) use startup::STARTUP_INTRO_PAGE_COUNT;

use std::path::Path;

use crossterm::event::KeyEvent;
use ec_data::CoreGameData;

use crate::app::Action;
use crate::model::PlayerContext;
use crate::startup::StartupPhase;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenId {
    Startup(StartupPhase),
    FirstTimeMenu,
    FirstTimeHelp,
    FirstTimeEmpires,
    FirstTimeIntro,
    FirstTimeJoinConfirm,
    FirstTimeJoinEmpireName,
    FirstTimeJoinEmpireConfirm,
    FirstTimeJoinSummary,
    FirstTimeJoinNoPending,
    FirstTimeHomeworldName,
    FirstTimeHomeworldConfirm,
    MainMenu,
    GeneralMenu,
    GeneralHelp,
    FleetHelp,
    FleetMenu,
    FleetList(FleetListMode),
    FleetReview,
    FleetRoeSelect,
    FleetDetach,
    FleetEta,
    PlanetMenu,
    PlanetHelp,
    PlanetAutoCommissionConfirm,
    PlanetAutoCommissionDone,
    PlanetBuildMenu,
    PlanetBuildHelp,
    PlanetBuildReview,
    PlanetBuildList,
    PlanetBuildChange,
    PlanetBuildAbortConfirm,
    PlanetBuildSpecify,
    PlanetBuildQuantity,
    PlanetCommissionMenu,
    PlanetListSortPrompt(PlanetListMode),
    PlanetBriefList(PlanetListSort),
    PlanetDetailList(PlanetListSort),
    PlanetTaxPrompt,
    PlanetTaxDone,
    PlanetTransportPlanetSelect(PlanetTransportMode),
    PlanetTransportFleetSelect(PlanetTransportMode),
    PlanetTransportQuantityPrompt(PlanetTransportMode),
    PlanetTransportDone(PlanetTransportMode),
    Starmap,
    PartialStarmapPrompt,
    PartialStarmapView,
    PlanetDatabaseList,
    PlanetDatabaseDetail,
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
    Rankings(ec_data::EmpireProductionRankingSort),
    Reports,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandMenu {
    Main,
    General,
    Fleet,
    Planet,
    PlanetBuild,
}

pub fn command_menu_label(menu: CommandMenu) -> &'static str {
    match menu {
        CommandMenu::Main => "MAIN COMMAND",
        CommandMenu::General => "GENERAL COMMAND",
        CommandMenu::Fleet => "FLEET COMMAND",
        CommandMenu::Planet => "PLANET COMMAND",
        CommandMenu::PlanetBuild => "BUILD COMMAND",
    }
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
