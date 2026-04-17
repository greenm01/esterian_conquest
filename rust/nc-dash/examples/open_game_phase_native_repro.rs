use nc_dash::{
    NativeLaunchOptions, run_unlocked_cache_write_snapshot_native_repro,
    run_unlocked_fetch_only_open_game_native_repro, run_unlocked_persisted_snapshot_native_repro,
    startup::{NativeBackendPreference, NativeWindowMode},
};
use nc_nostr::state_sync::GameState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PhaseMode {
    FetchOnly,
    PersistedSnapshot,
    CacheWrite,
}

impl PhaseMode {
    fn parse(value: &str) -> Result<Self, Box<dyn std::error::Error>> {
        match value {
            "fetch-only" => Ok(Self::FetchOnly),
            "persisted-snapshot" => Ok(Self::PersistedSnapshot),
            "cache-write" => Ok(Self::CacheWrite),
            _ => Err(format!(
                "unknown mode '{value}'; expected fetch-only, persisted-snapshot, or cache-write"
            )
            .into()),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::FetchOnly => "fetch-only",
            Self::PersistedSnapshot => "persisted-snapshot",
            Self::CacheWrite => "cache-write",
        }
    }
}

fn print_usage() {
    println!(
        "Usage: cargo run -p nc-dash --example open_game_phase_native_repro -- --mode <fetch-only|persisted-snapshot|cache-write> --snapshot <path> --password <password> [--relay <url>] [--backend auto|wayland|x11] [--windowed|--fullscreen] [--diagnostic] [--freeze-live-updates] [--no-live-session] [--live-no-private] [--no-hosted-sessions]"
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
        PhaseMode,
        NativeLaunchOptions,
        Option<String>,
        String,
        std::path::PathBuf,
    ),
    Box<dyn std::error::Error>,
> {
    let mut mode: Option<PhaseMode> = None;
    let mut native = NativeLaunchOptions::default();
    let mut explicit_windowed = false;
    let mut explicit_fullscreen = false;
    let mut relay_override: Option<String> = None;
    let mut password: Option<String> = None;
    let mut snapshot_path: Option<std::path::PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--mode" => {
                let Some(value) = args.next() else {
                    return Err(
                        "--mode requires one of: fetch-only, persisted-snapshot, cache-write"
                            .into(),
                    );
                };
                mode = Some(PhaseMode::parse(&value)?);
            }
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
            "--diagnostic" => native.diagnostic_mode = true,
            "--freeze-live-updates" => native.freeze_live_updates = true,
            "--no-hosted-sessions" => native.disable_hosted_sessions = true,
            "--no-live-session" => native.disable_live_session = true,
            "--live-no-private" => native.disable_live_private_stream = true,
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
            "--help" | "-h" => return Err("help requested".into()),
            other => return Err(format!("unrecognized argument: {other}").into()),
        }
    }
    Ok((
        mode.ok_or("--mode is required")?,
        native,
        relay_override,
        password.ok_or("--password is required")?,
        snapshot_path.ok_or("--snapshot is required")?,
    ))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mode, native, relay_override, password, snapshot_path) = match parse_args() {
        Ok(options) => options,
        Err(err) if err.to_string() == "help requested" => {
            print_usage();
            return Ok(());
        }
        Err(err) => return Err(err),
    };

    eprintln!(
        "open_game_phase_native_repro: mode={}, backend={}, window_mode={}, pid={}",
        mode.label(),
        native.backend_preference.cli_label(),
        native.window_mode.cli_label(),
        std::process::id()
    );

    let snapshot: GameState = serde_json::from_slice(&std::fs::read(&snapshot_path)?)?;
    match mode {
        PhaseMode::FetchOnly => run_unlocked_fetch_only_open_game_native_repro(
            &password,
            relay_override,
            &snapshot,
            native,
        ),
        PhaseMode::PersistedSnapshot => run_unlocked_persisted_snapshot_native_repro(
            &password,
            relay_override,
            &snapshot,
            native,
        ),
        PhaseMode::CacheWrite => run_unlocked_cache_write_snapshot_native_repro(
            &password,
            relay_override,
            &snapshot,
            native,
        ),
    }
}
