use crossterm::event::KeyEvent;

use super::{Action, state::App};
use crate::domains::startup::state::FirstTimeOnboardingMode;
use crate::screen::layout::{command_line_row_for, draw_command_line_prompt_text_padded};
use crate::screen::{COMMAND_LABEL, PlayfieldBuffer, ScreenId};

impl App {
    pub fn request_quit(&mut self) {
        if self.current_screen == ScreenId::FirstTimeJoinEmpireName
            && self.startup_state.first_time_onboarding_mode
                == FirstTimeOnboardingMode::HostedInvite
            && !self.startup_state.first_time_rename_preloaded_empire
        {
            self.prepare_hosted_invite_quit_warning();
        }
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
        let row = self
            .find_active_prompt_row(buffer)
            .unwrap_or_else(|| command_line_row_for(self.screen_geometry));
        draw_command_line_prompt_text_padded(buffer, row, COMMAND_LABEL, "Are you sure Y/[N] ->");
        buffer.clear_cursor();
    }

    fn find_active_prompt_row(&self, buffer: &PlayfieldBuffer) -> Option<usize> {
        (0..buffer.height())
            .rev()
            .find(|&row| buffer.plain_line(row).contains(" <- "))
    }
}
