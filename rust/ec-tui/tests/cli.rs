use std::path::PathBuf;

use ec_tui::{AppMode, CliOptions, parse_args, resolve_game_dir};

#[test]
fn parse_args_defaults_to_player_mode_and_current_dir() {
    let cwd = PathBuf::from("/tmp/ecgame");
    let parsed = parse_args(std::iter::empty(), cwd.clone()).unwrap();
    assert_eq!(
        parsed,
        CliOptions {
            mode: AppMode::Player,
            dir: cwd,
        }
    );
}

#[test]
fn parse_args_supports_util_subcommand_and_optional_dir() {
    let cwd = PathBuf::from("/tmp/ecgame");
    let parsed = parse_args(
        ["util", "/tmp/ecutil"].into_iter().map(String::from),
        cwd,
    )
    .unwrap();
    assert_eq!(
        parsed,
        CliOptions {
            mode: AppMode::Util,
            dir: PathBuf::from("/tmp/ecutil"),
        }
    );
}

#[test]
fn resolve_game_dir_falls_back_to_repo_post_maint_snapshot() {
    let options = CliOptions {
        mode: AppMode::Util,
        dir: PathBuf::from("/tmp/not-a-game-dir"),
    };
    let resolved = resolve_game_dir(options);
    assert!(resolved.dir.ends_with("fixtures/ecmaint-post/v1.5"));
}
