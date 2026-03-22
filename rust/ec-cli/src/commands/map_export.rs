use std::fs;
use std::path::Path;

use ec_data::{
    CampaignStore, DEFAULT_CAMPAIGN_DB_NAME, build_player_starmap_projection_from_snapshots,
};

pub fn export_player_starmap(
    dir: &Path,
    player_record_index_1_based: usize,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
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
    let csv_path = output_path.with_extension("csv");
    let details_csv_path = output_path.with_file_name(
        output_path
            .file_stem()
            .map(|stem| format!("{}-DETAILS.csv", stem.to_string_lossy()))
            .unwrap_or_else(|| "map-DETAILS.csv".to_string()),
    );
    fs::write(output_path, projection.render_ascii_export())?;
    fs::write(&csv_path, projection.render_csv_export())?;
    fs::write(&details_csv_path, projection.render_csv_details_export())?;
    println!(
        "Exported player {} starmap to {}, {}, and {}",
        player_record_index_1_based,
        output_path.display(),
        csv_path.display(),
        details_csv_path.display()
    );
    Ok(())
}
