use crate::model::ClassicLoginState;
use crate::reports::ReportsPreview;

// StartupPhase and StartupSequence now live in nc-session.
// Re-export for backward compatibility within nc-game.
use nc_session::startup::StartupSummary as SharedStartupSummary;
pub use nc_session::startup::{StartupPhase, StartupSequence};

/// nc-game's extended startup summary that includes game-specific login state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupSummary {
    pub game_year: u16,
    pub login_state: ClassicLoginState,
    pub pending_results: bool,
    pub pending_messages: bool,
    pub results_line_count: usize,
    pub message_line_count: usize,
}

impl StartupSummary {
    pub fn from_reports(
        game_year: u16,
        login_state: ClassicLoginState,
        pending_results: bool,
        pending_messages: bool,
        reports: &ReportsPreview,
    ) -> Self {
        Self {
            game_year,
            login_state,
            pending_results,
            pending_messages,
            results_line_count: reports.results_lines.len(),
            message_line_count: reports.message_lines.len(),
        }
    }

    /// Convert to the shared summary for use with StartupSequence.
    pub fn to_shared(&self) -> SharedStartupSummary {
        let show_login_review = matches!(
            self.login_state,
            ClassicLoginState::MatchedPreloadedFirstLogin | ClassicLoginState::ReturningPlayer
        );
        SharedStartupSummary {
            game_year: self.game_year,
            show_login_review,
            pending_results: self.pending_results,
            pending_messages: self.pending_messages,
            results_line_count: self.results_line_count,
            message_line_count: self.message_line_count,
        }
    }
}
