//! Startup phase state machine shared between nc-game and nc-helm.
//!
//! `StartupPhase` is the canonical flow:
//!   Splash → Intro → LoginSummary → Results → Messages → Complete
//!
//! Each frontend drives the rendering of each phase independently.
//! nc-game renders at 80×25; nc-helm renders at fullscreen.

/// Phase in the startup intro flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupPhase {
    Splash,
    Intro,
    LoginSummary,
    Results,
    Messages,
    Complete,
}

/// Minimal summary of what the startup flow should show.
///
/// Frontends build this from their game state after loading the campaign.
/// Game-specific types (ClassicLoginState, ReportsPreview) are not referenced
/// here — callers project them into these booleans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupSummary {
    pub game_year: u16,
    /// Whether the player is returning (has login review, results, messages).
    pub show_login_review: bool,
    pub pending_results: bool,
    pub pending_messages: bool,
    pub results_line_count: usize,
    pub message_line_count: usize,
}

/// Drives the startup phase progression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupSequence {
    current: StartupPhase,
    show_login_review: bool,
    show_results: bool,
    show_messages: bool,
}

impl StartupSequence {
    pub fn new(summary: &StartupSummary) -> Self {
        Self {
            current: StartupPhase::Splash,
            show_login_review: summary.show_login_review,
            show_results: summary.show_login_review && summary.pending_results,
            show_messages: summary.show_login_review && summary.pending_messages,
        }
    }

    pub fn current(&self) -> StartupPhase {
        self.current
    }

    pub fn advance(&mut self) -> StartupPhase {
        self.current = match self.current {
            StartupPhase::Splash | StartupPhase::Intro => {
                if self.show_login_review {
                    StartupPhase::LoginSummary
                } else {
                    StartupPhase::Complete
                }
            }
            StartupPhase::LoginSummary => {
                if self.show_results {
                    StartupPhase::Results
                } else if self.show_messages {
                    StartupPhase::Messages
                } else {
                    StartupPhase::Complete
                }
            }
            StartupPhase::Results => {
                if self.show_messages {
                    StartupPhase::Messages
                } else {
                    StartupPhase::Complete
                }
            }
            StartupPhase::Messages | StartupPhase::Complete => StartupPhase::Complete,
        };
        self.current
    }

    pub fn skip_intro(&mut self) -> StartupPhase {
        self.current = if self.show_login_review {
            StartupPhase::LoginSummary
        } else {
            StartupPhase::Complete
        };
        self.current
    }
}
