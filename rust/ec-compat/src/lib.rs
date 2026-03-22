use std::fs;
use std::path::{Path, PathBuf};

use ec_data::{
    CampaignStore, CampaignStoreError, CoreGameData, DatabaseDat, MaintenanceEvents,
    QueuedPlayerMail, derive_campaign_seed_from_runtime, load_mail_queue,
};

pub use ec_data::{
    build_database_dat, decode_report_block_rows, encode_report_block_rows,
    extract_player_intel_from_compat_database, merge_player_intel_from_compat,
    rebuild_results_bytes,
};

pub fn import_directory_snapshot(
    store: &CampaignStore,
    dir: &Path,
) -> Result<i64, CampaignStoreError> {
    import_directory_snapshot_with_seed(store, dir, None)
}

pub fn import_directory_snapshot_with_seed(
    store: &CampaignStore,
    dir: &Path,
    campaign_seed: Option<u64>,
) -> Result<i64, CampaignStoreError> {
    let game_data = CoreGameData::load(dir)?;
    let database = load_database_snapshot_or_default(dir, &game_data)?;
    let results_bytes = read_optional_path(dir.join("RESULTS.DAT"))?;
    let queued_mail = load_mail_queue_file(dir)?;
    let report_block_rows = decode_report_block_rows(&results_bytes);
    let derived_seed = campaign_seed.or_else(|| {
        Some(derive_campaign_seed_from_runtime(
            &game_data,
            &report_block_rows,
            &queued_mail,
        ))
    });
    let planet_intel_by_viewer = extract_player_intel_from_compat_database(
        &game_data,
        &database,
        game_data.conquest.game_year(),
    );
    store.save_runtime_state_structured_with_intel_and_seed(
        &game_data,
        &report_block_rows,
        &queued_mail,
        &planet_intel_by_viewer,
        derived_seed,
    )
}

pub fn export_latest_snapshot_to_dir(
    store: &CampaignStore,
    dir: &Path,
) -> Result<u16, CampaignStoreError> {
    let Some((snapshot_id, year)) = store.latest_snapshot_metadata()? else {
        return Ok(0);
    };
    export_snapshot_to_dir(store, snapshot_id, dir)?;
    Ok(year)
}

pub fn export_snapshot_to_dir(
    store: &CampaignStore,
    snapshot_id: i64,
    dir: &Path,
) -> Result<(), CampaignStoreError> {
    fs::create_dir_all(dir).map_err(|source| CampaignStoreError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    let game_data = store.load_snapshot_game_data(snapshot_id)?;
    game_data.save(dir)?;
    let planet_intel_by_viewer = store
        .load_snapshot_planet_intel_by_viewer(snapshot_id, game_data.conquest.player_count())?;
    let template_database = load_optional_database_template(dir, &game_data)?;
    let database = build_database_dat(
        &game_data,
        &game_data.planets,
        &planet_intel_by_viewer,
        &MaintenanceEvents::default(),
        template_database.as_ref(),
    );
    write_path(dir.join("DATABASE.DAT"), &database.to_bytes())?;

    let report_rows = store.load_snapshot_report_block_rows(snapshot_id, true)?;
    let active: Vec<_> = report_rows
        .iter()
        .filter(|row| !row.recipient_deleted)
        .cloned()
        .collect();
    let results_bytes = rebuild_results_bytes(&active).unwrap_or_default();
    write_path(dir.join("RESULTS.DAT"), &results_bytes)?;
    write_path(dir.join("MESSAGES.DAT"), &[])?;
    Ok(())
}

fn read_optional_path(path: PathBuf) -> Result<Vec<u8>, CampaignStoreError> {
    match fs::read(&path) {
        Ok(bytes) => Ok(bytes),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(source) => Err(CampaignStoreError::Io { path, source }),
    }
}

fn write_path(path: PathBuf, bytes: &[u8]) -> Result<(), CampaignStoreError> {
    fs::write(&path, bytes).map_err(|source| CampaignStoreError::Io { path, source })
}

fn load_mail_queue_file(dir: &Path) -> Result<Vec<QueuedPlayerMail>, CampaignStoreError> {
    let path = dir.join("RUSTMAIL.QUE");
    if !path.exists() {
        return Ok(Vec::new());
    }
    load_mail_queue(dir).map_err(|err| CampaignStoreError::Io {
        path,
        source: std::io::Error::other(err.to_string()),
    })
}

fn load_database_snapshot_or_default(
    dir: &Path,
    game_data: &CoreGameData,
) -> Result<DatabaseDat, CampaignStoreError> {
    let path = dir.join("DATABASE.DAT");
    match fs::read(&path) {
        Ok(bytes) => Ok(DatabaseDat::parse(&bytes)?),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            Ok(default_unknown_database_template(game_data))
        }
        Err(source) => Err(CampaignStoreError::Io { path, source }),
    }
}

fn load_optional_database_template(
    dir: &Path,
    game_data: &CoreGameData,
) -> Result<Option<DatabaseDat>, CampaignStoreError> {
    let path = dir.join("DATABASE.DAT");
    match fs::read(&path) {
        Ok(bytes) => DatabaseDat::parse(&bytes).map(Some).map_err(Into::into),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            Ok(Some(default_unknown_database_template(game_data)))
        }
        Err(source) => Err(CampaignStoreError::Io { path, source }),
    }
}

fn default_unknown_database_template(game_data: &CoreGameData) -> DatabaseDat {
    let names = vec!["UNKNOWN".to_string(); game_data.planets.records.len()];
    DatabaseDat::generate_from_planets_and_year(
        &names,
        game_data.conquest.game_year(),
        game_data.conquest.player_count() as usize,
        None,
    )
}
