#[path = "support/repro_support.rs"]
mod repro_support;

use nc_dash::{ScreenGeometry, lobby::hosted::dashboard::build_hosted_dash_app};
use nc_nostr::state_sync::GameState;
use repro_support::{BackendPreference, run_rendered_ui_repro};

fn print_usage() {
    println!(
        "Usage: cargo run -p nc-dash --example hosted_snapshot_file_repro -- --snapshot <path> [--backend auto|wayland|x11]"
    );
}

fn parse_args() -> Result<(BackendPreference, std::path::PathBuf), Box<dyn std::error::Error>> {
    let mut backend = BackendPreference::Auto;
    let mut snapshot_path: Option<std::path::PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--backend" => {
                let Some(value) = args.next() else {
                    return Err("--backend requires one of: auto, wayland, x11".into());
                };
                backend = BackendPreference::parse(&value)
                    .ok_or("--backend must be one of: auto, wayland, x11")?;
            }
            "--snapshot" => {
                let Some(value) = args.next() else {
                    return Err("--snapshot requires a path".into());
                };
                snapshot_path = Some(std::path::PathBuf::from(value));
            }
            "--help" | "-h" => {
                return Err("help requested".into());
            }
            other => {
                return Err(format!("unrecognized argument: {other}").into());
            }
        }
    }
    let snapshot = snapshot_path.ok_or("--snapshot is required")?;
    Ok((backend, snapshot))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (backend, snapshot_path) = match parse_args() {
        Ok(options) => options,
        Err(err) if err.to_string() == "help requested" => {
            print_usage();
            return Ok(());
        }
        Err(err) => return Err(err),
    };

    let snapshot: GameState = serde_json::from_slice(&std::fs::read(&snapshot_path)?)?;
    let app =
        build_hosted_dash_app(&snapshot, ScreenGeometry::new(120, 40)).expect("hosted dash app");
    run_rendered_ui_repro("hosted_snapshot_file_repro", backend, move || {
        app.render_ui_for_repro()
    })
}
