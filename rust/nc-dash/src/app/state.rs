//! Dashboard application state.

use nc_data::{
    CampaignStore, CoreGameData, PlanetIntelSnapshot, PlayerActivityState, ProductionItemKind,
    QueuedPlayerMail, ReportBlockRow,
};
use nc_session::startup::{StartupPhase, StartupSequence, StartupSummary};
use nc_ui::ScreenGeometry;
use std::collections::{BTreeMap, BTreeSet};

/// Which panel has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelFocus {
    Map,
    Economy,
    Planets,
    Fleets,
    WarRecord,
    Galaxy,
    Diplomacy,
}

impl PanelFocus {
    pub fn next(self) -> Self {
        match self {
            Self::Map => Self::Economy,
            Self::Economy => Self::Planets,
            Self::Planets => Self::Fleets,
            Self::Fleets => Self::WarRecord,
            Self::WarRecord => Self::Galaxy,
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
            Self::WarRecord => Self::Fleets,
            Self::Galaxy => Self::WarRecord,
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
pub enum MapViewMode {
    Readable,
    Fill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpContext {
    Global,
    PlanetList,
    PlanetListSort,
    PlanetListFilter,
    PlanetBuildSpecify,
    PlanetBuildQuantity,
    PromptInput,
    FleetList,
    FleetListSort,
    FleetListFilter,
    FleetMissionPicker,
    FleetOrderInput,
    StarbaseMove,
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
    BuildSpecify,
    BuildQuantity,
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
    pub build_planet_record_index_1_based: Option<usize>,
    pub build_unit_input: String,
    pub build_unit_status: Option<String>,
    pub build_selected_kind: Option<ProductionItemKind>,
    pub build_quantity_input: String,
    pub build_quantity_status: Option<String>,
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
            build_planet_record_index_1_based: None,
            build_unit_input: String::new(),
            build_unit_status: None,
            build_selected_kind: None,
            build_quantity_input: String::new(),
            build_quantity_status: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetOverlayRowKey {
    Fleet(usize),
    Starbase(usize),
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
    MissionPicker,
    OrderTarget,
    OrderTargetX,
    OrderTargetY,
    OrderConfirm,
    StarbaseMoveDecision,
    StarbaseMoveDestination,
    StarbaseHaltConfirm,
}

#[derive(Debug, Clone)]
pub struct FleetOverlayState {
    pub selected: usize,
    pub scroll: usize,
    pub jump_input: String,
    pub sort: FleetOverlaySort,
    pub filter: FleetOverlayFilter,
    pub prompt_mode: FleetOverlayPromptMode,
    pub active_row_key: Option<FleetOverlayRowKey>,
    pub mission_picker_input: String,
    pub mission_picker_cursor: usize,
    pub mission_picker_status: Option<String>,
    pub order_mission_code: Option<u8>,
    pub order_status: Option<String>,
    pub order_input: String,
    pub order_target_x_input: String,
    pub order_target_y_input: String,
    pub order_confirm_input: String,
    pub starbase_move_input: String,
    pub starbase_move_status: Option<String>,
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
            active_row_key: None,
            mission_picker_input: String::new(),
            mission_picker_cursor: 0,
            mission_picker_status: None,
            order_mission_code: None,
            order_status: None,
            order_input: String::new(),
            order_target_x_input: String::new(),
            order_target_y_input: String::new(),
            order_confirm_input: String::new(),
            starbase_move_input: String::new(),
            starbase_move_status: None,
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
    pub _game_dir: std::path::PathBuf,
    pub campaign_store: Option<CampaignStore>,
    pub game_data: CoreGameData,
    pub owned_planet_years: BTreeMap<usize, u16>,
    pub planet_scorch_orders: BTreeSet<usize>,
    pub report_block_rows: Vec<ReportBlockRow>,
    pub queued_mail: Vec<QueuedPlayerMail>,
    pub planet_intel_snapshots: Vec<PlanetIntelSnapshot>,
    pub player_activity_states: Vec<PlayerActivityState>,
    pub player_war_stats: nc_data::PlayerWarStatsState,
    /// Full terminal dimensions (canvas).
    pub geometry: ScreenGeometry,
    /// Dashboard frame dimensions (map + panels + borders).
    pub frame: ScreenGeometry,
    pub player_record_index_1_based: usize,

    // Startup flow
    pub _startup_sequence: StartupSequence,
    pub _startup_phase: StartupPhase,

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
    pub map_view_mode: MapViewMode,
    pub map_zoom_level: u8,
    pub map_coord_input: String,

    // Panel scroll positions
    pub diplomacy_scroll: usize,

    // Overlay-local state
    pub planet_overlay: PlanetOverlayState,
    pub fleet_overlay: FleetOverlayState,
    pub intel_overlay: IntelOverlayState,
    pub diplomacy_overlay: ListOverlayState,
    pub inbox_overlay: InboxOverlayState,

    pub is_terminal_too_small: bool,
    pub should_quit: bool,
}

impl DashApp {
    pub fn new(
        game_dir: std::path::PathBuf,
        campaign_store: Option<CampaignStore>,
        game_data: CoreGameData,
        owned_planet_years: BTreeMap<usize, u16>,
        planet_scorch_orders: BTreeSet<usize>,
        report_block_rows: Vec<ReportBlockRow>,
        queued_mail: Vec<QueuedPlayerMail>,
        planet_intel_snapshots: Vec<PlanetIntelSnapshot>,
        player_activity_states: Vec<PlayerActivityState>,
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
        let [crosshair_x, crosshair_y] =
            initial_crosshair_coords(&game_data, player_record_index_1_based);
        Self {
            _game_dir: game_dir,
            campaign_store,
            game_data,
            owned_planet_years,
            planet_scorch_orders,
            report_block_rows,
            queued_mail,
            planet_intel_snapshots,
            player_activity_states,
            player_war_stats: nc_data::PlayerWarStatsState::for_player(player_record_index_1_based),
            geometry,
            frame,
            player_record_index_1_based,
            _startup_phase: StartupPhase::Complete,
            _startup_sequence: startup_sequence,
            focus: PanelFocus::Map,
            overlay: ActiveOverlay::None,
            popup: ActivePopup::None,
            help_return_overlay: ActiveOverlay::None,
            help_context: HelpContext::Global,
            autopilot_on: false,
            crosshair_x,
            crosshair_y,
            map_view_mode: MapViewMode::Readable,
            map_zoom_level: 0,
            map_coord_input: String::new(),
            diplomacy_scroll: 0,
            planet_overlay: PlanetOverlayState::default(),
            fleet_overlay: FleetOverlayState::default(),
            intel_overlay: IntelOverlayState::default(),
            diplomacy_overlay: ListOverlayState::default(),
            inbox_overlay: InboxOverlayState::default(),
            is_terminal_too_small: false,
            should_quit: false,
        }
    }

    #[cfg(test)]
    pub fn new_for_tests(
        game_dir: std::path::PathBuf,
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
        Self::new(
            game_dir,
            None,
            game_data,
            owned_planet_years,
            planet_scorch_orders,
            report_block_rows,
            queued_mail,
            planet_intel_snapshots,
            Vec::new(),
            geometry,
            frame,
            player_record_index_1_based,
        )
    }
}

fn initial_crosshair_coords(
    game_data: &CoreGameData,
    player_record_index_1_based: usize,
) -> [u8; 2] {
    if let Some(coords) = game_data
        .player_homeworld_seed_coords_current_known()
        .get(player_record_index_1_based.saturating_sub(1))
        .and_then(|coords| *coords)
    {
        return coords;
    }
    let Some(player) = game_data
        .player
        .records
        .get(player_record_index_1_based.saturating_sub(1))
    else {
        return [1, 1];
    };
    let homeworld_index = player.homeworld_planet_index_1_based_raw() as usize;
    game_data
        .planets
        .records
        .get(homeworld_index.saturating_sub(1))
        .map(|planet| planet.coords_raw())
        .unwrap_or([1, 1])
}

#[cfg(test)]
mod tests {
    use super::{DashApp, PanelFocus};
    use nc_data::GameStateBuilder;
    use nc_ui::ScreenGeometry;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    #[test]
    fn focus_cycle_skips_removed_reports_panel() {
        assert_eq!(PanelFocus::Diplomacy.next(), PanelFocus::Map);
        assert_eq!(PanelFocus::Map.prev(), PanelFocus::Diplomacy);
        assert_eq!(PanelFocus::Fleets.next(), PanelFocus::WarRecord);
        assert_eq!(PanelFocus::Galaxy.prev(), PanelFocus::WarRecord);
    }

    #[test]
    fn new_app_starts_crosshair_on_player_homeworld() {
        let app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        let coords = app.game_data.player_homeworld_seed_coords_current_known()[0]
            .expect("player one homeworld seed");

        assert_eq!([app.crosshair_x, app.crosshair_y], coords);
    }
}
