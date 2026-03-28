use crossterm::event::KeyEvent;
use ec_ui::buffer::PlayfieldBuffer;
use ec_ui::prompt::draw_command_line_prompt_text_at;
use ec_ui::theme::classic;

use super::event::{is_back_key, is_cancel_confirm_key, is_help_key, is_yes_key};
use super::help::HelpTopic;
use super::layout::{
    COMMAND_ROW, PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH, Rect, centered_rect, draw_box,
};
use super::state::PickerState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoticeLevel {
    Notice,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerOverlay {
    Notice { level: NoticeLevel, message: String },
    Help(HelpTopic),
    QuitConfirm,
}

pub fn handle_overlay_key(key: KeyEvent, state: &mut PickerState) {
    let Some(current) = state.overlay.clone() else {
        return;
    };

    match current {
        PickerOverlay::Notice { .. } => {
            if is_back_key(key) || matches!(key.code, crossterm::event::KeyCode::Enter) {
                state.overlay = None;
            }
        }
        PickerOverlay::Help(_) => {
            if is_help_key(key)
                || is_back_key(key)
                || matches!(key.code, crossterm::event::KeyCode::Enter)
            {
                state.overlay = None;
            }
        }
        PickerOverlay::QuitConfirm => {
            if is_yes_key(key) {
                state.overlay = None;
                state.quit = true;
            } else if is_cancel_confirm_key(key) {
                state.overlay = None;
            }
        }
    }
}

pub fn render_overlay(buffer: &mut PlayfieldBuffer, state: &PickerState) {
    match state.overlay.as_ref() {
        Some(PickerOverlay::Notice { level, message }) => {
            let label = match level {
                NoticeLevel::Notice => "NOTICE",
                NoticeLevel::Error => "ERROR",
            };
            let prompt = format!("{message} <Q> ->");
            draw_command_line_prompt_text_at(buffer, COMMAND_ROW, label, &prompt);
            buffer.clear_cursor();
        }
        Some(PickerOverlay::Help(topic)) => {
            render_help_overlay(buffer, *topic);
        }
        Some(PickerOverlay::QuitConfirm) => {
            draw_command_line_prompt_text_at(
                buffer,
                COMMAND_ROW,
                "COMMAND",
                "Are you sure Y/[N] ->",
            );
            buffer.clear_cursor();
        }
        None => {}
    }
}

fn render_help_overlay(buffer: &mut PlayfieldBuffer, topic: HelpTopic) {
    let spec = topic.spec();
    let mut content_width = spec
        .rows
        .iter()
        .map(|row| row.command.len() + 3 + row.description.len())
        .max()
        .unwrap_or(0);
    if let Some(note) = spec.note {
        content_width = content_width.max(note.len());
    }
    let width = (content_width + 4)
        .max(spec.title.len() + 4)
        .min(PLAYFIELD_WIDTH.saturating_sub(8));
    let row_count = spec.rows.len() + usize::from(spec.note.is_some());
    let popup_height = (row_count + 2) as u16;
    let popup = centered_rect(
        ((width * 100) / PLAYFIELD_WIDTH).max(40) as u16,
        popup_height,
        Rect::new(0, 0, PLAYFIELD_WIDTH as u16, PLAYFIELD_HEIGHT as u16),
    );
    let popup = Rect::new(popup.x, popup.y, width as u16, popup_height);
    let pad = Rect::new(
        popup.x.saturating_sub(1),
        popup.y.saturating_sub(1),
        (popup.width + 2).min(PLAYFIELD_WIDTH as u16 - popup.x.saturating_sub(1)),
        (popup.height + 2).min(PLAYFIELD_HEIGHT as u16 - popup.y.saturating_sub(1)),
    );
    buffer.fill_rect(
        pad.y as usize,
        pad.x as usize,
        pad.width as usize,
        pad.height as usize,
        classic::help_panel_style(),
    );
    draw_box(
        buffer,
        popup,
        spec.title,
        classic::table_chrome_style(),
        classic::table_header_style(),
    );
    buffer.fill_rect(
        popup.y as usize + 1,
        popup.x as usize + 1,
        popup.width.saturating_sub(2) as usize,
        popup.height.saturating_sub(2) as usize,
        classic::help_panel_style(),
    );
    let mut row = popup.y as usize + 1;
    let col = popup.x as usize + 2;
    for line in spec.rows.iter().map(format_help_row) {
        buffer.write_text_clipped(row, col, &line, classic::help_panel_style());
        row += 1;
    }
    if let Some(note) = spec.note {
        buffer.write_text_clipped(row, col, note, classic::help_panel_style());
    }
    draw_command_line_prompt_text_at(buffer, COMMAND_ROW, "COMMANDS", "<Q> <?> ->");
    buffer.clear_cursor();
}

fn format_help_row(row: &super::help::HelpRow) -> String {
    format!("{:<4} {}", row.command, row.description)
}
