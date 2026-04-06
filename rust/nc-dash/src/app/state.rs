//! Dashboard application state.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use nc_data::{CoreGameData, PlanetIntelSnapshot, QueuedPlayerMail, ReportBlockRow};
use nc_session::startup::{StartupPhase, StartupSequence, StartupSummary};
use nc_ui::ScreenGeometry;

/// Which panel has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelFocus {
    Map,
    Economy,
    Planets,
    Fleets,
    Galaxy,
    Diplomacy,
}

impl PanelFocus {
    pub fn next(self) -> Self {
        match self {
            Self::Map => Self::Economy,
            Self::Economy => Self::Planets,
            Self::Planets => Self::Fleets,
            Self::Fleets => Self::Galaxy,
            Self::Galaxy => Self::Diplomacy,
            Self::Diplomacy => Self::Map,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Map => Self::Diplomacy,
            Self::Economy => Self::Map,
            Self::Planets => Self::Economy,
            Self::Fleets => Self::Planets,
            Self::Galaxy => Self::Fleets,
            Self::Diplomacy => Self::Galaxy,
        }
    }
}

/// Which fullscreen overlay is open (if any).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveOverlay {
    None,
    PlanetList,
    FleetList,
    IntelDatabase,
    Inbox,
    Diplomacy,
    Settings,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePopup {
    None,
    PlanetDetail { planet_record_index_1_based: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpContext {
    Global,
    PlanetList,
    PlanetListSort,
    PlanetListFilter,
    PromptInput,
    FleetList,
    FleetListSort,
    FleetListFilter,
    IntelDatabase,
    IntelDatabaseSort,
    IntelDatabaseFilter,
    Inbox,
    Diplomacy,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboxFocus {
    List,
    Preview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboxFilter {
    All,
    Reports,
    Messages,
}

impl InboxFilter {
    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Reports => "Reports",
            Self::Messages => "Messages",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ListOverlayState {
    pub selected: usize,
    pub scroll: usize,
    pub jump_input: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetOverlaySort {
    CurrentProduction,
    Location,
    MaxProduction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetOverlayFilter {
    All,
    Range { anchor: [u8; 2], radius: u8 },
    Starbase,
    Stardock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetOverlayPromptMode {
    None,
    SortMenu,
    FilterMenu,
    FilterRangeCoords,
    FilterRangeDistance,
}

#[derive(Debug, Clone)]
pub struct PlanetOverlayState {
    pub selected: usize,
    pub scroll: usize,
    pub jump_input: String,
    pub sort: PlanetOverlaySort,
    pub filter: PlanetOverlayFilter,
    pub prompt_mode: PlanetOverlayPromptMode,
    pub prompt_input: String,
    pub prompt_default: String,
    pub pending_range_anchor: Option<[u8; 2]>,
}

impl Default for PlanetOverlayState {
    fn default() -> Self {
        Self {
            selected: 0,
            scroll: 0,
            jump_input: String::new(),
            sort: PlanetOverlaySort::CurrentProduction,
            filter: PlanetOverlayFilter::All,
            prompt_mode: PlanetOverlayPromptMode::None,
            prompt_input: String::new(),
            prompt_default: String::new(),
            pending_range_anchor: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetOverlaySort {
    Id,
    Location,
    Order,
    Eta,
    Strength,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetOverlayFilter {
    All,
    Holding,
    Moving,
    Combat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetOverlayPromptMode {
    None,
    SortMenu,
    FilterMenu,
}

#[derive(Debug, Clone)]
pub struct FleetOverlayState {
    pub selected: usize,
    pub scroll: usize,
    pub jump_input: String,
    pub sort: FleetOverlaySort,
    pub filter: FleetOverlayFilter,
    pub prompt_mode: FleetOverlayPromptMode,
}

impl Default for FleetOverlayState {
    fn default() -> Self {
        Self {
            selected: 0,
            scroll: 0,
            jump_input: String::new(),
            sort: FleetOverlaySort::Id,
            filter: FleetOverlayFilter::All,
            prompt_mode: FleetOverlayPromptMode::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntelOverlaySort {
    Location,
    Range([u8; 2]),
    Empire,
    MaxProduction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntelOverlayFilter {
    All,
    Range { anchor: [u8; 2], radius: u8 },
    Empire(u8),
    MaxProduction(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntelOverlayPromptMode {
    None,
    SortMenu,
    SortRangeInput,
    FilterMenu,
    FilterRangeCoords,
    FilterRangeDistance,
    FilterEmpireInput,
    FilterMaxProductionInput,
}

#[derive(Debug, Clone)]
pub struct IntelOverlayState {
    pub selected: usize,
    pub scroll: usize,
    pub jump_input: String,
    pub sort: IntelOverlaySort,
    pub filter: IntelOverlayFilter,
    pub prompt_mode: IntelOverlayPromptMode,
    pub prompt_input: String,
    pub prompt_default: String,
    pub pending_range_anchor: Option<[u8; 2]>,
}

impl Default for IntelOverlayState {
    fn default() -> Self {
        Self {
            selected: 0,
            scroll: 0,
            jump_input: String::new(),
            sort: IntelOverlaySort::Location,
            filter: IntelOverlayFilter::All,
            prompt_mode: IntelOverlayPromptMode::None,
            prompt_input: String::new(),
            prompt_default: String::new(),
            pending_range_anchor: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InboxOverlayState {
    pub selected: usize,
    pub scroll: usize,
    pub preview_scroll: usize,
    pub focus: InboxFocus,
    pub filter: InboxFilter,
    pub current_year_only: bool,
    pub delete_confirm: bool,
    pub jump_input: String,
}

impl Default for InboxOverlayState {
    fn default() -> Self {
        Self {
            selected: 0,
            scroll: 0,
            preview_scroll: 0,
            focus: InboxFocus::List,
            filter: InboxFilter::All,
            current_year_only: false,
            delete_confirm: false,
            jump_input: String::new(),
        }
    }
}

/// Dashboard application state.
pub struct DashApp {
    pub game_dir: PathBuf,
    pub game_data: CoreGameData,
    pub owned_planet_years: BTreeMap<usize, u16>,
    pub planet_scorch_orders: BTreeSet<usize>,
    pub report_block_rows: Vec<ReportBlockRow>,
    pub queued_mail: Vec<QueuedPlayerMail>,
    pub planet_intel_snapshots: Vec<PlanetIntelSnapshot>,
    /// Full terminal dimensions (canvas).
    pub geometry: ScreenGeometry,
    /// Dashboard frame dimensions (map + panels + borders).
    pub frame: ScreenGeometry,
    pub player_record_index_1_based: usize,

    // Startup flow
    pub startup_sequence: StartupSequence,
    pub startup_phase: StartupPhase,

    // Dashboard navigation
    pub focus: PanelFocus,
    pub overlay: ActiveOverlay,
    pub popup: ActivePopup,
    pub help_return_overlay: ActiveOverlay,
    pub help_context: HelpContext,
    pub autopilot_on: bool,

    // Starmap crosshair (1-based sector coords)
    pub crosshair_x: u8,
    pub crosshair_y: u8,
    pub map_coord_input: String,

    // Panel scroll positions
    pub diplomacy_scroll: usize,

    // Overlay-local state
    pub planet_overlay: PlanetOverlayState,
    pub fleet_overlay: FleetOverlayState,
    pub intel_overlay: IntelOverlayState,
    pub diplomacy_overlay: ListOverlayState,
    pub inbox_overlay: InboxOverlayState,

    pub should_quit: bool,
}

impl DashApp {
    pub fn new(
        game_dir: PathBuf,
        game_data: CoreGameData,
        owned_planet_years: BTreeMap<usize, u16>,
        planet_scorch_orders: BTreeSet<usize>,
        report_block_rows: Vec<ReportBlockRow>,
        queued_mail: Vec<QueuedPlayerMail>,
        planet_intel_snapshots: Vec<PlanetIntelSnapshot>,
        geometry: ScreenGeometry,
        frame: ScreenGeometry,
        player_record_index_1_based: usize,
    ) -> Self {
        let startup_summary = StartupSummary {
            game_year: game_data.conquest.game_year(),
            show_login_review: false,
            pending_results: false,
            pending_messages: false,
            results_line_count: 0,
            message_line_count: 0,
        };
        let startup_sequence = StartupSequence::new(&startup_summary);
        Self {
            game_dir,
            game_data,
            owned_planet_years,
            planet_scorch_orders,
            report_block_rows,
            queued_mail,
            planet_intel_snapshots,
            geometry,
            frame,
            player_record_index_1_based,
            startup_phase: StartupPhase::Complete,
            startup_sequence,
            focus: PanelFocus::Map,
            overlay: ActiveOverlay::None,
            popup: ActivePopup::None,
            help_return_overlay: ActiveOverlay::None,
            help_context: HelpContext::Global,
            autopilot_on: false,
            crosshair_x: 1,
            crosshair_y: 1,
            map_coord_input: String::new(),
            diplomacy_scroll: 0,
            planet_overlay: PlanetOverlayState::default(),
            fleet_overlay: FleetOverlayState::default(),
            intel_overlay: IntelOverlayState::default(),
            diplomacy_overlay: ListOverlayState::default(),
            inbox_overlay: InboxOverlayState::default(),
            should_quit: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PanelFocus;

    #[test]
    fn focus_cycle_skips_removed_reports_panel() {
        assert_eq!(PanelFocus::Diplomacy.next(), PanelFocus::Map);
        assert_eq!(PanelFocus::Map.prev(), PanelFocus::Diplomacy);
    }
}
