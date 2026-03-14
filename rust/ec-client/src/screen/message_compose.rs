use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::Action;
use crate::screen::layout::{draw_command_prompt, draw_plain_prompt, draw_title_bar, new_playfield};
use crate::screen::table::{format_empire_id, TableColumn, write_table_window};
use crate::screen::{PlayfieldBuffer, ScreenFrame};
use crate::theme::classic;

pub struct MessageComposeScreen;
const RECIPIENT_VISIBLE_ROWS: usize = 10;

const RECIPIENT_COLUMNS: [TableColumn<'static>; 2] = [
    TableColumn::right("ID", 3),
    TableColumn::left("Empire", 28),
];

impl MessageComposeScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_recipient(
        &mut self,
        frame: &ScreenFrame<'_>,
        input: &str,
        status: Option<&str>,
        scroll_offset: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "COMMUNICATE (SEND MESSAGE):");
        buffer.write_text(2, 0, "Available empires:", classic::body_style());
        let rows = frame
            .game_data
            .player
            .records
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx + 1 != frame.player.record_index_1_based)
            .map(|(idx, player)| {
                let empire_id = idx + 1;
                let name = player.controlled_empire_name_summary();
                let fallback = player.legacy_status_name_summary();
                let display = if !name.is_empty() { name } else { fallback };
                vec![format_empire_id(empire_id as u8), display]
            })
            .collect::<Vec<_>>();
        write_table_window(
            &mut buffer,
            4,
            &RECIPIENT_COLUMNS,
            &rows,
            scroll_offset,
            RECIPIENT_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
        );
        let prompt_row = 17;
        let prompt = format!("Enter recipient empire number: {input}");
        let cursor_col = draw_plain_prompt(&mut buffer, prompt_row, &prompt);
        if let Some(status) = status {
            buffer.write_text(18, 0, status, classic::status_value_style());
        }
        draw_command_prompt(&mut buffer, 19, "GENERAL COMMAND", "ARROWS J K Q");
        buffer.set_cursor(cursor_col as u16, prompt_row as u16);
        Ok(buffer)
    }

    pub fn render_body(
        &mut self,
        recipient_label: &str,
        body: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "COMMUNICATE (SEND MESSAGE):");
        buffer.write_text(1, 0, &format!("To: {recipient_label}"), classic::status_value_style());
        buffer.write_text(2, 0, "Ctrl-S send  Ctrl-X cancel", classic::body_style());
        buffer.write_text(3, 0, "-------------------------------------------------------------------------------", classic::menu_style());

        let wrapped = wrap_body_lines(body, 79);
        let visible = 12usize;
        let start = wrapped.len().saturating_sub(visible);
        for (idx, line) in wrapped.iter().skip(start).take(visible).enumerate() {
            buffer.write_text(4 + idx, 0, line, classic::body_style());
        }
        if let Some(status) = status {
            buffer.write_text(17, 0, status, classic::status_value_style());
        }
        draw_command_prompt(&mut buffer, 19, "GENERAL COMMAND", "CTRL-S CTRL-X");
        let visible_lines = wrapped.iter().skip(start).take(visible).collect::<Vec<_>>();
        let last_row = 4 + visible_lines.len().saturating_sub(1);
        let last_col = visible_lines
            .last()
            .map(|line| line.chars().count())
            .unwrap_or(0);
        buffer.set_cursor(last_col as u16, last_row as u16);
        Ok(buffer)
    }

    pub fn render_sent(
        &mut self,
        status: &str,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "COMMUNICATE (SEND MESSAGE):");
        buffer.write_text(3, 0, status, classic::status_value_style());
        draw_command_prompt(&mut buffer, 6, "GENERAL COMMAND", "SLAP A KEY");
        Ok(buffer)
    }

    pub fn handle_recipient_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => Action::ScrollComposeRecipients(-1),
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => Action::ScrollComposeRecipients(1),
            KeyCode::PageUp => Action::ScrollComposeRecipients(-8),
            KeyCode::PageDown => Action::ScrollComposeRecipients(8),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenGeneralMenu,
            KeyCode::Enter => Action::SubmitComposeRecipient,
            KeyCode::Backspace => Action::BackspaceComposeRecipient,
            KeyCode::Char(ch) if ch.is_ascii_digit() => Action::AppendComposeRecipientChar(ch),
            _ => Action::Noop,
        }
    }

    pub fn handle_body_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('s') | KeyCode::Char('S') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::SendComposedMessage,
            KeyCode::Char('x') | KeyCode::Char('X') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::OpenGeneralMenu,
            KeyCode::Backspace => Action::BackspaceComposeBody,
            KeyCode::Enter => Action::InsertComposeNewline,
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => Action::AppendComposeBodyChar(ch),
            _ => Action::Noop,
        }
    }

    pub fn handle_sent_key(&self, _key: KeyEvent) -> Action {
        Action::OpenGeneralMenu
    }
}

fn wrap_body_lines(body: &str, width: usize) -> Vec<String> {
    if body.is_empty() {
        return vec![String::new()];
    }

    let mut out = Vec::new();
    for source_line in body.split('\n') {
        if source_line.is_empty() {
            out.push(String::new());
            continue;
        }
        let chars = source_line.chars().collect::<Vec<_>>();
        let mut start = 0usize;
        while start < chars.len() {
            let end = usize::min(start + width, chars.len());
            out.push(chars[start..end].iter().collect());
            start = end;
        }
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}
