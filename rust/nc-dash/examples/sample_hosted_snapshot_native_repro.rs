#[path = "support/native_options.rs"]
mod native_options;

use nc_dash::run_sample_hosted_snapshot_native_repro;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let native = match native_options::parse_native_options() {
        Ok(native) => native,
        Err(err) if err.to_string() == "help requested" => {
            native_options::print_usage("sample_hosted_snapshot_native_repro");
            return Ok(());
        }
        Err(err) => return Err(err),
    };

    eprintln!(
        "sample_hosted_snapshot_native_repro: backend={}, window_mode={}, pid={}",
        native.backend_preference.cli_label(),
        native.window_mode.cli_label(),
        std::process::id()
    );
    run_sample_hosted_snapshot_native_repro(native)
}
