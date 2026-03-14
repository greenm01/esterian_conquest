use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ec_data::QueuedPlayerMail;

use crate::app::Action;
use crate::screen::layout::{
    draw_command_prompt, draw_plain_prompt, draw_title_bar, new_playfield,
};
use crate::screen::table::{format_empire_id, write_table_window, TableColumn};
use crate::screen::{PlayfieldBuffer, ScreenFrame};
use crate::theme::classic;

pub struct MessageComposeScreen;
pub(crate) const RECIPIENT_VISIBLE_ROWS: usize = 10;
pub(crate) const OUTBOX_VISIBLE_ROWS: usize = 9;
pub(crate) const COMPOSE_SUBJECT_LIMIT: usize = 60;
pub(crate) const COMPOSE_BODY_LIMIT: usize = 1000;

const RECIPIENT_COLUMNS: [TableColumn<'static>; 2] =
    [TableColumn::right("ID", 3), TableColumn::left("Empire", 28)];

const OUTBOX_COLUMNS: [TableColumn<'static>; 3] = [
    TableColumn::right("No", 3),
    TableColumn::left("To", 28),
    TableColumn::left("Subject", 34),
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
        buffer.write_text(
            3,
            0,
            "Press D to review or delete queued outgoing messages.",
            classic::body_style(),
        );
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
            5,
            &RECIPIENT_COLUMNS,
            &rows,
            scroll_offset,
            RECIPIENT_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
        );
        let prompt_row = 18;
        let prompt = format!("Enter recipient empire number: {input}");
        let cursor_col = draw_plain_prompt(&mut buffer, prompt_row, &prompt);
        if let Some(status) = status {
            buffer.write_text(17, 0, status, classic::status_value_style());
        }
        draw_command_prompt(&mut buffer, 19, "GENERAL COMMAND", "ARROWS J K D Q");
        buffer.set_cursor(cursor_col as u16, prompt_row as u16);
        Ok(buffer)
    }

    pub fn render_subject(
        &mut self,
        recipient_label: &str,
        subject: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "COMMUNICATE (SEND MESSAGE):");
        buffer.write_text(
            2,
            0,
            &format!("To: {recipient_label}"),
            classic::status_value_style(),
        );
        let prompt = format!("Enter message subject: {subject}");
        let cursor_col = draw_plain_prompt(&mut buffer, 4, &prompt);
        if let Some(status) = status {
            buffer.write_text(6, 0, status, classic::status_value_style());
        }
        draw_command_prompt(&mut buffer, 19, "GENERAL COMMAND", "ENTER Q");
        buffer.set_cursor(cursor_col as u16, 4);
        Ok(buffer)
    }

    pub fn render_body(
        &mut self,
        recipient_label: &str,
        subject: &str,
        body: &str,
        cursor_index: usize,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "COMMUNICATE (SEND MESSAGE):");
        buffer.write_text(
            1,
            0,
            &format!("To: {recipient_label}"),
            classic::status_value_style(),
        );
        buffer.write_text(
            2,
            0,
            &format!("Subject: {subject}"),
            classic::status_value_style(),
        );
        buffer.write_text(3, 0, "Ctrl-E send  Ctrl-X cancel", classic::body_style());
        buffer.write_text(
            4,
            0,
            "-------------------------------------------------------------------------------",
            classic::menu_style(),
        );

        let wrapped = wrap_body_segments(body, 79);
        let visible = 11usize;
        let cursor_segment = cursor_segment_index(&wrapped, cursor_index);
        let start = visible_window_start(wrapped.len(), visible, cursor_segment);
        for (idx, segment) in wrapped.iter().skip(start).take(visible).enumerate() {
            buffer.write_text(5 + idx, 0, &segment.text, classic::body_style());
        }
        if let Some(status) = status {
            buffer.write_text(17, 0, status, classic::status_value_style());
        }
        buffer.write_text(
            18,
            0,
            &format!("Chars: {}/{}", body.chars().count(), COMPOSE_BODY_LIMIT),
            classic::body_style(),
        );
        draw_command_prompt(&mut buffer, 19, "GENERAL COMMAND", "CTRL-E CTRL-X");
        let cursor_row = 5 + cursor_segment.saturating_sub(start);
        let cursor_col = wrapped
            .get(cursor_segment)
            .map(|segment| cursor_index.saturating_sub(segment.start))
            .unwrap_or(0);
        buffer.set_cursor(cursor_col as u16, cursor_row as u16);
        Ok(buffer)
    }

    pub fn render_send_confirm(
        &mut self,
        recipient_label: &str,
        subject: &str,
        body: &str,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = self.render_body(
            recipient_label,
            subject,
            body,
            body.chars().count(),
            Some("Send this message after turn maintenance?"),
        )?;
        draw_command_prompt(&mut buffer, 19, "SEND MESSAGE", "Y N");
        buffer.clear_cursor();
        Ok(buffer)
    }

    pub fn render_discard_confirm(
        &mut self,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "COMMUNICATE (SEND MESSAGE):");
        buffer.write_text(
            4,
            0,
            "Discard this unsent message draft?",
            classic::status_value_style(),
        );
        buffer.write_text(
            6,
            0,
            "Press Y to discard it, or any other key to keep editing.",
            classic::body_style(),
        );
        draw_command_prompt(&mut buffer, 19, "GENERAL COMMAND", "Y N");
        Ok(buffer)
    }

    pub fn render_outbox(
        &mut self,
        queue: &[QueuedPlayerMail],
        input: &str,
        status: Option<&str>,
        scroll_offset: usize,
        game_data: &ec_data::CoreGameData,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "COMMUNICATE (SEND MESSAGE):");
        buffer.write_text(
            2,
            0,
            "Queued messages awaiting turn maintenance:",
            classic::body_style(),
        );
        let rows = queue
            .iter()
            .enumerate()
            .map(|(idx, mail)| {
                let recipient = compose_empire_label(game_data, mail.recipient_empire_id);
                let subject = if mail.subject.trim().is_empty() {
                    "<no subject>".to_string()
                } else {
                    mail.subject.clone()
                };
                vec![format!("{:02}", idx + 1), recipient, subject]
            })
            .collect::<Vec<_>>();
        write_table_window(
            &mut buffer,
            4,
            &OUTBOX_COLUMNS,
            &rows,
            scroll_offset,
            OUTBOX_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
        );
        let prompt = format!("Enter queued message number to delete: {input}");
        let cursor_col = draw_plain_prompt(&mut buffer, 16, &prompt);
        if let Some(status) = status {
            buffer.write_text(18, 0, status, classic::status_value_style());
        }
        draw_command_prompt(&mut buffer, 19, "GENERAL COMMAND", "ARROWS J K Q");
        buffer.set_cursor(cursor_col as u16, 16);
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
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::ScrollComposeRecipients(-1)
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::ScrollComposeRecipients(1)
            }
            KeyCode::PageUp => Action::ScrollComposeRecipients(-8),
            KeyCode::PageDown => Action::ScrollComposeRecipients(8),
            KeyCode::Char('d') | KeyCode::Char('D') => Action::OpenComposeMessageOutbox,
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenGeneralMenu,
            KeyCode::Enter => Action::SubmitComposeRecipient,
            KeyCode::Backspace => Action::BackspaceComposeRecipient,
            KeyCode::Char(ch) if ch.is_ascii_digit() => Action::AppendComposeRecipientChar(ch),
            _ => Action::Noop,
        }
    }

    pub fn handle_subject_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::OpenComposeMessageRecipient
            }
            KeyCode::Enter => Action::SubmitComposeSubject,
            KeyCode::Backspace => Action::BackspaceComposeSubject,
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                Action::AppendComposeSubjectChar(ch)
            }
            _ => Action::Noop,
        }
    }

    pub fn handle_body_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('e') | KeyCode::Char('E')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                Action::OpenComposeMessageSendConfirm
            }
            KeyCode::Char('x') | KeyCode::Char('X')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                Action::OpenComposeMessageDiscardConfirm
            }
            KeyCode::Left => Action::MoveComposeBodyCursorLeft,
            KeyCode::Right => Action::MoveComposeBodyCursorRight,
            KeyCode::Up => Action::MoveComposeBodyCursorUp,
            KeyCode::Down => Action::MoveComposeBodyCursorDown,
            KeyCode::Home => Action::MoveComposeBodyCursorHome,
            KeyCode::End => Action::MoveComposeBodyCursorEnd,
            KeyCode::Backspace => Action::BackspaceComposeBody,
            KeyCode::Delete => Action::DeleteComposeBodyChar,
            KeyCode::Enter => Action::InsertComposeNewline,
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                Action::AppendComposeBodyChar(ch)
            }
            _ => Action::Noop,
        }
    }

    pub fn handle_discard_confirm_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => Action::ConfirmDiscardComposedMessage,
            _ => Action::OpenComposeMessageBody,
        }
    }

    pub fn handle_send_confirm_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => Action::ConfirmSendComposedMessage,
            _ => Action::OpenComposeMessageBody,
        }
    }

    pub fn handle_outbox_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::ScrollComposeOutbox(-1)
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::ScrollComposeOutbox(1)
            }
            KeyCode::PageUp => Action::ScrollComposeOutbox(-8),
            KeyCode::PageDown => Action::ScrollComposeOutbox(8),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::OpenComposeMessageRecipient
            }
            KeyCode::Enter => Action::DeleteQueuedComposeMessage,
            KeyCode::Backspace => Action::BackspaceComposeOutboxInput,
            KeyCode::Char(ch) if ch.is_ascii_digit() => Action::AppendComposeOutboxChar(ch),
            _ => Action::Noop,
        }
    }

    pub fn handle_sent_key(&self, _key: KeyEvent) -> Action {
        Action::OpenGeneralMenu
    }
}

#[derive(Debug, Clone)]
struct WrappedSegment {
    start: usize,
    end: usize,
    text: String,
}

fn wrap_body_segments(body: &str, width: usize) -> Vec<WrappedSegment> {
    if body.is_empty() {
        return vec![WrappedSegment {
            start: 0,
            end: 0,
            text: String::new(),
        }];
    }

    let mut out = Vec::new();
    let chars = body.chars().collect::<Vec<_>>();
    let mut line_start = 0usize;
    let mut idx = 0usize;
    while idx <= chars.len() {
        let line_end = if idx == chars.len() {
            idx
        } else if chars[idx] == '\n' {
            idx
        } else {
            idx += 1;
            continue;
        };

        if line_start == line_end {
            out.push(WrappedSegment {
                start: line_start,
                end: line_end,
                text: String::new(),
            });
        } else {
            let mut seg_start = line_start;
            while seg_start < line_end {
                let seg_end = usize::min(seg_start + width, line_end);
                out.push(WrappedSegment {
                    start: seg_start,
                    end: seg_end,
                    text: chars[seg_start..seg_end].iter().collect(),
                });
                seg_start = seg_end;
            }
        }

        if idx == chars.len() {
            break;
        }
        idx += 1;
        line_start = idx;
    }

    if out.is_empty() {
        out.push(WrappedSegment {
            start: 0,
            end: 0,
            text: String::new(),
        });
    }
    out
}

fn cursor_segment_index(segments: &[WrappedSegment], cursor_index: usize) -> usize {
    for (idx, segment) in segments.iter().enumerate() {
        if cursor_index >= segment.start && cursor_index <= segment.end {
            return idx;
        }
    }
    segments.len().saturating_sub(1)
}

fn visible_window_start(total: usize, visible: usize, cursor_segment: usize) -> usize {
    if total <= visible {
        return 0;
    }
    let max_start = total - visible;
    cursor_segment
        .saturating_sub(visible.saturating_sub(1))
        .min(max_start)
}

fn compose_empire_label(game_data: &ec_data::CoreGameData, empire_id: u8) -> String {
    let Some(player) = game_data
        .player
        .records
        .get(empire_id.saturating_sub(1) as usize)
    else {
        return format!("Empire {empire_id:02}");
    };
    let name = player.controlled_empire_name_summary();
    let fallback = player.legacy_status_name_summary();
    let display = if !name.is_empty() { name } else { fallback };
    format!("{} {}", format_empire_id(empire_id), display)
}
