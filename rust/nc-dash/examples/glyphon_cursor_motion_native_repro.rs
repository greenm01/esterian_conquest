#[path = "support/native_options.rs"]
mod native_options;

use nc_dash::run_glyphon_cursor_motion_native_repro;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let native = match native_options::parse_native_options() {
        Ok(native) => native,
        Err(err) if err.to_string() == "help requested" => {
            native_options::print_usage("glyphon_cursor_motion_native_repro");
            return Ok(());
        }
        Err(err) => return Err(err),
    };

    eprintln!(
        "glyphon_cursor_motion_native_repro: backend={}, window_mode={}, pid={}",
        native.backend_preference.cli_label(),
        native.window_mode.cli_label(),
        std::process::id()
    );
    run_glyphon_cursor_motion_native_repro(native)
}
