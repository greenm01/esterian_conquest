use nc_data::hosted::HostedStore;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledGame {
    pub game_id: String,
    pub due_unix_seconds: i64,
}

pub struct Scheduler {
    games_root: PathBuf,
}

impl Scheduler {
    pub fn new(games_root: PathBuf) -> Self {
        Self { games_root }
    }

    pub fn check_due_games(&self) -> Vec<String> {
        let mut due_games = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&self.games_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let db_path = path.join("hosted.db");
                    if db_path.exists() {
                        if let Some(game_id) = path.file_name().and_then(|n| n.to_str()) {
                            if self.is_due(game_id, &db_path) {
                                due_games.push(game_id.to_string());
                            }
                        }
                    }
                }
            }
        }

        due_games
    }

    fn is_due(&self, game_id: &str, db_path: &PathBuf) -> bool {
        let store = match HostedStore::open(db_path) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let settings = match nc_data::hosted::get_settings(store.connection(), game_id) {
            Ok(s) => s,
            Err(_) => return false,
        };

        if !settings.maintenance_enabled {
            return false;
        }

        let now = chrono::Utc::now().timestamp();

        match settings.maintenance_next_due_unix_seconds {
            Some(due) => now >= due,
            None => true,
        }
    }

    pub fn get_next_maintenance_time(&self, game_id: &str, db_path: &PathBuf) -> Option<i64> {
        let store = match HostedStore::open(db_path) {
            Ok(s) => s,
            Err(_) => return None,
        };

        let settings = match nc_data::hosted::get_settings(store.connection(), game_id) {
            Ok(s) => s,
            Err(_) => return None,
        };

        if !settings.maintenance_enabled {
            return None;
        }

        match settings.maintenance_next_due_unix_seconds {
            Some(due) => Some(due),
            None => {
                let interval = settings.maintenance_interval_minutes as i64 * 60;
                Some(chrono::Utc::now().timestamp() + interval)
            }
        }
    }
}
