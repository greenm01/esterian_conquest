pub mod buffer;
pub mod help;
pub mod layout;
pub mod table;
pub mod table_selection;

pub mod empire_profile {
    pub use crate::domains::empire::screens::empire_profile::*;
}
pub mod empire_status {
    pub use crate::domains::empire::screens::empire_status::*;
}
pub mod enemies {
    pub use crate::domains::empire::screens::enemies::*;
}
pub mod first_time {
    pub use crate::domains::startup::screens::first_time::*;
}
pub mod fleet {
    pub use crate::domains::fleet::screens::fleet::*;
}
pub mod general_menu {
    pub use crate::domains::startup::screens::general_menu::*;
}
pub mod main_menu {
    pub use crate::domains::startup::screens::main_menu::*;
}
pub mod message_compose {
    pub use crate::domains::messaging::screens::message_compose::*;
}
pub mod partial_starmap {
    pub use crate::domains::starmap::screens::partial_starmap::*;
}
pub mod planet_build {
    pub use crate::domains::planet::screens::planet_build::*;
}
pub mod planet_commission {
    pub use crate::domains::planet::screens::planet_commission::*;
}
pub mod planet_database {
    pub use crate::domains::planet::screens::planet_database::*;
}
pub mod planet_info {
    pub use crate::domains::planet::screens::planet_info::*;
}
pub mod planet_list {
    pub use crate::domains::planet::screens::planet_list::*;
}
pub mod planet_menu {
    pub use crate::domains::planet::screens::planet_menu::*;
}
pub mod planet_tax {
    pub use crate::domains::planet::screens::planet_tax::*;
}
pub mod planet_transport {
    pub use crate::domains::planet::screens::planet_transport::*;
}
pub mod rankings {
    pub use crate::domains::empire::screens::rankings::*;
}
pub mod reports {
    pub use crate::domains::startup::screens::reports::*;
}
pub mod starbase {
    pub use crate::domains::starbase::screens::starbase::*;
}
pub mod starmap {
    pub use crate::domains::starmap::screens::starmap::*;
}
pub mod startup {
    pub use crate::domains::startup::screens::startup::*;
}
pub mod theme_picker {
    pub use crate::domains::startup::screens::theme_picker::*;
}

pub use crate::domains::empire::screens::empire_profile::EmpireProfileScreen;
pub use crate::domains::empire::screens::empire_status::EmpireStatusScreen;
pub use crate::domains::empire::screens::enemies::EnemiesScreen;
pub use crate::domains::empire::screens::rankings::RankingsScreen;
pub use crate::domains::fleet::missions::{
    FLEET_MISSION_OPTIONS, FleetMissionOption, FleetMissionRequirement,
};
pub use crate::domains::fleet::screens::fleet::{
    FleetDetachClass, FleetDetachMode, FleetDetachScreen, FleetEtaMode, FleetEtaScreen,
    FleetGroupOrderMode, FleetGroupScreen, FleetListFilter, FleetListFilterPromptMode,
    FleetListScreen, FleetListSort, FleetMenuScreen, FleetMessageScreen,
    FleetMissionPickerScreen, FleetReviewScreen, FleetRow, FleetSingleOrderMode,
    FleetSingleOrderScreen, FleetTransferMode, FleetTransferScreen,
};
pub use crate::domains::messaging::screens::message_compose::MessageComposeScreen;
pub(crate) use crate::domains::messaging::screens::message_compose::{
    COMPOSE_BODY_LIMIT, COMPOSE_SUBJECT_LIMIT,
};
pub use crate::domains::planet::screens::planet_build::{
    PlanetBuildChangeRow, PlanetBuildListRow, PlanetBuildMenuView, PlanetBuildOrder,
    PlanetBuildScreen, build_order_summary,
};
pub use crate::domains::planet::screens::planet_commission::{
    PlanetCommissionDraftRow, PlanetCommissionPickerRow, PlanetCommissionRow,
    PlanetCommissionScreen, PlanetCommissionView,
};
pub use crate::domains::planet::screens::planet_database::{
    PlanetDatabaseFilter, PlanetDatabaseFilterMode, PlanetDatabasePromptMode, PlanetDatabaseRow,
    PlanetDatabaseScreen, PlanetDatabaseSort, PlanetDatabaseSortMode,
};
pub use crate::domains::planet::screens::planet_info::{PlanetInfoScreen, parse_planet_coords};
pub use crate::domains::planet::screens::planet_list::{
    PlanetListFilter, PlanetListFilterMode, PlanetListFilterPromptMode, PlanetListMode,
    PlanetListScreen, PlanetListSort,
};
pub use crate::domains::planet::screens::planet_menu::PlanetMenuScreen;
pub use crate::domains::planet::screens::planet_tax::PlanetTaxScreen;
pub use crate::domains::planet::screens::planet_transport::{
    PlanetTransportFleetRow, PlanetTransportMode, PlanetTransportPlanetRow, PlanetTransportScreen,
};
pub use crate::domains::starbase::screens::starbase::{
    StarbaseListScreen, StarbaseMenuScreen, StarbaseReviewScreen, StarbaseRow,
};
pub use crate::domains::starmap::screens::partial_starmap::PartialStarmapScreen;
pub use crate::domains::starmap::screens::starmap::StarmapScreen;
pub(crate) use crate::domains::startup::screens::first_time::FIRST_TIME_INTRO_PAGE_COUNT;
pub use crate::domains::startup::screens::first_time::{
    FirstTimeEmpiresScreen, FirstTimeIntroScreen, FirstTimeMenuScreen, render_colony_world_confirm,
    render_colony_world_name, render_first_time_homeworld_confirm,
    render_first_time_homeworld_name, render_first_time_join_name,
    render_first_time_join_name_confirm, render_first_time_join_no_pending,
    render_first_time_join_summary, render_first_time_reserved_prompt,
    render_preloaded_first_login_rename_prompt,
};
pub use crate::domains::startup::screens::general_menu::GeneralMenuScreen;
pub use crate::domains::startup::screens::main_menu::MainMenuScreen;
pub use crate::domains::startup::screens::reports::ReportsScreen;
pub(crate) use crate::domains::startup::screens::startup::STARTUP_INTRO_PAGE_COUNT;
pub(crate) use crate::domains::startup::screens::startup::STARTUP_SPLASH_PAGE_COUNT;
pub use crate::domains::startup::screens::startup::{StartupReviewMode, StartupScreen};
pub use crate::domains::startup::screens::theme_picker::ThemePickerScreen;
pub use buffer::{AnsiColor, Cell, CellStyle, GameColor, PlayfieldBuffer, StyledSpan};
pub use layout::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH, ScreenGeometry};
pub use nc_engine::{
    BUILD_UNITS, BuildUnitSpec, build_kind_count_label, build_kind_name,
    build_quantity_from_points, build_unit_spec, build_unit_spec_by_kind, max_quantity,
};
pub(crate) use table::format_fleet_number;

use std::collections::BTreeMap;
use std::path::Path;

use crossterm::event::KeyEvent;
use nc_data::{CoreGameData, PlanetIntelSnapshot};

use crate::app::Action;
use crate::model::PlayerContext;
use crate::startup::StartupPhase;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl SortDirection {
    pub const fn toggle(self) -> Self {
        match self {
            Self::Asc => Self::Desc,
            Self::Desc => Self::Asc,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenId {
    Startup(StartupPhase),
    FirstTimeMenu,
    FirstTimeEmpires,
    FirstTimeIntro,
    FirstTimeReservedPrompt,
    FirstTimePreloadedRenamePrompt,
    FirstTimeJoinEmpireName,
    FirstTimeJoinEmpireConfirm,
    FirstTimeJoinSummary,
    FirstTimeJoinNoPending,
    FirstTimeHomeworldName,
    FirstTimeHomeworldConfirm,
    ColonyWorldName,
    ColonyWorldConfirm,
    ThemePicker,
    MainMenu,
    GeneralMenu,
    StarbaseMenu,
    StarbaseList,
    StarbaseReviewSelect,
    StarbaseReview,
    FleetMenu,
    FleetList,
    FleetListFilterPrompt,
    FleetListSortPrompt,
    FleetReview,
    FleetOrder,
    FleetGroupOrder,
    FleetMissionPicker,
    FleetTransfer,
    FleetDetach,
    FleetEta,
    FleetMessage,
    PlanetMenu,
    PlanetBuildMenu,
    PlanetBuildList,
    PlanetBuildChange,
    PlanetBuildSpecify,
    PlanetBuildQuantity,
    PlanetCommissionPicker,
    PlanetCommissionMenu,
    PlanetCommissionDraft,
    PlanetCommissionResult,
    PlanetAutoCommissionReport,
    PlanetListFilterPrompt(PlanetListMode),
    PlanetListSortPrompt(PlanetListMode),
    PlanetList(PlanetListMode, PlanetListSort),
    PlanetTransportPlanetSelect(PlanetTransportMode),
    PlanetTransportFleetSelect(PlanetTransportMode),
    PlanetTransportQuantityPrompt(PlanetTransportMode),
    PlanetTransportDone(PlanetTransportMode),
    Starmap,
    PartialStarmapView,
    PlanetDatabaseList,
    PlanetDatabaseFilterPrompt,
    PlanetDatabaseSortPrompt,
    PlanetInfoDetail,
    Enemies,
    ComposeMessageRecipient,
    ComposeMessageSubject,
    ComposeMessageBody,
    ComposeMessageOutbox,
    ComposeMessageDiscardConfirm,
    ComposeMessageSendConfirm,
    ComposeMessageSent,
    EmpireStatus,
    EmpireProfile,
    Rankings(nc_data::EmpireProductionRankingSort),
    Reports,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandMenu {
    Main,
    General,
    Fleet,
    Starbase,
    Planet,
    PlanetBuild,
}

pub const COMMAND_LABEL: &str = "COMMAND";

pub fn command_menu_label(menu: CommandMenu) -> &'static str {
    match menu {
        CommandMenu::Main => "MAIN COMMAND",
        CommandMenu::General => "GENERAL COMMAND",
        CommandMenu::Fleet => "FLEET COMMAND",
        CommandMenu::Starbase => "STARBASE COMMAND",
        CommandMenu::Planet => "PLANET COMMAND",
        CommandMenu::PlanetBuild => "BUILD COMMAND",
    }
}

pub fn format_sector_coords(coords: [u8; 2]) -> String {
    format!("[{},{}]", coords[0], coords[1])
}

pub fn format_sector_coords_zero_padded(coords: [u8; 2]) -> String {
    format!("[{:02},{:02}]", coords[0], coords[1])
}

pub fn format_sector_coords_padded(coords: [u8; 2]) -> String {
    format!("[{:>2},{:>2}]", coords[0], coords[1])
}

pub fn format_sector_coords_table(coords: [u8; 2]) -> String {
    nc_ui::coords::format_sector_coords_table(coords)
}

pub fn format_sector_coords_default(coords: [u8; 2]) -> String {
    nc_ui::coords::format_sector_coords_default(coords)
}

pub struct ScreenFrame<'a> {
    pub game_dir: &'a Path,
    pub game_data: &'a CoreGameData,
    pub player: &'a PlayerContext,
    pub campaign_seed: u64,
    pub planet_intel_snapshots: &'a BTreeMap<usize, PlanetIntelSnapshot>,
    pub owned_planet_years: &'a BTreeMap<usize, u16>,
    pub geometry: ScreenGeometry,
}

pub trait Screen {
    fn render(
        &mut self,
        frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>>;

    fn handle_key(&self, key: KeyEvent) -> Action;
}
