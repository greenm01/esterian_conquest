#[path = "support/repro_support.rs"]
mod repro_support;

use crossterm::event::{KeyModifiers, MouseEvent, MouseEventKind};
use nc_dash::{DashApp, ScreenGeometry, lobby::hosted::dashboard::build_hosted_dash_app};
use nc_nostr::state_sync::GameState;
use repro_support::{BackendPreference, run_stateful_rendered_ui_repro};

fn print_usage() {
    println!(
        "Usage: cargo run -p nc-dash --example hosted_snapshot_mouse_repro -- --snapshot <path> [--backend auto|wayland|x11]"
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

fn mouse_move(column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind: MouseEventKind::Moved,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
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
    run_stateful_rendered_ui_repro(
        "hosted_snapshot_mouse_repro",
        backend,
        app,
        |app: &mut DashApp| app.render_ui_for_repro(),
        |app: &mut DashApp, step| match step {
            0 => {
                let Some(target) = app.first_owned_planet_coords_for_repro() else {
                    return Some("no owned planet coords");
                };
                let Some((column, row)) = app.screen_point_for_sector_for_repro(target) else {
                    return Some("no screen point for owned planet");
                };
                let _ = app.dispatch_mouse_event_for_repro(mouse_move(column, row));
                Some("hover owned planet sector")
            }
            1 => {
                let target = [app.crosshair_x, app.crosshair_y];
                let Some((column, row)) = app.screen_point_for_sector_for_repro(target) else {
                    return Some("no screen point for crosshair sector");
                };
                let _ = app.dispatch_mouse_event_for_repro(mouse_move(column, row));
                Some("repeat hover on current sector")
            }
            _ => None,
        },
    )
}
