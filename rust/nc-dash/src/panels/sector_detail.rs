//! Right panel: condensed detail for the currently selected map sector.

use nc_ui::PlayfieldBuffer;

use crate::app::state::DashApp;
use crate::layout::{self, PanelWidgetFrame};
use crate::planet_view::selected_planet_detail;
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: PanelWidgetFrame) {
    layout::write_panel_title(buf, frame, "SECTOR DETAIL", theme::section_title_style());

    let Some(detail) = selected_planet_detail(app) else {
        layout::write_panel_body_line(buf, frame, 0, "empty sector", theme::dim_style());
        return;
    };

    for (row_idx, line) in prioritized_widget_rows(&detail.widget_lines, frame.body.height)
        .into_iter()
        .enumerate()
    {
        layout::write_panel_body_line(buf, frame, row_idx, &line, theme::value_style());
    }
}

fn prioritized_widget_rows(rows: &[String], max_rows: usize) -> Vec<String> {
    rows.iter().take(max_rows).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::prioritized_widget_rows;
    use crate::app::state::DashApp;
    use crate::layout::{dashboard_widget_frames, geometry::dashboard_geometry};
    use crate::theme;
    use nc_data::GameStateBuilder;
    use nc_ui::{PlayfieldBuffer, ScreenGeometry};
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    #[test]
    fn widget_rows_keep_top_priority_lines_when_height_is_tight() {
        let rows = vec![
            String::from("Planet  Foo"),
            String::from("Owner   You"),
            String::from("Econ    10|9|8"),
            String::from("Def     1|2|3"),
            String::from("State   Normal"),
            String::from("Build   Nothing"),
            String::from("Docked  Nothing"),
        ];

        assert_eq!(prioritized_widget_rows(&rows, 5), rows[..5].to_vec());
    }

    #[test]
    fn empty_sector_uses_compact_copy() {
        let app = DashApp::new(
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
            ScreenGeometry::new(160, 40),
            dashboard_geometry(18),
            1,
        );
        let widgets = dashboard_widget_frames(app.geometry, app.frame);
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            theme::body_style(),
        );

        super::draw(&mut buffer, &app, widgets.right_sector_detail);

        assert!(buffer
            .plain_line(widgets.right_sector_detail.body.row)
            .contains("empty sector"));
    }
}
