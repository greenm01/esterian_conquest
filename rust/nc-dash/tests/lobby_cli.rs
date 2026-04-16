use std::path::PathBuf;

use nc_dash::startup::{
    LaunchCommand, LaunchTarget, LobbyStartupOptions, NativeLaunchOptions, NativeWindowMode,
    parse_launch_command,
};

fn parse(args: &[&str]) -> LaunchCommand {
    let argv = std::iter::once("nc-dash".to_string())
        .chain(args.iter().map(|arg| arg.to_string()))
        .collect::<Vec<_>>();
    parse_launch_command(argv).expect("parse command")
}

#[test]
fn no_args_launch_lobby_by_default() {
    assert_eq!(
        parse(&[]),
        LaunchCommand::Launch(LaunchTarget::Lobby(LobbyStartupOptions::default()))
    );
}

#[test]
fn explicit_dir_opens_dashboard_path() {
    assert_eq!(
        parse(&["--dir", "/tmp/example-game"]),
        LaunchCommand::Launch(LaunchTarget::Dashboard {
            game_dir: PathBuf::from("/tmp/example-game"),
            native: NativeLaunchOptions {
                window_mode: NativeWindowMode::MaximizedWindow,
                ..NativeLaunchOptions::default()
            },
        })
    );
}

#[test]
fn positional_path_keeps_dashboard_compatibility() {
    assert_eq!(
        parse(&["/tmp/legacy-game"]),
        LaunchCommand::Launch(LaunchTarget::Dashboard {
            game_dir: PathBuf::from("/tmp/legacy-game"),
            native: NativeLaunchOptions {
                window_mode: NativeWindowMode::MaximizedWindow,
                ..NativeLaunchOptions::default()
            },
        })
    );
}
