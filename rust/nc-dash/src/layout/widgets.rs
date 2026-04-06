//! Explicit widget frames for the dashboard side panels and center map.

use nc_ui::{CellStyle, PlayfieldBuffer, ScreenGeometry};

use crate::layout::geometry::{CELL_WIDTH, ROW_LABEL_COLS, SIDE_PANEL_WIDTH};

/// Left content column width.
pub const LEFT_WIDTH: usize = SIDE_PANEL_WIDTH;
/// Right content column width.
pub const RIGHT_WIDTH: usize = SIDE_PANEL_WIDTH;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WidgetRect {
    pub col: usize,
    pub row: usize,
    pub width: usize,
    pub height: usize,
}

impl WidgetRect {
    pub const fn last_col(self) -> usize {
        self.col + self.width.saturating_sub(1)
    }

    pub const fn last_row(self) -> usize {
        self.row + self.height.saturating_sub(1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanelWidgetFrame {
    pub outer: WidgetRect,
    pub title_row: usize,
    pub body: WidgetRect,
}

impl PanelWidgetFrame {
    pub const fn title_col(self) -> usize {
        self.outer.col + 1
    }

    pub const fn title_width(self) -> usize {
        self.outer.width.saturating_sub(1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapWidgetFrame {
    pub outer: WidgetRect,
    pub axis_row: usize,
    pub grid: WidgetRect,
    pub status_row: usize,
    pub row_label_cols: usize,
    pub cell_width: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DashboardWidgetFrames {
    pub outer_top: usize,
    pub outer_bottom: usize,
    pub header_bar_row: usize,
    pub header_divider_row: usize,
    pub footer_divider_row: usize,
    pub footer_bar_row: usize,
    pub left_divider_col: usize,
    pub right_divider_col: usize,
    pub left_economy: PanelWidgetFrame,
    pub left_planets: PanelWidgetFrame,
    pub left_fleets: PanelWidgetFrame,
    pub center_map: MapWidgetFrame,
    pub right_galaxy: PanelWidgetFrame,
    pub right_diplomacy: PanelWidgetFrame,
    pub right_reports: PanelWidgetFrame,
}

pub const fn frame_offset_for(canvas: ScreenGeometry, frame: ScreenGeometry) -> (usize, usize) {
    let ox = canvas.width().saturating_sub(frame.width()) / 2;
    let oy = canvas.height().saturating_sub(frame.height()) / 2;
    (ox, oy)
}

pub fn dashboard_widget_frames(
    canvas: ScreenGeometry,
    frame: ScreenGeometry,
) -> DashboardWidgetFrames {
    let (ox, oy) = frame_offset_for(canvas, frame);
    let fw = frame.width();
    let fh = frame.height();

    let outer_top = oy;
    let outer_bottom = oy + fh.saturating_sub(1);
    let header_bar_row = outer_top + 1;
    let header_divider_row = outer_top + 2;
    let footer_divider_row = outer_bottom.saturating_sub(2);
    let footer_bar_row = outer_bottom.saturating_sub(1);

    let left_divider_col = ox + 1 + LEFT_WIDTH;
    let right_divider_col = ox + fw.saturating_sub(1 + RIGHT_WIDTH);

    let content_top = header_divider_row + 1;
    let content_bottom = footer_divider_row.saturating_sub(1);
    let map_size = fh.saturating_sub(8);

    let left_col = ox + 1;
    let right_col = right_divider_col + 1;
    let center_col = left_divider_col + 1;
    let center_width = right_divider_col.saturating_sub(center_col);

    let left_sep_1 = content_top + 5;
    let right_sep_1 = content_top + 6;
    let lower_title_row = (oy + fh / 2 + 2).min(footer_divider_row.saturating_sub(5));
    let lower_sep = lower_title_row.saturating_sub(1);

    let center_outer = WidgetRect {
        col: center_col,
        row: content_top,
        width: center_width,
        height: content_bottom.saturating_sub(content_top) + 1,
    };
    let center_map = MapWidgetFrame {
        outer: center_outer,
        axis_row: content_top,
        grid: WidgetRect {
            col: center_col,
            row: content_top + 1,
            width: center_width,
            height: map_size,
        },
        status_row: content_top + 1 + map_size,
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
            LEFT_WIDTH,
            content_top,
            left_sep_1.saturating_sub(1),
        ),
        left_planets: panel_widget_frame(
            left_col,
            LEFT_WIDTH,
            left_sep_1 + 1,
            lower_sep.saturating_sub(1),
        ),
        left_fleets: panel_widget_frame(left_col, LEFT_WIDTH, lower_sep + 1, content_bottom),
        center_map,
        right_galaxy: panel_widget_frame(
            right_col,
            RIGHT_WIDTH,
            content_top,
            right_sep_1.saturating_sub(1),
        ),
        right_diplomacy: panel_widget_frame(
            right_col,
            RIGHT_WIDTH,
            right_sep_1 + 1,
            lower_sep.saturating_sub(1),
        ),
        right_reports: panel_widget_frame(right_col, RIGHT_WIDTH, lower_sep + 1, content_bottom),
    }
}

fn panel_widget_frame(col: usize, width: usize, top: usize, bottom: usize) -> PanelWidgetFrame {
    let outer = WidgetRect {
        col,
        row: top,
        width,
        height: bottom.saturating_sub(top) + 1,
    };
    PanelWidgetFrame {
        outer,
        title_row: top,
        body: WidgetRect {
            col: col + 1,
            row: top + 1,
            width: width.saturating_sub(1),
            height: bottom.saturating_sub(top),
        },
    }
}

fn assert_text_fits_span(context: &str, text: &str, width: usize) {
    let text_width = text.chars().count();
    assert!(
        text_width <= width,
        "{context} overruns its widget span: text width {text_width} exceeds allowed width {width}"
    );
}

fn assert_row_col_in_buffer(buf: &PlayfieldBuffer, row: usize, col: usize, context: &str) {
    assert!(
        row < buf.height(),
        "{context} row {row} is outside buffer height {}",
        buf.height()
    );
    assert!(
        col < buf.width(),
        "{context} col {col} is outside buffer width {}",
        buf.width()
    );
}

pub fn write_strict_span(
    buf: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    width: usize,
    text: &str,
    style: CellStyle,
    context: &str,
) {
    assert_row_col_in_buffer(buf, row, col, context);
    assert!(
        col + width <= buf.width(),
        "{context} span overruns buffer width: end {} exceeds {}",
        col + width,
        buf.width()
    );
    assert_text_fits_span(context, text, width);
    buf.write_text(row, col, text, style);
}

pub fn write_clipped(
    buf: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    width: usize,
    text: &str,
    style: CellStyle,
) {
    if width == 0 {
        return;
    }
    assert_row_col_in_buffer(buf, row, col, "clipped widget write");
    assert!(
        col + width <= buf.width(),
        "clipped widget write span overruns buffer width: end {} exceeds {}",
        col + width,
        buf.width()
    );
    let clipped: String = text.chars().take(width).collect();
    buf.write_text_clipped(row, col, &clipped, style);
}

pub fn write_panel_title(
    buf: &mut PlayfieldBuffer,
    frame: PanelWidgetFrame,
    title: &str,
    style: CellStyle,
) {
    write_strict_span(
        buf,
        frame.title_row,
        frame.title_col(),
        frame.title_width(),
        title,
        style,
        "panel title",
    );
}

pub fn write_panel_body_line(
    buf: &mut PlayfieldBuffer,
    frame: PanelWidgetFrame,
    body_row_offset: usize,
    text: &str,
    style: CellStyle,
) {
    if body_row_offset >= frame.body.height {
        return;
    }
    assert!(
        frame.body.col + frame.body.width <= buf.width(),
        "panel body write overruns buffer width"
    );
    write_clipped(
        buf,
        frame.body.row + body_row_offset,
        frame.body.col,
        frame.body.width,
        text,
        style,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::geometry::dashboard_geometry;

    #[test]
    fn dashboard_widgets_partition_columns_without_overlap() {
        let canvas = ScreenGeometry::new(160, 40);
        let frame = dashboard_geometry(18);
        let widgets = dashboard_widget_frames(canvas, frame);

        assert_eq!(
            widgets.left_economy.outer.col,
            frame_offset_for(canvas, frame).0 + 1
        );
        assert_eq!(widgets.center_map.outer.col, widgets.left_divider_col + 1,);
        assert_eq!(
            widgets.center_map.outer.last_col(),
            widgets.right_divider_col.saturating_sub(1),
        );
        assert!(widgets.left_economy.outer.last_row() < widgets.left_planets.outer.row);
        assert!(widgets.left_planets.outer.last_row() < widgets.left_fleets.outer.row);
        assert!(widgets.right_galaxy.outer.last_row() < widgets.right_diplomacy.outer.row);
        assert!(widgets.right_diplomacy.outer.last_row() < widgets.right_reports.outer.row);
    }

    #[test]
    fn center_map_frame_owns_axis_grid_and_status_rows() {
        let canvas = ScreenGeometry::new(160, 40);
        let frame = dashboard_geometry(18);
        let widgets = dashboard_widget_frames(canvas, frame);

        assert_eq!(widgets.center_map.axis_row, widgets.center_map.outer.row);
        assert_eq!(widgets.center_map.grid.row, widgets.center_map.axis_row + 1);
        assert_eq!(widgets.center_map.grid.height, 18);
        assert_eq!(
            widgets.center_map.status_row,
            widgets.center_map.grid.last_row() + 1,
        );
        assert_eq!(
            widgets.center_map.status_row,
            widgets.center_map.outer.last_row(),
        );
    }

    #[test]
    #[should_panic(expected = "panel title overruns its widget span")]
    fn panel_title_panics_when_it_overruns_widget_span() {
        let mut buffer = PlayfieldBuffer::new(
            40,
            10,
            CellStyle::new(nc_ui::GameColor::White, nc_ui::GameColor::Black, false),
        );
        let frame = PanelWidgetFrame {
            outer: WidgetRect {
                col: 1,
                row: 1,
                width: 8,
                height: 4,
            },
            title_row: 1,
            body: WidgetRect {
                col: 2,
                row: 2,
                width: 7,
                height: 3,
            },
        };

        write_panel_title(
            &mut buffer,
            frame,
            "TOO LONG PANEL TITLE",
            CellStyle::new(nc_ui::GameColor::White, nc_ui::GameColor::Black, false),
        );
    }
}
