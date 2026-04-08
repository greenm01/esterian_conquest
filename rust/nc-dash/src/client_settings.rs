use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use kdl::{KdlDocument, KdlValue};

const APP_DIR_NAME: &str = "nostrian-conflict";
const SETTINGS_FILE_NAME: &str = "settings.kdl";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DashClientSettings {
    pub follow_mouse_on_map: bool,
    pub dense_empty_sector_dots: bool,
}

impl Default for DashClientSettings {
    fn default() -> Self {
        Self {
            follow_mouse_on_map: true,
            dense_empty_sector_dots: false,
        }
    }
}

pub fn settings_path() -> PathBuf {
    config_root().join(SETTINGS_FILE_NAME)
}

pub fn load_client_settings_from(
    path: &Path,
) -> Result<DashClientSettings, Box<dyn std::error::Error>> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(DashClientSettings::default());
        }
        Err(err) => return Err(err.into()),
    };
    parse_client_settings(&raw)
}

pub fn save_client_settings_to(
    settings: &DashClientSettings,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(render_client_settings(settings).as_bytes())?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

fn config_root() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .map(|home| home.join(".config"))
            .unwrap_or_else(|| PathBuf::from("."))
    });
    base.join(APP_DIR_NAME)
}

fn parse_client_settings(raw: &str) -> Result<DashClientSettings, Box<dyn std::error::Error>> {
    let defaults = DashClientSettings::default();
    let document: KdlDocument = match raw.parse() {
        Ok(document) => document,
        Err(_) => return Ok(defaults),
    };
    let Some(settings) = document
        .nodes()
        .iter()
        .find(|node| node.name().value() == "settings")
    else {
        return Ok(defaults);
    };

    Ok(DashClientSettings {
        follow_mouse_on_map: settings
            .get("follow-mouse")
            .and_then(setting_bool_value)
            .unwrap_or(defaults.follow_mouse_on_map),
        dense_empty_sector_dots: settings
            .get("dense-empty-sector-dots")
            .and_then(setting_bool_value)
            .unwrap_or(defaults.dense_empty_sector_dots),
    })
}

fn render_client_settings(settings: &DashClientSettings) -> String {
    format!(
        "settings follow-mouse=#{} dense-empty-sector-dots=#{}\n",
        settings.follow_mouse_on_map, settings.dense_empty_sector_dots
    )
}

fn setting_bool_value(value: &KdlValue) -> Option<bool> {
    value.as_bool().or_else(|| match value.as_string() {
        Some("true") => Some(true),
        Some("false") => Some(false),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        DashClientSettings, load_client_settings_from, render_client_settings,
        save_client_settings_to,
    };
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn missing_settings_file_uses_defaults() {
        let path = unique_temp_path("missing");

        let settings = load_client_settings_from(&path).expect("load defaults");

        assert_eq!(settings, DashClientSettings::default());
    }

    #[test]
    fn partial_settings_kdl_falls_back_to_defaults() {
        let settings = super::parse_client_settings("settings dense-empty-sector-dots=#true\n")
            .expect("parse partial settings");

        assert_eq!(
            settings,
            DashClientSettings {
                follow_mouse_on_map: true,
                dense_empty_sector_dots: true,
            }
        );
    }

    #[test]
    fn settings_round_trip_through_kdl_file() {
        let path = unique_temp_path("roundtrip");
        let settings = DashClientSettings {
            follow_mouse_on_map: false,
            dense_empty_sector_dots: true,
        };

        save_client_settings_to(&settings, &path).expect("save settings");
        let loaded = load_client_settings_from(&path).expect("reload settings");

        assert_eq!(loaded, settings);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rendered_settings_use_expected_keys() {
        let rendered = render_client_settings(&DashClientSettings::default());

        assert!(rendered.contains("follow-mouse=#true"));
        assert!(rendered.contains("dense-empty-sector-dots=#false"));
    }

    #[test]
    fn malformed_kdl_falls_back_to_defaults() {
        let settings =
            super::parse_client_settings("settings follow-mouse=").expect("malformed fallback");

        assert_eq!(settings, DashClientSettings::default());
    }

    fn unique_temp_path(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("nc-dash-{tag}-{nanos}.kdl"))
    }
}
