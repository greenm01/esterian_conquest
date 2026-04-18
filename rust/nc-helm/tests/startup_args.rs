use nc_helm::{LaunchCommand, LaunchTarget, NativeWindowMode, parse_launch_command};

#[test]
fn windowed_mode_is_not_maximized_mode_anymore() {
    let command = parse_launch_command(vec!["nc-helm".to_string(), "--windowed".to_string()])
        .expect("windowed launch should parse");
    match command {
        LaunchCommand::Launch(LaunchTarget::Lobby(options)) => {
            assert_eq!(options.native.window_mode, NativeWindowMode::Windowed);
        }
        other => panic!("expected launch command, got {other:?}"),
    }
}

#[test]
fn fullscreen_flag_still_selects_borderless_fullscreen() {
    let command = parse_launch_command(vec!["nc-helm".to_string(), "--fullscreen".to_string()])
        .expect("fullscreen launch should parse");
    match command {
        LaunchCommand::Launch(LaunchTarget::Lobby(options)) => {
            assert_eq!(
                options.native.window_mode,
                NativeWindowMode::BorderlessFullscreen
            );
        }
        other => panic!("expected launch command, got {other:?}"),
    }
}
