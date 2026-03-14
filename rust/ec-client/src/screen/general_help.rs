use crossterm::event::KeyEvent;

use crate::app::Action;
use crate::screen::layout::{draw_command_prompt, draw_title_bar, new_playfield};
use crate::screen::{PlayfieldBuffer, Screen, ScreenFrame};
use crate::theme::classic;

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
        draw_title_bar(&mut buffer, 0, "GENERAL COMMAND HELP:");

        buffer.fill_row(2, classic::help_header_style());
        buffer.write_text(
            2,
            0,
            "Help - General Command Center command descriptions:",
            classic::help_header_style(),
        );

        for row in 3..16 {
            buffer.fill_row(row, classic::help_panel_style());
        }
        for (idx, line) in HELP_LINES.iter().enumerate() {
            buffer.write_text(3 + idx, 0, line, classic::help_panel_style());
        }

        draw_command_prompt(&mut buffer, 19, "GENERAL COMMAND", "SLAP A KEY");
        Ok(buffer)
    }

    fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::OpenGeneralMenu
    }
}
