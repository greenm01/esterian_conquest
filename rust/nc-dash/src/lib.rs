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
pub use rendered::RenderedUi;
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

fn run_dashboard_from_dir(game_dir: std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let app = dashboard_launch::DashLaunchState::from_local_dir(game_dir)?
        .into_app(crate::geometry::ScreenGeometry::new(1, 1))?;
    native::run(app)
}

pub(crate) fn show_fatal_error(message: &str) {
    eprintln!("error: {message}");
}
