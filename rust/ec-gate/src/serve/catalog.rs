use std::path::{Path, PathBuf};

use ec_data::{CampaignStore, GameConfig, HostedSeat};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedGame {
    pub game_id: String,
    pub game_name: String,
    pub seats: Vec<HostedSeat>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedGameEntry {
    pub dir: PathBuf,
    pub game: HostedGame,
}

pub fn load_hosted_games(game_dirs: &[PathBuf]) -> Result<Vec<HostedGameEntry>, String> {
    game_dirs
        .iter()
        .map(|dir| load_hosted_game(dir))
        .collect::<Result<Vec<_>, _>>()
}

pub fn load_hosted_game(dir: &Path) -> Result<HostedGameEntry, String> {
    let game_id = dir
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| format!("cannot derive game-id from {}", dir.display()))?
        .to_string();
    let store = CampaignStore::open_default_in_dir(dir)
        .map_err(|err| format!("cannot open {}: {err}", dir.join("ecgame.db").display()))?;
    let seats = store
        .hosted_seats()
        .map_err(|err| format!("cannot load hosted seats for {}: {err}", dir.display()))?;
    if seats.is_empty() {
        let legacy_roster = dir.join("roster.kdl");
        if legacy_roster.exists() {
            return Err(format!(
                "{} still uses roster.kdl; run `ec-sysop nostr migrate-roster --dir {}` first",
                dir.display(),
                dir.display()
            ));
        }
        return Err(format!(
            "{} has no hosted seats in ecgame.db; initialize hosted seats before serving",
            dir.display()
        ));
    }
    let config_path = dir.join("config.kdl");
    let game_config = GameConfig::load_kdl(&config_path)
        .map_err(|err| format!("cannot load {}: {err}", config_path.display()))?;
    Ok(HostedGameEntry {
        dir: dir.to_path_buf(),
        game: HostedGame {
            game_id,
            game_name: game_config.game_name,
            seats,
        },
    })
}
