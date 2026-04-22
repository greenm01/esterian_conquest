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
    pub follow_mouse_on_map: bool,
    pub theme_key: String,
    pub window_width: Option<u16>,
    pub window_height: Option<u16>,
    pub window_maximized: bool,
}

impl Default for DashClientSettings {
    fn default() -> Self {
        Self {
            follow_mouse_on_map: true,
            theme_key: "tokyo-night".to_string(),
            window_width: None,
            window_height: None,
            window_maximized: false,
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
            "follow_mouse_on_map" => settings.follow_mouse_on_map = value.trim() == "true",
            "theme_key" => settings.theme_key = value.trim().to_string(),
            "window_width" => settings.window_width = value.trim().parse::<u16>().ok(),
            "window_height" => settings.window_height = value.trim().parse::<u16>().ok(),
            "window_maximized" => settings.window_maximized = value.trim() == "true",
            _ => {}
        }
    }
    Ok(settings)
}

pub fn save_client_settings_to(
    settings: &DashClientSettings,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp)?;
    writeln!(file, "follow_mouse_on_map={}", settings.follow_mouse_on_map)?;
    writeln!(file, "theme_key={}", settings.theme_key)?;
    if let Some(width) = settings.window_width {
        writeln!(file, "window_width={width}")?;
    }
    if let Some(height) = settings.window_height {
        writeln!(file, "window_height={height}")?;
    }
    writeln!(file, "window_maximized={}", settings.window_maximized)?;
    fs::rename(tmp, path)?;
    Ok(())
}
