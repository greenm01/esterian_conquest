use std::fs;
use std::path::Path;

use ec_compat::extract_player_intel_from_compat_database;
use ec_data::{
    CampaignStore, CoreGameData, DEFAULT_CAMPAIGN_DB_NAME, DatabaseDat,
    build_player_starmap_projection_from_snapshots,
};

pub fn export_player_starmap(
    dir: &Path,
    player_record_index_1_based: usize,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let store_path = dir.join(DEFAULT_CAMPAIGN_DB_NAME);
    let projection = if store_path.exists() {
        let store = CampaignStore::open(store_path)?;
        if let Some(runtime_state) = store.load_latest_runtime_state()? {
            let snapshots = store
                .latest_planet_intel_for_viewer(player_record_index_1_based as u8)?
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect();
            build_player_starmap_projection_from_snapshots(
                &runtime_state.game_data,
                &snapshots,
                player_record_index_1_based as u8,
            )
        } else {
            let game_data = CoreGameData::load(dir)?;
            let database = DatabaseDat::parse(&fs::read(dir.join("DATABASE.DAT"))?)?;
            let snapshots = extract_player_intel_from_compat_database(
                &game_data,
                &database,
                game_data.conquest.game_year(),
            );
            build_player_starmap_projection_from_snapshots(
                &game_data,
                &snapshots[player_record_index_1_based - 1],
                player_record_index_1_based as u8,
            )
        }
    } else {
        let game_data = CoreGameData::load(dir)?;
        let database = DatabaseDat::parse(&fs::read(dir.join("DATABASE.DAT"))?)?;
        let snapshots = extract_player_intel_from_compat_database(
            &game_data,
            &database,
            game_data.conquest.game_year(),
        );
        build_player_starmap_projection_from_snapshots(
            &game_data,
            &snapshots[player_record_index_1_based - 1],
            player_record_index_1_based as u8,
        )
    };
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
