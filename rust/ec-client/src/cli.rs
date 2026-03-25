use std::io::IsTerminal;
use std::path::PathBuf;

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use crate::app::{apply_action, App, AppConfig, AppOutcome};
use crate::terminal::stdout::StdoutTerminal;
use crate::terminal::ColorMode;
use crate::terminal::OutputEncoding;
use crate::terminal::Terminal;
use crate::theme;
use ec_data::game_config::{GameConfig, DEFAULT_GAME_CONFIG_KDL};

pub fn run(args: impl IntoIterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    let parsed_args = args.into_iter().collect::<Vec<_>>();
    let (config, encoding, color_mode) = parse_args(&parsed_args)?;

    // Load (or bootstrap) config.kdl before anything else.
    let game_config = load_or_bootstrap_game_config(&config.game_dir)?;
    let config = AppConfig {
        game_config: game_config.clone(),
        ..config
    };

    // Initialise theme; the config supplies the optional custom theme path.
    theme::initialize_from_game_dir(&config.game_dir, game_config.theme.clone())?;

    let mut app = App::load(config)?;
    let mut terminal = StdoutTerminal::with_encoding_and_color_mode(encoding, color_mode);

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

fn parse_args(
    args: &[String],
) -> Result<(AppConfig, OutputEncoding, ColorMode), Box<dyn std::error::Error>> {
    let mut dir = None;
    let mut player = None;
    let mut export_root = std::env::var_os("EC_CLIENT_EXPORT_ROOT").map(PathBuf::from);
    let mut queue_dir = std::env::var_os("EC_CLIENT_QUEUE_DIR").map(PathBuf::from);
    let mut encoding = OutputEncoding::Utf8;
    let mut explicit_color_mode: Option<ColorMode> = None;

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
            "--encoding" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --encoding (utf8 or cp437)".into());
                };
                encoding = match value.as_str() {
                    "utf8" | "utf-8" => OutputEncoding::Utf8,
                    "cp437" | "CP437" => OutputEncoding::Cp437,
                    other => {
                        return Err(
                            format!("unknown encoding '{other}'; expected utf8 or cp437").into(),
                        );
                    }
                };
                idx += 2;
            }
            "--color-mode" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err(
                        "missing value for --color-mode (ansi16, 256, truecolor, or auto)".into(),
                    );
                };
                explicit_color_mode = Some(match value.as_str() {
                    "ansi16" | "ansi-16" | "16" => ColorMode::Ansi16,
                    "256" | "color256" | "color-256" => ColorMode::Color256,
                    "truecolor" | "true-color" | "24bit" | "24-bit" => ColorMode::TrueColor,
                    "auto" => {
                        // Treat "auto" as an explicit no-op: use detection below.
                        explicit_color_mode = None;
                        idx += 2;
                        continue;
                    }
                    other => {
                        return Err(format!(
                            "unknown color mode '{other}'; expected ansi16, 256, truecolor, or auto"
                        )
                        .into());
                    }
                });
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

    // Resolve color mode: explicit flag > env-based detection > Ansi16 safe default
    // when CP437 encoding is in use (BBS door context).
    let color_mode = explicit_color_mode.unwrap_or_else(|| {
        if encoding == OutputEncoding::Cp437 {
            // BBS/door clients: assume classic 16-color unless explicitly overridden.
            ColorMode::Ansi16
        } else {
            detect_color_mode()
        }
    });

    Ok((
        AppConfig {
            game_dir: dir,
            player_record_index_1_based,
            export_root,
            queue_dir,
            // Placeholder; overwritten in run() after loading config.kdl.
            game_config: GameConfig::default(),
        },
        encoding,
        color_mode,
    ))
}

/// Detect the terminal's color depth from standard environment variables.
///
/// Detection order:
/// 1. `COLORTERM=truecolor` or `COLORTERM=24bit` → [`ColorMode::TrueColor`]
/// 2. `TERM` containing `256color`               → [`ColorMode::Color256`]
/// 3. Fallback                                   → [`ColorMode::Ansi16`]
///
/// This is intentionally conservative: we only claim a richer mode when there
/// is explicit evidence, so the fallback is always the safest (16-color) choice.
pub fn detect_color_mode() -> ColorMode {
    if let Ok(colorterm) = std::env::var("COLORTERM") {
        let ct = colorterm.to_ascii_lowercase();
        if ct == "truecolor" || ct == "24bit" {
            return ColorMode::TrueColor;
        }
    }
    if let Ok(term) = std::env::var("TERM") {
        if term.contains("256color") {
            return ColorMode::Color256;
        }
    }
    ColorMode::Ansi16
}

fn print_usage() {
    println!("Usage:");
    println!(
        "  ec-client --dir <game_dir> --player <1-based empire index> \
         [--encoding <utf8|cp437>] [--color-mode <ansi16|256|truecolor|auto>] \
         [--export-root <dir>] [--queue-dir <dir>]"
    );
    println!();
    println!("Color modes:");
    println!("  ansi16     Classic 16-color ANSI (BBS doors, legacy terminals)");
    println!("  256        256-color xterm palette");
    println!("  truecolor  24-bit RGB (default for modern local/SSH terminals)");
    println!("  auto       Detect from COLORTERM/TERM environment variables (default)");
}

/// Load `config.kdl` from the game directory, bootstrapping the default if
/// absent.
///
/// Returns the parsed [`GameConfig`] on success, or a descriptive error if the
/// file is present but invalid.
fn load_or_bootstrap_game_config(
    game_dir: &std::path::Path,
) -> Result<GameConfig, Box<dyn std::error::Error>> {
    let config_path = game_dir.join("config.kdl");
    if !config_path.exists() {
        std::fs::write(&config_path, DEFAULT_GAME_CONFIG_KDL)?;
    }
    GameConfig::load_kdl(&config_path)
        .map_err(|err| format!("{}: {}", config_path.display(), err).into())
}
