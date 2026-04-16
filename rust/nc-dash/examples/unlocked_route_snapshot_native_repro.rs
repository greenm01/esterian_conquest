use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nc_dash::{
    LobbyApp, ScreenGeometry,
    lobby::hosted::dashboard::build_hosted_dash_app,
    lobby::models::JoinedGameRow,
    lobby::state::{HostedGameView, LobbyRoute},
    startup::{NativeBackendPreference, NativeWindowMode},
};
use nc_nostr::state_sync::GameState;

#[path = "support/repro_support.rs"]
mod repro_support;

fn print_usage() {
    println!(
        "Usage: cargo run -p nc-dash --example unlocked_route_snapshot_native_repro -- --snapshot <path> --password <password> [--relay <url>] [--backend auto|wayland|x11] [--windowed|--fullscreen] [--diagnostic] [--freeze-live-updates] [--no-live-session] [--live-no-private] [--no-hosted-sessions]"
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

fn parse_args(
) -> Result<(nc_dash::NativeLaunchOptions, Option<String>, String, std::path::PathBuf), Box<dyn std::error::Error>>
{
    let mut native = nc_dash::NativeLaunchOptions::default();
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
        native,
        relay_override,
        password.ok_or("--password is required")?,
        snapshot_path.ok_or("--snapshot is required")?,
    ))
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn hosted_game_view(snapshot: &GameState, geometry: ScreenGeometry) -> HostedGameView {
    let dashboard = build_hosted_dash_app(snapshot, geometry).expect("hosted dash app");
    let seat = u8::try_from(snapshot.player_seat).expect("player seat fits in u8");
    HostedGameView {
        row: JoinedGameRow::new(
            &snapshot.game_id,
            "joined",
            &snapshot.player_name,
            "nc-host",
            "ws://127.0.0.1:8080",
            "daemon",
            Some(seat),
            &format!("y{} t{}", snapshot.year, snapshot.turn),
        ),
        snapshot: snapshot.clone(),
        dashboard,
        submit_input: String::new(),
        submit_status: None,
    }
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

    let snapshot: GameState = serde_json::from_slice(&std::fs::read(&snapshot_path)?)?;
    let geometry = ScreenGeometry::new(120, 40);
    let mut app = LobbyApp::new(nc_dash::startup::LobbyStartupOptions {
        relay_override,
        native,
    });
    app.geometry = geometry;
    let loaded = app.transport.unlock(&password)?;
    app.state.apply_loaded(loaded);
    app.state.route = LobbyRoute::Home;

    repro_support::run_stateful_rendered_ui_repro(
        "unlocked_route_snapshot_native_repro",
        match native.backend_preference {
            NativeBackendPreference::Auto => repro_support::BackendPreference::Auto,
            NativeBackendPreference::Wayland => repro_support::BackendPreference::Wayland,
            NativeBackendPreference::X11 => repro_support::BackendPreference::X11,
        },
        app,
        |app| app.render_ui_for_repro(),
        move |app, step| match step {
            0 => {
                app.dispatch_key_event_for_test(key(KeyCode::Tab));
                Some("home tab key before hosted open")
            }
            1 => {
                app.state.hosted_game = Some(hosted_game_view(&snapshot, geometry));
                app.state.route = LobbyRoute::HostedGame;
                Some("inject hosted snapshot after event loop start")
            }
            _ => None,
        },
    )
}
