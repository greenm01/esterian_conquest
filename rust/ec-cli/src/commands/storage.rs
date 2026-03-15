use std::path::Path;

use ec_data::{CampaignStore, DEFAULT_CAMPAIGN_DB_NAME};

pub fn import_directory_to_db(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let store = CampaignStore::open_default_in_dir(dir)?;
    let snapshot_id = store.import_directory_snapshot(dir)?;
    println!(
        "Imported {} into {} as snapshot {}.",
        dir.display(),
        store.path().display(),
        snapshot_id
    );
    Ok(())
}

pub fn export_latest_db_snapshot(
    source_dir: &Path,
    target_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let store = CampaignStore::open(source_dir.join(DEFAULT_CAMPAIGN_DB_NAME))?;
    let year = store.export_latest_snapshot_to_dir(target_dir)?;
    if year == 0 {
        println!("No snapshots found in {}.", store.path().display());
    } else {
        println!(
            "Exported year {} from {} to {}.",
            year,
            store.path().display(),
            target_dir.display()
        );
    }
    Ok(())
}
