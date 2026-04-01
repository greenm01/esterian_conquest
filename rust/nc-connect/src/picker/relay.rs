use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nc_ui::buffer::PlayfieldBuffer;
use nc_ui::theme::classic;

use super::connecting::{PendingConnectRequest, queue_connect_request};
use super::event::{is_back_key, is_help_key};
use super::flows::{move_selection, persist_cached_game_relay};
use super::layout::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH, Rect, centered_rect, draw_box, truncate};
use super::state::{BODY_PAGE, ConnectDisplay, ConnectOrigin, PickerState, Screen};
use crate::cache::CachedGame;
use crate::config::{
    ConnectConfig, RelayEntry, RelayStatus, load_config, save_config, update_relay_result,
    validate_relay_url,
};
use crate::connect::map_fetch::fetch_map_bundle;
use crate::connect::resolve::{derive_relay_url, resolve_server};
use crate::input_field::{draw_labeled_input_row, input_width};
use crate::map_store::save_map_bundle;
use crate::text_wrap::{wrapped_lines, write_wrapped_lines_clamped};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayPromptAction {
    Connect,
    DownloadMaps,
    EditGame,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelaySummary {
    pub url: String,
    pub is_default: bool,
    pub status: RelayStatus,
    pub last_error: Option<String>,
    pub last_checked: Option<String>,
    pub game_count: usize,
}

pub fn relay_summaries(state: &PickerState) -> Vec<RelaySummary> {
    let config = load_config().unwrap_or_else(|_| ConnectConfig::empty());
    let mut relays = config.relays.clone();

    for game in &state.cache.games {
        let Some(url) = game.relay_url.as_deref().filter(|value| !value.is_empty()) else {
            continue;
        };
        if relays.iter().all(|relay| relay.url != url) {
            relays.push(RelayEntry {
                url: url.to_string(),
                is_default: false,
                status: RelayStatus::Unknown,
                last_error: None,
                last_checked: None,
            });
        }
    }

    let mut rows: Vec<_> = relays
        .into_iter()
        .map(|relay| RelaySummary {
            game_count: state
                .cache
                .games
                .iter()
                .filter(|game| game.relay_url.as_deref() == Some(relay.url.as_str()))
                .count(),
            url: relay.url,
            is_default: relay.is_default,
            status: relay.status,
            last_error: relay.last_error,
            last_checked: relay.last_checked,
        })
        .collect();

    rows.sort_by(|a, b| {
        b.is_default
            .cmp(&a.is_default)
            .then_with(|| b.game_count.cmp(&a.game_count))
            .then_with(|| a.url.cmp(&b.url))
    });
    rows
}

pub fn relay_games(state: &PickerState, relay_url: &str) -> Vec<CachedGame> {
    let mut games: Vec<_> = state
        .cache
        .games
        .iter()
        .filter(|game| game.relay_url.as_deref() == Some(relay_url))
        .cloned()
        .collect();
    games.sort_by(|a, b| {
        b.last_connected
            .as_deref()
            .cmp(&a.last_connected.as_deref())
            .then_with(|| b.joined.cmp(&a.joined))
    });
    games
}

pub fn open_relay_list(state: &mut PickerState) {
    state.screen = Screen::RelayList;
}

pub fn open_selected_game_relay_prompt(state: &mut PickerState) {
    let sorted = state.cache.sorted();
    let Some(game) = sorted.get(state.selected).copied().cloned() else {
        state.show_error("No joined games yet.");
        return;
    };
    let relay_url = game
        .relay_url
        .clone()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| derive_relay_url(&game.server));
    state.relay_input = relay_url;
    state.overlay = Some(super::overlay::PickerOverlay::GameRelayPrompt {
        index: state.selected,
        action: RelayPromptAction::EditGame,
        error: None,
    });
}

fn open_selected_relay_game_relay_prompt(state: &mut PickerState, relay_url: &str) {
    let games = relay_games(state, relay_url);
    let Some(game) = games.get(state.relay_game_selected) else {
        state.show_error("No joined games use this relay.");
        return;
    };
    let Some(index) = state
        .cache
        .sorted()
        .iter()
        .position(|candidate| candidate.id == game.id && candidate.npub == game.npub)
    else {
        state.show_error("selected game no longer exists");
        return;
    };
    state.relay_input = game
        .relay_url
        .clone()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| derive_relay_url(&game.server));
    state.overlay = Some(super::overlay::PickerOverlay::GameRelayPrompt {
        index,
        action: RelayPromptAction::EditGame,
        error: None,
    });
}

pub fn open_relay_editor(
    state: &mut PickerState,
    summary: Option<&RelaySummary>,
    error: Option<String>,
) {
    state.relay_input = summary.map(|relay| relay.url.clone()).unwrap_or_default();
    state.overlay = Some(super::overlay::PickerOverlay::RelayEditor {
        original_url: summary.map(|relay| relay.url.clone()),
        title: if summary.is_some() {
            "EDIT RELAY".to_string()
        } else {
            "ADD RELAY".to_string()
        },
        instruction: if summary.is_some() {
            "Update this relay URL. Joined games on this relay will move with it.".to_string()
        } else {
            "Add a relay for future joins or relay-grouped game management.".to_string()
        },
        error,
    });
}

pub fn handle_relay_editor_key(
    key: KeyEvent,
    state: &mut PickerState,
    original_url: Option<&str>,
    title: &str,
    instruction: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match key.code {
        KeyCode::Enter => {
            let relay_url = match validate_relay_url(&state.relay_input) {
                Ok(Some(relay)) => relay,
                Ok(None) => {
                    state.overlay = Some(super::overlay::PickerOverlay::RelayEditor {
                        original_url: original_url.map(str::to_string),
                        title: title.to_string(),
                        instruction: instruction.to_string(),
                        error: Some("relay URL must not be empty".to_string()),
                    });
                    return Ok(());
                }
                Err(err) => {
                    state.overlay = Some(super::overlay::PickerOverlay::RelayEditor {
                        original_url: original_url.map(str::to_string),
                        title: title.to_string(),
                        instruction: instruction.to_string(),
                        error: Some(err),
                    });
                    return Ok(());
                }
            };

            let mut config = load_config().unwrap_or_else(|_| ConnectConfig::empty());
            let mut was_default = false;
            if let Some(old_url) = original_url {
                was_default = config
                    .relay_entry(old_url)
                    .map(|relay| relay.is_default)
                    .unwrap_or(false);
                if old_url != relay_url && config.relay_entry(&relay_url).is_some() {
                    config.remove_relay(old_url);
                } else {
                    if let Some(entry) = config.relay_entry_mut(old_url) {
                        entry.url = relay_url.clone();
                        entry.status = RelayStatus::Unknown;
                        entry.last_error = None;
                        entry.last_checked = None;
                    } else {
                        config.upsert_relay(relay_url.clone());
                    }
                }
                for game in &mut state.cache.games {
                    if game.relay_url.as_deref() == Some(old_url) {
                        game.relay_url = Some(relay_url.clone());
                    }
                }
            } else {
                config.upsert_relay(relay_url.clone());
            }

            if was_default || config.default_relay_url().is_none() {
                config.set_default_relay(&relay_url);
            } else {
                config.normalize_relays();
            }

            match save_config(&config) {
                Ok(()) => {
                    crate::cache::save_cache(&state.cache)?;
                    state.relay_input.clear();
                    state.overlay = None;
                }
                Err(err) => {
                    state.overlay = Some(super::overlay::PickerOverlay::RelayEditor {
                        original_url: original_url.map(str::to_string),
                        title: title.to_string(),
                        instruction: instruction.to_string(),
                        error: Some(err.to_string()),
                    });
                }
            }
        }
        KeyCode::Backspace => {
            state.relay_input.pop();
        }
        KeyCode::Char(ch) if matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT) => {
            state.relay_input.push(ch);
        }
        _ if is_back_key(key) => {
            state.relay_input.clear();
            state.overlay = None;
        }
        _ => {}
    }
    Ok(())
}

pub fn handle_relay_list_key(
    key: KeyEvent,
    state: &mut PickerState,
) -> Result<(), Box<dyn std::error::Error>> {
    let relays = relay_summaries(state);
    let relay_count = relays.len();
    if is_help_key(key) {
        state.open_help();
        return Ok(());
    }

    match key {
        key if is_back_key(key) => state.screen = Screen::GameList,
        KeyEvent {
            code: KeyCode::Char('a' | 'A'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => open_relay_editor(state, None, None),
        KeyEvent {
            code: KeyCode::Char('e' | 'E'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => {
            if relay_count == 0 {
                state.show_error("No relays known yet.");
            } else {
                open_relay_editor(state, relays.get(state.relay_selected), None);
            }
        }
        KeyEvent {
            code: KeyCode::Char('s' | 'S'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => {
            let Some(selected) = relays.get(state.relay_selected) else {
                state.show_error("No relays known yet.");
                return Ok(());
            };
            let mut config = load_config().unwrap_or_else(|_| ConnectConfig::empty());
            config.set_default_relay(&selected.url);
            save_config(&config)?;
        }
        KeyEvent {
            code: KeyCode::Char('d' | 'D'),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } => {
            let Some(selected) = relays.get(state.relay_selected) else {
                state.show_error("No relays known yet.");
                return Ok(());
            };
            if selected.game_count > 0 {
                state.show_error("relay still has joined games; move or delete them first");
            } else {
                state.overlay = Some(super::overlay::PickerOverlay::RelayDeleteConfirm {
                    url: selected.url.clone(),
                    step: 1,
                });
            }
        }
        KeyEvent {
            code: KeyCode::Char('j'),
            modifiers: KeyModifiers::NONE,
            ..
        }
        | KeyEvent {
            code: KeyCode::Down,
            ..
        } => move_selection(&mut state.relay_selected, 1, relay_count),
        KeyEvent {
            code: KeyCode::Char('k'),
            modifiers: KeyModifiers::NONE,
            ..
        }
        | KeyEvent {
            code: KeyCode::Up, ..
        } => move_selection(&mut state.relay_selected, -1, relay_count),
        KeyEvent {
            code: KeyCode::Char('d'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }
        | KeyEvent {
            code: KeyCode::PageDown,
            ..
        } => move_selection(&mut state.relay_selected, BODY_PAGE, relay_count),
        KeyEvent {
            code: KeyCode::Char('u'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }
        | KeyEvent {
            code: KeyCode::PageUp,
            ..
        } => move_selection(&mut state.relay_selected, -BODY_PAGE, relay_count),
        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => {
            let Some(selected) = relays.get(state.relay_selected) else {
                state.show_error("No relays known yet.");
                return Ok(());
            };
            state.relay_game_selected = 0;
            state.screen = Screen::RelayGames {
                relay_url: selected.url.clone(),
            };
        }
        _ => {}
    }
    Ok(())
}

pub fn handle_relay_games_key(
    key: KeyEvent,
    state: &mut PickerState,
    relay_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let games = relay_games(state, relay_url);
    let game_count = games.len();
    if is_help_key(key) {
        state.open_help();
        return Ok(());
    }
    match key {
        key if is_back_key(key) => state.screen = Screen::RelayList,
        KeyEvent {
            code: KeyCode::Char('R'),
            modifiers: KeyModifiers::SHIFT,
            ..
        } => open_selected_relay_game_relay_prompt(state, relay_url),
        KeyEvent {
            code: KeyCode::Char('j'),
            modifiers: KeyModifiers::NONE,
            ..
        }
        | KeyEvent {
            code: KeyCode::Down,
            ..
        } => move_selection(&mut state.relay_game_selected, 1, game_count),
        KeyEvent {
            code: KeyCode::Char('k'),
            modifiers: KeyModifiers::NONE,
            ..
        }
        | KeyEvent {
            code: KeyCode::Up, ..
        } => move_selection(&mut state.relay_game_selected, -1, game_count),
        KeyEvent {
            code: KeyCode::Char('d'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }
        | KeyEvent {
            code: KeyCode::PageDown,
            ..
        } => move_selection(&mut state.relay_game_selected, BODY_PAGE, game_count),
        KeyEvent {
            code: KeyCode::Char('u'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }
        | KeyEvent {
            code: KeyCode::PageUp,
            ..
        } => move_selection(&mut state.relay_game_selected, -BODY_PAGE, game_count),
        _ => {}
    }
    Ok(())
}

pub fn render_relay_editor_popup(
    buffer: &mut PlayfieldBuffer,
    title: &str,
    instruction: &str,
    input: &str,
    error: Option<&str>,
) {
    let popup = draw_relay_popup_frame(buffer, title, error.map_or(0, relay_error_lines));
    render_relay_popup_body(buffer, popup, instruction, input, error);
}

pub fn relay_status_label(status: RelayStatus) -> &'static str {
    match status {
        RelayStatus::Unknown => "unknown",
        RelayStatus::Ok => "ok",
        RelayStatus::Timeout => "timeout",
        RelayStatus::ConnectFailed => "connect-fail",
        RelayStatus::ProtocolError => "protocol",
    }
}

pub fn remember_relay_success(relay_url: &str) {
    let _ = update_relay_result(relay_url, RelayStatus::Ok, None);
}

pub fn remember_relay_error(relay_url: &str, message: &str) {
    let lowered = message.to_ascii_lowercase();
    let status = if lowered.contains("timed out") {
        RelayStatus::Timeout
    } else if lowered.contains("relay")
        || lowered.contains("connect")
        || lowered.contains("websocket")
        || lowered.contains("dns")
    {
        RelayStatus::ConnectFailed
    } else {
        RelayStatus::ProtocolError
    };
    let _ = update_relay_result(relay_url, status, Some(message));
}

pub fn open_game_relay_prompt(
    state: &mut PickerState,
    index: usize,
    server_host: &str,
    action: RelayPromptAction,
) {
    state.relay_input = derive_relay_url(server_host);
    state.overlay = Some(super::overlay::PickerOverlay::GameRelayPrompt {
        index,
        action,
        error: None,
    });
}

pub fn handle_game_relay_key(
    key: KeyEvent,
    index: usize,
    action: RelayPromptAction,
    state: &mut PickerState,
    player_keys: &nostr_sdk::Keys,
    gate_npub: &str,
    rt: &tokio::runtime::Runtime,
) -> Result<(), Box<dyn std::error::Error>> {
    match key.code {
        KeyCode::Enter => {
            let relay_url = match validate_relay_url(&state.relay_input) {
                Ok(Some(relay)) => relay,
                Ok(None) => {
                    state.overlay = Some(super::overlay::PickerOverlay::GameRelayPrompt {
                        index,
                        action,
                        error: Some("relay URL must not be empty".to_string()),
                    });
                    return Ok(());
                }
                Err(err) => {
                    state.overlay = Some(super::overlay::PickerOverlay::GameRelayPrompt {
                        index,
                        action,
                        error: Some(err),
                    });
                    return Ok(());
                }
            };
            match action {
                RelayPromptAction::Connect => {
                    queue_prompted_relay_connect(state, gate_npub, index, relay_url)?
                }
                RelayPromptAction::DownloadMaps => submit_map_download_with_prompted_relay(
                    state,
                    player_keys,
                    gate_npub,
                    rt,
                    index,
                    relay_url,
                    action,
                )?,
                RelayPromptAction::EditGame => {
                    persist_cached_game_relay(state, index, &relay_url)?;
                    state.relay_input.clear();
                    state.overlay = None;
                }
            }
        }
        KeyCode::Backspace => {
            state.relay_input.pop();
        }
        KeyCode::Char(ch) if matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT) => {
            state.relay_input.push(ch);
        }
        _ if is_back_key(key) => {
            state.relay_input.clear();
            state.overlay = None;
        }
        _ => {}
    }
    Ok(())
}

pub fn render_game_relay_popup(
    buffer: &mut PlayfieldBuffer,
    input: &str,
    error: Option<&str>,
    action: RelayPromptAction,
) {
    let popup = draw_relay_popup_frame(buffer, "GAME RELAY", error.map_or(0, relay_error_lines));
    let instruction = match action {
        RelayPromptAction::Connect => {
            "This game has no saved relay yet. Enter the relay to reconnect."
        }
        RelayPromptAction::DownloadMaps => {
            "This game has no saved relay yet. Enter the relay to download maps."
        }
        RelayPromptAction::EditGame => "Set the relay for this joined game only.",
    };
    render_relay_popup_body(buffer, popup, instruction, input, error);
}

fn queue_prompted_relay_connect(
    state: &mut PickerState,
    gate_npub: &str,
    index: usize,
    relay_url: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let sorted = state.cache.sorted();
    let Some(game) = sorted.get(index).copied().cloned() else {
        state.overlay = Some(super::overlay::PickerOverlay::GameRelayPrompt {
            index,
            action: RelayPromptAction::Connect,
            error: Some("selected game no longer exists".to_string()),
        });
        return Ok(());
    };
    let config = load_config().unwrap_or_else(|_| ConnectConfig::empty());
    let server_str = format!("{}:{}", game.server, game.port);
    let mut target = match resolve_server(&server_str, &config) {
        Ok(target) => target,
        Err(err) => {
            state.overlay = Some(super::overlay::PickerOverlay::GameRelayPrompt {
                index,
                action: RelayPromptAction::Connect,
                error: Some(format!("unable to resolve server: {err}")),
            });
            return Ok(());
        }
    };
    target.relay_url = relay_url.clone();
    target.game_id = Some(game.id.clone());
    let effective_gate = if !game.gate_npub.is_empty() {
        game.gate_npub.clone()
    } else {
        gate_npub.to_string()
    };
    drop(sorted);

    queue_connect_request(
        state,
        PendingConnectRequest {
            origin: ConnectOrigin::GameRelayPrompt { index },
            display: ConnectDisplay::from_game(&game.name, &target),
            target,
            gate_npub: effective_gate,
        },
    );
    Ok(())
}

fn submit_map_download_with_prompted_relay(
    state: &mut PickerState,
    keys: &nostr_sdk::Keys,
    gate_npub: &str,
    rt: &tokio::runtime::Runtime,
    index: usize,
    relay_url: String,
    action: RelayPromptAction,
) -> Result<(), Box<dyn std::error::Error>> {
    let sorted = state.cache.sorted();
    let Some(game) = sorted.get(index).copied().cloned() else {
        state.overlay = Some(super::overlay::PickerOverlay::GameRelayPrompt {
            index,
            action,
            error: Some("selected game no longer exists".to_string()),
        });
        return Ok(());
    };
    let effective_gate = if !game.gate_npub.is_empty() {
        game.gate_npub.clone()
    } else if !gate_npub.is_empty() {
        gate_npub.to_string()
    } else {
        state.overlay = Some(super::overlay::PickerOverlay::GameRelayPrompt {
            index,
            action,
            error: Some(
                "Gate key not known for this game. Reconnect once, then try M again.".to_string(),
            ),
        });
        return Ok(());
    };
    let config = load_config().unwrap_or_else(|_| ConnectConfig::empty());
    let server_str = format!("{}:{}", game.server, game.port);
    let mut target = match resolve_server(&server_str, &config) {
        Ok(target) => target,
        Err(err) => {
            state.overlay = Some(super::overlay::PickerOverlay::GameRelayPrompt {
                index,
                action,
                error: Some(format!("unable to resolve server: {err}")),
            });
            return Ok(());
        }
    };
    target.relay_url = relay_url.clone();
    target.game_id = Some(game.id.clone());
    drop(sorted);

    match rt.block_on(fetch_map_bundle(keys, &target, &effective_gate, &game.id)) {
        Ok(bundle) => {
            match save_map_bundle(&bundle, &target.relay_url, state.maps_root.as_path()) {
                Ok(path) => {
                    persist_cached_game_relay(state, index, &relay_url)?;
                    remember_relay_success(&relay_url);
                    state.relay_input.clear();
                    state.overlay = None;
                    state.show_notice(format!("Maps saved to {}", path.display()));
                }
                Err(err) => {
                    state.overlay = Some(super::overlay::PickerOverlay::GameRelayPrompt {
                        index,
                        action,
                        error: Some(format!("unable to save maps: {err}")),
                    });
                }
            }
        }
        Err(err) => {
            remember_relay_error(&relay_url, &err);
            state.overlay = Some(super::overlay::PickerOverlay::GameRelayPrompt {
                index,
                action,
                error: Some(format!("unable to download maps: {err}")),
            });
        }
    }
    Ok(())
}

fn draw_relay_popup_frame(buffer: &mut PlayfieldBuffer, title: &str, error_lines: usize) -> Rect {
    let height = 7 + error_lines as u16;
    let popup = centered_rect(
        ((72 * 100) / PLAYFIELD_WIDTH).max(40) as u16,
        height,
        Rect::new(0, 0, PLAYFIELD_WIDTH as u16, PLAYFIELD_HEIGHT as u16),
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
        classic::table_body_style(),
    );
    popup
}

fn render_relay_popup_body(
    buffer: &mut PlayfieldBuffer,
    popup: Rect,
    instruction: &str,
    input: &str,
    error: Option<&str>,
) {
    let left = popup.x as usize + 2;
    let inner_right = popup.x as usize + popup.width as usize - 2;
    let inner_width = popup.width.saturating_sub(4) as usize;
    buffer.write_text_clipped(
        popup.y as usize + 1,
        left,
        &truncate(instruction, inner_width),
        classic::table_body_style(),
    );
    let field_row = popup.y as usize + 3;
    let label = "Relay:";
    let input_col = left + label.chars().count() + 1;
    draw_labeled_input_row(
        buffer,
        field_row,
        left,
        label,
        input,
        input_width(inner_right, input_col),
        classic::status_label_style(),
        classic::prompt_hotkey_style(),
    );
    if let Some(error) = error {
        write_wrapped_lines_clamped(
            buffer,
            popup.y as usize + 5,
            left,
            inner_width,
            popup.height.saturating_sub(6) as usize,
            error,
            classic::error_style(),
        );
    }
}

fn relay_error_lines(error: &str) -> usize {
    wrapped_lines(error, 68).len()
}
