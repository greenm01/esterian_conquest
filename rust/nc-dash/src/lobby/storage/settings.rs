use std::fs;
use std::io::Write;
use std::path::Path;

use kdl::{KdlDocument, KdlValue};

use super::paths;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LobbySettingsRecord {
    pub lock_timeout_minutes: u16,
    pub follow_mouse_on_map: bool,
    pub dense_empty_sector_dots: bool,
    pub theme_key: String,
}

pub const DEFAULT_LOCK_TIMEOUT_MINUTES: u16 = 10;
pub const LOCK_TIMEOUT_OPTIONS: [u16; 5] = [0, 5, 10, 15, 30];

impl Default for LobbySettingsRecord {
    fn default() -> Self {
        Self {
            lock_timeout_minutes: DEFAULT_LOCK_TIMEOUT_MINUTES,
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
        lock_timeout_minutes: settings
            .get("lock-timeout-minutes")
            .and_then(setting_u16_value)
            .map(normalize_lock_timeout_minutes)
            .unwrap_or(defaults.lock_timeout_minutes),
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
        "settings lock-timeout-minutes={} follow-mouse=#{} dense-empty-sector-dots=#{} theme-key=\"{}\"\n",
        settings.lock_timeout_minutes,
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

fn setting_u16_value(value: &KdlValue) -> Option<u16> {
    value
        .as_integer()
        .and_then(|minutes| u16::try_from(minutes).ok())
        .or_else(|| {
            value.as_string().and_then(|minutes| {
                minutes
                    .parse::<u16>()
                    .ok()
                    .map(normalize_lock_timeout_minutes)
            })
        })
}

fn normalize_theme_key(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace('_', "-")
}

pub fn normalize_lock_timeout_minutes(value: u16) -> u16 {
    if LOCK_TIMEOUT_OPTIONS.contains(&value) {
        value
    } else {
        DEFAULT_LOCK_TIMEOUT_MINUTES
    }
}

pub fn lock_timeout_label(value: u16) -> String {
    if value == 0 {
        "Off".to_string()
    } else {
        format!("{value} min")
    }
}

fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
