use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::screen::layout::{
    draw_command_prompt, draw_menu_row, draw_title_bar, new_playfield, MenuEntry,
};
use crate::screen::{CommandMenu, PlayfieldBuffer, Screen, ScreenFrame};

pub struct MainMenuScreen;

impl MainMenuScreen {
    pub fn new() -> Self {
        Self
    }
}

impl Screen for MainMenuScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "MAIN MENU: ");
        draw_menu_row(
            &mut buffer,
            1,
            &[
                MenuEntry::new(2, "H", "elp with commands"),
                MenuEntry::new(24, "G", "ENERAL COMMAND MENU..."),
                MenuEntry::new(53, "B", "rief Empire Report"),
            ],
        );
        draw_menu_row(
            &mut buffer,
            2,
            &[
                MenuEntry::new(2, "Q", "uit back to BBS"),
                MenuEntry::new(24, "P", "LANET COMMAND MENU..."),
                MenuEntry::new(53, "I", "nfo about a Planet"),
            ],
        );
        draw_menu_row(
            &mut buffer,
            3,
            &[
                MenuEntry::new(2, "X", "pert mode ON/OFF"),
                MenuEntry::new(24, "F", "LEET COMMAND MENU..."),
                MenuEntry::new(53, "D", "etailed Empire Report"),
            ],
        );
        draw_menu_row(
            &mut buffer,
            4,
            &[
                MenuEntry::new(2, "V", "iew Partial Map"),
                MenuEntry::new(24, "T", "otal Planet Database"),
            ],
        );
        draw_command_prompt(&mut buffer, 5, "MAIN COMMAND", "H Q X V G P F T I B D");
        Ok(buffer)
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('b') | KeyCode::Char('B') => Action::OpenEmpireStatus,
            KeyCode::Char('d') | KeyCode::Char('D') => Action::OpenEmpireProfile,
            KeyCode::Char('g') | KeyCode::Char('G') => Action::OpenGeneralMenu,
            KeyCode::Char('i') | KeyCode::Char('I') => Action::OpenPlanetInfoPrompt(CommandMenu::Main),
            KeyCode::Char('p') | KeyCode::Char('P') => Action::OpenPlanetMenu,
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::Quit,
            KeyCode::Char('t') | KeyCode::Char('T') => Action::OpenPlanetDatabase,
            KeyCode::Char('v') | KeyCode::Char('V') => {
                Action::OpenPartialStarmapPrompt(CommandMenu::Main)
            }
            _ => Action::Noop,
        }
    }
}
