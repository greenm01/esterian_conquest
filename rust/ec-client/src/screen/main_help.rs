use crossterm::event::KeyEvent;

use crate::app::Action;
use crate::screen::layout::{draw_help_panel, new_playfield};
use crate::screen::{PlayfieldBuffer, Screen, ScreenFrame};

pub struct MainHelpScreen;

const HELP_LINES: [&str; 11] = [
    "<A> - ANSI stays on. The stars look better in color.",
    "<B> - display all empire information in brief format",
    "<D> - display all empire information in detailed format",
    "<F> - bring up the Fleet Command Center menu",
    "<G> - bring up the General Command Center menu",
    "<H> - describe Main Menu commands",
    "<I> - show Intelligence on what you know about any planet",
    "<P> - bring up the Planet Command Center menu",
    "<Q> - quit Esterian Conquest and returns you back to Jump Start",
    "<T> - list database information about planets",
    "<V> - display a portion of the map (goto GENERAL MENU for entire map)",
];

impl MainHelpScreen {
    pub fn new() -> Self {
        Self
    }
}

impl Screen for MainHelpScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_help_panel(
            &mut buffer,
            "HELP WITH COMMANDS:",
            "Help - Main Menu command descriptions:",
            &HELP_LINES,
            "MAIN COMMAND",
        );
        Ok(buffer)
    }

    fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::OpenMainMenu
    }
}
