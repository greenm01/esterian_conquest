use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::model::ReviewSummary;
use crate::reports::ReportsPreview;
use crate::screen::layout::write_prompt;
use crate::screen::{Screen, ScreenFrame};
use crate::terminal::Terminal;
use crate::theme::classic;

pub struct ReportsScreen {
    preview: ReportsPreview,
    summary: ReviewSummary,
}

impl ReportsScreen {
    pub fn new(preview: ReportsPreview, summary: ReviewSummary) -> Self {
        Self { preview, summary }
    }
}

impl Screen for ReportsScreen {
    fn render(
        &mut self,
        terminal: &mut dyn Terminal,
        frame: &ScreenFrame<'_>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut lines = 0;
        terminal.clear()?;
        terminal.write_line(&classic::title_bar("MESSAGES / RESULTS REVIEW: ", 78))?;
        lines += 1;
        terminal.write_line("")?;
        lines += 1;
        terminal.write_line(&classic::status_line(
            "Player: ",
            &format!(
                "{}  Empire: {}",
            frame.player.record_index_1_based,
            display_or_unknown(&frame.player.empire_name)
            ),
        ))?;
        lines += 1;
        terminal.write_line("")?;
        lines += 1;
        terminal.write_line("RESULTS.DAT")?;
        lines += 1;
        terminal.write_line("-----------")?;
        lines += 1;
        lines += write_section(
            terminal,
            self.summary.reviewable_results,
            &self.preview.results_lines,
        )?;
        terminal.write_line("")?;
        lines += 1;
        terminal.write_line("MESSAGES.DAT")?;
        lines += 1;
        terminal.write_line("------------")?;
        lines += 1;
        lines += write_section(
            terminal,
            self.summary.reviewable_messages,
            &self.preview.message_lines,
        )?;
        terminal.write_line("")?;
        lines += 1;
        write_prompt(
            terminal,
            lines,
            &classic::command_prompt("GENERAL COMMAND", "Q"),
        )?;
        terminal.flush()
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenGeneralMenu,
            _ => Action::Noop,
        }
    }
}

fn write_section(
    terminal: &mut dyn Terminal,
    reviewable: bool,
    lines: &[String],
) -> Result<usize, Box<dyn std::error::Error>> {
    if !reviewable {
        terminal.write_line("  <none currently reviewable>")?;
        return Ok(1);
    }

    if lines.is_empty() {
        terminal.write_line("  <none>")?;
        return Ok(1);
    }

    let mut written = 0;
    for line in lines.iter().take(10) {
        terminal.write_line(&format!("  {line}"))?;
        written += 1;
    }
    if lines.len() > 10 {
        terminal.write_line(&format!("  ... {} more line(s)", lines.len() - 10))?;
        written += 1;
    }
    Ok(written)
}

fn display_or_unknown(value: &str) -> &str {
    if value.is_empty() { "<unknown>" } else { value }
}
