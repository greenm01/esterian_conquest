use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::screen::layout::{draw_command_prompt, draw_title_bar, new_playfield};
use crate::screen::{PlayfieldBuffer, Screen};
use crate::theme::classic;

pub struct PlanetAutoCommissionScreen;

impl PlanetAutoCommissionScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_confirm(&mut self) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "AUTO-COMMISSION SHIPS:");
        buffer.write_text(
            3,
            0,
            "Automatically commission all ships and starbases in stardock?",
            classic::body_style(),
        );
        buffer.write_text(
            5,
            0,
            "Press Y to commission them now, or Q to cancel.",
            classic::status_value_style(),
        );
        draw_command_prompt(&mut buffer, 19, "PLANET COMMAND", "Y Q");
        Ok(buffer)
    }

    pub fn render_done(
        &mut self,
        status: &str,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "AUTO-COMMISSION SHIPS:");
        buffer.write_text(4, 0, status, classic::status_value_style());
        draw_command_prompt(&mut buffer, 19, "PLANET COMMAND", "SLAP A KEY");
        Ok(buffer)
    }
}

impl Screen for PlanetAutoCommissionScreen {
    fn render(
        &mut self,
        _frame: &crate::screen::ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        Ok(new_playfield())
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => Action::ConfirmPlanetAutoCommission,
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenPlanetMenu,
            _ => Action::OpenPlanetMenu,
        }
    }
}
