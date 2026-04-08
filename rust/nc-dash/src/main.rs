//! nc-dash — Full-screen dashboard TUI for Nostrian Conquest.

mod app;
mod diplomacy_view;
mod inbox;
mod layout;
mod native;
mod overlays;
mod panels;
mod planet_view;
mod popups;
mod startup;
mod theme;

use nc_data::CampaignStore;
use nc_ui::ScreenGeometry;

use app::state::DashApp;

pub fn run(args: impl IntoIterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = args.into_iter().collect();

    if matches!(args.get(1).map(String::as_str), Some("--help" | "-h")) {
        print_usage();
        return Ok(());
    }

    let game_dir = args
        .get(1)
        .map(std::path::PathBuf::from)
        .ok_or("Usage: nc-dash <game-dir>")?;

    // Load game data first so we know the map size.
    let campaign_store = CampaignStore::open_default_in_dir(&game_dir)?;
    let state = campaign_store
        .load_latest_runtime_state()?
        .ok_or("No runtime snapshots found — run maintenance first.")?;

    // Default to player 1. Future: resolve from args/session.
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
        ScreenGeometry::new(1, 1),
        ScreenGeometry::new(0, 0),
        player_record_index_1_based,
    );
    app.player_war_stats = player_war_stats;
    let required = layout::dashboard::required_dashboard_frame(&app);
    app.geometry = required;
    app.frame = required;
    app.is_terminal_too_small = false;

    native::run(app)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run(std::env::args())
}

pub(crate) fn show_fatal_error(message: &str) {
    eprintln!("error: {message}");
}

fn print_usage() {
    eprintln!("nc-dash — Nostrian Conquest full-screen dashboard");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    nc-dash <game-dir> [OPTIONS]");
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("    --help, -h    Show this help");
}
