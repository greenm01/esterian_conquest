//! Right panel: world counts by category.

use std::collections::BTreeMap;

use crate::app::state::DashApp;
use crate::layout::{self, PanelWidgetFrame};
use crate::panels::starmap::{StarmapMarkerKind, marker_kind_for_world};
use crate::theme;
use nc_data::build_player_starmap_projection_from_snapshots;
use nc_ui::PlayfieldBuffer;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: PanelWidgetFrame) {
    layout::write_panel_title(buf, frame, "KNOWN GALAXY", theme::section_title_style());

    let viewer_empire_id = app.player_record_index_1_based as u8;
    let snapshot_map = app
        .planet_intel_snapshots
        .iter()
        .cloned()
        .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
        .collect::<BTreeMap<_, _>>();
    let projection = build_player_starmap_projection_from_snapshots(
        &app.game_data,
        &snapshot_map,
        viewer_empire_id,
    );
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

    layout::write_panel_body_line(
        buf,
        frame,
        0,
        &format!(" Owned   O{:4}", owned),
        theme::friendly_style(),
    );
    layout::write_panel_body_line(
        buf,
        frame,
        1,
        &format!(" Unowned #{:4}", unowned),
        theme::dim_style(),
    );
    layout::write_panel_body_line(
        buf,
        frame,
        2,
        &format!(" Neutral #{:4}", neutral),
        theme::label_style(),
    );
    layout::write_panel_body_line(
        buf,
        frame,
        3,
        &format!(" Enemy   #{:4}", enemy),
        theme::enemy_style(),
    );
    layout::write_panel_body_line(
        buf,
        frame,
        4,
        &format!(" ICD     ◊{:4}", icd),
        theme::icd_style(),
    );
    layout::write_panel_body_line(
        buf,
        frame,
        5,
        &format!(" Partial *{:4}", partial),
        theme::value_style(),
    );
    layout::write_panel_body_line(
        buf,
        frame,
        6,
        &format!(" Unknown ?{:4}", unknown),
        theme::dim_style(),
    );
}
