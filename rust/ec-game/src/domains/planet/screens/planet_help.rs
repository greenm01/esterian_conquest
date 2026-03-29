use crossterm::event::KeyEvent;

use crate::app::Action;
use crate::domains::planet::PlanetAction;
use crate::screen::help::{MenuHelpTopic, draw_full_screen_help, menu_help_spec};
use crate::screen::layout::new_playfield;
use crate::screen::{PlayfieldBuffer, Screen, ScreenFrame};

pub struct PlanetHelpScreen;

impl PlanetHelpScreen {
    pub fn new() -> Self {
        Self
    }
}

impl Screen for PlanetHelpScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_full_screen_help(&mut buffer, menu_help_spec(MenuHelpTopic::Planet, false));
        Ok(buffer)
    }

    fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::Planet(PlanetAction::OpenMenu)
    }
}
