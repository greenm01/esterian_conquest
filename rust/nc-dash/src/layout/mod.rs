//! Three-column frame, border drawing, resize handling.

pub mod geometry;

use nc_ui::{PlayfieldBuffer, ScreenGeometry};

use crate::app::state::DashApp;
use crate::layout::geometry::{SIDE_PANEL_WIDTH, MAX_RENDER_WIDTH};
use crate::theme;

/// Left panel width in terminal columns.
pub const LEFT_WIDTH: usize = SIDE_PANEL_WIDTH;
/// Right panel width in terminal columns.
pub const RIGHT_WIDTH: usize = SIDE_PANEL_WIDTH;

/// Compute the center (map) column width given total terminal width.
pub fn center_width(total_width: usize) -> usize {
    let capped = total_width.min(MAX_RENDER_WIDTH);
    capped.saturating_sub(LEFT_WIDTH + RIGHT_WIDTH + 4) // 4 for borders
}

/// Create a new PlayfieldBuffer sized to the dashboard geometry.
pub fn new_dashboard_buffer(geometry: ScreenGeometry) -> PlayfieldBuffer {
    PlayfieldBuffer::new(geometry.width(), geometry.height(), theme::body_style())
}

/// Draw the three-column border frame, header divider, and footer divider.
pub fn draw_frame(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let w = buf.width();
    let h = buf.height();
    let left_divider = LEFT_WIDTH + 1; // +1 for the border char itself
    let right_divider = w.saturating_sub(RIGHT_WIDTH + 1);
    let header_row = 0;
    let footer_row = h.saturating_sub(1);
    let border_style = theme::border_style();

    // Horizontal dividers.
    for col in 0..w {
        buf.set_cell(header_row + 1, col, '─', border_style);
        buf.set_cell(footer_row.saturating_sub(1), col, '─', border_style);
    }

    // Vertical dividers (between columns).
    for row in 1..h.saturating_sub(1) {
        buf.set_cell(row, left_divider, '│', border_style);
        buf.set_cell(row, right_divider, '│', border_style);
    }

    // Tee joints at the horizontal dividers.
    buf.set_cell(header_row + 1, left_divider, '├', border_style);
    buf.set_cell(header_row + 1, right_divider, '┤', border_style);
    buf.set_cell(footer_row.saturating_sub(1), left_divider, '├', border_style);
    buf.set_cell(footer_row.saturating_sub(1), right_divider, '┤', border_style);
}

/// Draw the header bar: branding, empire name, stats.
pub fn draw_header(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let row = 0;
    let w = buf.width();
    buf.fill_row(row, theme::header_style());

    // Left: branding.
    buf.write_text(row, 1, "NOSTRIAN CONQUEST", theme::title_style());

    // Center: empire name (player record 0-based).
    let empire_name = app
        .game_data
        .player
        .records
        .get(app.player_record_index_1_based.saturating_sub(1))
        .map(|p| String::from_utf8_lossy(p.empire_name_bytes()).trim_end_matches('\0').to_string())
        .unwrap_or_default();
    let name_col = w / 2 - empire_name.len() / 2;
    buf.write_text(row, name_col, &empire_name, theme::title_style());

    // Right: year, planets, fleets, autopilot, tax.
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
            f.owner_empire_raw() == app.player_record_index_1_based as u8
                && f.has_any_force()
        })
        .count();
    let tax = player.map(|p| p.tax_rate()).unwrap_or(0);
    let ap = if app.autopilot_on { "ON " } else { "OFF" };
    let right_str = format!(
        "Y{year}  Planets:{planet_count}  Fleets:{fleet_count}  Autopilot:{ap}  Tax:{tax}%"
    );
    let right_col = w.saturating_sub(right_str.len() + 1);
    let ap_style = if app.autopilot_on {
        theme::alert_style()
    } else {
        theme::header_style()
    };
    buf.write_text(row, right_col, &right_str, ap_style);
}

/// Draw the footer hotkey bar.
pub fn draw_footer(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let row = buf.height().saturating_sub(1);
    buf.fill_row(row, theme::footer_style());
    let _ = app; // future: context-sensitive hotkeys
    buf.write_text(
        row,
        1,
        "P:Planets  F:Fleets  I:Intel  R:Inbox  D:Diplomacy  A:Autopilot  X:Tax  S:Settings  Q:Quit  ?",
        theme::footer_style(),
    );
}
