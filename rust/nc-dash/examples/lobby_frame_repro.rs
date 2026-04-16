#[path = "support/repro_support.rs"]
mod repro_support;

use nc_dash::{LobbyApp, ScreenGeometry, lobby::state::LobbyRoute};
use repro_support::{parse_args, print_usage, run_rendered_ui_repro};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = match parse_args() {
        Ok(options) => options,
        Err(err) if err.to_string() == "help requested" => {
            print_usage("lobby_frame_repro");
            return Ok(());
        }
        Err(err) => return Err(err),
    };

    let app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    run_rendered_ui_repro("lobby_frame_repro", options.backend, move || app.render_ui_for_repro())
}
