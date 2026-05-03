use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use nc_client::paths::data_root;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersistedWindowState {
    pub width: u16,
    pub height: u16,
    pub maximized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashClientSettings {
    pub theme_key: String,
    pub window_width: Option<u16>,
    pub window_height: Option<u16>,
    pub window_maximized: bool,
    pub last_seen_report_keys: BTreeMap<String, String>,
}

impl Default for DashClientSettings {
    fn default() -> Self {
        Self {
            theme_key: "tokyo-night".to_string(),
            window_width: None,
            window_height: None,
            window_maximized: false,
            last_seen_report_keys: BTreeMap::new(),
        }
    }
}

impl DashClientSettings {
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

pub fn settings_path() -> PathBuf {
    data_root().join("helm-dashboard-settings.txt")
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
    let mut settings = DashClientSettings::default();
    for line in raw.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "theme_key" => settings.theme_key = value.trim().to_string(),
            "window_width" => settings.window_width = value.trim().parse::<u16>().ok(),
            "window_height" => settings.window_height = value.trim().parse::<u16>().ok(),
            "window_maximized" => settings.window_maximized = value.trim() == "true",
            k if k.starts_with("last_seen_report:") => {
                let id = &k["last_seen_report:".len()..];
                settings
                    .last_seen_report_keys
                    .insert(id.to_string(), value.trim().to_string());
            }
            _ => {}
        }
    }
    Ok(settings)
}

#[allow(dead_code)]
pub fn save_client_settings_to(
    settings: &DashClientSettings,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp)?;
    writeln!(file, "theme_key={}", settings.theme_key)?;
    if let Some(width) = settings.window_width {
        writeln!(file, "window_width={width}")?;
    }
    if let Some(height) = settings.window_height {
        writeln!(file, "window_height={height}")?;
    }
    writeln!(file, "window_maximized={}", settings.window_maximized)?;
    for (id, key) in &settings.last_seen_report_keys {
        writeln!(file, "last_seen_report:{id}={key}")?;
    }
    file.sync_all()?;
    drop(file);
    fs::rename(tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{DashClientSettings, load_client_settings_from, save_client_settings_to};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_settings_path(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("nc-helm-{name}-{}-{nanos}.txt", std::process::id()))
    }

    #[test]
    fn legacy_follow_mouse_setting_is_ignored() {
        let path = temp_settings_path("legacy-follow-mouse");
        fs::write(
            &path,
            "follow_mouse_on_map=false\ntheme_key=classic\nwindow_maximized=true\n",
        )
        .expect("write settings");

        let settings = load_client_settings_from(&path).expect("load settings");

        assert_eq!(settings.theme_key, "classic");
        assert!(settings.window_maximized);
        fs::remove_file(path).expect("remove settings");
    }

    #[test]
    fn saving_settings_does_not_rewrite_removed_follow_mouse_key() {
        let path = temp_settings_path("save-without-follow-mouse");
        save_client_settings_to(&DashClientSettings::default(), &path).expect("save settings");

        let raw = fs::read_to_string(&path).expect("read settings");

        assert!(!raw.contains("follow_mouse_on_map"));
        fs::remove_file(path).expect("remove settings");
    }

    #[test]
    fn last_seen_report_keys_are_persisted() {
        let path = temp_settings_path("last-seen-reports");
        let mut settings = DashClientSettings::default();
        settings
            .last_seen_report_keys
            .insert("game1".to_string(), "turn1".to_string());
        settings
            .last_seen_report_keys
            .insert("game2".to_string(), "turn2".to_string());

        save_client_settings_to(&settings, &path).expect("save settings");
        let loaded = load_client_settings_from(&path).expect("load settings");

        assert_eq!(loaded.last_seen_report_keys.get("game1"), Some(&"turn1".to_string()));
        assert_eq!(loaded.last_seen_report_keys.get("game2"), Some(&"turn2".to_string()));
        fs::remove_file(path).expect("remove settings");
    }
}
