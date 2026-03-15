use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::screen::PlayfieldBuffer;
use crate::screen::layout::{draw_command_prompt, draw_title_bar, new_playfield};
use crate::theme::classic;

pub struct StarmapScreen;

impl StarmapScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_prompt(
        &mut self,
        export_status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "MAP OF THE GALAXY:");
        buffer.write_text(
            2,
            0,
            "This function sends the entire map of the galaxy to you non-stop.",
            classic::body_style(),
        );
        buffer.write_text(
            3,
            0,
            "It is intended to allow you to capture the map to a text file.",
            classic::body_style(),
        );
        buffer.write_text(
            5,
            0,
            "Turn on your telnet client screen capture now.",
            classic::status_value_style(),
        );
        buffer.write_text(
            6,
            0,
            "Press E to export printable map files, Q to abort, or slap a key",
            classic::body_style(),
        );
        buffer.write_text(7, 0, "to begin the text dump.", classic::body_style());
        if let Some(status) = export_status {
            buffer.write_text(9, 0, status, classic::status_value_style());
        }
        draw_command_prompt(&mut buffer, 11, "GALAXY MAP", "SLAP A KEY");
        Ok(buffer)
    }

    pub fn render_complete(&mut self) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "MAP OF THE GALAXY:");
        buffer.write_text(3, 0, "Text dump complete.", classic::body_style());
        buffer.write_text(
            5,
            0,
            "Turn off screen capture in your telnet client now.",
            classic::status_value_style(),
        );
        draw_command_prompt(&mut buffer, 8, "GENERAL COMMAND", "SLAP A KEY");
        Ok(buffer)
    }

    pub fn render_dump_page(
        &mut self,
        lines: &[String],
        offset: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        const PAGE_LINES: usize = 16;
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "MAP OF THE GALAXY:");
        for (row, line) in lines.iter().skip(offset).take(PAGE_LINES).enumerate() {
            buffer.write_text(2 + row, 0, line, classic::body_style());
        }
        draw_command_prompt(&mut buffer, 19, "GALAXY MAP", "SLAP A KEY");
        Ok(buffer)
    }

    pub fn handle_prompt_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenGeneralMenu,
            KeyCode::Char('e') | KeyCode::Char('E') => Action::ExportStarmap,
            _ => Action::BeginStarmapDump,
        }
    }

    pub fn handle_complete_key(&self, _key: KeyEvent) -> Action {
        Action::OpenGeneralMenu
    }

    pub fn handle_dump_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenGeneralMenu,
            _ => Action::AdvanceStarmapPage,
        }
    }
}
