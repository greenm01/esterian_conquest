use std::fs;
use std::path::Path;

use ec_data::build_player_map_export_data;

pub fn export_player_starmap(
    dir: &Path,
    player_record_index_1_based: usize,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let export = build_player_map_export_data(dir, player_record_index_1_based)?;
    let csv_path = output_path.with_extension("csv");
    let details_csv_path = output_path.with_file_name(
        output_path
            .file_stem()
            .map(|stem| format!("{}-DETAILS.csv", stem.to_string_lossy()))
            .unwrap_or_else(|| "map-DETAILS.csv".to_string()),
    );
    fs::write(output_path, export.ascii_export)?;
    fs::write(&csv_path, export.csv_export)?;
    fs::write(&details_csv_path, export.csv_details_export)?;
    println!(
        "Exported player {} starmap to {}, {}, and {}",
        player_record_index_1_based,
        output_path.display(),
        csv_path.display(),
        details_csv_path.display()
    );
    Ok(())
}
