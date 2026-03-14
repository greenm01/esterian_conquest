use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::model::GeneralMenuSummary;
use crate::screen::{Screen, ScreenFrame};
use crate::terminal::Terminal;

pub struct GeneralMenuScreen {
    summary: GeneralMenuSummary,
}

impl GeneralMenuScreen {
    pub fn new(summary: GeneralMenuSummary) -> Self {
        Self { summary }
    }
}

impl Screen for GeneralMenuScreen {
    fn render(
        &mut self,
        terminal: &mut dyn Terminal,
        frame: &ScreenFrame<'_>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        terminal.clear()?;
        terminal.write_line("GENERAL COMMAND CENTER")?;
        terminal.write_line("======================")?;
        terminal.write_line("")?;
        terminal.write_line(&format!(
            "Player {}  Empire: {}",
            frame.player.record_index_1_based,
            display_or_unknown(&frame.player.empire_name)
        ))?;
        terminal.write_line(&format!("Directory {}", frame.game_dir.display()))?;
        terminal.write_line("")?;
        terminal.write_line("COMMANDS")?;
        terminal.write_line("--------")?;
        terminal.write_line("  A  Autopilot ON/OFF")?;
        terminal.write_line("  S  Status, your")?;
        terminal.write_line("  P  Profile of your empire")?;
        terminal.write_line("  M  Map of the galaxy")?;
        terminal.write_line("  C  Communicate (send message)")?;
        terminal.write_line("  O  Other empires (rankings)")?;
        terminal.write_line("  E  Enemies, declare or list")?;
        terminal.write_line("  R  Review messages/results")?;
        terminal.write_line("  D  Delete ALL messages/results")?;
        terminal.write_line("  Q  Back to Main Menu")?;
        terminal.write_line("")?;
        terminal.write_line("CURRENT REVIEW STATE")?;
        terminal.write_line("--------------------")?;
        terminal.write_line(&format!(
            "  Pending messages: {}",
            yes_no(self.summary.pending_messages)
        ))?;
        terminal.write_line(&format!(
            "  Pending results:  {}",
            yes_no(self.summary.pending_results)
        ))?;
        terminal.write_line("")?;
        terminal.write_line("STATUS")?;
        terminal.write_line("------")?;
        terminal.write_line("  R opens the first reports/messages preview.")?;
        terminal.write_line("  Other commands are still placeholders in this pass.")?;
        terminal.flush()
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('r') | KeyCode::Char('R') => Action::OpenReports,
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenMainMenu,
            _ => Action::Noop,
        }
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn display_or_unknown(value: &str) -> &str {
    if value.is_empty() { "<unknown>" } else { value }
}
