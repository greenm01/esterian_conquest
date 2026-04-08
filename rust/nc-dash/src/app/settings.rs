use crate::client_settings;

use super::state::DashApp;

impl DashApp {
    pub(crate) fn toggle_follow_mouse_on_map_setting(&mut self) {
        self.client_settings.follow_mouse_on_map = !self.client_settings.follow_mouse_on_map;
        self.persist_client_settings();
    }

    pub(crate) fn toggle_dense_empty_sector_dots_setting(&mut self) {
        self.client_settings.dense_empty_sector_dots =
            !self.client_settings.dense_empty_sector_dots;
        self.persist_client_settings();
    }

    pub(crate) fn clear_settings_status(&mut self) {
        self.settings_overlay.status_message = None;
    }

    fn persist_client_settings(&mut self) {
        let Some(path) = self.client_settings_path.as_deref() else {
            self.settings_overlay.status_message = None;
            return;
        };

        match client_settings::save_client_settings_to(&self.client_settings, path) {
            Ok(()) => {
                self.settings_overlay.status_message = Some(String::from("Saved local settings"));
            }
            Err(err) => {
                self.settings_overlay.status_message = Some(format!("Save failed: {err}"));
            }
        }
    }
}
