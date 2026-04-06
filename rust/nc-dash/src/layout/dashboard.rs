//! Measured dashboard layout sized from actual widget content.

use nc_ui::ScreenGeometry;

use crate::app::state::{DashApp, MapViewMode};
use crate::layout::geometry::{
    CELL_WIDTH, MAP_LEFT_PADDING, MAP_RIGHT_PADDING, MAP_VERTICAL_PADDING, ROW_LABEL_COLS,
    dashboard_frame_geometry, minimum_projected_map_height, minimum_projected_map_width,
    preferred_readable_map_width,
};
use crate::layout::widgets::{
    DashboardWidgetFrames, MapWidgetFrame, WidgetRect, frame_offset_for, panel_widget_frame,
};
use crate::panels::{diplomacy, economy, fleets, known_galaxy, planets, sector_detail};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DashboardLayout {
    pub frame: ScreenGeometry,
    pub widgets: DashboardWidgetFrames,
    pub left_width: usize,
    pub right_width: usize,
}

pub fn dashboard_layout(app: &DashApp) -> DashboardLayout {
    let measurements = measure_dashboard(app);
    let frame = match app.map_view_mode {
        MapViewMode::Readable => preferred_readable_frame(app.geometry, measurements),
        MapViewMode::Fill => app.geometry,
    };
    let content_height = frame.height().saturating_sub(6);
    let center_width = frame.width().saturating_sub(
        1 + measurements.left_width + 1 + 1 + measurements.right_width + 1,
    );
    let map_block_width = match app.map_view_mode {
        MapViewMode::Readable => measurements.preferred_center_width.min(center_width),
        MapViewMode::Fill => center_width,
    };
    let map_block_height = match app.map_view_mode {
        MapViewMode::Readable => measurements.minimum_map_height.min(content_height),
        MapViewMode::Fill => content_height,
    };
    let widgets = build_widget_frames(
        app.geometry,
        frame,
        content_height,
        measurements.left_width,
        measurements.right_width,
        measurements.left_heights,
        measurements.right_heights,
        map_block_width,
        map_block_height,
    );

    DashboardLayout {
        frame,
        widgets,
        left_width: measurements.left_width,
        right_width: measurements.right_width,
    }
}

pub fn required_dashboard_frame(app: &DashApp) -> ScreenGeometry {
    let measurements = measure_dashboard(app);

    dashboard_frame_geometry(
        measurements.minimum_center_width,
        measurements.left_width,
        measurements.right_width,
        measurements.minimum_content_height,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DashboardMeasurements {
    left_width: usize,
    right_width: usize,
    left_heights: [usize; 3],
    right_heights: [usize; 3],
    minimum_center_width: usize,
    preferred_center_width: usize,
    minimum_map_height: usize,
    minimum_content_height: usize,
}

fn measure_dashboard(app: &DashApp) -> DashboardMeasurements {
    let map_size = nc_data::map_size_for_player_count(app.game_data.conquest.player_count()) as usize;

    let economy_rows = economy::body_rows(app);
    let planet_rows = planets::body_rows(app);
    let fleet_rows = fleets::body_rows(app);
    let galaxy_rows = known_galaxy::body_rows(app);
    let diplomacy_rows = diplomacy::body_rows(app);

    let left_width = [
        panel_outer_width(economy::TITLE, styled_row_width(&economy_rows)),
        panel_outer_width(planets::TITLE, styled_row_width(&planet_rows)),
        panel_outer_width(fleets::TITLE, styled_row_width(&fleet_rows)),
    ]
    .into_iter()
    .max()
    .unwrap_or(1);
    let right_width = [
        panel_outer_width(known_galaxy::TITLE, styled_row_width(&galaxy_rows)),
        panel_outer_width(diplomacy::TITLE, diplomacy_body_width(&diplomacy_rows)),
        panel_outer_width(sector_detail::TITLE, sector_detail::preferred_body_width(app)),
    ]
    .into_iter()
    .max()
    .unwrap_or(1);
    let left_heights = [
        panel_outer_height(economy_rows.len()),
        panel_outer_height(planet_rows.len()),
        panel_outer_height(fleet_rows.len()),
    ];
    let right_heights = [
        panel_outer_height(galaxy_rows.len()),
        panel_outer_height(diplomacy_rows.len().max(1)),
        panel_outer_height(sector_detail::MAX_BODY_ROWS),
    ];
    let minimum_map_height = minimum_projected_map_height(map_size);
    let minimum_content_height = minimum_map_height
        .max(stack_height(left_heights))
        .max(stack_height(right_heights));

    DashboardMeasurements {
        left_width,
        right_width,
        left_heights,
        right_heights,
        minimum_center_width: minimum_projected_map_width(map_size),
        preferred_center_width: preferred_readable_map_width(map_size),
        minimum_map_height,
        minimum_content_height,
    }
}

fn preferred_readable_frame(
    canvas: ScreenGeometry,
    measurements: DashboardMeasurements,
) -> ScreenGeometry {
    let preferred = dashboard_frame_geometry(
        measurements.preferred_center_width,
        measurements.left_width,
        measurements.right_width,
        measurements.minimum_content_height,
    );
    ScreenGeometry::new(
        preferred.width().min(canvas.width()),
        preferred.height().min(canvas.height()),
    )
}

fn panel_outer_width(title: &str, body_width: usize) -> usize {
    title.chars().count().max(body_width) + 1
}

fn panel_outer_height(body_rows: usize) -> usize {
    1 + body_rows
}

fn stack_height(heights: [usize; 3]) -> usize {
    heights.into_iter().sum::<usize>() + 2
}

fn styled_row_width(rows: &[(String, nc_ui::CellStyle)]) -> usize {
    rows.iter()
        .map(|(row, _)| row.chars().count())
        .max()
        .unwrap_or(0)
}

fn diplomacy_body_width(rows: &[diplomacy::DiplomacyPanelRow]) -> usize {
    if rows.is_empty() {
        return " (none)".chars().count();
    }
    let name_width = rows
        .iter()
        .map(|row| row.name.chars().count())
        .max()
        .unwrap_or(0);
    let status_width = rows
        .iter()
        .map(|row| row.status.chars().count())
        .max()
        .unwrap_or(0);
    1 + name_width + 1 + status_width
}

fn build_widget_frames(
    canvas: ScreenGeometry,
    frame: ScreenGeometry,
    content_height: usize,
    left_width: usize,
    right_width: usize,
    left_heights: [usize; 3],
    right_heights: [usize; 3],
    map_block_width: usize,
    map_block_height: usize,
) -> DashboardWidgetFrames {
    let (ox, oy) = frame_offset_for(canvas, frame);

    let outer_top = oy;
    let outer_bottom = oy + frame.height().saturating_sub(1);
    let header_bar_row = outer_top + 1;
    let header_divider_row = outer_top + 2;
    let footer_divider_row = outer_bottom.saturating_sub(2);
    let footer_bar_row = outer_bottom.saturating_sub(1);
    let content_top = header_divider_row + 1;
    let left_col = ox + 1;
    let left_divider_col = left_col + left_width;
    let center_col = left_divider_col + 1;
    let center_width = frame
        .width()
        .saturating_sub(2 + left_width + right_width + 2);
    let right_divider_col = center_col + center_width;
    let right_col = right_divider_col + 1;

    let left_planets_top = content_top + left_heights[0] + 1;
    let left_fleets_top = left_planets_top + left_heights[1] + 1;
    let right_diplomacy_top = content_top + right_heights[0] + 1;
    let right_sector_top = right_diplomacy_top + right_heights[1] + 1;

    let center_outer = WidgetRect {
        col: center_col,
        row: content_top,
        width: center_width,
        height: content_height,
    };
    let map_block = WidgetRect {
        col: center_col + center_width.saturating_sub(map_block_width) / 2,
        row: content_top + content_height.saturating_sub(map_block_height) / 2,
        width: map_block_width,
        height: map_block_height,
    };
    let axis_row = map_block.row + MAP_VERTICAL_PADDING;
    let grid = WidgetRect {
        col: map_block.col + MAP_LEFT_PADDING,
        row: axis_row + 1,
        width: map_block
            .width
            .saturating_sub(MAP_LEFT_PADDING + MAP_RIGHT_PADDING),
        height: map_block
            .height
            .saturating_sub(1 + MAP_VERTICAL_PADDING.saturating_mul(2)),
    };
    let center_map = MapWidgetFrame {
        outer: center_outer,
        map_block,
        axis_row,
        grid,
        bottom_pad_row: map_block.last_row(),
        row_label_cols: ROW_LABEL_COLS,
        cell_width: CELL_WIDTH,
    };

    DashboardWidgetFrames {
        outer_top,
        outer_bottom,
        header_bar_row,
        header_divider_row,
        footer_divider_row,
        footer_bar_row,
        left_divider_col,
        right_divider_col,
        left_economy: panel_widget_frame(
            left_col,
            left_width,
            content_top,
            content_top + left_heights[0].saturating_sub(1),
        ),
        left_planets: panel_widget_frame(
            left_col,
            left_width,
            left_planets_top,
            left_planets_top + left_heights[1].saturating_sub(1),
        ),
        left_fleets: panel_widget_frame(
            left_col,
            left_width,
            left_fleets_top,
            left_fleets_top + left_heights[2].saturating_sub(1),
        ),
        center_map,
        right_galaxy: panel_widget_frame(
            right_col,
            right_width,
            content_top,
            content_top + right_heights[0].saturating_sub(1),
        ),
        right_diplomacy: panel_widget_frame(
            right_col,
            right_width,
            right_diplomacy_top,
            right_diplomacy_top + right_heights[1].saturating_sub(1),
        ),
        right_sector_detail: panel_widget_frame(
            right_col,
            right_width,
            right_sector_top,
            right_sector_top + right_heights[2].saturating_sub(1),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{dashboard_layout, required_dashboard_frame};
    use crate::app::state::{DashApp, MapViewMode};
    use nc_data::GameStateBuilder;
    use nc_ui::ScreenGeometry;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    #[test]
    fn side_columns_are_measured_from_widget_content() {
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
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        let layout = dashboard_layout(&app);

        assert_eq!(layout.left_width, layout.widgets.left_economy.outer.width);
        assert!(layout.right_width >= "SECTOR DETAIL".chars().count() + 1);
        assert_eq!(
            layout.widgets.center_map.outer.height,
            layout.frame.height().saturating_sub(6)
        );
        assert!(layout.frame.width() < app.geometry.width());
    }

    #[test]
    fn fill_mode_uses_full_terminal_canvas() {
        let mut app = DashApp::new(
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
        app.map_view_mode = MapViewMode::Fill;
        let layout = dashboard_layout(&app);

        assert_eq!(layout.frame, app.geometry);
        assert_eq!(layout.widgets.center_map.map_block, layout.widgets.center_map.outer);
    }

    #[test]
    fn required_frame_stays_based_on_minimum_projected_map() {
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
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        let required = required_dashboard_frame(&app);
        let readable = dashboard_layout(&app).frame;

        assert!(required.width() < readable.width());
    }
}
