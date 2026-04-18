#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NativeWindowMode {
    #[default]
    Windowed,
    BorderlessFullscreen,
}

impl NativeWindowMode {
    pub fn cli_label(self) -> &'static str {
        match self {
            Self::Windowed => "windowed",
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
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LaunchTargetOptions {
    pub relay_override: Option<String>,
    pub native: NativeLaunchOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchTarget {
    Lobby(LaunchTargetOptions),
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
    let mut native = NativeLaunchOptions::default();
    let mut explicit_windowed = false;
    let mut explicit_fullscreen = false;
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => return Ok(LaunchCommand::Help),
            "--windowed" => {
                if explicit_fullscreen {
                    return Err("cannot combine --windowed and --fullscreen".into());
                }
                explicit_windowed = true;
                native.window_mode = NativeWindowMode::Windowed;
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
            other if other.starts_with('-') => {
                return Err(format!("unrecognized option: {other}").into());
            }
            other => {
                return Err(format!("unexpected positional argument: {other}").into());
            }
        }
    }

    Ok(LaunchCommand::Launch(LaunchTarget::Lobby(
        LaunchTargetOptions {
            relay_override,
            native,
        },
    )))
}

pub fn print_usage() {
    eprintln!("nc-helm — Nostrian Conquest hosted player client");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!(
        "    nc-helm [--relay <url>] [--windowed | --fullscreen] [--backend <auto|wayland|x11>] [--diagnostic]"
    );
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("    --help, -h       Show this help");
    eprintln!("    --relay <url>    Override the hosted relay for this session");
    eprintln!("    --windowed       Open in a normal decorated resizable window");
    eprintln!("    --fullscreen     Force borderless fullscreen for this session");
    eprintln!("    --backend <...>  Select native backend: auto (default), wayland, or x11");
    eprintln!("    --diagnostic     Enable verbose native diagnostics");
}
