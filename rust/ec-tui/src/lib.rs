use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppMode {
    Player,
    Util,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CliOptions {
    pub mode: AppMode,
    pub dir: PathBuf,
}

pub fn resolve_game_dir(options: CliOptions) -> CliOptions {
    if looks_like_game_dir(&options.dir) {
        return options;
    }

    let repo_default = repo_root().join("fixtures/ecmaint-post/v1.5");
    if looks_like_game_dir(&repo_default) {
        CliOptions {
            mode: options.mode,
            dir: repo_default,
        }
    } else {
        options
    }
}

pub fn parse_args(
    mut args: impl Iterator<Item = String>,
    current_dir: PathBuf,
) -> Result<CliOptions, Box<dyn std::error::Error>> {
    match args.next() {
        None => Ok(CliOptions {
            mode: AppMode::Player,
            dir: current_dir,
        }),
        Some(first) if first == "util" => Ok(CliOptions {
            mode: AppMode::Util,
            dir: args.next().map(PathBuf::from).unwrap_or(current_dir),
        }),
        Some(first) => Ok(CliOptions {
            mode: AppMode::Player,
            dir: PathBuf::from(first),
        }),
    }
}

fn looks_like_game_dir(dir: &Path) -> bool {
    ["PLAYER.DAT", "PLANETS.DAT", "SETUP.DAT", "CONQUEST.DAT"]
        .into_iter()
        .all(|name| dir.join(name).is_file())
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
