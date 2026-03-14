use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::reports::ReportsPreview;
use crate::screen::{Screen, ScreenFrame};
use crate::terminal::Terminal;

pub struct ReportsScreen {
    preview: ReportsPreview,
}

impl ReportsScreen {
    pub fn new(preview: ReportsPreview) -> Self {
        Self { preview }
    }
}

impl Screen for ReportsScreen {
    fn render(
        &mut self,
        terminal: &mut dyn Terminal,
        frame: &ScreenFrame<'_>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        terminal.clear()?;
        terminal.write_line("MESSAGES / RESULTS REVIEW")?;
        terminal.write_line("=========================")?;
        terminal.write_line("")?;
        terminal.write_line(&format!(
            "Player {}  Empire: {}",
            frame.player.record_index_1_based,
            display_or_unknown(&frame.player.empire_name)
        ))?;
        terminal.write_line("")?;
        terminal.write_line("RESULTS.DAT")?;
        terminal.write_line("-----------")?;
        write_section(terminal, &self.preview.results_lines)?;
        terminal.write_line("")?;
        terminal.write_line("MESSAGES.DAT")?;
        terminal.write_line("------------")?;
        write_section(terminal, &self.preview.message_lines)?;
        terminal.write_line("")?;
        terminal.write_line("Q returns to the General Command menu.")?;
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
    lines: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if lines.is_empty() {
        terminal.write_line("  <none>")?;
        return Ok(());
    }

    for line in lines.iter().take(10) {
        terminal.write_line(&format!("  {line}"))?;
    }
    if lines.len() > 10 {
        terminal.write_line(&format!("  ... {} more line(s)", lines.len() - 10))?;
    }
    Ok(())
}

fn display_or_unknown(value: &str) -> &str {
    if value.is_empty() { "<unknown>" } else { value }
}
