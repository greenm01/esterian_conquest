use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::model::MainMenuSummary;
use crate::screen::{Screen, ScreenFrame};
use crate::terminal::Terminal;

pub struct MainMenuScreen {
    summary: MainMenuSummary,
}

impl MainMenuScreen {
    pub fn new(summary: MainMenuSummary) -> Self {
        Self { summary }
    }
}

impl Screen for MainMenuScreen {
    fn render(
        &mut self,
        terminal: &mut dyn Terminal,
        frame: &ScreenFrame<'_>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        terminal.clear()?;
        terminal.write_line("ESTERIAN CONQUEST")?;
        terminal.write_line("=================")?;
        terminal.write_line("")?;
        terminal.write_line(&format!(
            "Player {}  Handle: {}  Empire: {}",
            frame.player.record_index_1_based,
            display_or_unknown(&frame.player.handle),
            display_or_unknown(&frame.player.empire_name)
        ))?;
        terminal.write_line(&format!(
            "Game year {}  Players {}  Directory {}",
            self.summary.game_year,
            self.summary.player_count,
            frame.game_dir.display()
        ))?;
        terminal.write_line("")?;
        terminal.write_line("MAIN MENU COMMANDS")?;
        terminal.write_line("------------------")?;
        terminal.write_line("  G  GENERAL COMMAND MENU")?;
        terminal.write_line("  P  PLANET COMMAND MENU")?;
        terminal.write_line("  F  FLEET COMMAND MENU")?;
        terminal.write_line("  B  Brief Empire Report")?;
        terminal.write_line("  D  Detailed Empire Report")?;
        terminal.write_line("  T  Total Planet Database")?;
        terminal.write_line("  A  ANSI color ON/OFF")?;
        terminal.write_line("  Q  Quit")?;
        terminal.write_line("")?;
        terminal.write_line("CURRENT SUMMARY")?;
        terminal.write_line("----------------")?;
        terminal.write_line(&format!("  Owned planets: {}", self.summary.owned_planets))?;
        terminal.write_line(&format!("  Owned fleets:  {}", self.summary.owned_fleets))?;
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
        terminal.write_line("  G opens the General Command menu.")?;
        terminal.write_line("  Q exits. Other commands are placeholders in this first pass.")?;
        terminal.flush()
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('g') | KeyCode::Char('G') => Action::OpenGeneralMenu,
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::Quit,
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
