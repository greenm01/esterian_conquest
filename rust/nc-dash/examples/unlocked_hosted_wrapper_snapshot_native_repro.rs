use nc_dash::{
    NativeLaunchOptions, run_unlocked_hosted_wrapper_snapshot_native_repro,
    startup::{NativeBackendPreference, NativeWindowMode},
};
use nc_nostr::state_sync::GameState;

fn print_usage() {
    println!(
        "Usage: cargo run -p nc-dash --example unlocked_hosted_wrapper_snapshot_native_repro -- --snapshot <path> --password <password> [--relay <url>] [--backend auto|wayland|x11] [--windowed|--fullscreen] [--diagnostic] [--freeze-live-updates] [--no-live-session] [--live-no-private] [--no-hosted-sessions]"
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

fn parse_args() -> Result<
    (
        NativeLaunchOptions,
        Option<String>,
        String,
        std::path::PathBuf,
    ),
    Box<dyn std::error::Error>,
> {
    let mut native = NativeLaunchOptions::default();
    let mut explicit_windowed = false;
    let mut explicit_fullscreen = false;
    let mut relay_override: Option<String> = None;
    let mut password: Option<String> = None;
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
            "--freeze-live-updates" => {
                native.freeze_live_updates = true;
            }
            "--no-hosted-sessions" => {
                native.disable_hosted_sessions = true;
            }
            "--no-live-session" => {
                native.disable_live_session = true;
            }
            "--live-no-private" => {
                native.disable_live_private_stream = true;
            }
            "--relay" => {
                let Some(value) = args.next() else {
                    return Err("--relay requires a value".into());
                };
                relay_override = Some(value);
            }
            "--password" => {
                let Some(value) = args.next() else {
                    return Err("--password requires a value".into());
                };
                password = Some(value);
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
    let password = password.ok_or("--password is required")?;
    let snapshot = snapshot_path.ok_or("--snapshot is required")?;
    Ok((native, relay_override, password, snapshot))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (native, relay_override, password, snapshot_path) = match parse_args() {
        Ok(options) => options,
        Err(err) if err.to_string() == "help requested" => {
            print_usage();
            return Ok(());
        }
        Err(err) => return Err(err),
    };

    eprintln!(
        "unlocked_hosted_wrapper_snapshot_native_repro: backend={}, window_mode={}, pid={}",
        native.backend_preference.cli_label(),
        native.window_mode.cli_label(),
        std::process::id()
    );

    let snapshot: GameState = serde_json::from_slice(&std::fs::read(&snapshot_path)?)?;
    run_unlocked_hosted_wrapper_snapshot_native_repro(&password, relay_override, &snapshot, native)
}
