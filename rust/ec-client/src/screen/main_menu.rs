use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::screen::layout::write_prompt;
use crate::screen::{Screen, ScreenFrame};
use crate::terminal::Terminal;
use crate::theme::classic::{self, MenuEntry};

pub struct MainMenuScreen;

impl MainMenuScreen {
    pub fn new() -> Self {
        Self
    }
}

impl Screen for MainMenuScreen {
    fn render(
        &mut self,
        terminal: &mut dyn Terminal,
        _frame: &ScreenFrame<'_>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        terminal.clear()?;
        terminal.write_line(&classic::title_bar("MAIN MENU: ", 78))?;
        terminal.write_line(&classic::menu_row(&[
            MenuEntry::new("H", "elp with commands", 22),
            MenuEntry::new("G", "ENERAL COMMAND MENU...", 27),
            MenuEntry::new("B", "rief Empire Report", 23),
        ]))?;
        terminal.write_line(&classic::menu_row(&[
            MenuEntry::new("Q", "uit back to BBS", 22),
            MenuEntry::new("P", "LANET COMMAND MENU...", 27),
            MenuEntry::new("I", "nfo about a Planet", 23),
        ]))?;
        terminal.write_line(&classic::menu_row(&[
            MenuEntry::new("X", "pert mode ON/OFF", 22),
            MenuEntry::new("F", "LEET COMMAND MENU...", 27),
            MenuEntry::new("D", "etailed Empire Report", 23),
        ]))?;
        terminal.write_line(&classic::menu_row(&[
            MenuEntry::new("V", "iew Partial Map", 22),
            MenuEntry::new("T", "otal Planet Database", 27),
            MenuEntry::new("", "", 23),
        ]))?;
        terminal.write_line("")?;
        write_prompt(
            terminal,
            6,
            &classic::command_prompt("MAIN COMMAND", "H Q X V G P F T I B D"),
        )?;
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
