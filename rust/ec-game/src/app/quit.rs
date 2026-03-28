use crossterm::event::KeyEvent;

use super::{Action, state::App};
use crate::screen::PlayfieldBuffer;
use crate::screen::layout::{command_line_row_for, draw_command_line_prompt_text_at};

impl App {
    pub fn request_quit(&mut self) {
        self.quit_confirm_open = true;
    }

    pub fn cancel_quit_prompt(&mut self) {
        self.quit_confirm_open = false;
    }

    pub fn handle_quit_confirm_key(&self, key: KeyEvent) -> Action {
        match key.code {
            crossterm::event::KeyCode::Char('y') | crossterm::event::KeyCode::Char('Y') => {
                Action::Quit
            }
            crossterm::event::KeyCode::Enter
            | crossterm::event::KeyCode::Char('n')
            | crossterm::event::KeyCode::Char('N')
            | crossterm::event::KeyCode::Char('q')
            | crossterm::event::KeyCode::Char('Q')
            | crossterm::event::KeyCode::Esc => Action::CancelQuitPrompt,
            _ => Action::Noop,
        }
    }

    pub fn render_quit_confirm(&self, buffer: &mut PlayfieldBuffer) {
        let row = command_line_row_for(self.screen_geometry);
        draw_command_line_prompt_text_at(buffer, row, "COMMAND", "Are you sure Y/[N] ->");
        buffer.clear_cursor();
    }
}
