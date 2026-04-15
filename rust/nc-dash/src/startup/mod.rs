//! Startup parsing and mode selection for `nc-dash`.

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NativeWindowMode {
    #[default]
    MaximizedWindow,
    BorderlessFullscreen,
}

impl NativeWindowMode {
    pub fn cli_label(self) -> &'static str {
        match self {
            Self::MaximizedWindow => "maximized",
            Self::BorderlessFullscreen => "fullscreen",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LobbyStartupOptions {
    pub relay_override: Option<String>,
    pub window_mode: NativeWindowMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchTarget {
    Lobby(LobbyStartupOptions),
    Dashboard {
        game_dir: PathBuf,
        window_mode: NativeWindowMode,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchCommand {
    Help,
    Launch(LaunchTarget),
}

pub fn parse_launch_command(
    args: impl IntoIterator<Item = String>,
) -> Result<LaunchCommand, Box<dyn std::error::Error>> {
    let args: Vec<String> = args.into_iter().collect();
    parse_launch_args(&args[1..])
}

fn parse_launch_args(args: &[String]) -> Result<LaunchCommand, Box<dyn std::error::Error>> {
    let mut relay_override = None;
    let mut explicit_lobby = false;
    let mut dashboard_dir: Option<PathBuf> = None;
    let mut positional_game_dir: Option<PathBuf> = None;
    let mut window_mode = NativeWindowMode::MaximizedWindow;
    let mut explicit_windowed = false;
    let mut explicit_fullscreen = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => return Ok(LaunchCommand::Help),
            "--lobby" => {
                explicit_lobby = true;
                i += 1;
            }
            "--windowed" => {
                if explicit_fullscreen {
                    return Err("cannot combine --windowed and --fullscreen".into());
                }
                explicit_windowed = true;
                window_mode = NativeWindowMode::MaximizedWindow;
                i += 1;
            }
            "--fullscreen" => {
                if explicit_windowed {
                    return Err("cannot combine --windowed and --fullscreen".into());
                }
                explicit_fullscreen = true;
                window_mode = NativeWindowMode::BorderlessFullscreen;
                i += 1;
            }
            "--relay" => {
                let value = args.get(i + 1).ok_or("--relay requires a value")?;
                relay_override = Some(value.clone());
                i += 2;
            }
            "--dir" => {
                let value = args.get(i + 1).ok_or("--dir requires a value")?;
                dashboard_dir = Some(PathBuf::from(value));
                i += 2;
            }
            other if other.starts_with('-') => {
                return Err(format!("unrecognized option: {other}").into());
            }
            other => {
                if positional_game_dir.is_some() {
                    return Err("too many positional arguments".into());
                }
                positional_game_dir = Some(PathBuf::from(other));
                i += 1;
            }
        }
    }

    if dashboard_dir.is_some() && (explicit_lobby || relay_override.is_some()) {
        return Err("cannot combine dashboard and lobby startup options".into());
    }

    if let Some(game_dir) = dashboard_dir.or(positional_game_dir) {
        return Ok(LaunchCommand::Launch(LaunchTarget::Dashboard {
            game_dir,
            window_mode,
        }));
    }

    Ok(LaunchCommand::Launch(LaunchTarget::Lobby(
        LobbyStartupOptions {
            relay_override,
            window_mode,
        },
    )))
}

pub fn print_usage() {
    eprintln!("nc-dash — Nostrian Conquest hosted lobby and dashboard");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    nc-dash [--lobby] [--relay <url>] [--windowed | --fullscreen]");
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("    --help, -h       Show this help");
    eprintln!("    --lobby          Open the hosted lobby explicitly");
    eprintln!("    --relay <url>    Override the hosted relay for this session");
    eprintln!("    --windowed       Open in a maximized window (default)");
    eprintln!("    --fullscreen     Force borderless fullscreen");
    eprintln!();
    eprintln!("DEVELOPER:");
    eprintln!("    nc-dash --dir <path> [--windowed | --fullscreen]");
    eprintln!("                         Open a local dashboard directly");
}

#[cfg(test)]
mod tests {
    use super::{
        LaunchCommand, LaunchTarget, LobbyStartupOptions, NativeWindowMode, parse_launch_args,
    };
    use std::path::PathBuf;

    fn parse(args: &[&str]) -> LaunchCommand {
        parse_launch_args(&args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>())
            .expect("parse command")
    }

    #[test]
    fn no_args_launches_lobby() {
        assert_eq!(
            parse(&[]),
            LaunchCommand::Launch(LaunchTarget::Lobby(LobbyStartupOptions::default()))
        );
    }

    #[test]
    fn explicit_dir_launches_dashboard() {
        assert_eq!(
            parse(&["--dir", "/tmp/game"]),
            LaunchCommand::Launch(LaunchTarget::Dashboard {
                game_dir: PathBuf::from("/tmp/game"),
                window_mode: NativeWindowMode::MaximizedWindow,
            })
        );
    }

    #[test]
    fn positional_path_is_dashboard_compat_alias() {
        assert_eq!(
            parse(&["/tmp/game"]),
            LaunchCommand::Launch(LaunchTarget::Dashboard {
                game_dir: PathBuf::from("/tmp/game"),
                window_mode: NativeWindowMode::MaximizedWindow,
            })
        );
    }

    #[test]
    fn relay_flag_attaches_to_lobby() {
        assert_eq!(
            parse(&["--relay", "wss://relay.example.com"]),
            LaunchCommand::Launch(LaunchTarget::Lobby(LobbyStartupOptions {
                relay_override: Some("wss://relay.example.com".to_string()),
                window_mode: NativeWindowMode::MaximizedWindow,
            }))
        );
    }

    #[test]
    fn fullscreen_flag_attaches_to_lobby() {
        assert_eq!(
            parse(&["--fullscreen"]),
            LaunchCommand::Launch(LaunchTarget::Lobby(LobbyStartupOptions {
                relay_override: None,
                window_mode: NativeWindowMode::BorderlessFullscreen,
            }))
        );
    }

    #[test]
    fn fullscreen_flag_attaches_to_dashboard() {
        assert_eq!(
            parse(&["--dir", "/tmp/game", "--fullscreen"]),
            LaunchCommand::Launch(LaunchTarget::Dashboard {
                game_dir: PathBuf::from("/tmp/game"),
                window_mode: NativeWindowMode::BorderlessFullscreen,
            })
        );
    }

    #[test]
    fn conflicting_window_mode_flags_fail() {
        let args = ["--windowed", "--fullscreen"]
            .iter()
            .map(|arg| arg.to_string())
            .collect::<Vec<_>>();
        let err = parse_launch_args(&args).expect_err("conflicting window flags should fail");
        assert!(
            err.to_string()
                .contains("cannot combine --windowed and --fullscreen")
        );
    }

    #[test]
    fn windowed_flag_attaches_to_dashboard() {
        assert_eq!(
            parse(&["--dir", "/tmp/game", "--windowed"]),
            LaunchCommand::Launch(LaunchTarget::Dashboard {
                game_dir: PathBuf::from("/tmp/game"),
                window_mode: NativeWindowMode::MaximizedWindow,
            })
        );
    }
}
