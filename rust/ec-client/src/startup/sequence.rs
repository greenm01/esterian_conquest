use crate::reports::ReportsPreview;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupPhase {
    Splash,
    Intro,
    LoginSummary,
    Results,
    Messages,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupSummary {
    pub game_year: u16,
    pub pending_results: bool,
    pub pending_messages: bool,
    pub results_line_count: usize,
    pub message_line_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupSequence {
    current: StartupPhase,
    show_results: bool,
    show_messages: bool,
}

impl StartupSummary {
    pub fn from_reports(
        game_year: u16,
        pending_results: bool,
        pending_messages: bool,
        reports: &ReportsPreview,
    ) -> Self {
        Self {
            game_year,
            pending_results,
            pending_messages,
            results_line_count: reports.results_lines.len(),
            message_line_count: reports.message_lines.len(),
        }
    }
}

impl StartupSequence {
    pub fn new(summary: &StartupSummary) -> Self {
        Self {
            current: StartupPhase::Splash,
            show_results: summary.pending_results && !summary.results_line_count.eq(&0),
            show_messages: summary.pending_messages && !summary.message_line_count.eq(&0),
        }
    }

    pub fn current(&self) -> StartupPhase {
        self.current
    }

    pub fn advance(&mut self) -> StartupPhase {
        self.current = match self.current {
            StartupPhase::Splash => StartupPhase::LoginSummary,
            StartupPhase::Intro => StartupPhase::LoginSummary,
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

    pub fn open_intro(&mut self) -> StartupPhase {
        self.current = StartupPhase::Intro;
        self.current
    }
}
