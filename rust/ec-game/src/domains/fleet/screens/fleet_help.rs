use crossterm::event::KeyEvent;

use crate::app::Action;
use crate::domains::fleet::FleetAction;
use crate::screen::help::{MenuHelpTopic, draw_full_screen_help, menu_help_spec};
use crate::screen::layout::new_playfield;
use crate::screen::{PlayfieldBuffer, Screen, ScreenFrame};

pub struct FleetHelpScreen;

impl FleetHelpScreen {
    pub fn new() -> Self {
        Self
    }
}

impl Screen for FleetHelpScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_full_screen_help(&mut buffer, menu_help_spec(MenuHelpTopic::Fleet, false));
        Ok(buffer)
    }

    fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::Fleet(FleetAction::OpenMenu)
    }
}
