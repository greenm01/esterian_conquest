use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ec_ui::buffer::{CellStyle, PlayfieldBuffer};
use ec_ui::prompt::{draw_command_line_prompt_text_at, draw_command_line_prompt_text_at_col};
use ec_ui::theme::classic;

use super::connecting::render_connecting_popup;
use super::event::{is_back_key, is_cancel_confirm_key, is_help_key, is_yes_key};
use super::help::HelpTopic;
use super::layout::{
    PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH, Rect, centered_rect, draw_box, format_help_row, truncate,
};
use super::refresh::render_refreshing_popup;
use super::relay::{
    RelayPromptAction, handle_default_relay_key, handle_game_relay_key, render_default_relay_popup,
    render_game_relay_popup,
};
use super::state::{PickerSession, PickerState};
use crate::cache::save_cache;
use crate::input_field::{draw_labeled_input_row, input_width};
use crate::wallet::{delete_identity, set_identity_alias};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeLevel {
    Notice,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerOverlay {
    Notice {
        level: NoticeLevel,
        message: String,
    },
    ClaimingInvite {
        lines: Vec<String>,
    },
    Connecting {
        lines: Vec<String>,
    },
    RefreshingGame {
        lines: Vec<String>,
    },
    Help(HelpTopic),
    QuitConfirm,
    DefaultRelayEditor {
        error: Option<String>,
    },
    GameRelayPrompt {
        index: usize,
        action: RelayPromptAction,
        error: Option<String>,
    },
    WalletDetail {
        index: usize,
    },
    WalletDeleteConfirm {
        index: usize,
        step: u8,
    },
    GameDeleteConfirm {
        index: usize,
        step: u8,
    },
    JoinCodePopup {
        error: Option<String>,
    },
}

pub fn handle_overlay_key(
    key: KeyEvent,
    state: &mut PickerState,
    picker_session: Option<&mut PickerSession>,
    gate_npub: &str,
    maps_root: &std::path::Path,
    rt: Option<&tokio::runtime::Runtime>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(current) = state.overlay.clone() else {
        return Ok(());
    };

    match current {
        PickerOverlay::Notice { level, .. } => {
            if level == NoticeLevel::Error
                || is_back_key(key)
                || matches!(key.code, crossterm::event::KeyCode::Enter)
            {
                state.overlay = None;
            }
        }
        PickerOverlay::ClaimingInvite { .. }
        | PickerOverlay::Connecting { .. }
        | PickerOverlay::RefreshingGame { .. } => {}
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
        PickerOverlay::DefaultRelayEditor { .. } => {
            handle_default_relay_key(key, state)?;
        }
        PickerOverlay::GameRelayPrompt { index, action, .. } => {
            let Some(picker_session) = picker_session else {
                return Ok(());
            };
            let Some(rt) = rt else {
                return Ok(());
            };
            handle_game_relay_key(
                key,
                index,
                action,
                state,
                &picker_session.keys,
                gate_npub,
                maps_root,
                rt,
            )?;
        }
        PickerOverlay::WalletDetail { index } => {
            let Some(picker_session) = picker_session else {
                return Ok(());
            };
            match key.code {
                KeyCode::Enter => {
                    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                        set_identity_alias(
                            &mut picker_session.wallet,
                            index,
                            Some(state.alias_input.clone()),
                        )?;
                        picker_session.save()?;
                        Ok(())
                    })();
                    state.alias_input.clear();
                    state.overlay = None;
                    if let Err(err) = result {
                        state.show_error(err.to_string());
                    }
                }
                KeyCode::Backspace => {
                    state.alias_input.pop();
                }
                KeyCode::Char(ch)
                    if matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT) =>
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
                    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                        let npub = delete_identity(&mut picker_session.wallet, index)?;
                        let _ = state.cache.remove_by_npub(&npub);
                        save_cache(&state.cache)?;
                        picker_session.refresh_active_identity()?;
                        picker_session.save()?;
                        Ok(())
                    })();
                    state.overlay = None;
                    state.wallet_selected = state
                        .wallet_selected
                        .min(picker_session.wallet.identities.len().saturating_sub(1));
                    clamp_game_selection(state);
                    if let Err(err) = result {
                        state.show_error(err.to_string());
                    }
                }
            } else if is_cancel_confirm_key(key) {
                state.overlay = None;
            }
        }
        PickerOverlay::GameDeleteConfirm { index, step } => {
            if is_yes_key(key) {
                if step < 3 {
                    state.overlay = Some(PickerOverlay::GameDeleteConfirm {
                        index,
                        step: step + 1,
                    });
                } else {
                    let game_id = state.cache.sorted().get(index).map(|game| game.id.clone());
                    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                        let Some(game_id) = game_id else {
                            return Err("selected game no longer exists".into());
                        };
                        if !state.cache.remove(&game_id) {
                            return Err("selected game no longer exists".into());
                        }
                        save_cache(&state.cache)?;
                        Ok(())
                    })();
                    state.overlay = None;
                    clamp_game_selection(state);
                    if let Err(err) = result {
                        state.show_error(err.to_string());
                    }
                }
            } else if is_cancel_confirm_key(key) {
                state.overlay = None;
            }
        }
        PickerOverlay::JoinCodePopup { .. } => {
            super::input::handle_join_code_popup_key(key, state, gate_npub)?;
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
            render_notice_popup(buffer, *level, message);
            buffer.clear_cursor();
        }
        Some(PickerOverlay::ClaimingInvite { lines }) => {
            super::connecting::render_status_popup(buffer, "CLAIMING INVITE", lines);
        }
        Some(PickerOverlay::Connecting { lines }) => {
            render_connecting_popup(buffer, lines);
        }
        Some(PickerOverlay::RefreshingGame { lines }) => {
            render_refreshing_popup(buffer, lines);
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
        Some(PickerOverlay::DefaultRelayEditor { error }) => {
            render_default_relay_popup(buffer, &state.relay_input, error.as_deref());
        }
        Some(PickerOverlay::GameRelayPrompt { action, error, .. }) => {
            render_game_relay_popup(buffer, &state.relay_input, error.as_deref(), *action);
        }
        Some(PickerOverlay::WalletDetail { index }) => {
            if let Some(session) = session {
                render_wallet_detail_popup(buffer, session, *index, &state.alias_input);
            }
        }
        Some(PickerOverlay::WalletDeleteConfirm { index, step }) => {
            if let Some(session) = session {
                let popup = render_wallet_delete_popup(buffer, session, *index);
                draw_command_line_prompt_text_at_col(
                    buffer,
                    popup_command_row(popup, command_row),
                    popup.x as usize,
                    "WALLET COMMAND",
                    delete_prompt(*step),
                );
                buffer.clear_cursor();
            }
        }
        Some(PickerOverlay::GameDeleteConfirm { index, step }) => {
            let popup = render_game_delete_popup(buffer, state, *index);
            draw_command_line_prompt_text_at_col(
                buffer,
                popup_command_row(popup, command_row),
                popup.x as usize,
                "COMMAND",
                delete_prompt(*step),
            );
            buffer.clear_cursor();
        }
        Some(PickerOverlay::JoinCodePopup { error }) => {
            render_join_code_popup(buffer, &state.join_input, error.as_deref());
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

pub fn render_wallet_add_popup(buffer: &mut PlayfieldBuffer, input: &str) {
    let popup = draw_modal_frame(
        buffer,
        "ADD OR IMPORT IDENTITY",
        72,
        7,
        classic::table_body_style(),
    );
    let left = popup.x as usize + 2;
    let field_row = popup.y as usize + 4;
    buffer.write_text_clipped(
        popup.y as usize + 1,
        left,
        "Paste an nsec or leave blank to create a new keypair.",
        classic::table_body_style(),
    );
    let label = "Nsec:";
    let input_col = left + label.chars().count() + 1;
    draw_labeled_input_row(
        buffer,
        field_row,
        left,
        label,
        input,
        input_width(popup.x as usize + popup.width as usize - 2, input_col),
        classic::status_label_style(),
        classic::prompt_hotkey_style(),
    );
}

fn render_notice_popup(buffer: &mut PlayfieldBuffer, level: NoticeLevel, message: &str) {
    let title = match level {
        NoticeLevel::Notice => "NOTICE",
        NoticeLevel::Error => "ERROR",
    };
    let style = match level {
        NoticeLevel::Notice => classic::table_body_style(),
        NoticeLevel::Error => classic::error_style(),
    };
    let lines = wrapped_lines(message, PLAYFIELD_WIDTH.saturating_sub(14));
    render_modal_box(buffer, title, &lines, style);
}

fn render_wallet_detail_popup(
    buffer: &mut PlayfieldBuffer,
    session: &PickerSession,
    index: usize,
    alias_input: &str,
) {
    let Some(identity) = session.selected_identity(index) else {
        return;
    };
    let npub = crate::wallet::identity_npub(identity).unwrap_or_else(|_| "<invalid>".to_string());
    let popup = draw_modal_frame(
        buffer,
        "WALLET IDENTITY",
        72,
        11,
        classic::table_body_style(),
    );
    let left = popup.x as usize + 2;
    let mut row = popup.y as usize + 1;
    let label = "Alias:";
    let input_col = left + label.chars().count() + 1;
    draw_labeled_input_row(
        buffer,
        row,
        left,
        label,
        alias_input,
        input_width(popup.x as usize + popup.width as usize - 2, input_col),
        classic::status_label_style(),
        classic::prompt_hotkey_style(),
    );
    row += 2;
    buffer.write_text_clipped(
        row,
        left,
        &format!("Type: {}", identity.identity_type.as_str()),
        classic::table_body_style(),
    );
    row += 1;
    buffer.write_text_clipped(
        row,
        left,
        &format!(
            "Created: {}",
            truncate(&identity.created, popup.width.saturating_sub(6) as usize)
        ),
        classic::table_body_style(),
    );
    row += 2;
    buffer.write_text_clipped(row, left, "Npub:", classic::status_label_style());
    row += 1;
    for line in wrapped_lines(&npub, popup.width.saturating_sub(6) as usize) {
        buffer.write_text_clipped(row, left, &line, classic::table_body_style());
        row += 1;
    }
    buffer.write_text_clipped(row, left, "Nsec:", classic::status_label_style());
    row += 1;
    let nsec_line = truncate(&identity.nsec, popup.width.saturating_sub(6) as usize);
    buffer.write_text_clipped(row, left, &nsec_line, classic::table_body_style());
}

fn render_wallet_delete_popup(
    buffer: &mut PlayfieldBuffer,
    session: &PickerSession,
    index: usize,
) -> Rect {
    let Some(identity) = session.selected_identity(index) else {
        return Rect::new(0, 0, 0, 0);
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
    )
}

fn render_game_delete_popup(
    buffer: &mut PlayfieldBuffer,
    state: &PickerState,
    index: usize,
) -> Rect {
    let sorted = state.cache.sorted();
    let Some(game) = sorted.get(index) else {
        return Rect::new(0, 0, 0, 0);
    };
    let lines = [
        format!(
            "Empire: {}",
            truncate(game.player_name.as_deref().unwrap_or(""), 48)
        ),
        format!("Game: {}", truncate(&game.name, 50)),
        format!(
            "Server: {}",
            truncate(&format!("{}:{}", game.server, game.port), 48)
        ),
        format!("Seat: {}", game.seat),
        String::new(),
        "This removes the game from your local picker only.".to_string(),
        "It does not delete the wallet identity or the remote seat.".to_string(),
    ];
    render_modal_box(buffer, "DELETE GAME", &lines, classic::table_body_style())
}

fn delete_prompt(step: u8) -> &'static str {
    match step {
        1 => "Are you sure? Y/[N] ->",
        2 => "Are you really sure? Y/[N] ->",
        _ => "Are you sure-sure? Last chance to bail! Y/[N] ->",
    }
}

fn clamp_game_selection(state: &mut PickerState) {
    let len = state.cache.sorted().len();
    if len == 0 {
        state.selected = 0;
    } else if state.selected >= len {
        state.selected = len - 1;
    }
}

fn wrapped_lines(text: &str, max_width: usize) -> Vec<String> {
    if max_width <= 1 {
        return vec![truncate(text, 1)];
    }
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let word_len = word.chars().count();
        if word_len > max_width {
            if !current.is_empty() {
                lines.push(current);
                current = String::new();
            }
            let mut rest = word;
            while rest.chars().count() > max_width {
                let chunk = rest.chars().take(max_width).collect::<String>();
                lines.push(chunk);
                rest = &rest[rest
                    .char_indices()
                    .nth(max_width)
                    .map(|(idx, _)| idx)
                    .unwrap_or(rest.len())..];
            }
            if !rest.is_empty() {
                current.push_str(rest);
            }
            continue;
        }

        let current_len = current.chars().count();
        let needed = if current.is_empty() {
            word_len
        } else {
            current_len + 1 + word_len
        };
        if needed > max_width && !current.is_empty() {
            lines.push(current);
            current = word.to_string();
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
    }

    if lines.is_empty() && current.is_empty() {
        lines.push(String::new());
    } else if !current.is_empty() {
        lines.push(current);
    }

    lines
}

pub(crate) fn draw_modal_frame(
    buffer: &mut PlayfieldBuffer,
    title: &str,
    preferred_width: usize,
    height: u16,
    body_style: CellStyle,
) -> Rect {
    let popup = centered_rect(
        ((preferred_width.min(PLAYFIELD_WIDTH.saturating_sub(8)) * 100) / PLAYFIELD_WIDTH).max(40)
            as u16,
        height,
        Rect::new(0, 0, PLAYFIELD_WIDTH as u16, PLAYFIELD_HEIGHT as u16),
    );
    let popup = Rect::new(
        popup.x,
        popup.y,
        popup.width.min(PLAYFIELD_WIDTH as u16 - popup.x),
        popup.height.min(PLAYFIELD_HEIGHT as u16 - popup.y),
    );
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
    popup
}

fn render_modal_box(
    buffer: &mut PlayfieldBuffer,
    title: &str,
    lines: &[String],
    body_style: ec_ui::buffer::CellStyle,
) -> Rect {
    let content_width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let width = (content_width + 4)
        .max(title.chars().count() + 4)
        .min(PLAYFIELD_WIDTH.saturating_sub(8));
    let height = (lines.len() + 2) as u16;
    let popup = draw_modal_frame(buffer, title, width, height, body_style);
    let mut row = popup.y as usize + 1;
    let col = popup.x as usize + 2;
    for line in lines {
        buffer.write_text_clipped(row, col, line, body_style);
        row += 1;
    }
    popup
}

fn popup_command_row(popup: Rect, fallback: usize) -> usize {
    if popup.width == 0 || popup.height == 0 {
        return fallback;
    }
    let row = popup.y as usize + popup.height as usize;
    row.min(PLAYFIELD_HEIGHT.saturating_sub(1))
}

fn render_join_code_popup(buffer: &mut PlayfieldBuffer, input: &str, error: Option<&str>) {
    let has_error = error.is_some();
    let height: u16 = if has_error { 9 } else { 8 };
    let popup = draw_modal_frame(buffer, "JOIN GAME", 76, height, classic::table_body_style());

    let left = popup.x as usize + 2;
    let inner_right = popup.x as usize + popup.width as usize - 2;
    let inner_width = popup.width.saturating_sub(4) as usize;

    let instruction = "Paste the ecinv1... invite code from your sysop, then press Enter.";
    for (idx, line) in wrapped_lines(instruction, inner_width).into_iter().take(2).enumerate() {
        buffer.write_text_clipped(
            popup.y as usize + 1 + idx,
            left,
            &line,
            classic::table_body_style(),
        );
    }

    let label = "Invite:";
    let input_row = popup.y as usize + 4;
    let input_col = left + label.chars().count() + 1;
    let width = input_width(inner_right, input_col);
    let visible = compact_invite_input(input, width);
    buffer.write_text_clipped(input_row, left, label, classic::status_label_style());
    for offset in 0..width {
        buffer.set_cell(input_row, input_col + offset, ' ', classic::prompt_hotkey_style());
    }
    let cursor_col = input_col
        + buffer.write_text_clipped(input_row, input_col, &visible, classic::prompt_hotkey_style());
    if cursor_col < buffer.width() {
        buffer.set_cursor(cursor_col as u16, input_row as u16);
    }

    if let Some(err) = error {
        buffer.write_text_clipped(
            popup.y as usize + 6,
            left,
            &truncate(err, inner_width),
            classic::error_style(),
        );
    }

    let hint_row = if has_error {
        popup.y as usize + 7
    } else {
        popup.y as usize + 6
    };
    buffer.write_text_clipped(
        hint_row,
        left,
        &truncate("Enter=join   Esc=cancel   Backspace=erase", inner_width),
        classic::table_chrome_style(),
    );
}

fn compact_invite_input(input: &str, width: usize) -> String {
    let width = width.max(1);
    let chars: Vec<char> = input.chars().collect();
    if chars.len() <= width {
        return input.to_string();
    }
    if width <= 10 {
        return chars[chars.len() - width..].iter().collect();
    }

    let head_len = 6.min(width.saturating_sub(4));
    let tail_len = width.saturating_sub(head_len + 3);
    let head: String = chars[..head_len].iter().collect();
    let tail: String = chars[chars.len() - tail_len..].iter().collect();
    format!("{head}...{tail}")
}
