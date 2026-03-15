use std::io::IsTerminal;
use std::path::PathBuf;

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use crate::app::{App, AppConfig, AppOutcome, apply_action};
use crate::terminal::Terminal;
use crate::terminal::stdout::StdoutTerminal;

pub fn run(args: impl IntoIterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    let parsed_args = args.into_iter().collect::<Vec<_>>();
    let config = parse_args(&parsed_args)?;
    let mut app = App::load(config)?;
    let mut terminal = StdoutTerminal::new();

    if std::io::stdout().is_terminal() {
        run_interactive(&mut app, &mut terminal)
    } else {
        app.render(&mut terminal)
    }
}

fn run_interactive(
    app: &mut App,
    terminal: &mut StdoutTerminal,
) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let result = run_interactive_inner(app, terminal);
    disable_raw_mode()?;
    let cleanup_result = terminal.clear_and_restore();
    result.and(cleanup_result)
}

fn run_interactive_inner(
    app: &mut App,
    terminal: &mut StdoutTerminal,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        app.render(terminal)?;
        let key = terminal.read_key()?;
        let action = app.handle_key(key);
        let outcome = apply_action(app, action);
        if matches!(outcome, AppOutcome::Quit) {
            return Ok(());
        }
    }
}

fn parse_args(args: &[String]) -> Result<AppConfig, Box<dyn std::error::Error>> {
    let mut dir = None;
    let mut player = None;
    let mut export_root = std::env::var_os("EC_CLIENT_EXPORT_ROOT").map(PathBuf::from);
    let mut queue_dir = std::env::var_os("EC_CLIENT_QUEUE_DIR").map(PathBuf::from);
    let mut startup_config = std::env::var_os("EC_CLIENT_STARTUP_CONFIG").map(PathBuf::from);

    let mut idx = 1;
    while idx < args.len() {
        match args[idx].as_str() {
            "--dir" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --dir".into());
                };
                dir = Some(PathBuf::from(value));
                idx += 2;
            }
            "--player" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --player".into());
                };
                player = Some(value.parse::<usize>()?);
                idx += 2;
            }
            "--export-root" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --export-root".into());
                };
                export_root = Some(PathBuf::from(value));
                idx += 2;
            }
            "--queue-dir" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --queue-dir".into());
                };
                queue_dir = Some(PathBuf::from(value));
                idx += 2;
            }
            "--startup-config" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --startup-config".into());
                };
                startup_config = Some(PathBuf::from(value));
                idx += 2;
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other => {
                return Err(format!("unknown argument: {other}").into());
            }
        }
    }

    let dir = dir.ok_or("usage: ec-client --dir <game_dir> --player <1-based empire index>")?;
    let player_record_index_1_based =
        player.ok_or("usage: ec-client --dir <game_dir> --player <1-based empire index>")?;
    if player_record_index_1_based == 0 {
        return Err("--player must be >= 1".into());
    }

    Ok(AppConfig {
        game_dir: dir,
        player_record_index_1_based,
        export_root,
        queue_dir,
        startup_config,
    })
}

fn print_usage() {
    println!("Usage:");
    println!("  ec-client --dir <game_dir> --player <1-based empire index> [--export-root <dir>] [--queue-dir <dir>] [--startup-config <kdl>]");
}
