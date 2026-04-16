//! nc-dash — Full-screen dashboard and hosted lobby client.

mod app;
mod branding;
mod buffer;
mod client_settings;
mod coords;
mod dashboard_launch;
mod diplomacy_view;
mod geometry;
mod inbox;
mod layout;
pub mod lobby;
mod modal;
mod modal_ratatui;
mod native;
mod native_grid;
mod overlays;
mod panels;
mod planet_view;
mod popups;
mod prompt;
mod rendered;
pub mod startup;
mod table;
mod table_filter;
mod table_layout;
mod table_selection;
mod theme;

use startup::{LaunchCommand, LaunchTarget};

pub use buffer::PlayfieldBuffer;
pub use geometry::ScreenGeometry;
pub use app::state::DashApp;
pub use lobby::LobbyApp;
pub use rendered::{RenderedUi, blit_rendered_ui};
pub use startup::{LobbyStartupOptions, NativeLaunchOptions, parse_launch_command};

pub fn run(args: impl IntoIterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    match parse_launch_command(args)? {
        LaunchCommand::Help => {
            startup::print_usage();
            Ok(())
        }
        LaunchCommand::Launch(LaunchTarget::Lobby(options)) => {
            let native_options = options.native;
            native::run(lobby::LobbyApp::new(options), native_options)
        }
        LaunchCommand::Launch(LaunchTarget::Dashboard { game_dir, native }) => {
            run_dashboard_from_dir(game_dir, native)
        }
    }
}

pub fn main_entry() -> Result<(), Box<dyn std::error::Error>> {
    run(std::env::args())
}

#[doc(hidden)]
pub fn build_native_terminal_for_repro(
    window: std::sync::Arc<winit::window::Window>,
) -> Result<
    ratatui::Terminal<ratatui_wgpu::WgpuBackend<'static, 'static>>,
    Box<dyn std::error::Error>,
> {
    native_grid::build_native_terminal(window)
}

fn run_dashboard_from_dir(
    game_dir: std::path::PathBuf,
    native_options: startup::NativeLaunchOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = dashboard_launch::DashLaunchState::from_local_dir(game_dir)?
        .into_app(crate::geometry::ScreenGeometry::new(1, 1))?;
    native::run(app, native_options)
}

#[doc(hidden)]
pub fn run_hosted_snapshot_native_repro(
    snapshot: &nc_nostr::state_sync::GameState,
    native_options: startup::NativeLaunchOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = crate::lobby::hosted::dashboard::build_hosted_dash_app(
        snapshot,
        crate::geometry::ScreenGeometry::new(120, 40),
    )?;
    native::run(app, native_options)
}

#[doc(hidden)]
pub fn run_hosted_wrapper_snapshot_native_repro(
    snapshot: &nc_nostr::state_sync::GameState,
    native_options: startup::NativeLaunchOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let geometry = crate::geometry::ScreenGeometry::new(120, 40);
    let seat = u8::try_from(snapshot.player_seat)?;
    let dashboard =
        crate::lobby::hosted::dashboard::build_hosted_dash_app(snapshot, geometry)?;
    let mut app = crate::lobby::LobbyApp::new_for_tests(crate::lobby::state::LobbyRoute::HostedGame, geometry);
    app.state.hosted_game = Some(crate::lobby::state::HostedGameView {
        row: crate::lobby::models::JoinedGameRow::new(
            &snapshot.game_id,
            "joined",
            &snapshot.player_name,
            "nc-host",
            "ws://127.0.0.1:8080",
            "daemon",
            Some(seat),
            &format!("y{} t{}", snapshot.year, snapshot.turn),
        ),
        snapshot: snapshot.clone(),
        dashboard,
        submit_input: String::new(),
        submit_status: None,
    });
    native::run(app, native_options)
}

#[doc(hidden)]
pub fn run_unlocked_hosted_wrapper_snapshot_native_repro(
    password: &str,
    relay_override: Option<String>,
    snapshot: &nc_nostr::state_sync::GameState,
    native_options: startup::NativeLaunchOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let geometry = crate::geometry::ScreenGeometry::new(120, 40);
    let mut app = crate::lobby::LobbyApp::new(crate::startup::LobbyStartupOptions {
        relay_override,
        native: native_options,
    });
    app.geometry = geometry;
    let loaded = app.transport.unlock(password).map_err(|err| err.to_string())?;
    app.state.apply_loaded(loaded);
    let seat = u8::try_from(snapshot.player_seat)?;
    let dashboard =
        crate::lobby::hosted::dashboard::build_hosted_dash_app(snapshot, geometry)?;
    app.state.hosted_game = Some(crate::lobby::state::HostedGameView {
        row: crate::lobby::models::JoinedGameRow::new(
            &snapshot.game_id,
            "joined",
            &snapshot.player_name,
            "nc-host",
            "ws://127.0.0.1:8080",
            "daemon",
            Some(seat),
            &format!("y{} t{}", snapshot.year, snapshot.turn),
        ),
        snapshot: snapshot.clone(),
        dashboard,
        submit_input: String::new(),
        submit_status: None,
    });
    app.state.route = crate::lobby::state::LobbyRoute::HostedGame;
    app.popup_position = None;
    native::run(app, native_options)
}

#[doc(hidden)]
pub fn run_unlocked_open_game_native_repro(
    password: &str,
    relay_override: Option<String>,
    snapshot_hint: &nc_nostr::state_sync::GameState,
    native_options: startup::NativeLaunchOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let geometry = crate::geometry::ScreenGeometry::new(120, 40);
    let mut app = unlocked_lobby_app(password, relay_override, native_options, geometry)?;
    let row = resolve_row_for_snapshot_hint(&app, snapshot_hint)?;
    let snapshot = app
        .transport
        .open_game(&row)
        .map_err(|err| err.message)?;
    if native_options.diagnostic_mode {
        let path = dump_diagnostic_snapshot("last-open-game-repro-snapshot.json", &snapshot);
        eprintln!(
            "unlocked_open_game_native_repro: dumped fetched snapshot to {}",
            path.display()
        );
        eprintln!(
            "unlocked_open_game_native_repro: fetched state_hash={} hint_state_hash={}",
            snapshot.state_hash, snapshot_hint.state_hash
        );
    }
    install_hosted_snapshot(&mut app, row, snapshot, geometry)?;
    native::run(app, native_options)
}

#[doc(hidden)]
pub fn run_unlocked_fetch_only_open_game_native_repro(
    password: &str,
    relay_override: Option<String>,
    snapshot_hint: &nc_nostr::state_sync::GameState,
    native_options: startup::NativeLaunchOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let geometry = crate::geometry::ScreenGeometry::new(120, 40);
    let mut app = unlocked_lobby_app(password, relay_override, native_options, geometry)?;
    let row = resolve_row_for_snapshot_hint(&app, snapshot_hint)?;
    let snapshot = app.transport.open_game_fetch_only(&row)?;
    if native_options.diagnostic_mode {
        let path = dump_diagnostic_snapshot("last-fetch-only-open-game-snapshot.json", &snapshot);
        eprintln!(
            "unlocked_fetch_only_open_game_native_repro: dumped fetched snapshot to {}",
            path.display()
        );
        eprintln!(
            "unlocked_fetch_only_open_game_native_repro: fetched state_hash={} hint_state_hash={}",
            snapshot.state_hash, snapshot_hint.state_hash
        );
    }
    install_hosted_snapshot(&mut app, row, snapshot, geometry)?;
    native::run(app, native_options)
}

#[doc(hidden)]
pub fn run_unlocked_persisted_snapshot_native_repro(
    password: &str,
    relay_override: Option<String>,
    snapshot: &nc_nostr::state_sync::GameState,
    native_options: startup::NativeLaunchOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let geometry = crate::geometry::ScreenGeometry::new(120, 40);
    let mut app = unlocked_lobby_app(password, relay_override, native_options, geometry)?;
    let row = resolve_row_for_snapshot_hint(&app, snapshot)?;
    let persist = app
        .transport
        .persist_open_game_state_for_repro(&row, snapshot)?;
    if native_options.diagnostic_mode {
        eprintln!(
            "unlocked_persisted_snapshot_native_repro: had_baseline={} had_draft={} cleared_stale_draft={}",
            persist.had_baseline, persist.had_draft, persist.cleared_stale_draft
        );
    }
    install_hosted_snapshot(&mut app, row, snapshot.clone(), geometry)?;
    native::run(app, native_options)
}

#[doc(hidden)]
pub fn run_unlocked_cache_write_snapshot_native_repro(
    password: &str,
    relay_override: Option<String>,
    snapshot: &nc_nostr::state_sync::GameState,
    native_options: startup::NativeLaunchOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let geometry = crate::geometry::ScreenGeometry::new(120, 40);
    let mut app = unlocked_lobby_app(password, relay_override, native_options, geometry)?;
    let row = resolve_row_for_snapshot_hint(&app, snapshot)?;
    let persist = app
        .transport
        .persist_open_game_state_for_repro(&row, snapshot)?;
    app.transport.update_open_game_cache_for_repro(&row, snapshot)?;
    if native_options.diagnostic_mode {
        eprintln!(
            "unlocked_cache_write_snapshot_native_repro: had_baseline={} had_draft={} cleared_stale_draft={} cache_write=true",
            persist.had_baseline, persist.had_draft, persist.cleared_stale_draft
        );
    }
    install_hosted_snapshot(&mut app, row, snapshot.clone(), geometry)?;
    native::run(app, native_options)
}

fn unlocked_lobby_app(
    password: &str,
    relay_override: Option<String>,
    native_options: startup::NativeLaunchOptions,
    geometry: crate::geometry::ScreenGeometry,
) -> Result<crate::lobby::LobbyApp, Box<dyn std::error::Error>> {
    let mut app = crate::lobby::LobbyApp::new(crate::startup::LobbyStartupOptions {
        relay_override,
        native: native_options,
    });
    app.geometry = geometry;
    let loaded = app.transport.unlock(password).map_err(|err| err.to_string())?;
    app.state.apply_loaded(loaded);
    Ok(app)
}

fn resolve_row_for_snapshot_hint(
    app: &crate::lobby::LobbyApp,
    snapshot_hint: &nc_nostr::state_sync::GameState,
) -> Result<crate::lobby::models::JoinedGameRow, Box<dyn std::error::Error>> {
    app.state
        .joined_games
        .iter()
        .find(|row| row.game_id == snapshot_hint.game_id)
        .cloned()
        .or_else(|| {
            let seat = u8::try_from(snapshot_hint.player_seat).ok()?;
            app.state
                .open_games
                .iter()
                .find(|row| row.game_id == snapshot_hint.game_id)
                .map(|row| crate::lobby::models::JoinedGameRow {
                    game_id: row.game_id.clone(),
                    status: "joined".to_string(),
                    game_tier: row.game_tier.clone(),
                    game: row.game.clone(),
                    host: row.host.clone(),
                    host_contact_npub: row.host_contact_npub.clone(),
                    relay_url: row.relay_url.clone(),
                    daemon_pubkey: row.daemon_pubkey.clone(),
                    seat: Some(seat),
                    turn_summary: row.turn_summary.clone(),
                    invite_address: None,
                    last_turn: Some(snapshot_hint.turn),
                    last_hash: Some(snapshot_hint.state_hash.clone()),
                })
        })
        .ok_or_else(|| {
            format!(
                "game '{}' not found in joined or open rows after unlock",
                snapshot_hint.game_id
            )
            .into()
        })
}

fn install_hosted_snapshot(
    app: &mut crate::lobby::LobbyApp,
    row: crate::lobby::models::JoinedGameRow,
    snapshot: nc_nostr::state_sync::GameState,
    geometry: crate::geometry::ScreenGeometry,
) -> Result<(), Box<dyn std::error::Error>> {
    let dashboard = crate::lobby::hosted::dashboard::build_hosted_dash_app(&snapshot, geometry)?;
    app.state.hosted_game = Some(crate::lobby::state::HostedGameView {
        row,
        snapshot,
        dashboard,
        submit_input: String::new(),
        submit_status: None,
    });
    app.state.route = crate::lobby::state::LobbyRoute::HostedGame;
    app.popup_position = None;
    Ok(())
}

fn dump_diagnostic_snapshot(
    filename: &str,
    snapshot: &nc_nostr::state_sync::GameState,
) -> std::path::PathBuf {
    let path = nc_client::paths::data_root().join(filename);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_vec_pretty(snapshot) {
        let _ = std::fs::write(&path, json);
    }
    path
}

pub(crate) fn show_fatal_error(message: &str) {
    eprintln!("error: {message}");
}
