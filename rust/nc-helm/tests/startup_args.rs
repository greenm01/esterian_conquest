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

#[test]
fn dir_flag_selects_local_launch_mode() {
    let command = parse_launch_command(vec![
        "nc-helm".to_string(),
        "--dir".to_string(),
        "/tmp/nc-helm-lab/map45-p25".to_string(),
    ])
    .expect("local launch should parse");
    match command {
        LaunchCommand::Launch(LaunchTarget::Local(options)) => {
            assert_eq!(
                options.game_dir.to_string_lossy(),
                "/tmp/nc-helm-lab/map45-p25"
            );
        }
        other => panic!("expected local launch command, got {other:?}"),
    }
}

#[test]
fn relay_and_dir_flags_conflict() {
    let result = parse_launch_command(vec![
        "nc-helm".to_string(),
        "--relay".to_string(),
        "ws://127.0.0.1:8080".to_string(),
        "--dir".to_string(),
        "/tmp/nc-helm-lab/map45-p25".to_string(),
    ]);

    assert!(result.is_err());
    let message = result
        .expect_err("relay and dir should conflict")
        .to_string();
    assert!(message.contains("cannot combine"));
    assert!(message.contains("--relay"));
    assert!(message.contains("--dir"));
}
