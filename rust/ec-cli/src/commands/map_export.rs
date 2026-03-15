use std::fs;
use std::path::Path;

use ec_data::{CoreGameData, DatabaseDat, build_player_starmap_projection};

pub fn export_player_starmap(
    dir: &Path,
    player_record_index_1_based: usize,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let game_data = CoreGameData::load(dir)?;
    let database = load_database_dat(dir)?;
    let projection =
        build_player_starmap_projection(&game_data, &database, player_record_index_1_based as u8);
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

fn load_database_dat(dir: &Path) -> Result<DatabaseDat, Box<dyn std::error::Error>> {
    let bytes = fs::read(dir.join("DATABASE.DAT"))?;
    Ok(DatabaseDat::parse(&bytes)?)
}
