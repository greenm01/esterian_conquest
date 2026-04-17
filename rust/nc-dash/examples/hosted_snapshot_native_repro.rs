use nc_dash::{
    NativeLaunchOptions,
    lobby::hosted::dashboard::build_hosted_dash_app,
    run_hosted_snapshot_native_repro,
    startup::{NativeBackendPreference, NativeWindowMode},
};
use nc_nostr::state_sync::GameState;

fn print_usage() {
    println!(
        "Usage: cargo run -p nc-dash --example hosted_snapshot_native_repro -- --snapshot <path> [--backend auto|wayland|x11] [--windowed|--fullscreen] [--diagnostic]"
    );
}

fn parse_backend(value: &str) -> Result<NativeBackendPreference, Box<dyn std::error::Error>> {
    match value {
        "auto" => Ok(NativeBackendPreference::Auto),
        "wayland" => Ok(NativeBackendPreference::Wayland),
        "x11" => Ok(NativeBackendPreference::X11),
        _ => Err(format!("unknown backend '{value}'; expected auto, wayland, or x11").into()),
    }
}

fn parse_args() -> Result<(NativeLaunchOptions, std::path::PathBuf), Box<dyn std::error::Error>> {
    let mut native = NativeLaunchOptions::default();
    let mut explicit_windowed = false;
    let mut explicit_fullscreen = false;
    let mut snapshot_path: Option<std::path::PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--backend" => {
                let Some(value) = args.next() else {
                    return Err("--backend requires one of: auto, wayland, x11".into());
                };
                native.backend_preference = parse_backend(&value)?;
            }
            "--windowed" => {
                if explicit_fullscreen {
                    return Err("cannot combine --windowed and --fullscreen".into());
                }
                explicit_windowed = true;
                native.window_mode = NativeWindowMode::MaximizedWindow;
            }
            "--fullscreen" => {
                if explicit_windowed {
                    return Err("cannot combine --windowed and --fullscreen".into());
                }
                explicit_fullscreen = true;
                native.window_mode = NativeWindowMode::BorderlessFullscreen;
            }
            "--diagnostic" => {
                native.diagnostic_mode = true;
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
    Ok((native, snapshot))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (native, snapshot_path) = match parse_args() {
        Ok(options) => options,
        Err(err) if err.to_string() == "help requested" => {
            print_usage();
            return Ok(());
        }
        Err(err) => return Err(err),
    };

    eprintln!(
        "hosted_snapshot_native_repro: backend={}, window_mode={}, pid={}",
        native.backend_preference.cli_label(),
        native.window_mode.cli_label(),
        std::process::id()
    );

    let snapshot: GameState = serde_json::from_slice(&std::fs::read(&snapshot_path)?)?;
    let _ = build_hosted_dash_app(&snapshot, nc_dash::ScreenGeometry::new(120, 40))
        .expect("hosted dash app");
    run_hosted_snapshot_native_repro(&snapshot, native)
}
