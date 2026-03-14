use crossterm::event::KeyEvent;

use crate::app::Action;
use crate::screen::layout::{draw_help_panel, new_playfield};
use crate::screen::{PlayfieldBuffer, Screen, ScreenFrame};

pub struct GeneralHelpScreen;

const HELP_LINES: [&str; 13] = [
    "<A> - allow maintenance to issue orders and builds automatically",
    "<C> - type and send messages to other players in the game",
    "<D> - delete all messages and result reports from your message base",
    "<E> - list and declare your enemies",
    "<H> - describe General Command Center commands",
    "<I> - show intelligence on what you know about any planet",
    "<M> - display the entire game map for capture or export",
    "<O> - list all empires in the order you specify",
    "<P> - display the profile of your empire",
    "<Q> - quit the General Command Center and return to the Main Menu",
    "<R> - display pending messages or reports",
    "<S> - display time left to play and other status information",
    "<V> - display a portion of the map; use M for the whole map",
];

impl GeneralHelpScreen {
    pub fn new() -> Self {
        Self
    }
}

impl Screen for GeneralHelpScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_help_panel(
            &mut buffer,
            "GENERAL COMMAND HELP:",
            "Help - General Command Center command descriptions:",
            &HELP_LINES,
            "GENERAL COMMAND",
        );
        Ok(buffer)
    }

    fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::OpenGeneralMenu
    }
}
