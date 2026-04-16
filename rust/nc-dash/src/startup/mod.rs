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
            Self::MaximizedWindow => "windowed",
            Self::BorderlessFullscreen => "fullscreen",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NativeBackendPreference {
    #[default]
    Auto,
    Wayland,
    X11,
}

impl NativeBackendPreference {
    pub fn cli_label(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Wayland => "wayland",
            Self::X11 => "x11",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "auto" => Ok(Self::Auto),
            "wayland" => Ok(Self::Wayland),
            "x11" => Ok(Self::X11),
            _ => Err(format!(
                "unknown backend '{value}'; expected auto, wayland, or x11"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NativeLaunchOptions {
    pub window_mode: NativeWindowMode,
    pub backend_preference: NativeBackendPreference,
    pub diagnostic_mode: bool,
    pub serialize_redraws: bool,
    pub freeze_live_updates: bool,
    pub disable_hosted_sessions: bool,
    pub disable_live_session: bool,
    pub disable_live_private_stream: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LobbyStartupOptions {
    pub relay_override: Option<String>,
    pub native: NativeLaunchOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchTarget {
    Lobby(LobbyStartupOptions),
    Dashboard {
        game_dir: PathBuf,
        native: NativeLaunchOptions,
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
    let mut native = NativeLaunchOptions::default();
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
                native.window_mode = NativeWindowMode::MaximizedWindow;
                i += 1;
            }
            "--fullscreen" => {
                if explicit_windowed {
                    return Err("cannot combine --windowed and --fullscreen".into());
                }
                explicit_fullscreen = true;
                native.window_mode = NativeWindowMode::BorderlessFullscreen;
                i += 1;
            }
            "--diagnostic" => {
                native.diagnostic_mode = true;
                i += 1;
            }
            "--serialize-redraws" => {
                native.serialize_redraws = true;
                i += 1;
            }
            "--freeze-live-updates" => {
                native.freeze_live_updates = true;
                i += 1;
            }
            "--no-hosted-sessions" => {
                native.disable_hosted_sessions = true;
                i += 1;
            }
            "--no-live-session" => {
                native.disable_live_session = true;
                i += 1;
            }
            "--live-no-private" => {
                native.disable_live_private_stream = true;
                i += 1;
            }
            "--backend" => {
                let value = args.get(i + 1).ok_or("--backend requires a value")?;
                native.backend_preference = NativeBackendPreference::parse(value)
                    .map_err(|err| -> Box<dyn std::error::Error> { err.into() })?;
                i += 2;
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
            native,
        }));
    }

    Ok(LaunchCommand::Launch(LaunchTarget::Lobby(
        LobbyStartupOptions {
            relay_override,
            native,
        },
    )))
}

pub fn print_usage() {
    eprintln!("nc-dash — Nostrian Conquest hosted lobby and dashboard");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!(
        "    nc-dash [--lobby] [--relay <url>] [--windowed | --fullscreen] [--backend <auto|wayland|x11>] [--diagnostic] [--serialize-redraws] [--freeze-live-updates] [--live-no-private] [--no-live-session] [--no-hosted-sessions]"
    );
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("    --help, -h       Show this help");
    eprintln!("    --lobby          Open the hosted lobby explicitly");
    eprintln!("    --relay <url>    Override the hosted relay for this session");
    eprintln!(
        "    --windowed       Open in a decorated system window and restore the last saved size/state"
    );
    eprintln!("    --fullscreen     Force borderless fullscreen for this session only");
    eprintln!("    --backend <...>  Select native backend: auto (default), wayland, or x11");
    eprintln!(
        "    --diagnostic     Write detailed native startup diagnostics to ~/.local/share/nc/nc-dash.log"
    );
    eprintln!(
        "    --serialize-redraws  Defer redraw requests to the event-loop wait phase for race diagnostics"
    );
    eprintln!(
        "    --freeze-live-updates  Disable lobby poll/update churn for diagnostic isolation"
    );
    eprintln!(
        "    --live-no-private      Keep public catalog/notices, but suppress private hosted live traffic"
    );
    eprintln!(
        "    --no-live-session      Keep hosted request/open support but skip the hosted live-session stream"
    );
    eprintln!(
        "    --no-hosted-sessions   Skip creating hosted session/live-session objects for diagnostic isolation"
    );
    eprintln!();
    eprintln!("DEVELOPER:");
    eprintln!(
        "    nc-dash --dir <path> [--windowed | --fullscreen] [--backend <auto|wayland|x11>] [--diagnostic] [--serialize-redraws] [--freeze-live-updates] [--live-no-private] [--no-live-session] [--no-hosted-sessions]"
    );
    eprintln!("                         Open a local dashboard directly");
}

#[cfg(test)]
mod tests {
    use super::{
        LaunchCommand, LaunchTarget, LobbyStartupOptions, NativeBackendPreference,
        NativeLaunchOptions, NativeWindowMode, parse_launch_args,
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
                native: NativeLaunchOptions::default(),
            })
        );
    }

    #[test]
    fn positional_path_is_dashboard_compat_alias() {
        assert_eq!(
            parse(&["/tmp/game"]),
            LaunchCommand::Launch(LaunchTarget::Dashboard {
                game_dir: PathBuf::from("/tmp/game"),
                native: NativeLaunchOptions::default(),
            })
        );
    }

    #[test]
    fn relay_flag_attaches_to_lobby() {
        assert_eq!(
            parse(&["--relay", "wss://relay.example.com"]),
            LaunchCommand::Launch(LaunchTarget::Lobby(LobbyStartupOptions {
                relay_override: Some("wss://relay.example.com".to_string()),
                native: NativeLaunchOptions::default(),
            }))
        );
    }

    #[test]
    fn fullscreen_flag_attaches_to_lobby() {
        assert_eq!(
            parse(&["--fullscreen"]),
            LaunchCommand::Launch(LaunchTarget::Lobby(LobbyStartupOptions {
                relay_override: None,
                native: NativeLaunchOptions {
                    window_mode: NativeWindowMode::BorderlessFullscreen,
                    ..NativeLaunchOptions::default()
                },
            }))
        );
    }

    #[test]
    fn fullscreen_flag_attaches_to_dashboard() {
        assert_eq!(
            parse(&["--dir", "/tmp/game", "--fullscreen"]),
            LaunchCommand::Launch(LaunchTarget::Dashboard {
                game_dir: PathBuf::from("/tmp/game"),
                native: NativeLaunchOptions {
                    window_mode: NativeWindowMode::BorderlessFullscreen,
                    ..NativeLaunchOptions::default()
                },
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
                native: NativeLaunchOptions::default(),
            })
        );
    }

    #[test]
    fn backend_flag_attaches_to_lobby() {
        assert_eq!(
            parse(&["--backend", "wayland"]),
            LaunchCommand::Launch(LaunchTarget::Lobby(LobbyStartupOptions {
                relay_override: None,
                native: NativeLaunchOptions {
                    backend_preference: NativeBackendPreference::Wayland,
                    ..NativeLaunchOptions::default()
                },
            }))
        );
    }

    #[test]
    fn freeze_live_updates_flag_attaches_to_lobby() {
        assert_eq!(
            parse(&["--freeze-live-updates"]),
            LaunchCommand::Launch(LaunchTarget::Lobby(LobbyStartupOptions {
                relay_override: None,
                native: NativeLaunchOptions {
                    freeze_live_updates: true,
                    ..NativeLaunchOptions::default()
                },
            }))
        );
    }

    #[test]
    fn diagnostic_flag_attaches_to_dashboard() {
        assert_eq!(
            parse(&["--dir", "/tmp/game", "--diagnostic"]),
            LaunchCommand::Launch(LaunchTarget::Dashboard {
                game_dir: PathBuf::from("/tmp/game"),
                native: NativeLaunchOptions {
                    diagnostic_mode: true,
                    ..NativeLaunchOptions::default()
                },
            })
        );
    }

    #[test]
    fn serialize_redraws_flag_attaches_to_lobby() {
        assert_eq!(
            parse(&["--serialize-redraws"]),
            LaunchCommand::Launch(LaunchTarget::Lobby(LobbyStartupOptions {
                relay_override: None,
                native: NativeLaunchOptions {
                    serialize_redraws: true,
                    ..NativeLaunchOptions::default()
                },
            }))
        );
    }

    #[test]
    fn no_hosted_sessions_flag_attaches_to_lobby() {
        assert_eq!(
            parse(&["--no-hosted-sessions"]),
            LaunchCommand::Launch(LaunchTarget::Lobby(LobbyStartupOptions {
                relay_override: None,
                native: NativeLaunchOptions {
                    disable_hosted_sessions: true,
                    ..NativeLaunchOptions::default()
                },
            }))
        );
    }

    #[test]
    fn no_live_session_flag_attaches_to_lobby() {
        assert_eq!(
            parse(&["--no-live-session"]),
            LaunchCommand::Launch(LaunchTarget::Lobby(LobbyStartupOptions {
                relay_override: None,
                native: NativeLaunchOptions {
                    disable_live_session: true,
                    ..NativeLaunchOptions::default()
                },
            }))
        );
    }

    #[test]
    fn live_no_private_flag_attaches_to_lobby() {
        assert_eq!(
            parse(&["--live-no-private"]),
            LaunchCommand::Launch(LaunchTarget::Lobby(LobbyStartupOptions {
                relay_override: None,
                native: NativeLaunchOptions {
                    disable_live_private_stream: true,
                    ..NativeLaunchOptions::default()
                },
            }))
        );
    }

    #[test]
    fn invalid_backend_value_fails() {
        let args = ["--backend", "bogus"]
            .iter()
            .map(|arg| arg.to_string())
            .collect::<Vec<_>>();
        let err = parse_launch_args(&args).expect_err("invalid backend should fail");
        assert!(
            err.to_string()
                .contains("unknown backend 'bogus'; expected auto, wayland, or x11")
        );
    }
}
