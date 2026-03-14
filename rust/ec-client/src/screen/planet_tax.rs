use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::screen::layout::{
    draw_command_prompt, draw_plain_prompt, draw_status_line, draw_title_bar, new_playfield,
};
use crate::screen::PlayfieldBuffer;
use crate::theme::classic;

pub struct PlanetTaxScreen;

impl PlanetTaxScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_prompt(
        &mut self,
        current_tax: &str,
        input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "PLANET COMMAND:");
        draw_centered_warning(&mut buffer, 3, "Warning:");
        draw_centered_warning(
            &mut buffer,
            5,
            "Taxes in excess of 65% may actually REDUCE your planets'",
        );
        draw_centered_warning(&mut buffer, 6, "productivities!");
        let prefix = "Empire tax rate (0 - 100): [";
        let prefix_col = draw_plain_prompt(&mut buffer, 10, prefix);
        let default_start = prefix_col;
        let default_end = default_start
            + buffer.write_text(
                10,
                default_start,
                current_tax,
                classic::prompt_hotkey_style(),
            );
        let suffix_start =
            default_end + buffer.write_text(10, default_end, "] -> ", classic::prompt_style());
        let input_end = suffix_start
            + buffer.write_text(10, suffix_start, input, classic::prompt_hotkey_style());
        if let Some(status) = status {
            draw_status_line(&mut buffer, 12, "Error: ", status);
        }
        draw_command_prompt(&mut buffer, 19, "PLANET COMMAND", "ENTER Q");
        buffer.set_cursor(input_end as u16, 10);
        Ok(buffer)
    }

    pub fn render_done(
        &mut self,
        status: &str,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "PLANET COMMAND:");
        draw_status_line(&mut buffer, 8, "", status);
        draw_command_prompt(&mut buffer, 19, "PLANET COMMAND", "SLAP A KEY");
        Ok(buffer)
    }

    pub fn handle_prompt_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenPlanetMenu,
            KeyCode::Enter => Action::SubmitPlanetTax,
            KeyCode::Backspace => Action::BackspacePlanetTaxInput,
            KeyCode::Char(ch) if ch.is_ascii_digit() => Action::AppendPlanetTaxChar(ch),
            _ => Action::Noop,
        }
    }

    pub fn handle_done_key(&self, _key: KeyEvent) -> Action {
        Action::OpenPlanetMenu
    }
}

fn draw_centered_warning(buffer: &mut PlayfieldBuffer, row: usize, text: &str) {
    let col = buffer.width().saturating_sub(text.chars().count()) / 2;
    buffer.write_text(row, col, text, classic::alert_style());
}
