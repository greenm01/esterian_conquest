use std::fs;
use std::io::Write;
use std::path::Path;

use kdl::{KdlDocument, KdlValue};

use super::paths;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LobbySettingsRecord {
    pub follow_mouse_on_map: bool,
    pub dense_empty_sector_dots: bool,
    pub theme_key: String,
}

impl Default for LobbySettingsRecord {
    fn default() -> Self {
        Self {
            follow_mouse_on_map: true,
            dense_empty_sector_dots: false,
            theme_key: "tokyo-night".to_string(),
        }
    }
}

pub fn settings_path() -> std::path::PathBuf {
    paths::settings_path()
}

pub fn load_settings_from(path: &Path) -> Result<LobbySettingsRecord, Box<dyn std::error::Error>> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(LobbySettingsRecord::default());
        }
        Err(err) => return Err(err.into()),
    };
    parse_settings_kdl(&raw)
}

pub fn save_settings_to(
    settings: &LobbySettingsRecord,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(render_settings_kdl(settings).as_bytes())?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn parse_settings_kdl(raw: &str) -> Result<LobbySettingsRecord, Box<dyn std::error::Error>> {
    let defaults = LobbySettingsRecord::default();
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

    Ok(LobbySettingsRecord {
        follow_mouse_on_map: settings
            .get("follow-mouse")
            .and_then(setting_bool_value)
            .unwrap_or(defaults.follow_mouse_on_map),
        dense_empty_sector_dots: settings
            .get("dense-empty-sector-dots")
            .and_then(setting_bool_value)
            .unwrap_or(defaults.dense_empty_sector_dots),
        theme_key: settings
            .get("theme-key")
            .and_then(|value| value.as_string())
            .map(normalize_theme_key)
            .unwrap_or(defaults.theme_key),
    })
}

pub fn render_settings_kdl(settings: &LobbySettingsRecord) -> String {
    format!(
        "settings follow-mouse=#{} dense-empty-sector-dots=#{} theme-key=\"{}\"\n",
        settings.follow_mouse_on_map,
        settings.dense_empty_sector_dots,
        escape(&settings.theme_key)
    )
}

fn setting_bool_value(value: &KdlValue) -> Option<bool> {
    value.as_bool().or_else(|| match value.as_string() {
        Some("true") => Some(true),
        Some("false") => Some(false),
        _ => None,
    })
}

fn normalize_theme_key(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace('_', "-")
}

fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
