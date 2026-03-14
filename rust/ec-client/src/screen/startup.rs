use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::reports::ReportsPreview;
use crate::screen::layout::{
    draw_centered_text, draw_plain_prompt, draw_status_line, draw_title_bar, new_playfield,
};
use crate::screen::{PlayfieldBuffer, ScreenFrame};
use crate::startup::{StartupPhase, StartupSummary};
use crate::theme::classic;

pub struct StartupScreen {
    summary: StartupSummary,
    reports: ReportsPreview,
}

impl StartupScreen {
    pub fn new(summary: StartupSummary, reports: ReportsPreview) -> Self {
        Self { summary, reports }
    }

    pub fn render_phase(
        &mut self,
        frame: &ScreenFrame<'_>,
        phase: StartupPhase,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        match phase {
            StartupPhase::Splash => self.render_splash(frame),
            StartupPhase::Intro => self.render_intro(),
            StartupPhase::LoginSummary => self.render_login_summary(frame),
            StartupPhase::Results => {
                self.render_report_lines(frame, "PENDING RESULTS", &self.reports.results_lines)
            }
            StartupPhase::Messages => {
                self.render_report_lines(frame, "PENDING MESSAGES", &self.reports.message_lines)
            }
            StartupPhase::Complete => Ok(new_playfield()),
        }
    }

    pub fn handle_key(&self, phase: StartupPhase, key: KeyEvent) -> Action {
        match (phase, key.code) {
            (StartupPhase::Splash, KeyCode::Char('y') | KeyCode::Char('Y')) => {
                Action::OpenStartupIntro
            }
            (_, KeyCode::Char('q') | KeyCode::Char('Q')) => Action::Quit,
            _ => Action::AdvanceStartup,
        }
    }

    fn render_splash(
        &self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_centered_text(&mut buffer, 1, "ESTERIAN CONQUEST", classic::bright_style());
        draw_centered_text(&mut buffer, 2, "Ver 1.60", classic::bright_style());
        buffer.write_text(
            4,
            0,
            "Welcome back to Esterian Conquest, Ver 1.60",
            classic::body_style(),
        );
        buffer.write_text(
            5,
            0,
            "-------------------------------------------------------------------------------",
            classic::body_style(),
        );
        buffer.write_text(
            6,
            0,
            "Use Ctrl-S throughout the game to pause and resume output.",
            classic::body_style(),
        );
        buffer.write_text(
            7,
            0,
            "Use Ctrl-X or Ctrl-K to abort most listings.",
            classic::body_style(),
        );
        buffer.write_text(
            8,
            0,
            "Copyright (C) 1990, 1991 by Bentley C. Griffith.",
            classic::body_style(),
        );
        draw_plain_prompt(&mut buffer, 10, "View Introduction? Y/[N] ->");
        Ok(buffer)
    }

    fn render_intro(&self) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_centered_text(
            &mut buffer,
            0,
            "Esterian Conquest Ver 1.60",
            classic::bright_style(),
        );
        for (row, line) in INTRO_LINES.iter().enumerate() {
            buffer.write_text(row + 2, 0, line, classic::body_style());
        }
        let last_row = (INTRO_LINES.len() + 1) as u16;
        let last_col = INTRO_LINES
            .last()
            .map(|line| line.chars().count() as u16)
            .unwrap_or(0);
        buffer.set_cursor(last_col, last_row);
        Ok(buffer)
    }

    fn render_login_summary(
        &self,
        frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "LOGIN STATUS: ");
        draw_status_line(
            &mut buffer,
            2,
            "Empire: ",
            &format!(
                "{}  Player {}",
                display_or_unknown(&frame.player.empire_name),
                frame.player.record_index_1_based
            ),
        );

        if self.summary.pending_results {
            buffer.write_text(
                4,
                0,
                &format!(
                    "You have {} report line(s) pending.",
                    self.summary.results_line_count
                ),
                classic::body_style(),
            );
        } else {
            buffer.write_text(4, 0, "You have no reports pending.", classic::body_style());
        }

        if self.summary.pending_messages {
            buffer.write_text(
                5,
                0,
                &format!(
                    "You have undeleted messages: {} line(s) currently reviewable.",
                    self.summary.message_line_count
                ),
                classic::body_style(),
            );
        } else {
            buffer.write_text(
                5,
                0,
                "You have no undeleted messages.",
                classic::body_style(),
            );
        }

        draw_plain_prompt(
            &mut buffer,
            7,
            "Press any key to continue to the login-time review flow.",
        );
        Ok(buffer)
    }

    fn render_report_lines(
        &self,
        frame: &ScreenFrame<'_>,
        title: &str,
        lines: &[String],
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        let mut row = 0;
        draw_title_bar(&mut buffer, row, &format!("{title}: "));
        row += 2;
        draw_status_line(
            &mut buffer,
            row,
            "Empire: ",
            &format!(
                "{}  Player {}",
                display_or_unknown(&frame.player.empire_name),
                frame.player.record_index_1_based
            ),
        );
        row += 2;

        for line in lines.iter().take(12) {
            buffer.write_text(row, 0, line, classic::body_style());
            row += 1;
        }
        if lines.len() > 12 {
            row += 1;
            buffer.write_text(
                row,
                0,
                &format!("... {} more line(s)", lines.len() - 12),
                classic::body_style(),
            );
            row += 1;
        }

        row += 1;
        draw_plain_prompt(&mut buffer, row, "Press any key to continue.");
        Ok(buffer)
    }
}

const INTRO_LINES: [&str; 16] = [
    "Beyond the mapped frontiers of the old Esterian dominion lies a small galaxy",
    "of contested solar systems. The old masters are gone. Their stations are",
    "silent, their patrols vanished, and their subjects left with fleets,",
    "factories, and enough knowledge to build empires.",
    "",
    "You rise as one of the new Star Masters. From a single world and a few small",
    "fleets, you must tax, build, scout, bargain, threaten, and strike before",
    "rival powers can do the same. Some systems will join your banner willingly.",
    "Others will require persuasion from orbit.",
    "",
    "In profound respect and admiration to Bentley C. Griffith and his fellow",
    "pioneers, who between 1990 and 1992 forged the enduring legend of Esterian",
    "Conquest-a digital realm where star empires rose and fell across BBS",
    "screens-and to the ancient dreamers, strategists, and storytellers whose",
    "timeless visions of galactic dominion first lit the way for every commander",
    "who still dares to claim worlds among these endless stars.",
];

fn display_or_unknown(value: &str) -> &str {
    if value.is_empty() {
        "<unknown>"
    } else {
        value
    }
}
