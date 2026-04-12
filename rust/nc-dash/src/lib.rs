//! nc-dash — Full-screen dashboard and hosted lobby client.

mod app;
mod client_settings;
mod diplomacy_view;
mod inbox;
pub mod lobby;
mod layout;
mod native;
mod overlays;
mod panels;
mod planet_view;
mod popups;
pub mod startup;
mod theme;

use std::path::PathBuf;

use nc_data::CampaignStore;
use nc_ui::ScreenGeometry;

use app::state::DashApp;
use startup::{LaunchCommand, LaunchTarget};

pub use startup::{LobbyStartupOptions, parse_launch_command};

pub fn run(args: impl IntoIterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    match parse_launch_command(args)? {
        LaunchCommand::Help => {
            startup::print_usage();
            Ok(())
        }
        LaunchCommand::Launch(LaunchTarget::Lobby(options)) => {
            native::run(lobby::LobbyApp::new(options))
        }
        LaunchCommand::Launch(LaunchTarget::Dashboard { game_dir }) => {
            run_dashboard_from_dir(game_dir)
        }
    }
}

pub fn main_entry() -> Result<(), Box<dyn std::error::Error>> {
    run(std::env::args())
}

fn run_dashboard_from_dir(game_dir: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let campaign_store = CampaignStore::open_default_in_dir(&game_dir)?;
    let state = campaign_store
        .load_latest_runtime_state()?
        .ok_or("No runtime snapshots found — run maintenance first.")?;

    let player_record_index_1_based: usize = 1;
    let owned_planet_years =
        campaign_store.latest_owned_planet_years_for_empire(player_record_index_1_based as u8)?;
    let planet_intel_snapshots =
        campaign_store.latest_planet_intel_for_viewer(player_record_index_1_based as u8)?;
    let player_war_stats = campaign_store
        .latest_player_war_stats(state.game_data.conquest.player_count())?
        .get(player_record_index_1_based.saturating_sub(1))
        .copied()
        .unwrap_or_else(|| nc_data::PlayerWarStatsState::for_player(player_record_index_1_based));
    let player_activity_states =
        campaign_store.latest_player_activity_states(state.game_data.conquest.player_count())?;
    let player_lifecycle_states =
        campaign_store.latest_player_lifecycle_states(state.game_data.conquest.player_count())?;

    let mut app = DashApp::new(
        game_dir,
        Some(campaign_store.clone()),
        state.game_data,
        owned_planet_years,
        state.planet_scorch_orders,
        state.report_block_rows,
        state.queued_mail,
        planet_intel_snapshots,
        player_activity_states,
        player_lifecycle_states,
        state.winner_state,
        ScreenGeometry::new(1, 1),
        ScreenGeometry::new(0, 0),
        player_record_index_1_based,
    );
    app.player_war_stats = player_war_stats;
    let client_settings_path = client_settings::settings_path();
    app.client_settings = client_settings::load_client_settings_from(&client_settings_path)?;
    app.client_settings_path = Some(client_settings_path);
    let required = layout::dashboard::required_dashboard_frame(&app);
    app.geometry = required;
    app.frame = required;
    app.is_terminal_too_small = false;

    native::run(app)
}

pub(crate) fn show_fatal_error(message: &str) {
    eprintln!("error: {message}");
}
