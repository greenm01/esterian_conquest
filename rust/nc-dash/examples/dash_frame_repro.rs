#[path = "support/repro_support.rs"]
mod repro_support;

use nc_dash::{DashApp, ScreenGeometry};
use repro_support::{parse_args, print_usage, run_rendered_ui_repro};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = match parse_args() {
        Ok(options) => options,
        Err(err) if err.to_string() == "help requested" => {
            print_usage("dash_frame_repro");
            return Ok(());
        }
        Err(err) => return Err(err),
    };

    let app = DashApp::new_for_repro(ScreenGeometry::new(160, 40), ScreenGeometry::new(108, 26));
    run_rendered_ui_repro("dash_frame_repro", options.backend, move || app.render_ui_for_repro())
}
