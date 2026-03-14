use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::reports::ReportsPreview;
use crate::screen::layout::write_prompt;
use crate::screen::ScreenFrame;
use crate::startup::{StartupPhase, StartupSummary};
use crate::terminal::Terminal;
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
        terminal: &mut dyn Terminal,
        frame: &ScreenFrame<'_>,
        phase: StartupPhase,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match phase {
            StartupPhase::Splash => self.render_splash(terminal, frame),
            StartupPhase::Intro => self.render_intro(terminal),
            StartupPhase::LoginSummary => self.render_login_summary(terminal, frame),
            StartupPhase::Results => {
                self.render_report_lines(terminal, frame, "PENDING RESULTS", &self.reports.results_lines)
            }
            StartupPhase::Messages => {
                self.render_report_lines(terminal, frame, "PENDING MESSAGES", &self.reports.message_lines)
            }
            StartupPhase::Complete => Ok(()),
        }
    }

    pub fn handle_key(&self, phase: StartupPhase, key: KeyEvent) -> Action {
        match (phase, key.code) {
            (StartupPhase::Splash, KeyCode::Char('y') | KeyCode::Char('Y')) => Action::OpenStartupIntro,
            (_, KeyCode::Char('q') | KeyCode::Char('Q')) => Action::Quit,
            _ => Action::AdvanceStartup,
        }
    }

    fn render_splash(
        &self,
        terminal: &mut dyn Terminal,
        _frame: &ScreenFrame<'_>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut lines = 0;
        terminal.clear()?;
        terminal.write_line("")?;
        lines += 1;
        terminal.write_line(&classic::centered_text("ESTERIAN CONQUEST", 78))?;
        lines += 1;
        terminal.write_line(&classic::centered_text("Ver 1.60", 78))?;
        lines += 1;
        terminal.write_line("")?;
        lines += 1;
        terminal.write_line("Welcome back to Esterian Conquest, Ver 1.60")?;
        lines += 1;
        terminal.write_line("-------------------------------------------------------------------------------")?;
        lines += 1;
        terminal.write_line("Use Ctrl-S throughout the game to pause and resume output.")?;
        lines += 1;
        terminal.write_line("Use Ctrl-X or Ctrl-K to abort most listings.")?;
        lines += 1;
        terminal.write_line("Copyright (C) 1990, 1991 by Bentley C. Griffith.")?;
        lines += 1;
        terminal.write_line("")?;
        lines += 1;
        write_prompt(terminal, lines, "View Introduction? Y/[N] ->")?;
        terminal.flush()
    }

    fn render_intro(
        &self,
        terminal: &mut dyn Terminal,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut lines = 0;
        terminal.clear()?;
        terminal.write_line(&classic::centered_text("Esterian Conquest Ver 1.60", 78))?;
        lines += 1;
        terminal.write_line("")?;
        lines += 1;
        terminal.write_line("Beyond the mapped frontiers of the old Esterian dominion lies a small")?;
        lines += 1;
        terminal.write_line("galaxy of contested solar systems. The old masters are gone. Their")?;
        lines += 1;
        terminal.write_line("stations are silent, their patrols vanished, and their subjects left")?;
        lines += 1;
        terminal.write_line("with fleets, factories, and enough knowledge to build empires.")?;
        lines += 1;
        terminal.write_line("")?;
        lines += 1;
        terminal.write_line("You rise as one of the new Star Masters. From a single world and a few")?;
        lines += 1;
        terminal.write_line("small fleets, you must tax, build, scout, bargain, threaten, and")?;
        lines += 1;
        terminal.write_line("strike before rival powers can do the same. Some systems will join")?;
        lines += 1;
        terminal.write_line("your banner willingly. Others will require persuasion from orbit.")?;
        lines += 1;
        terminal.write_line("")?;
        lines += 1;
        terminal.write_line("Each maintenance marks the passage of a year. In that span, fleets")?;
        lines += 1;
        terminal.write_line("cross the dark between stars, colonies grow or starve, alliances turn")?;
        lines += 1;
        terminal.write_line("cold, and wars are decided by distance, industry, mathematics, and will.")?;
        lines += 1;
        terminal.write_line("")?;
        lines += 1;
        terminal.write_line("In profound respect and admiration to Bentley C. Griffith and his")?;
        lines += 1;
        terminal.write_line("fellow pioneers, who between 1990 and 1992 forged the enduring")?;
        lines += 1;
        terminal.write_line("legend of Esterian Conquest—a digital realm where star empires rose")?;
        lines += 1;
        terminal.write_line("and fell across BBS screens—and to the ancient dreamers, strategists,")?;
        lines += 1;
        terminal.write_line("and storytellers whose timeless visions of galactic dominion first")?;
        lines += 1;
        terminal.write_line("lit the way for every commander who still dares to claim worlds")?;
        lines += 1;
        terminal.write_line("among these endless stars.")?;
        lines += 1;
        terminal.write_line("")?;
        lines += 1;
        write_prompt(terminal, lines, "(Press Return) ")?;
        terminal.flush()
    }

    fn render_login_summary(
        &self,
        terminal: &mut dyn Terminal,
        frame: &ScreenFrame<'_>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut lines = 0;
        terminal.clear()?;
        terminal.write_line(&classic::title_bar("LOGIN STATUS: ", 78))?;
        lines += 1;
        terminal.write_line("")?;
        lines += 1;
        terminal.write_line(&classic::status_line(
            "Empire: ",
            &format!(
                "{}  Player {}",
            display_or_unknown(&frame.player.empire_name),
            frame.player.record_index_1_based
            ),
        ))?;
        lines += 1;
        terminal.write_line("")?;
        lines += 1;

        if self.summary.pending_results {
            terminal.write_line(&format!(
                "You have {} report line(s) pending.",
                self.summary.results_line_count
            ))?;
        } else {
            terminal.write_line("You have no reports pending.")?;
        }
        lines += 1;

        if self.summary.pending_messages {
            terminal.write_line(&format!(
                "You have undeleted messages: {} line(s) currently reviewable.",
                self.summary.message_line_count
            ))?;
        } else {
            terminal.write_line("You have no undeleted messages.")?;
        }
        lines += 1;

        terminal.write_line("")?;
        lines += 1;
        write_prompt(
            terminal,
            lines,
            "Press any key to continue to the login-time review flow.",
        )?;
        terminal.flush()
    }

    fn render_report_lines(
        &self,
        terminal: &mut dyn Terminal,
        frame: &ScreenFrame<'_>,
        title: &str,
        lines: &[String],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut lines_written = 0;
        terminal.clear()?;
        terminal.write_line(&classic::title_bar(&format!("{title}: "), 78))?;
        lines_written += 1;
        terminal.write_line("")?;
        lines_written += 1;
        terminal.write_line(&classic::status_line(
            "Empire: ",
            &format!(
                "{}  Player {}",
            display_or_unknown(&frame.player.empire_name),
            frame.player.record_index_1_based
            ),
        ))?;
        lines_written += 1;
        terminal.write_line("")?;
        lines_written += 1;

        for line in lines.iter().take(12) {
            terminal.write_line(line)?;
            lines_written += 1;
        }
        if lines.len() > 12 {
            terminal.write_line("")?;
            lines_written += 1;
            terminal.write_line(&format!("... {} more line(s)", lines.len() - 12))?;
            lines_written += 1;
        }

        terminal.write_line("")?;
        lines_written += 1;
        write_prompt(terminal, lines_written, "Press any key to continue.")?;
        terminal.flush()
    }
}

fn display_or_unknown(value: &str) -> &str {
    if value.is_empty() { "<unknown>" } else { value }
}
