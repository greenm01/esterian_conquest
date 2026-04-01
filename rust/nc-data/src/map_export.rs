use std::path::Path;

use crate::{
    CampaignStore, DEFAULT_CAMPAIGN_DB_NAME, build_player_starmap_projection_from_snapshots,
};

pub const STARMAP_TEXT_FILE_NAME: &str = "starmap.txt";
pub const STARMAP_CSV_FILE_NAME: &str = "starmap.csv";
pub const STARMAP_DETAILS_CSV_FILE_NAME: &str = "starmap-DETAILS.csv";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerMapExportFile {
    pub name: &'static str,
    pub contents: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerMapExportData {
    pub ascii_export: String,
    pub csv_export: String,
    pub csv_details_export: String,
}

impl PlayerMapExportData {
    pub fn fixed_named_files(&self) -> Vec<PlayerMapExportFile> {
        vec![
            PlayerMapExportFile {
                name: STARMAP_TEXT_FILE_NAME,
                contents: self.ascii_export.clone(),
            },
            PlayerMapExportFile {
                name: STARMAP_CSV_FILE_NAME,
                contents: self.csv_export.clone(),
            },
            PlayerMapExportFile {
                name: STARMAP_DETAILS_CSV_FILE_NAME,
                contents: self.csv_details_export.clone(),
            },
        ]
    }
}

pub fn build_player_map_export_data(
    dir: &Path,
    player_record_index_1_based: usize,
) -> Result<PlayerMapExportData, Box<dyn std::error::Error>> {
    let store_path = dir.join(DEFAULT_CAMPAIGN_DB_NAME);
    if !store_path.exists() {
        return Err(format!(
            "{} not found; import or create runtime state before using map-export",
            store_path.display()
        )
        .into());
    }

    let store = CampaignStore::open(store_path)?;
    let Some(runtime_state) = store.load_latest_runtime_state()? else {
        return Err(format!(
            "{} has no runtime snapshots; import or create runtime state before using map-export",
            dir.display()
        )
        .into());
    };

    let snapshots = store
        .latest_planet_intel_for_viewer(player_record_index_1_based as u8)?
        .into_iter()
        .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
        .collect();
    let projection = build_player_starmap_projection_from_snapshots(
        &runtime_state.game_data,
        &snapshots,
        player_record_index_1_based as u8,
    );

    Ok(PlayerMapExportData {
        ascii_export: projection.render_ascii_export(),
        csv_export: projection.render_csv_export(),
        csv_details_export: projection.render_csv_details_export(),
    })
}
