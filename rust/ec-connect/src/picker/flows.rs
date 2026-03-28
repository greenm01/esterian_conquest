use std::path::Path;

use nostr_sdk::Keys;

use ec_ui::session::TerminalSession;

use crate::cache::save_cache;
use crate::connect::map_fetch::fetch_map_bundle;
use crate::connect::resolve::{resolve_invite, resolve_server};
use crate::connect::session::{DisambigMode, SessionOutcome, run_session};
use crate::map_store::save_map_bundle;

use super::relay::{RelayPromptAction, open_game_relay_prompt};
use super::state::{PickerSession, PickerState, Screen};

pub fn connect_selected(
    state: &mut PickerState,
    picker_session: &mut PickerSession,
    gate_npub: &str,
    maps_root: &Path,
    rt: &tokio::runtime::Runtime,
    session: &mut TerminalSession,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::config::ConnectConfig;
    use crate::config::load_config;

    let sorted = state.cache.sorted();
    let Some(game) = sorted.get(state.selected).copied().cloned() else {
        return Ok(());
    };

    let config = load_config().unwrap_or_else(|_| ConnectConfig::empty());
    if game
        .relay_url
        .as_ref()
        .filter(|value| !value.is_empty())
        .is_none()
        && config.relay.is_none()
    {
        open_game_relay_prompt(
            state,
            state.selected,
            &game.server,
            RelayPromptAction::Connect,
        );
        return Ok(());
    }
    let server_str = format!("{}:{}", game.server, game.port);
    let mut target = resolve_server(&server_str, &config)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    if let Some(relay_url) = game.relay_url.as_ref().filter(|value| !value.is_empty()) {
        target.relay_url = relay_url.clone();
    }
    target.game_id = Some(game.id.clone());
    let effective_gate = if !game.gate_npub.is_empty() {
        game.gate_npub.clone()
    } else {
        gate_npub.to_string()
    };
    let missing_cached_relay = game.relay_url.is_none() && config.relay.is_none();
    drop(sorted);

    let outcome = run_suspended(session, || {
        rt.block_on(run_session(
            &picker_session.keys,
            target.clone(),
            &picker_session.npub,
            &effective_gate,
            DisambigMode::Picker,
            maps_root,
        ))
    })?;
    let outcome = annotate_missing_relay_timeout(outcome, missing_cached_relay);
    state.refresh_cache();
    apply_session_outcome(state, outcome, Some((target, effective_gate)));
    Ok(())
}

pub fn join_with_code(
    state: &mut PickerState,
    code: &str,
    picker_session: &mut PickerSession,
    gate_npub: &str,
    maps_root: &Path,
    rt: &tokio::runtime::Runtime,
    session: &mut TerminalSession,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::config::ConnectConfig;
    use crate::config::load_config;

    let config = load_config().unwrap_or_else(|_| ConnectConfig::empty());
    let target = match resolve_invite(code, &config) {
        Ok(target) => target,
        Err(err) => {
            state.show_error(format!("invalid invite code: {err}"));
            return Ok(());
        }
    };

    state.join_input.clear();
    state.screen = Screen::GameList;
    let outcome = run_suspended(session, || {
        rt.block_on(run_session(
            &picker_session.keys,
            target.clone(),
            &picker_session.npub,
            gate_npub,
            DisambigMode::Picker,
            maps_root,
        ))
    })?;
    state.refresh_cache();
    apply_join_outcome(state, outcome, target, gate_npub.to_string());
    Ok(())
}

pub fn redownload_selected_maps(
    state: &mut PickerState,
    keys: &Keys,
    gate_npub: &str,
    maps_root: &Path,
    rt: &tokio::runtime::Runtime,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::config::ConnectConfig;
    use crate::config::load_config;

    let sorted = state.cache.sorted();
    let Some(game) = sorted.get(state.selected).copied().cloned() else {
        state.show_error("No joined games yet.");
        return Ok(());
    };
    let game_id = game.id.clone();
    let game_server = game.server.clone();
    let game_port = game.port;
    let cached_gate_npub = game.gate_npub.clone();

    let effective_gate = if !cached_gate_npub.is_empty() {
        cached_gate_npub
    } else if !gate_npub.is_empty() {
        gate_npub.to_string()
    } else {
        state.show_error("Gate key not known for this game. Reconnect once, then try M again.");
        return Ok(());
    };

    let config = load_config().unwrap_or_else(|_| ConnectConfig::empty());
    if game
        .relay_url
        .as_ref()
        .filter(|value| !value.is_empty())
        .is_none()
        && config.relay.is_none()
    {
        open_game_relay_prompt(
            state,
            state.selected,
            &game_server,
            RelayPromptAction::DownloadMaps,
        );
        return Ok(());
    }
    let server_str = format!("{}:{}", game_server, game_port);
    let mut target = match resolve_server(&server_str, &config) {
        Ok(target) => target,
        Err(err) => {
            state.show_error(format!("unable to resolve server: {err}"));
            return Ok(());
        }
    };
    if let Some(relay_url) = game.relay_url.as_ref().filter(|value| !value.is_empty()) {
        target.relay_url = relay_url.clone();
    }
    target.game_id = Some(game_id.clone());
    drop(sorted);

    match rt.block_on(fetch_map_bundle(keys, &target, &effective_gate, &game_id)) {
        Ok(bundle) => {
            match save_map_bundle(&bundle, &target.server_host, target.server_port, maps_root) {
                Ok(path) => {
                    state.show_notice(format!("Maps saved to {}", path.display()));
                }
                Err(err) => {
                    state.show_error(format!("unable to save maps: {err}"));
                }
            }
        }
        Err(err) => {
            state.show_error(format!("unable to download maps: {err}"));
        }
    }

    Ok(())
}

pub fn move_selection(selected: &mut usize, delta: isize, total: usize) {
    if total == 0 {
        *selected = 0;
        return;
    }
    let current = *selected as isize;
    let max = total.saturating_sub(1) as isize;
    *selected = (current + delta).clamp(0, max) as usize;
}

pub fn persist_cached_game_relay(
    state: &mut PickerState,
    index: usize,
    relay_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let sorted = state.cache.sorted();
    let Some(game) = sorted.get(index).copied().cloned() else {
        return Err("selected game no longer exists".into());
    };
    drop(sorted);
    let mut updated = game;
    updated.relay_url = Some(relay_url.to_string());
    state.cache.upsert(updated);
    save_cache(&state.cache)?;
    Ok(())
}

fn run_suspended<T>(
    session: &mut TerminalSession,
    action: impl FnOnce() -> T,
) -> Result<T, Box<dyn std::error::Error>> {
    session.suspend_for_bridge()?;
    let result = action();
    session.resume_after_bridge()?;
    Ok(result)
}

fn annotate_missing_relay_timeout(
    outcome: SessionOutcome,
    missing_cached_relay: bool,
) -> SessionOutcome {
    if missing_cached_relay && matches!(outcome, SessionOutcome::Timeout) {
        SessionOutcome::Error(
            "handshake timed out. This cached game has no saved relay URL yet; reconnect once with an explicit relay or add the relay to config."
                .to_string(),
        )
    } else {
        outcome
    }
}

pub fn apply_session_outcome(
    state: &mut PickerState,
    outcome: SessionOutcome,
    retry_context: Option<(crate::connect::resolve::ResolvedTarget, String)>,
) {
    match outcome {
        SessionOutcome::Done { notice, .. } => {
            if let Some(notice) = notice
                .filter(|message| !message.trim().is_empty())
                .filter(|message| message != "For Griffith and glory.")
            {
                state.show_notice(notice);
            }
        }
        SessionOutcome::Error(msg) => {
            state.show_error(msg);
        }
        SessionOutcome::Timeout => {
            state.show_error("handshake timed out.");
        }
        SessionOutcome::NeedsDisambiguation { games } => {
            if let Some((target, gate_npub)) = retry_context {
                state.screen = Screen::GameSelect {
                    games,
                    selected: 0,
                    server_host: target.server_host,
                    server_port: target.server_port,
                    relay_url: target.relay_url,
                    gate_npub,
                };
            } else {
                state.show_error("unexpected disambiguation state.");
            }
        }
    }
}

fn apply_join_outcome(
    state: &mut PickerState,
    outcome: SessionOutcome,
    target: crate::connect::resolve::ResolvedTarget,
    gate_npub: String,
) {
    match outcome {
        SessionOutcome::Done { notice, .. } => {
            state.join_input.clear();
            state.screen = Screen::GameList;
            state.selected = 0;
            if let Some(notice) = notice
                .filter(|message| !message.trim().is_empty())
                .filter(|message| message != "For Griffith and glory.")
            {
                state.show_notice(notice);
            }
        }
        SessionOutcome::Error(msg) => {
            state.screen = Screen::JoinPrompt;
            state.show_error(msg);
        }
        SessionOutcome::Timeout => {
            state.screen = Screen::JoinPrompt;
            state.show_error("handshake timed out.");
        }
        SessionOutcome::NeedsDisambiguation { games } => {
            state.screen = Screen::GameSelect {
                games,
                selected: 0,
                server_host: target.server_host,
                server_port: target.server_port,
                relay_url: target.relay_url,
                gate_npub,
            };
        }
    }
}
