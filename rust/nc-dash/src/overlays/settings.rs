//! S overlay: settings — theme picker and mouse notes.

use nc_ui::PlayfieldBuffer;
use nc_ui::modal::Rect;
use nc_ui::table::TableFooter;

use crate::app::state::{ActiveOverlay, DashApp};
use crate::layout;
use crate::layout::MapWidgetFrame;
use crate::overlays::frame::{
    OverlaySizePolicy, assert_overlay_body_write_fits,
    draw_overlay_frame_for_body_in_map_with_origin, overlay_popup_rect_for_body_in_map,
    write_clipped,
};
use crate::theme;

const SETTINGS_LINES: &[(&str, &str)] = &[
    ("Theme", "Select color theme (T to open theme picker)"),
    (
        "Mouse",
        "Always on: drag overlays by title bar, click map to jump",
    ),
];

pub fn draw(buf: &mut PlayfieldBuffer, _app: &DashApp, map_frame: MapWidgetFrame) {
    draw_with_origin(buf, _app, map_frame);
}

fn draw_with_origin(buf: &mut PlayfieldBuffer, app: &DashApp, map_frame: MapWidgetFrame) {
    let label_width = layout::label_value_width(SETTINGS_LINES.iter().map(|(key, _)| *key));
    let body_width = SETTINGS_LINES
        .iter()
        .map(|(key, desc)| {
            layout::format_label_value(key, label_width, desc)
                .chars()
                .count()
        })
        .max()
        .unwrap_or(0);
    let frame = draw_overlay_frame_for_body_in_map_with_origin(
        buf,
        map_frame,
        "SETTINGS",
        body_width,
        SETTINGS_LINES.len(),
        TableFooter::Dismiss,
        app.overlay_position_for(ActiveOverlay::Settings),
    );
    assert_overlay_body_write_fits(frame, "SETTINGS", body_width, SETTINGS_LINES.len());
    for (idx, (key, desc)) in SETTINGS_LINES.iter().enumerate() {
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
    let label_width = layout::label_value_width(SETTINGS_LINES.iter().map(|(key, _)| *key));
    let body_width = SETTINGS_LINES
        .iter()
        .map(|(key, desc)| {
            layout::format_label_value(key, label_width, desc)
                .chars()
                .count()
        })
        .max()
        .unwrap_or(0);
    overlay_popup_rect_for_body_in_map(
        map_frame,
        "SETTINGS",
        body_width,
        SETTINGS_LINES.len(),
        OverlaySizePolicy::default(),
        TableFooter::Dismiss,
        app.overlay_position_for(ActiveOverlay::Settings),
    )
}
