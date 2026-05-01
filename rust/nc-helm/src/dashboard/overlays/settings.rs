//! S overlay: settings — local map behavior toggles.

use crate::dashboard::buffer::PlayfieldBuffer;
use crate::dashboard::table::{TableFooter, with_command_line_toast};

use crate::dashboard::app::state::{ActiveOverlay, DashApp};
use crate::dashboard::layout;
use crate::dashboard::layout::MapWidgetFrame;
use crate::dashboard::layout::dashboard;
use crate::dashboard::modal::{
    Rect, WrappedTextLines, compact_content_width, measure_modal_text_lines,
};
use crate::dashboard::overlays::frame::{
    OverlaySizePolicy, assert_overlay_body_write_fits, dashboard_overlay_parent_rect,
    draw_overlay_frame_for_body_in_parent_with_policy_and_origin,
    overlay_popup_rect_for_body_in_parent, write_clipped,
};
use crate::dashboard::theme;

pub fn draw(buf: &mut PlayfieldBuffer, _app: &DashApp, map_frame: MapWidgetFrame) {
    draw_with_origin(buf, _app, map_frame);
}

fn draw_with_origin(buf: &mut PlayfieldBuffer, app: &DashApp, map_frame: MapWidgetFrame) {
    let _ = map_frame;
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let wrapped = wrapped_settings_lines(app, parent);
    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        parent,
        "SETTINGS",
        wrapped.content_width,
        wrapped.lines.len(),
        OverlaySizePolicy::default(),
        with_command_line_toast(TableFooter::Dismiss, app.active_command_line_toast()),
        app.overlay_position_for(ActiveOverlay::Settings),
    );
    assert_overlay_body_write_fits(
        frame,
        "SETTINGS",
        wrapped.content_width,
        wrapped.lines.len(),
    );
    for (idx, line) in wrapped.lines.iter().enumerate() {
        write_clipped(
            buf,
            frame.body_row + idx,
            frame.body_col,
            frame.body_width,
            line,
            theme::label_style(),
        );
    }
}

pub(crate) fn popup_rect(app: &DashApp, map_frame: MapWidgetFrame) -> Rect {
    let _ = map_frame;
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let wrapped = wrapped_settings_lines(app, parent);
    overlay_popup_rect_for_body_in_parent(
        parent,
        "SETTINGS",
        wrapped.content_width,
        wrapped.lines.len(),
        OverlaySizePolicy::default(),
        with_command_line_toast(TableFooter::Dismiss, app.active_command_line_toast()),
        app.overlay_position_for(ActiveOverlay::Settings),
    )
}

fn wrapped_settings_lines(app: &DashApp, parent: Rect) -> WrappedTextLines {
    let lines = settings_lines(app);
    let label_width = layout::label_value_width(lines.iter().map(|(key, _)| key.as_str()));
    let formatted = lines
        .iter()
        .map(|(key, desc)| layout::format_label_value(key, label_width, desc))
        .collect::<Vec<_>>();
    measure_modal_text_lines(&formatted, compact_content_width(parent))
}

fn settings_lines(app: &DashApp) -> Vec<(String, String)> {
    let lines = vec![
        (
            String::from("Mouse Follow"),
            format!(
                "{} (M toggles hover-follow crosshair)",
                on_off(app.client_settings.follow_mouse_on_map)
            ),
        ),
        (
            String::from("Starmap"),
            String::from("Classic EC boxed coordinate grid"),
        ),
        (
            String::from("Map Clicks"),
            String::from("Always move crosshair and open sector actions"),
        ),
    ];
    lines
}

fn on_off(enabled: bool) -> &'static str {
    if enabled { "ON" } else { "OFF" }
}

#[cfg(test)]
mod tests {
    use super::settings_lines;
    use crate::dashboard::app::state::DashApp;
    use crate::dashboard::geometry::ScreenGeometry;
    use nc_data::GameStateBuilder;
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

        let lines = settings_lines(&app);

        assert!(lines.iter().any(|(key, value)| {
            key == "Mouse Follow" && value.contains("OFF") && value.contains("M toggles")
        }));
        assert!(lines.iter().any(|(key, value)| {
            key == "Starmap" && value.contains("Classic EC boxed coordinate grid")
        }));
    }
}
