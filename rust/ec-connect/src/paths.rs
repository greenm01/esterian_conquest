use std::path::PathBuf;

pub const APP_DIR_NAME: &str = "nc";

pub fn data_root() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .map(|h| h.join(".local").join("share"))
            .unwrap_or_else(|| PathBuf::from("."))
    });
    base.join(APP_DIR_NAME)
}

pub fn config_root() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .map(|h| h.join(".config"))
            .unwrap_or_else(|| PathBuf::from("."))
    });
    base.join(APP_DIR_NAME)
}

pub fn default_maps_root() -> PathBuf {
    let base = dirs::document_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .map(|h| h.join("Documents"))
            .unwrap_or_else(|| PathBuf::from("."))
    });
    base.join(APP_DIR_NAME).join("maps")
}
