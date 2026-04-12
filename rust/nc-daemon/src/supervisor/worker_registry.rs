use crate::game::worker::{spawn_worker, GameWorkerHandle};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;

pub struct WorkerRegistry {
    workers: RwLock<HashMap<String, GameWorkerHandle>>,
    games_root: PathBuf,
}

impl WorkerRegistry {
    pub fn new(games_root: PathBuf) -> Self {
        Self {
            workers: RwLock::new(HashMap::new()),
            games_root,
        }
    }

    pub async fn get_or_create(&self, game_id: String) -> GameWorkerHandle {
        let mut workers = self.workers.write().await;
        
        if let Some(handle) = workers.get(&game_id) {
            return handle.clone();
        }

        let db_path = self.games_root.join(&game_id).join("hosted.db");
        
        let handle = spawn_worker(game_id.clone(), db_path);
        
        workers.insert(game_id.clone(), handle.clone());
        tracing::debug!("Created new worker for game {}", game_id);
        
        handle
    }

    pub async fn remove(&self, game_id: &str) {
        let mut workers = self.workers.write().await;
        workers.remove(game_id);
        tracing::debug!("Removed worker for game {}", game_id);
    }

    pub async fn list_games(&self) -> Vec<String> {
        let workers = self.workers.read().await;
        workers.keys().cloned().collect()
    }

    pub async fn count(&self) -> usize {
        let workers = self.workers.read().await;
        workers.len()
    }
}

impl Clone for WorkerRegistry {
    fn clone(&self) -> Self {
        Self {
            workers: RwLock::new(HashMap::new()),
            games_root: self.games_root.clone(),
        }
    }
}
