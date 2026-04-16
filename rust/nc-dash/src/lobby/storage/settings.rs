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
    pub window_width: Option<u16>,
    pub window_height: Option<u16>,
    pub window_maximized: bool,
}

pub const DEFAULT_LOCK_TIMEOUT_MINUTES: u16 = 10;
pub const LOCK_TIMEOUT_OPTIONS: [u16; 5] = [0, 5, 10, 15, 30];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersistedWindowState {
    pub width: u16,
    pub height: u16,
    pub maximized: bool,
}

impl Default for LobbySettingsRecord {
    fn default() -> Self {
        Self {
            lock_timeout_minutes: DEFAULT_LOCK_TIMEOUT_MINUTES,
            follow_mouse_on_map: true,
            dense_empty_sector_dots: false,
            theme_key: "tokyo-night".to_string(),
            window_width: None,
            window_height: None,
            window_maximized: false,
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
        window_width: settings
            .get("window-width")
            .and_then(setting_dimension_u16_value),
        window_height: settings
            .get("window-height")
            .and_then(setting_dimension_u16_value),
        window_maximized: settings
            .get("window-maximized")
            .and_then(setting_bool_value)
            .unwrap_or(defaults.window_maximized),
    })
}

pub fn render_settings_kdl(settings: &LobbySettingsRecord) -> String {
    format!(
        "settings lock-timeout-minutes={} follow-mouse=#{} dense-empty-sector-dots=#{} theme-key=\"{}\"{}{} window-maximized=#{}\n",
        settings.lock_timeout_minutes,
        settings.follow_mouse_on_map,
        settings.dense_empty_sector_dots,
        escape(&settings.theme_key),
        render_optional_u16_field("window-width", settings.window_width),
        render_optional_u16_field("window-height", settings.window_height),
        settings.window_maximized,
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

fn setting_dimension_u16_value(value: &KdlValue) -> Option<u16> {
    value
        .as_integer()
        .and_then(|dimension| u16::try_from(dimension).ok())
        .filter(|dimension| *dimension > 0)
        .or_else(|| {
            value
                .as_string()
                .and_then(|dimension| dimension.parse::<u16>().ok())
                .filter(|dimension| *dimension > 0)
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

fn render_optional_u16_field(name: &str, value: Option<u16>) -> String {
    match value {
        Some(value) => format!(" {name}={value}"),
        None => String::new(),
    }
}

impl LobbySettingsRecord {
    pub fn persisted_window_state(&self) -> Option<PersistedWindowState> {
        let (Some(width), Some(height)) = (self.window_width, self.window_height) else {
            return None;
        };
        Some(PersistedWindowState {
            width,
            height,
            maximized: self.window_maximized,
        })
    }

    pub fn set_persisted_window_state(&mut self, state: PersistedWindowState) {
        self.window_width = Some(state.width);
        self.window_height = Some(state.height);
        self.window_maximized = state.maximized;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LobbySettingsRecord, PersistedWindowState, parse_settings_kdl, render_settings_kdl,
    };

    #[test]
    fn legacy_settings_without_window_state_use_defaults() {
        let settings = parse_settings_kdl(
            "settings lock-timeout-minutes=15 follow-mouse=#false dense-empty-sector-dots=#true theme-key=\"Amber_Dawn\"\n",
        )
        .expect("parse settings");

        assert_eq!(
            settings,
            LobbySettingsRecord {
                lock_timeout_minutes: 15,
                follow_mouse_on_map: false,
                dense_empty_sector_dots: true,
                theme_key: "amber-dawn".to_string(),
                window_width: None,
                window_height: None,
                window_maximized: false,
            }
        );
        assert_eq!(settings.persisted_window_state(), None);
    }

    #[test]
    fn settings_round_trip_with_window_state() {
        let settings = LobbySettingsRecord {
            lock_timeout_minutes: 5,
            follow_mouse_on_map: false,
            dense_empty_sector_dots: true,
            theme_key: "phosphor".to_string(),
            window_width: Some(1440),
            window_height: Some(900),
            window_maximized: true,
        };

        let rendered = render_settings_kdl(&settings);
        let reparsed = parse_settings_kdl(&rendered).expect("parse rendered settings");

        assert_eq!(reparsed, settings);
        assert_eq!(
            reparsed.persisted_window_state(),
            Some(PersistedWindowState {
                width: 1440,
                height: 900,
                maximized: true,
            })
        );
    }

    #[test]
    fn incomplete_window_state_is_ignored() {
        let settings = parse_settings_kdl(
            "settings window-width=1280 window-maximized=#true theme-key=\"tokyo-night\"\n",
        )
        .expect("parse settings");

        assert_eq!(settings.window_width, Some(1280));
        assert_eq!(settings.window_height, None);
        assert_eq!(settings.persisted_window_state(), None);
    }
}
