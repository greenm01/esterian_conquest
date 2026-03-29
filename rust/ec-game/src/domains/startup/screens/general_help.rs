use crossterm::event::KeyEvent;

use crate::app::Action;
use crate::screen::help::{MenuHelpTopic, draw_full_screen_help, menu_help_spec};
use crate::screen::layout::new_playfield;
use crate::screen::{PlayfieldBuffer, Screen, ScreenFrame};

pub struct GeneralHelpScreen;

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
        draw_full_screen_help(&mut buffer, menu_help_spec(MenuHelpTopic::General, false));
        Ok(buffer)
    }

    fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::OpenGeneralMenu
    }
}
