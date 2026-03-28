use crossterm::event::KeyEvent;
use ec_ui::buffer::PlayfieldBuffer;
use ec_ui::prompt::draw_command_line_prompt_text_at;
use ec_ui::theme::classic;

use super::event::{is_back_key, is_cancel_confirm_key, is_help_key, is_yes_key};
use super::help::HelpTopic;
use super::layout::{
    PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH, Rect, centered_rect, draw_box, format_help_row,
};
use super::state::{PickerSession, PickerState};
use crate::wallet::{delete_identity, set_identity_alias};

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
    WalletDetail { index: usize },
    WalletDeleteConfirm { index: usize, step: u8 },
}

pub fn handle_overlay_key(
    key: KeyEvent,
    state: &mut PickerState,
    picker_session: Option<&mut PickerSession>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(current) = state.overlay.clone() else {
        return Ok(());
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
        PickerOverlay::WalletDetail { index } => {
            let Some(picker_session) = picker_session else {
                return Ok(());
            };
            match key.code {
                crossterm::event::KeyCode::Enter => {
                    set_identity_alias(
                        &mut picker_session.wallet,
                        index,
                        Some(state.alias_input.clone()),
                    )?;
                    picker_session.save()?;
                    state.alias_input.clear();
                    state.overlay = None;
                    state.show_notice("Alias updated.");
                }
                crossterm::event::KeyCode::Backspace => {
                    state.alias_input.pop();
                }
                crossterm::event::KeyCode::Char(ch)
                    if matches!(
                        key.modifiers,
                        crossterm::event::KeyModifiers::NONE
                            | crossterm::event::KeyModifiers::SHIFT
                    ) =>
                {
                    if state.alias_input.chars().count() < 20 {
                        state.alias_input.push(ch);
                    }
                }
                _ if is_back_key(key) => {
                    state.alias_input.clear();
                    state.overlay = None;
                }
                _ => {}
            }
        }
        PickerOverlay::WalletDeleteConfirm { index, step } => {
            let Some(picker_session) = picker_session else {
                return Ok(());
            };
            if is_yes_key(key) {
                if step < 3 {
                    state.overlay = Some(PickerOverlay::WalletDeleteConfirm {
                        index,
                        step: step + 1,
                    });
                } else {
                    let npub = delete_identity(&mut picker_session.wallet, index)?;
                    picker_session.refresh_active_identity()?;
                    picker_session.save()?;
                    state.wallet_selected = state
                        .wallet_selected
                        .min(picker_session.wallet.identities.len().saturating_sub(1));
                    state.overlay = None;
                    state.show_notice(format!(
                        "Deleted identity: {}",
                        super::render::short_npub(&npub)
                    ));
                }
            } else if is_cancel_confirm_key(key) {
                state.overlay = None;
            }
        }
    }
    Ok(())
}

pub fn render_overlay(
    buffer: &mut PlayfieldBuffer,
    state: &PickerState,
    session: Option<&PickerSession>,
    command_row: usize,
) {
    match state.overlay.as_ref() {
        Some(PickerOverlay::Notice { level, message }) => {
            let label = match level {
                NoticeLevel::Notice => "NOTICE",
                NoticeLevel::Error => "ERROR",
            };
            let prompt = format!("{message} <Q> ->");
            draw_command_line_prompt_text_at(buffer, command_row, label, &prompt);
            buffer.clear_cursor();
        }
        Some(PickerOverlay::Help(topic)) => {
            render_help_overlay(buffer, *topic);
        }
        Some(PickerOverlay::QuitConfirm) => {
            draw_command_line_prompt_text_at(
                buffer,
                command_row,
                "COMMAND",
                "Are you sure Y/[N] ->",
            );
            buffer.clear_cursor();
        }
        Some(PickerOverlay::WalletDetail { index }) => {
            if let Some(session) = session {
                render_wallet_detail_popup(buffer, session, *index);
                let prompt = format!("Alias <Q> -> {}", state.alias_input);
                draw_command_line_prompt_text_at(buffer, command_row, "WALLET COMMAND", &prompt);
            }
        }
        Some(PickerOverlay::WalletDeleteConfirm { index, step }) => {
            if let Some(session) = session {
                render_wallet_delete_popup(buffer, session, *index);
                draw_command_line_prompt_text_at(
                    buffer,
                    command_row,
                    "WALLET COMMAND",
                    wallet_delete_prompt(*step),
                );
                buffer.clear_cursor();
            }
        }
        None => {}
    }
}

pub fn render_identity_popup(buffer: &mut PlayfieldBuffer, session: &PickerSession) {
    let lines = [
        format!("Alias: {}", session.active_alias().unwrap_or("(none)")),
        format!("Npub: {}", super::render::short_npub(&session.npub)),
        format!("Type: {}", session.active_identity_type()),
        format!("Wallet identities: {}", session.wallet.identities.len()),
        format!(
            "Wallet: {}",
            super::render::truncate(&crate::wallet::io::wallet_path().display().to_string(), 46,)
        ),
    ];
    render_modal_box(buffer, "IDENTITY INFO", &lines, classic::table_body_style());
}

fn render_help_overlay(buffer: &mut PlayfieldBuffer, topic: HelpTopic) {
    let spec = topic.spec();
    let mut lines = spec
        .rows
        .iter()
        .map(|row| format_help_row(row.command, row.description))
        .collect::<Vec<_>>();
    if let Some(note) = spec.note {
        lines.push(note.to_string());
    }
    render_modal_box(buffer, spec.title, &lines, classic::help_panel_style());
    buffer.clear_cursor();
}

fn render_wallet_detail_popup(buffer: &mut PlayfieldBuffer, session: &PickerSession, index: usize) {
    let Some(identity) = session.selected_identity(index) else {
        return;
    };
    let npub = crate::wallet::identity_npub(identity).unwrap_or_else(|_| "<invalid>".to_string());
    let lines = [
        format!("Alias: {}", identity.alias.as_deref().unwrap_or("(none)")),
        format!("Type: {}", identity.identity_type.as_str()),
        format!("Created: {}", identity.created),
        String::new(),
        "Npub:".to_string(),
        npub,
        String::new(),
        "Nsec:".to_string(),
        identity.nsec.clone(),
    ];
    render_modal_box(
        buffer,
        "WALLET IDENTITY",
        &lines,
        classic::table_body_style(),
    );
}

fn render_wallet_delete_popup(buffer: &mut PlayfieldBuffer, session: &PickerSession, index: usize) {
    let Some(identity) = session.selected_identity(index) else {
        return;
    };
    let npub = crate::wallet::identity_npub(identity).unwrap_or_else(|_| "<invalid>".to_string());
    let lines = [
        format!("Alias: {}", identity.alias.as_deref().unwrap_or("(none)")),
        format!("Npub: {}", super::render::short_npub(&npub)),
        String::new(),
        "Deleting this identity removes its keypair from this wallet.".to_string(),
        "Make sure you have copied the full nsec somewhere safe first.".to_string(),
    ];
    render_modal_box(
        buffer,
        "DELETE IDENTITY",
        &lines,
        classic::table_body_style(),
    );
}

fn wallet_delete_prompt(step: u8) -> &'static str {
    match step {
        1 => "Are you sure? Y/[N] ->",
        2 => "Are you really sure? Y/[N] ->",
        _ => "Are you sure-sure? Last chance to bail! Y/[N] ->",
    }
}

fn render_modal_box(
    buffer: &mut PlayfieldBuffer,
    title: &str,
    lines: &[String],
    body_style: ec_ui::buffer::CellStyle,
) {
    let content_width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let width = (content_width + 4)
        .max(title.chars().count() + 2)
        .min(PLAYFIELD_WIDTH.saturating_sub(8));
    let popup_height = (lines.len() + 2) as u16;
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
        title,
        classic::table_chrome_style(),
        classic::table_header_style(),
    );
    buffer.fill_rect(
        popup.y as usize + 1,
        popup.x as usize + 1,
        popup.width.saturating_sub(2) as usize,
        popup.height.saturating_sub(2) as usize,
        body_style,
    );
    let mut row = popup.y as usize + 1;
    let col = popup.x as usize + 2;
    for line in lines {
        buffer.write_text_clipped(row, col, line, body_style);
        row += 1;
    }
}
