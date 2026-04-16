#[path = "support/repro_support.rs"]
mod repro_support;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nc_dash::{LobbyApp, ScreenGeometry, lobby::state::LobbyRoute};
use repro_support::{parse_args, print_usage, run_stateful_rendered_ui_repro};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = match parse_args() {
        Ok(options) => options,
        Err(err) if err.to_string() == "help requested" => {
            print_usage("lobby_interaction_repro");
            return Ok(());
        }
        Err(err) => return Err(err),
    };

    let app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    run_stateful_rendered_ui_repro(
        "lobby_interaction_repro",
        options.backend,
        app,
        |app| app.render_ui_for_repro(),
        |app, step| match step {
            0 => {
                app.dispatch_key_event_for_test(key(KeyCode::Char('?')));
                Some("open help")
            }
            1 => {
                app.dispatch_key_event_for_test(key(KeyCode::Esc));
                Some("close help")
            }
            2 => {
                app.dispatch_key_event_for_test(key(KeyCode::Tab));
                Some("next tab")
            }
            3 => {
                app.dispatch_key_event_for_test(key(KeyCode::Tab));
                Some("next tab again")
            }
            _ => None,
        },
    )
}
