use std::path::{Path, PathBuf};

pub use crate::lobby::storage::settings::LobbySettingsRecord as DashClientSettings;

pub fn settings_path() -> PathBuf {
    crate::lobby::storage::settings::settings_path()
}

pub fn load_client_settings_from(
    path: &Path,
) -> Result<DashClientSettings, Box<dyn std::error::Error>> {
    crate::lobby::storage::settings::load_settings_from(path)
}

pub fn save_client_settings_to(
    settings: &DashClientSettings,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    crate::lobby::storage::settings::save_settings_to(settings, path)
}
