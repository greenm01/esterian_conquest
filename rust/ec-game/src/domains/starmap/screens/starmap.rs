use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::domains::starmap::StarmapAction;
use crate::screen::PlayfieldBuffer;
use crate::screen::layout::{
    ScreenGeometry, command_line_row_for, dismiss_prompt_row_for, draw_dismiss_prompt,
    draw_title_bar, new_playfield_for,
};
use crate::theme::classic;

pub struct StarmapScreen;

pub const STARMAP_DUMP_START_ROW: usize = 2;

pub fn starmap_dump_page_lines(geometry: ScreenGeometry) -> usize {
    command_line_row_for(geometry).saturating_sub(STARMAP_DUMP_START_ROW + 1)
}

impl StarmapScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_prompt(
        &mut self,
        geometry: ScreenGeometry,
        export_status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield_for(geometry);
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
        let mut last_content_row = 7;
        if let Some(status) = export_status {
            buffer.write_text(9, 0, status, classic::status_value_style());
            last_content_row = 9;
        }
        draw_dismiss_prompt(
            &mut buffer,
            dismiss_prompt_row_for(geometry, last_content_row),
        );
        Ok(buffer)
    }

    pub fn render_complete(
        &mut self,
        geometry: ScreenGeometry,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield_for(geometry);
        draw_title_bar(&mut buffer, 0, "MAP OF THE GALAXY:");
        buffer.write_text(3, 0, "Text dump complete.", classic::body_style());
        buffer.write_text(
            5,
            0,
            "Turn off screen capture in your telnet client now.",
            classic::status_value_style(),
        );
        draw_dismiss_prompt(&mut buffer, dismiss_prompt_row_for(geometry, 5));
        Ok(buffer)
    }

    pub fn render_dump_page(
        &mut self,
        geometry: ScreenGeometry,
        lines: &[String],
        offset: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield_for(geometry);
        draw_title_bar(&mut buffer, 0, "MAP OF THE GALAXY:");
        let mut last_content_row = 0;
        for (row, line) in lines
            .iter()
            .skip(offset)
            .take(starmap_dump_page_lines(geometry))
            .enumerate()
        {
            let screen_row = STARMAP_DUMP_START_ROW + row;
            buffer.write_text(screen_row, 0, line, classic::body_style());
            last_content_row = screen_row;
        }
        draw_dismiss_prompt(
            &mut buffer,
            dismiss_prompt_row_for(geometry, last_content_row),
        );
        Ok(buffer)
    }

    pub fn handle_prompt_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenGeneralMenu,
            KeyCode::Char('e') | KeyCode::Char('E') => Action::Starmap(StarmapAction::Export),
            _ => Action::Starmap(StarmapAction::BeginDump),
        }
    }

    pub fn handle_complete_key(&self, _key: KeyEvent) -> Action {
        Action::OpenGeneralMenu
    }

    pub fn handle_dump_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenGeneralMenu,
            _ => Action::Starmap(StarmapAction::AdvancePage),
        }
    }
}
