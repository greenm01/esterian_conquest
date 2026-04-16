#[path = "support/repro_support.rs"]
mod repro_support;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nc_dash::{DashApp, ScreenGeometry};
use repro_support::{parse_args, print_usage, run_stateful_rendered_ui_repro};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = match parse_args() {
        Ok(options) => options,
        Err(err) if err.to_string() == "help requested" => {
            print_usage("dash_interaction_repro");
            return Ok(());
        }
        Err(err) => return Err(err),
    };

    let app = DashApp::new_for_repro(ScreenGeometry::new(160, 40), ScreenGeometry::new(108, 26));
    run_stateful_rendered_ui_repro(
        "dash_interaction_repro",
        options.backend,
        app,
        |app| app.render_ui_for_repro(),
        |app, step| match step {
            0 => {
                app.dispatch_key_event_for_repro(key(KeyCode::Tab));
                Some("cycle focus")
            }
            1 => {
                app.dispatch_key_event_for_repro(key(KeyCode::Char('?')));
                Some("open help")
            }
            2 => {
                app.dispatch_key_event_for_repro(key(KeyCode::Esc));
                Some("close help")
            }
            3 => {
                app.dispatch_key_event_for_repro(key(KeyCode::Char('v')));
                Some("toggle map view")
            }
            _ => None,
        },
    )
}
