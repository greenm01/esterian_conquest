use std::path::PathBuf;

pub const APP_DIR_NAME: &str = "nc";

pub fn data_root() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .map(|home| home.join(".local").join("share"))
            .unwrap_or_else(|| PathBuf::from("."))
    });
    base.join(APP_DIR_NAME)
}

pub fn config_root() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .map(|home| home.join(".config"))
            .unwrap_or_else(|| PathBuf::from("."))
    });
    base.join(APP_DIR_NAME)
}

pub fn keychain_path() -> PathBuf {
    data_root().join("keychain.kdl")
}

pub fn cache_path() -> PathBuf {
    data_root().join("cache.kdl")
}

pub fn config_path() -> PathBuf {
    config_root().join("config.kdl")
}

pub fn settings_path() -> PathBuf {
    config_root().join("settings.kdl")
}
