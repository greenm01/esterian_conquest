//! Right panel: world counts by category.

use crate::dashboard::app::state::DashApp;
use crate::dashboard::buffer::{CellStyle, PlayfieldBuffer};
use crate::dashboard::layout::{self, PanelWidgetFrame};
use crate::dashboard::panels::starmap::{StarmapMarkerKind, cached_projection_for_app, marker_kind_for_world};
use crate::dashboard::theme;

pub(crate) const TITLE: &str = "KNOWN GALAXY";
pub(crate) const MIN_BODY_ROWS: usize = 5;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: PanelWidgetFrame) {
    layout::write_panel_title(buf, frame, TITLE, theme::section_title_style());

    for (row_idx, (text, style)) in body_rows(app).into_iter().enumerate() {
        if row_idx >= frame.body.height {
            break;
        }
        layout::write_panel_body_line(buf, frame, row_idx, &text, style);
    }
}

pub(crate) fn body_rows(app: &DashApp) -> Vec<(String, CellStyle)> {
    let viewer_empire_id = app.player_record_index_1_based as u8;
    let projection = cached_projection_for_app(app);
    let (mut owned, mut unowned, mut neutral, mut enemy, mut icd, mut partial, mut unknown) =
        (0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32);

    for world in &projection.worlds {
        match marker_kind_for_world(app, viewer_empire_id, world) {
            StarmapMarkerKind::Owned => owned += 1,
            StarmapMarkerKind::Unowned => unowned += 1,
            StarmapMarkerKind::Icd => icd += 1,
            StarmapMarkerKind::Enemy => enemy += 1,
            StarmapMarkerKind::Neutral => neutral += 1,
            StarmapMarkerKind::Partial => partial += 1,
            StarmapMarkerKind::Unknown => unknown += 1,
        }
    }

    vec![
        (format!(" Owned   O{:4}", owned), theme::friendly_style()),
        (format!(" Unowned #{:4}", unowned), theme::dim_style()),
        (format!(" Neutral #{:4}", neutral), theme::label_style()),
        (format!(" Enemy   #{:4}", enemy), theme::enemy_style()),
        (format!(" ICD     ◊{:4}", icd), theme::icd_style()),
        (format!(" Partial *{:4}", partial), theme::value_style()),
        (format!(" Unknown ?{:4}", unknown), theme::dim_style()),
    ]
}
