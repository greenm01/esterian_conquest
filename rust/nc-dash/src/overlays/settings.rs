//! S overlay: settings — local map behavior toggles.

use nc_ui::PlayfieldBuffer;
use nc_ui::modal::Rect;
use nc_ui::table::TableFooter;

use crate::app::state::{ActiveOverlay, DashApp};
use crate::layout;
use crate::layout::MapWidgetFrame;
use crate::layout::dashboard;
use crate::overlays::frame::{
    OverlaySizePolicy, assert_overlay_body_write_fits, dashboard_overlay_parent_rect,
    draw_overlay_frame_for_body_in_parent_with_policy_and_origin,
    overlay_popup_rect_for_body_in_parent, write_clipped,
};
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, _app: &DashApp, map_frame: MapWidgetFrame) {
    draw_with_origin(buf, _app, map_frame);
}

fn draw_with_origin(buf: &mut PlayfieldBuffer, app: &DashApp, map_frame: MapWidgetFrame) {
    let _ = map_frame;
    let lines = settings_lines(app);
    let label_width = layout::label_value_width(lines.iter().map(|(key, _)| key.as_str()));
    let body_width = lines
        .iter()
        .map(|(key, desc)| {
            layout::format_label_value(key, label_width, desc)
                .chars()
                .count()
        })
        .max()
        .unwrap_or(0);
    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets),
        "SETTINGS",
        body_width,
        lines.len(),
        OverlaySizePolicy::default(),
        TableFooter::Dismiss,
        app.overlay_position_for(ActiveOverlay::Settings),
    );
    assert_overlay_body_write_fits(frame, "SETTINGS", body_width, lines.len());
    for (idx, (key, desc)) in lines.iter().enumerate() {
        write_clipped(
            buf,
            frame.body_row + idx,
            frame.body_col,
            frame.body_width,
            &layout::format_label_value(key, label_width, desc),
            theme::label_style(),
        );
    }
}

pub(crate) fn popup_rect(app: &DashApp, map_frame: MapWidgetFrame) -> Rect {
    let lines = settings_lines(app);
    let label_width = layout::label_value_width(lines.iter().map(|(key, _)| key.as_str()));
    let body_width = lines
        .iter()
        .map(|(key, desc)| {
            layout::format_label_value(key, label_width, desc)
                .chars()
                .count()
        })
        .max()
        .unwrap_or(0);
    let _ = map_frame;
    overlay_popup_rect_for_body_in_parent(
        dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets),
        "SETTINGS",
        body_width,
        lines.len(),
        OverlaySizePolicy::default(),
        TableFooter::Dismiss,
        app.overlay_position_for(ActiveOverlay::Settings),
    )
}

fn settings_lines(app: &DashApp) -> Vec<(String, String)> {
    let mut lines = vec![
        (
            String::from("Mouse Follow"),
            format!(
                "{} (M toggles hover-follow crosshair)",
                on_off(app.client_settings.follow_mouse_on_map)
            ),
        ),
        (
            String::from("Grid Dots"),
            format!(
                "{} (G toggles full dense map-grid dots)",
                on_off(app.client_settings.dense_empty_sector_dots)
            ),
        ),
        (
            String::from("Map Clicks"),
            String::from("Always move crosshair and open sector actions"),
        ),
    ];
    if let Some(status) = app.settings_overlay.status_message.as_ref() {
        lines.push((String::from("Status"), status.clone()));
    }
    lines
}

fn on_off(enabled: bool) -> &'static str {
    if enabled { "ON" } else { "OFF" }
}

#[cfg(test)]
mod tests {
    use super::settings_lines;
    use crate::app::state::DashApp;
    use nc_data::GameStateBuilder;
    use nc_ui::ScreenGeometry;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    #[test]
    fn settings_overlay_shows_live_toggle_values() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.client_settings.follow_mouse_on_map = false;
        app.client_settings.dense_empty_sector_dots = true;

        let lines = settings_lines(&app);

        assert!(lines.iter().any(|(key, value)| {
            key == "Mouse Follow" && value.contains("OFF") && value.contains("M toggles")
        }));
        assert!(lines.iter().any(|(key, value)| {
            key == "Grid Dots" && value.contains("ON") && value.contains("G toggles")
        }));
    }
}
