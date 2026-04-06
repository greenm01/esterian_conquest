//! Three-column frame, border drawing, resize handling.

pub mod geometry;
pub mod widgets;

use nc_ui::{PlayfieldBuffer, ScreenGeometry};

use crate::app::state::DashApp;
use crate::theme;
pub use widgets::{
    DashboardWidgetFrames, MapWidgetFrame, PanelWidgetFrame, dashboard_widget_frames,
    frame_offset_for, write_clipped, write_panel_body_line, write_panel_title, write_strict_span,
};

/// Compute the (col, row) offset to center the frame in the canvas.
pub fn frame_offset(app: &DashApp) -> (usize, usize) {
    frame_offset_for(app.geometry, app.frame)
}

/// Create a new PlayfieldBuffer at the full canvas size, filled with theme bg.
pub fn new_dashboard_buffer(geometry: ScreenGeometry) -> PlayfieldBuffer {
    PlayfieldBuffer::new(geometry.width(), geometry.height(), theme::body_style())
}

/// Draw the complete outer border + column dividers + header/footer dividers.
pub fn draw_frame(
    buf: &mut PlayfieldBuffer,
    frame: ScreenGeometry,
    widgets: &DashboardWidgetFrames,
) {
    let (ox, _) = frame_offset_for(ScreenGeometry::new(buf.width(), buf.height()), frame);
    let fw = frame.width();
    let bs = theme::border_style();

    let left_div = widgets.left_divider_col;
    let right_div = widgets.right_divider_col;
    let top = widgets.outer_top;
    let bottom = widgets.outer_bottom;
    let header_div = widgets.header_divider_row;
    let footer_div = widgets.footer_divider_row;

    // Top and bottom outer edges.
    for c in ox..ox + fw {
        buf.set_cell(top, c, '─', bs);
        buf.set_cell(bottom, c, '─', bs);
    }
    // Left and right outer edges.
    for r in top..=bottom {
        buf.set_cell(r, ox, '│', bs);
        buf.set_cell(r, ox + fw.saturating_sub(1), '│', bs);
    }
    // Outer corners.
    buf.set_cell(top, ox, '┌', bs);
    buf.set_cell(top, ox + fw.saturating_sub(1), '┐', bs);
    buf.set_cell(bottom, ox, '└', bs);
    buf.set_cell(bottom, ox + fw.saturating_sub(1), '┘', bs);

    // Header divider.
    for c in (ox + 1)..(ox + fw.saturating_sub(1)) {
        buf.set_cell(header_div, c, '─', bs);
    }
    buf.set_cell(header_div, ox, '├', bs);
    buf.set_cell(header_div, ox + fw.saturating_sub(1), '┤', bs);

    // Footer divider.
    for c in (ox + 1)..(ox + fw.saturating_sub(1)) {
        buf.set_cell(footer_div, c, '─', bs);
    }
    buf.set_cell(footer_div, ox, '├', bs);
    buf.set_cell(footer_div, ox + fw.saturating_sub(1), '┤', bs);

    // Column dividers (between header and footer dividers).
    for r in (header_div + 1)..footer_div {
        buf.set_cell(r, left_div, '│', bs);
        buf.set_cell(r, right_div, '│', bs);
    }
    // Tee joints.
    buf.set_cell(header_div, left_div, '┬', bs);
    buf.set_cell(header_div, right_div, '┬', bs);
    buf.set_cell(footer_div, left_div, '┴', bs);
    buf.set_cell(footer_div, right_div, '┴', bs);

    draw_panel_separator(
        buf,
        ox,
        left_div,
        widgets.left_planets.outer.row.saturating_sub(1),
        '├',
        '┤',
        bs,
    );
    draw_panel_separator(
        buf,
        ox,
        left_div,
        widgets.left_fleets.outer.row.saturating_sub(1),
        '├',
        '┤',
        bs,
    );
    draw_panel_separator(
        buf,
        right_div,
        ox + fw.saturating_sub(1),
        widgets.right_diplomacy.outer.row.saturating_sub(1),
        '├',
        '┤',
        bs,
    );
    draw_panel_separator(
        buf,
        right_div,
        ox + fw.saturating_sub(1),
        widgets.right_reports.outer.row.saturating_sub(1),
        '├',
        '┤',
        bs,
    );
}

/// Draw header text on the interior header bar row.
pub fn draw_header(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let (ox, _) = frame_offset(app);
    let fw = app.frame.width();
    let widgets = dashboard_widget_frames(app.geometry, app.frame);
    let row = widgets.header_bar_row;
    let inner_right = ox + fw.saturating_sub(2);

    // Branding (after left corner).
    write_strict_span(
        buf,
        row,
        ox + 2,
        inner_right.saturating_sub(ox + 2) + 1,
        " NOSTRIAN CONQUEST ",
        theme::title_style(),
        "dashboard header branding",
    );

    // Empire name centered.
    let empire_name = app
        .game_data
        .player
        .records
        .get(app.player_record_index_1_based.saturating_sub(1))
        .map(|p| {
            String::from_utf8_lossy(p.empire_name_bytes())
                .trim_end_matches('\0')
                .to_string()
        })
        .unwrap_or_default();
    if !empire_name.is_empty() {
        let center = ox + fw / 2;
        let name_start = center.saturating_sub(empire_name.len() / 2);
        write_strict_span(
            buf,
            row,
            name_start,
            inner_right.saturating_sub(name_start) + 1,
            &empire_name,
            theme::title_style(),
            "dashboard header empire name",
        );
    }

    // Right-justified stats.
    let player = app
        .game_data
        .player
        .records
        .get(app.player_record_index_1_based.saturating_sub(1));
    let year = app.game_data.conquest.game_year();
    let planet_count = player.map(|p| p.planet_count_raw()).unwrap_or(0);
    let fleet_count = app
        .game_data
        .fleets
        .records
        .iter()
        .filter(|f| {
            f.owner_empire_raw() == app.player_record_index_1_based as u8 && f.has_any_force()
        })
        .count();
    let tax = player.map(|p| p.tax_rate()).unwrap_or(0);
    let ap = if app.autopilot_on { "ON" } else { "OFF" };
    let right_str = format!(
        "Y{year}  Planets:{planet_count}  Fleets:{fleet_count}  Autopilot:{ap}  Tax:{tax}% "
    );
    let right_col = (ox + fw).saturating_sub(right_str.len() + 1);
    let style = if app.autopilot_on {
        theme::alert_style()
    } else {
        theme::header_style()
    };
    write_strict_span(
        buf,
        row,
        right_col,
        inner_right.saturating_sub(right_col) + 1,
        &right_str,
        style,
        "dashboard header stats",
    );
}

/// Draw footer text on the interior footer bar row.
pub fn draw_footer(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let (ox, _) = frame_offset(app);
    let widgets = dashboard_widget_frames(app.geometry, app.frame);
    let row = widgets.footer_bar_row;
    let footer_text =
        " P:Planets F:Fleets I:Intel R:Inbox D:Diplomacy A:Autopilot X:Tax S:Settings Q:Quit ? ";
    write_strict_span(
        buf,
        row,
        ox + 2,
        app.frame.width().saturating_sub(3),
        footer_text,
        theme::footer_style(),
        "dashboard footer",
    );
}

fn draw_panel_separator(
    buf: &mut PlayfieldBuffer,
    left: usize,
    right: usize,
    row: usize,
    left_connector: char,
    right_connector: char,
    style: nc_ui::CellStyle,
) {
    if left > right {
        return;
    }
    for col in left..=right {
        let glyph = if col == left {
            left_connector
        } else if col == right {
            right_connector
        } else {
            '─'
        };
        buf.set_cell(row, col, glyph, style);
    }
}

#[cfg(test)]
fn line_char(buffer: &PlayfieldBuffer, row: usize, col: usize) -> Option<char> {
    buffer.row(row).get(col).map(|cell| cell.ch)
}

#[cfg(test)]
fn plain_separator(
    buf: &mut PlayfieldBuffer,
    left: usize,
    right: usize,
    row: usize,
    style: nc_ui::CellStyle,
) {
    for col in left..=right {
        buf.set_cell(row, col, '─', style);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::geometry::dashboard_geometry;

    #[test]
    fn write_clipped_respects_panel_width() {
        let mut buffer = PlayfieldBuffer::new(32, 4, theme::body_style());
        write_clipped(
            &mut buffer,
            1,
            2,
            8,
            "this text is wider than eight",
            theme::value_style(),
        );
        assert_eq!(buffer.plain_line(1), "  this tex");
    }

    #[test]
    fn panel_separator_stays_within_bounds() {
        let mut buffer = PlayfieldBuffer::new(20, 6, theme::body_style());
        plain_separator(&mut buffer, 2, 10, 3, theme::border_style());
        assert_eq!(buffer.plain_line(3), "  ─────────");
    }

    #[test]
    fn dashboard_frame_separators_follow_widget_boundaries() {
        let canvas = ScreenGeometry::new(160, 40);
        let frame = dashboard_geometry(18);
        let widgets = dashboard_widget_frames(canvas, frame);
        let mut buffer = PlayfieldBuffer::new(canvas.width(), canvas.height(), theme::body_style());

        draw_frame(&mut buffer, frame, &widgets);

        let left_sep_line: String = buffer
            .row(widgets.left_planets.outer.row.saturating_sub(1))
            .iter()
            .map(|cell| cell.ch)
            .collect();
        let right_sep_line: String = buffer
            .row(widgets.right_diplomacy.outer.row.saturating_sub(1))
            .iter()
            .map(|cell| cell.ch)
            .collect();

        assert_eq!(
            left_sep_line.chars().nth(widgets.left_economy.outer.col),
            Some('─')
        );
        assert_eq!(
            right_sep_line.chars().nth(widgets.right_galaxy.outer.col),
            Some('─')
        );
        assert_eq!(
            line_char(
                &buffer,
                widgets.left_planets.outer.row.saturating_sub(1),
                widgets.outer_top.saturating_sub(widgets.outer_top)
                    + frame_offset_for(canvas, frame).0,
            ),
            Some('├')
        );
    }

    #[test]
    fn dashboard_header_and_footer_rows_are_inside_the_outer_border() {
        let canvas = ScreenGeometry::new(160, 40);
        let frame = dashboard_geometry(18);
        let widgets = dashboard_widget_frames(canvas, frame);
        assert_eq!(widgets.header_bar_row, widgets.outer_top + 1);
        assert_eq!(widgets.header_divider_row, widgets.outer_top + 2);
        assert_eq!(widgets.footer_bar_row, widgets.outer_bottom - 1);
        assert_eq!(widgets.footer_divider_row, widgets.outer_bottom - 2);
    }

    #[test]
    fn side_panel_separators_draw_t_connectors_into_shell_and_dividers() {
        let canvas = ScreenGeometry::new(160, 40);
        let frame = dashboard_geometry(18);
        let widgets = dashboard_widget_frames(canvas, frame);
        let mut buffer = PlayfieldBuffer::new(canvas.width(), canvas.height(), theme::body_style());

        draw_frame(&mut buffer, frame, &widgets);

        let left_row = widgets.left_planets.outer.row.saturating_sub(1);
        let right_row = widgets.right_diplomacy.outer.row.saturating_sub(1);
        let left_border_col = frame_offset_for(canvas, frame).0;
        let right_border_col = left_border_col + frame.width().saturating_sub(1);

        assert_eq!(line_char(&buffer, left_row, left_border_col), Some('├'));
        assert_eq!(
            line_char(&buffer, left_row, widgets.left_divider_col),
            Some('┤')
        );
        assert_eq!(
            line_char(&buffer, right_row, widgets.right_divider_col),
            Some('├')
        );
        assert_eq!(line_char(&buffer, right_row, right_border_col), Some('┤'));
    }
}
