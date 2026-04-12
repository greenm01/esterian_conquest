use std::collections::HashSet;
use tokio::sync::RwLock;
use std::sync::Arc;

pub struct WorkerRegistry {
    games: Arc<RwLock<HashSet<String>>>,
}

impl WorkerRegistry {
    pub fn new() -> Self {
        Self {
            games: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub async fn track_game(&self, game_id: String) {
        let mut games = self.games.write().await;
        games.insert(game_id);
    }

    pub async fn untrack_game(&self, game_id: &str) {
        let mut games = self.games.write().await;
        games.remove(game_id);
    }

    pub async fn list_games(&self) -> Vec<String> {
        let games = self.games.read().await;
        games.iter().cloned().collect()
    }

    pub async fn count(&self) -> usize {
        let games = self.games.read().await;
        games.len()
    }
}

impl Clone for WorkerRegistry {
    fn clone(&self) -> Self {
        Self {
            games: Arc::clone(&self.games),
        }
    }
}

impl Default for WorkerRegistry {
    fn default() -> Self {
        Self::new()
    }
}
