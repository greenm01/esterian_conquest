use crossterm::event::KeyEvent;

use crate::app::Action;
use crate::screen::help::{MenuHelpTopic, draw_full_screen_help, menu_help_spec};
use crate::screen::layout::new_playfield;
use crate::screen::{PlayfieldBuffer, Screen, ScreenFrame};

pub struct MainHelpScreen;

impl MainHelpScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_for_mode(
        &mut self,
        door_mode: bool,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_full_screen_help(&mut buffer, menu_help_spec(MenuHelpTopic::Main, door_mode));
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
