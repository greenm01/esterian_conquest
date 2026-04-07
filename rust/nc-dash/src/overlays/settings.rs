//! S overlay: settings — theme picker, mouse toggle.

use nc_ui::PlayfieldBuffer;
use nc_ui::table::TableFooter;

use crate::app::state::DashApp;
use crate::layout;
use crate::layout::MapWidgetFrame;
use crate::overlays::frame::{
    assert_overlay_body_write_fits, draw_overlay_frame_for_body_in_map, write_clipped,
};
use crate::theme;

const SETTINGS_LINES: &[(&str, &str)] = &[
    ("Theme", "Select color theme (T to open theme picker)"),
    ("Mouse", "Toggle mouse support on/off (M)"),
];

pub fn draw(buf: &mut PlayfieldBuffer, _app: &DashApp, map_frame: MapWidgetFrame) {
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
    let frame = draw_overlay_frame_for_body_in_map(
        buf,
        map_frame,
        "SETTINGS",
        body_width,
        SETTINGS_LINES.len(),
        TableFooter::Dismiss,
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
