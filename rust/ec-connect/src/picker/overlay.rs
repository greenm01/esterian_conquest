use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ec_ui::buffer::{CellStyle, PlayfieldBuffer};
use ec_ui::modal::{ModalTheme, Rect};
use ec_ui::prompt::{draw_command_line_prompt_text_at, draw_command_line_prompt_text_at_col};
use ec_ui::theme::classic;

use super::connecting::cancel_active_connect;
use super::connecting::render_connecting_popup;
use super::event::{is_back_key, is_cancel_confirm_key, is_yes_key};
use super::help::HelpTopic;
use super::input::handle_maps_download_popup_key;
use super::layout::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH, truncate};
use super::refresh::render_refreshing_popup;
use super::relay::{
    RelayPromptAction, handle_game_relay_key, handle_relay_editor_key, render_game_relay_popup,
    render_relay_editor_popup,
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
    MapsDownloaded {
        path: PathBuf,
    },
    MapsDownloadPrompt {
        error: Option<String>,
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
    RelayEditor {
        original_url: Option<String>,
        title: String,
        instruction: String,
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
    RelayDeleteConfirm {
        url: String,
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
        PickerOverlay::MapsDownloaded { .. } => {
            state.overlay = None;
        }
        PickerOverlay::MapsDownloadPrompt { .. } => {
            handle_maps_download_popup_key(key, state, picker_session.as_deref(), gate_npub, rt)?;
        }
        PickerOverlay::ClaimingInvite { .. } | PickerOverlay::Connecting { .. } => {
            if is_back_key(key) {
                cancel_active_connect(state);
            }
        }
        PickerOverlay::RefreshingGame { .. } => {}
        PickerOverlay::Help(_) => {
            if !key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::ALT)
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
        PickerOverlay::RelayEditor {
            original_url,
            title,
            instruction,
            ..
        } => {
            handle_relay_editor_key(key, state, original_url.as_deref(), &title, &instruction)?;
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
        PickerOverlay::RelayDeleteConfirm { url, step } => {
            if is_yes_key(key) {
                if step < 3 {
                    state.overlay = Some(PickerOverlay::RelayDeleteConfirm {
                        url,
                        step: step + 1,
                    });
                } else {
                    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                        if state
                            .cache
                            .games
                            .iter()
                            .any(|game| game.relay_url.as_deref() == Some(url.as_str()))
                        {
                            return Err(
                                "relay still has joined games; move or delete them first".into()
                            );
                        }
                        let mut config = crate::config::load_config()
                            .unwrap_or_else(|_| crate::config::ConnectConfig::empty());
                        if !config.remove_relay(&url) {
                            return Err("selected relay no longer exists".into());
                        }
                        crate::config::save_config(&config)?;
                        Ok(())
                    })();
                    state.overlay = None;
                    clamp_relay_selection(state);
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
        Some(PickerOverlay::MapsDownloaded { path }) => {
            render_maps_downloaded_popup(buffer, path);
            buffer.clear_cursor();
        }
        Some(PickerOverlay::MapsDownloadPrompt { error }) => {
            render_maps_download_popup(buffer, &state.maps_input, error.as_deref());
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
        Some(PickerOverlay::RelayEditor {
            title,
            instruction,
            error,
            ..
        }) => {
            render_relay_editor_popup(
                buffer,
                title,
                instruction,
                &state.relay_input,
                error.as_deref(),
            );
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
                let popup = render_wallet_delete_popup(buffer, state, session, *index);
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
        Some(PickerOverlay::RelayDeleteConfirm { url, step }) => {
            let popup = render_relay_delete_popup(buffer, state, url);
            draw_command_line_prompt_text_at_col(
                buffer,
                popup_command_row(popup, command_row),
                popup.x as usize,
                "COMMAND",
                delete_prompt(*step),
            );
            buffer.clear_cursor();
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
    let mut lines =
        ec_ui::modal::format_help_rows(spec.rows.iter().map(|row| (row.command, row.description)));
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

fn render_maps_downloaded_popup(buffer: &mut PlayfieldBuffer, path: &PathBuf) {
    let mut lines = vec![
        "The starmap bundle for this game was downloaded.".to_string(),
        String::new(),
        "Saved to:".to_string(),
    ];
    lines.extend(wrapped_lines(
        &path.display().to_string(),
        PLAYFIELD_WIDTH.saturating_sub(18),
    ));
    lines.push(String::new());
    lines.push("Press any key to continue.".to_string());
    render_modal_box(buffer, "MAPS DOWNLOADED", &lines, classic::table_body_style());
}

fn render_maps_download_popup(buffer: &mut PlayfieldBuffer, input: &str, error: Option<&str>) {
    let has_error = error.is_some();
    let height: u16 = if has_error { 10 } else { 9 };
    let popup = draw_modal_frame(buffer, "DOWNLOAD MAPS", 76, height, classic::table_body_style());
    let left = popup.x as usize + 2;
    let inner_right = popup.x as usize + popup.width as usize - 2;
    let inner_width = popup.width.saturating_sub(4) as usize;

    let instruction =
        "Set the default folder for downloaded maps. Game folders are created under it automatically.";
    for (idx, line) in wrapped_lines(instruction, inner_width)
        .into_iter()
        .take(2)
        .enumerate()
    {
        buffer.write_text_clipped(
            popup.y as usize + 1 + idx,
            left,
            &line,
            classic::table_body_style(),
        );
    }

    let label = "Save to:";
    let input_row = popup.y as usize + 4;
    let input_col = left + label.chars().count() + 1;
    draw_labeled_input_row(
        buffer,
        input_row,
        left,
        label,
        input,
        input_width(inner_right, input_col),
        classic::status_label_style(),
        classic::prompt_hotkey_style(),
    );

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
        &truncate("Enter=save+download   Esc=cancel   Backspace=erase", inner_width),
        classic::table_chrome_style(),
    );
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
    state: &PickerState,
    session: &PickerSession,
    index: usize,
) -> Rect {
    let Some(identity) = session.selected_identity(index) else {
        return Rect::new(0, 0, 0, 0);
    };
    let npub = crate::wallet::identity_npub(identity).unwrap_or_else(|_| "<invalid>".to_string());
    let affected_games = state
        .cache
        .games
        .iter()
        .filter(|game| game.npub == npub)
        .count();
    let lines = [
        format!("Alias: {}", identity.alias.as_deref().unwrap_or("(none)")),
        format!("Npub: {}", super::render::short_npub(&npub)),
        String::new(),
        "Deleting this identity removes its keypair from this wallet.".to_string(),
        format!("Joined games removed from picker: {}", affected_games),
        "Make sure you have copied the full nsec somewhere safe first.".to_string(),
    ];
    render_modal_box(
        buffer,
        "DELETE IDENTITY",
        &lines,
        classic::table_body_style(),
    )
}

fn render_relay_delete_popup(buffer: &mut PlayfieldBuffer, state: &PickerState, url: &str) -> Rect {
    let game_count = state
        .cache
        .games
        .iter()
        .filter(|game| game.relay_url.as_deref() == Some(url))
        .count();
    let config =
        crate::config::load_config().unwrap_or_else(|_| crate::config::ConnectConfig::empty());
    let is_default = config.default_relay_url() == Some(url);
    let lines = [
        format!("Relay: {}", truncate(url, 50)),
        format!("Default: {}", if is_default { "yes" } else { "no" }),
        format!("Joined games on this relay: {}", game_count),
        String::new(),
        "This removes the relay from your local config only.".to_string(),
        "Joined games must be moved off this relay first.".to_string(),
    ];
    render_modal_box(buffer, "DELETE RELAY", &lines, classic::table_body_style())
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

fn clamp_relay_selection(state: &mut PickerState) {
    let len = super::relay::relay_summaries(state).len();
    if len == 0 {
        state.relay_selected = 0;
    } else if state.relay_selected >= len {
        state.relay_selected = len - 1;
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
    ec_ui::modal::draw_modal_frame(
        buffer,
        title,
        ((preferred_width.min(PLAYFIELD_WIDTH.saturating_sub(8)) * 100) / PLAYFIELD_WIDTH).max(40),
        height,
        ModalTheme {
            body_style,
            pad_style: classic::help_panel_style(),
            chrome_style: classic::table_chrome_style(),
            title_style: classic::table_header_style(),
        },
    )
}

fn render_modal_box(
    buffer: &mut PlayfieldBuffer,
    title: &str,
    lines: &[String],
    body_style: ec_ui::buffer::CellStyle,
) -> Rect {
    ec_ui::modal::render_modal_box(
        buffer,
        title,
        lines,
        ModalTheme {
            body_style,
            pad_style: classic::help_panel_style(),
            chrome_style: classic::table_chrome_style(),
            title_style: classic::table_header_style(),
        },
    )
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

    let instruction = "Paste the invite code from your sysop, then press Enter.";
    for (idx, line) in wrapped_lines(instruction, inner_width)
        .into_iter()
        .take(2)
        .enumerate()
    {
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
        buffer.set_cell(
            input_row,
            input_col + offset,
            ' ',
            classic::prompt_hotkey_style(),
        );
    }
    let cursor_col = input_col
        + buffer.write_text_clipped(
            input_row,
            input_col,
            &visible,
            classic::prompt_hotkey_style(),
        );
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
