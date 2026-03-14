use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::screen::layout::{
    MenuEntry, draw_command_prompt, draw_menu_entry, draw_menu_row, draw_title_bar, new_playfield,
};
use crate::screen::{PlayfieldBuffer, Screen, ScreenFrame};

pub struct GeneralMenuScreen;

impl GeneralMenuScreen {
    pub fn new() -> Self {
        Self
    }
}

impl Screen for GeneralMenuScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "GENERAL COMMAND CENTER:");
        draw_menu_entry(&mut buffer, 0, 25, "I", "nfo about a Planet");
        draw_menu_entry(&mut buffer, 0, 53, "C", "ommunicate (send message)");
        draw_menu_row(&mut buffer, 1, &[
            MenuEntry::new(2, "H", "elp with commands"),
            MenuEntry::new(27, "A", "utopilot ON/OFF"),
            MenuEntry::new(53, "R", "eview messages/results"),
        ]);
        draw_menu_row(&mut buffer, 2, &[
            MenuEntry::new(2, "Q", "uit to main menu"),
            MenuEntry::new(27, "S", "tatus, your"),
            MenuEntry::new(53, "D", "elete ALL messages/results"),
        ]);
        draw_menu_row(&mut buffer, 3, &[
            MenuEntry::new(2, "X", "pert mode ON/OFF"),
            MenuEntry::new(27, "P", "rofile of your empire"),
            MenuEntry::new(53, "O", "ther empires (rankings)"),
        ]);
        draw_menu_row(&mut buffer, 4, &[
            MenuEntry::new(2, "V", "iew Partial Starmap"),
            MenuEntry::new(27, "M", "ap of the galaxy"),
            MenuEntry::new(53, "E", "nemies, declare or list"),
        ]);
        draw_command_prompt(&mut buffer, 5, "GENERAL COMMAND", "H Q X V I A S P M C R D O E");
        Ok(buffer)
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('r') | KeyCode::Char('R') => Action::OpenReports,
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenMainMenu,
            _ => Action::Noop,
        }
    }
}
