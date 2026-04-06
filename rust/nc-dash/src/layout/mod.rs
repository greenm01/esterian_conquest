//! Three-column frame, border drawing, resize handling.

pub mod geometry;

use nc_ui::{PlayfieldBuffer, ScreenGeometry};

use crate::app::state::DashApp;
use crate::layout::geometry::SIDE_PANEL_WIDTH;
use crate::theme;

/// Left content column width.
pub const LEFT_WIDTH: usize = SIDE_PANEL_WIDTH;
/// Right content column width.
pub const RIGHT_WIDTH: usize = SIDE_PANEL_WIDTH;

pub fn section_footer_row(app: &DashApp, oy: usize) -> usize {
    oy + app.frame.height().saturating_sub(3)
}

pub fn left_economy_title_row(oy: usize) -> usize {
    oy + 2
}

pub fn left_planets_title_row(oy: usize) -> usize {
    oy + 8
}

pub fn left_fleets_title_row(app: &DashApp, oy: usize) -> usize {
    (oy + app.frame.height() / 2 + 2).min(section_footer_row(app, oy).saturating_sub(4))
}

pub fn right_galaxy_title_row(oy: usize) -> usize {
    oy + 2
}

pub fn right_diplomacy_title_row(oy: usize) -> usize {
    oy + 9
}

pub fn right_reports_title_row(app: &DashApp, oy: usize) -> usize {
    (oy + app.frame.height() / 2 + 2).min(section_footer_row(app, oy).saturating_sub(4))
}

pub fn left_panel_content_width() -> usize {
    LEFT_WIDTH.saturating_sub(1)
}

pub fn right_panel_content_width() -> usize {
    RIGHT_WIDTH.saturating_sub(1)
}

pub fn write_width_clipped(
    buf: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    width: usize,
    text: &str,
    style: nc_ui::CellStyle,
) {
    if width == 0 {
        return;
    }
    let clipped: String = text.chars().take(width).collect();
    buf.write_text_clipped(row, col, &clipped, style);
}

/// Compute the (col, row) offset to center the frame in the canvas.
pub fn frame_offset(app: &DashApp) -> (usize, usize) {
    let cw = app.geometry.width();
    let ch = app.geometry.height();
    let fw = app.frame.width();
    let fh = app.frame.height();
    let ox = cw.saturating_sub(fw) / 2;
    let oy = ch.saturating_sub(fh) / 2;
    (ox, oy)
}

/// Column of the left vertical divider relative to frame origin.
pub fn left_divider_col(ox: usize) -> usize {
    ox + 1 + LEFT_WIDTH
}

/// Column of the right vertical divider relative to frame origin.
pub fn right_divider_col(app: &DashApp, ox: usize) -> usize {
    ox + app.frame.width().saturating_sub(1 + RIGHT_WIDTH)
}

/// First column of the center (map) area.
pub fn center_start_col(ox: usize) -> usize {
    ox + 1 + LEFT_WIDTH + 1
}

/// Usable width of the center area.
pub fn center_width(app: &DashApp) -> usize {
    app.frame.width().saturating_sub(2 + LEFT_WIDTH + 1 + 1 + RIGHT_WIDTH)
}

/// First column of the right panel content.
pub fn right_content_col(app: &DashApp, ox: usize) -> usize {
    right_divider_col(app, ox) + 1
}

/// Create a new PlayfieldBuffer at the full canvas size, filled with theme bg.
pub fn new_dashboard_buffer(geometry: ScreenGeometry) -> PlayfieldBuffer {
    PlayfieldBuffer::new(geometry.width(), geometry.height(), theme::body_style())
}

/// Draw the complete outer border + column dividers + header/footer dividers.
pub fn draw_frame(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let (ox, oy) = frame_offset(app);
    let fw = app.frame.width();
    let fh = app.frame.height();
    let bs = theme::border_style();

    let left_div = left_divider_col(ox);
    let right_div = right_divider_col(app, ox);
    let top = oy;
    let bottom = oy + fh.saturating_sub(1);
    let header_div = top + 1;
    let footer_div = bottom.saturating_sub(1);

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
        ox + 1,
        left_div.saturating_sub(1),
        left_planets_title_row(oy).saturating_sub(1),
        bs,
    );
    draw_panel_separator(
        buf,
        ox + 1,
        left_div.saturating_sub(1),
        left_fleets_title_row(app, oy).saturating_sub(1),
        bs,
    );
    draw_panel_separator(
        buf,
        right_div + 1,
        ox + fw.saturating_sub(2),
        right_diplomacy_title_row(oy).saturating_sub(1),
        bs,
    );
    draw_panel_separator(
        buf,
        right_div + 1,
        ox + fw.saturating_sub(2),
        right_reports_title_row(app, oy).saturating_sub(1),
        bs,
    );
}

/// Draw header text on the top border row (between corners).
pub fn draw_header(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let (ox, oy) = frame_offset(app);
    let fw = app.frame.width();
    let row = oy; // top border row

    // Branding (after left corner).
    buf.write_text(row, ox + 2, " NOSTRIAN CONQUEST ", theme::title_style());

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
        buf.write_text(row, name_start, &empire_name, theme::title_style());
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
        .filter(|f| f.owner_empire_raw() == app.player_record_index_1_based as u8 && f.has_any_force())
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
    buf.write_text(row, right_col, &right_str, style);
}

/// Draw footer text on the bottom border row (between corners).
pub fn draw_footer(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let (ox, oy) = frame_offset(app);
    let row = oy + app.frame.height().saturating_sub(1);
    buf.write_text(
        row,
        ox + 2,
        " P:Planets F:Fleets I:Intel R:Inbox D:Diplomacy A:Autopilot X:Tax S:Settings Q:Quit ? ",
        theme::footer_style(),
    );
}

fn draw_panel_separator(
    buf: &mut PlayfieldBuffer,
    left: usize,
    right: usize,
    row: usize,
    style: nc_ui::CellStyle,
) {
    if left > right {
        return;
    }
    for col in left..=right {
        buf.set_cell(row, col, '─', style);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_width_clipped_respects_panel_width() {
        let mut buffer = PlayfieldBuffer::new(32, 4, theme::body_style());
        write_width_clipped(
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
        draw_panel_separator(&mut buffer, 2, 10, 3, theme::border_style());
        assert_eq!(buffer.plain_line(3), "  ─────────");
    }
}
