use nc_dash::startup::{NativeBackendPreference, NativeLaunchOptions, NativeWindowMode};

pub fn print_usage(example_name: &str) {
    println!(
        "Usage: cargo run -p nc-dash --example {example_name} -- [--backend auto|wayland|x11] [--windowed|--fullscreen] [--diagnostic]"
    );
}

pub fn parse_native_options() -> Result<NativeLaunchOptions, Box<dyn std::error::Error>> {
    let mut native = NativeLaunchOptions::default();
    let mut explicit_windowed = false;
    let mut explicit_fullscreen = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--backend" => {
                let Some(value) = args.next() else {
                    return Err("--backend requires one of: auto, wayland, x11".into());
                };
                native.backend_preference = match value.as_str() {
                    "auto" => NativeBackendPreference::Auto,
                    "wayland" => NativeBackendPreference::Wayland,
                    "x11" => NativeBackendPreference::X11,
                    _ => {
                        return Err(
                            format!("unknown backend '{value}'; expected auto, wayland, or x11")
                                .into(),
                        );
                    }
                };
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
            "--help" | "-h" => return Err("help requested".into()),
            other => return Err(format!("unrecognized argument: {other}").into()),
        }
    }
    Ok(native)
}
