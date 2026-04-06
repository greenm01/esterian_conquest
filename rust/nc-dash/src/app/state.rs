//! Dashboard application state.

use std::path::PathBuf;

use nc_data::{CoreGameData, QueuedPlayerMail, ReportBlockRow};
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
    Reports,
}

impl PanelFocus {
    pub fn next(self) -> Self {
        match self {
            Self::Map => Self::Economy,
            Self::Economy => Self::Planets,
            Self::Planets => Self::Fleets,
            Self::Fleets => Self::Galaxy,
            Self::Galaxy => Self::Diplomacy,
            Self::Diplomacy => Self::Reports,
            Self::Reports => Self::Map,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Map => Self::Reports,
            Self::Economy => Self::Map,
            Self::Planets => Self::Economy,
            Self::Fleets => Self::Planets,
            Self::Galaxy => Self::Fleets,
            Self::Diplomacy => Self::Galaxy,
            Self::Reports => Self::Diplomacy,
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

/// Dashboard application state.
pub struct DashApp {
    pub game_dir: PathBuf,
    pub game_data: CoreGameData,
    pub report_block_rows: Vec<ReportBlockRow>,
    pub queued_mail: Vec<QueuedPlayerMail>,
    pub geometry: ScreenGeometry,
    pub player_record_index_1_based: usize,

    // Startup flow
    pub startup_sequence: StartupSequence,
    pub startup_phase: StartupPhase,

    // Dashboard navigation
    pub focus: PanelFocus,
    pub overlay: ActiveOverlay,
    pub autopilot_on: bool,

    // Starmap crosshair (1-based sector coords)
    pub crosshair_x: u8,
    pub crosshair_y: u8,

    // Panel scroll positions
    pub planets_scroll: usize,
    pub fleets_scroll: usize,
    pub reports_scroll: usize,
    pub diplomacy_scroll: usize,

    pub should_quit: bool,
}

impl DashApp {
    pub fn new(
        game_dir: PathBuf,
        game_data: CoreGameData,
        report_block_rows: Vec<ReportBlockRow>,
        queued_mail: Vec<QueuedPlayerMail>,
        geometry: ScreenGeometry,
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
            report_block_rows,
            queued_mail,
            geometry,
            player_record_index_1_based,
            startup_phase: StartupPhase::Complete,
            startup_sequence,
            focus: PanelFocus::Map,
            overlay: ActiveOverlay::None,
            autopilot_on: false,
            crosshair_x: 1,
            crosshair_y: 1,
            planets_scroll: 0,
            fleets_scroll: 0,
            reports_scroll: 0,
            diplomacy_scroll: 0,
            should_quit: false,
        }
    }
}
