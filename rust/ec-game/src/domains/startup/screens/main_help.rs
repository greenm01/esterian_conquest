use crossterm::event::KeyEvent;

use crate::app::Action;
use crate::screen::layout::{draw_help_panel, new_playfield};
use crate::screen::{COMMAND_LABEL, PlayfieldBuffer, Screen, ScreenFrame};

pub struct MainHelpScreen;

const LOCAL_HELP_LINES: [&str; 12] = [
    "<C> - open the color theme picker",
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
    "<X> - hide/show command menus",
];

const DOOR_HELP_LINES: [&str; 12] = [
    "<A> - turn ANSI color on or off",
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
    "<X> - hide/show command menus",
];

impl MainHelpScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_for_mode(
        &mut self,
        door_mode: bool,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_help_panel(
            &mut buffer,
            "HELP WITH COMMANDS:",
            "Help - Main Menu command descriptions:",
            if door_mode {
                &DOOR_HELP_LINES
            } else {
                &LOCAL_HELP_LINES
            },
            COMMAND_LABEL,
        );
        Ok(buffer)
    }
}

impl Screen for MainHelpScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_for_mode(false)
    }

    fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::OpenMainMenu
    }
}
