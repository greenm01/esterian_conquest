use std::path::Path;

use nc_data::{CampaignStore, CoreGameData, DEFAULT_CAMPAIGN_DB_NAME};

pub fn current_runtime_year(game_dir: &Path) -> Result<u16, Box<dyn std::error::Error>> {
    let runtime_db_path = game_dir.join(DEFAULT_CAMPAIGN_DB_NAME);
    if runtime_db_path.exists() {
        let store = CampaignStore::open(&runtime_db_path)?;
        if let Some(runtime) = store.load_latest_runtime_state()? {
            return Ok(runtime.game_year);
        }
    }

    let game_data = CoreGameData::load(game_dir)?;
    Ok(game_data.conquest.game_year())
}
