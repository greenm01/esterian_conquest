use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::screen::layout::{draw_command_prompt, draw_title_bar, new_playfield};
use crate::screen::PlayfieldBuffer;
use crate::theme::classic;

pub struct DeleteReviewablesScreen;

impl DeleteReviewablesScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "DELETE ALL MESSAGES / RESULTS:");
        buffer.write_text(
            2,
            0,
            "This will clear all currently reviewable messages and results.",
            classic::body_style(),
        );
        buffer.write_text(
            4,
            0,
            "Press Y to delete everything, or Q to cancel.",
            classic::status_value_style(),
        );
        if let Some(status) = status {
            buffer.write_text(6, 0, status, classic::status_value_style());
        }
        draw_command_prompt(&mut buffer, 8, "GENERAL COMMAND", "Y Q");
        Ok(buffer)
    }

    pub fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => Action::ConfirmDeleteReviewables,
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenGeneralMenu,
            _ => Action::Noop,
        }
    }
}
