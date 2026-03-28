use std::path::Path;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ec_ui::buffer::PlayfieldBuffer;
use ec_ui::theme::classic;

use super::connecting::{PendingConnectRequest, queue_connect_request};
use super::event::is_back_key;
use super::flows::persist_cached_game_relay;
use super::layout::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH, Rect, centered_rect, draw_box, truncate};
use super::state::{ConnectDisplay, ConnectOrigin, PickerState};
use crate::config::{ConnectConfig, load_config, save_config, validate_relay_url};
use crate::connect::map_fetch::fetch_map_bundle;
use crate::connect::resolve::{derive_relay_url, resolve_server};
use crate::input_field::{draw_labeled_input_row, input_width};
use crate::map_store::save_map_bundle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayPromptAction {
    Connect,
    DownloadMaps,
}

const INVALID_STORED_RELAY_ERROR: &str =
    "Stored default relay is invalid. Enter a new relay URL.";

pub fn open_default_relay_editor(
    state: &mut PickerState,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config().unwrap_or_else(|_| ConnectConfig::empty());
    let (relay_input, error) = match config.relay.as_deref() {
        Some(saved) => match validate_relay_url(saved) {
            Ok(Some(valid)) => (valid, None),
            Ok(None) | Err(_) => (String::new(), Some(INVALID_STORED_RELAY_ERROR.to_string())),
        },
        None => (String::new(), None),
    };
    state.relay_input = relay_input;
    state.overlay = Some(super::overlay::PickerOverlay::DefaultRelayEditor { error });
    Ok(())
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

pub fn handle_default_relay_key(
    key: KeyEvent,
    state: &mut PickerState,
) -> Result<(), Box<dyn std::error::Error>> {
    match key.code {
        KeyCode::Enter => {
            let relay = match validate_relay_url(&state.relay_input) {
                Ok(relay) => relay,
                Err(err) => {
                    state.overlay = Some(super::overlay::PickerOverlay::DefaultRelayEditor {
                        error: Some(err),
                    });
                    return Ok(());
                }
            };
            let mut config = load_config().unwrap_or_else(|_| ConnectConfig::empty());
            config.relay = relay;
            match save_config(&config) {
                Ok(()) => {
                    state.relay_input.clear();
                    state.overlay = None;
                }
                Err(err) => {
                    state.overlay = Some(super::overlay::PickerOverlay::DefaultRelayEditor {
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

pub fn handle_game_relay_key(
    key: KeyEvent,
    index: usize,
    action: RelayPromptAction,
    state: &mut PickerState,
    player_keys: &nostr_sdk::Keys,
    gate_npub: &str,
    maps_root: &Path,
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
                    maps_root,
                    rt,
                    index,
                    relay_url,
                    action,
                )?,
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

pub fn render_default_relay_popup(buffer: &mut PlayfieldBuffer, input: &str, error: Option<&str>) {
    let popup = draw_relay_popup_frame(buffer, "DEFAULT RELAY", error.is_some());
    render_relay_popup_body(
        buffer,
        popup,
        "Set the default relay for new joins and old cached games.",
        input,
        error,
    );
}

pub fn render_game_relay_popup(
    buffer: &mut PlayfieldBuffer,
    input: &str,
    error: Option<&str>,
    action: RelayPromptAction,
) {
    let popup = draw_relay_popup_frame(buffer, "GAME RELAY", error.is_some());
    let instruction = match action {
        RelayPromptAction::Connect => {
            "This game has no saved relay yet. Enter the relay to reconnect."
        }
        RelayPromptAction::DownloadMaps => {
            "This game has no saved relay yet. Enter the relay to download maps."
        }
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
    maps_root: &Path,
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
            match save_map_bundle(&bundle, &target.server_host, target.server_port, maps_root) {
                Ok(path) => {
                    persist_cached_game_relay(state, index, &relay_url)?;
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
            state.overlay = Some(super::overlay::PickerOverlay::GameRelayPrompt {
                index,
                action,
                error: Some(format!("unable to download maps: {err}")),
            });
        }
    }
    Ok(())
}

fn draw_relay_popup_frame(buffer: &mut PlayfieldBuffer, title: &str, has_error: bool) -> Rect {
    let height = if has_error { 8 } else { 7 };
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
    buffer.write_text_clipped(
        popup.y as usize + 1,
        left,
        &truncate(instruction, popup.width.saturating_sub(4) as usize),
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
        buffer.write_text_clipped(
            popup.y as usize + 5,
            left,
            &truncate(error, popup.width.saturating_sub(4) as usize),
            classic::error_style(),
        );
    }
}
