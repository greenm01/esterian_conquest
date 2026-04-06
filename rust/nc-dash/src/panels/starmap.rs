//! Center panel: sector grid, crosshair, axis labels, status line.
//!
//! Grid: 3-char-wide × 1-row-tall cells. Full map always visible.
//! Red dashed crosshair at the cursor sector.

use nc_ui::{GameColor, PlayfieldBuffer};
use crate::app::state::DashApp;
use crate::layout::{LEFT_WIDTH, center_width};
use crate::theme;

/// Planet/fleet display symbols.
const SYM_EMPTY: char = '·';
const SYM_OWNED: char = '■';
const SYM_ENEMY: char = '●';
const SYM_NEUTRAL: char = 'o';
const SYM_ICD: char = '◊';

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let total_w = buf.width();
    let center_w = center_width(total_w);
    let map_start_col = LEFT_WIDTH + 2; // +2 for border + 1 pad

    let header_row = 2; // row axis header
    let grid_start_row = 3;

    // Column axis (01..18 or however many fit).
    let map_cols = (center_w.saturating_sub(3)) / 3; // 3 chars per sector, minus row label space
    let map_cols = map_cols.min(36).max(1);
    let row_label_cols = 3; // "18 " — 2 digits + space

    // Draw column numbers centered over each 3-char cell.
    for col_idx in 0..map_cols {
        let screen_col = map_start_col + row_label_cols + col_idx * 3;
        let label = format!("{:02}", col_idx + 1);
        buf.write_text(header_row, screen_col, &label, theme::dim_style());
    }

    // Draw grid rows.
    let map_rows = app.game_data.planets.records.len().min(36).max(1);
    let map_rows = if map_rows == 0 { 18 } else { map_rows }.min(36);

    // Collect planet info for rendering.
    let player_empire = app.player_record_index_1_based as u8;

    for row_idx in 0..map_rows {
        let row_y = (map_rows - row_idx) as u8; // rows descend: 18 at top, 01 at bottom
        let screen_row = grid_start_row + row_idx;
        let is_crosshair_row = row_y == app.crosshair_y;

        // Row label.
        let label = format!("{:02} ", row_y);
        buf.write_text(screen_row, map_start_col, &label, theme::dim_style());

        // Grid cells.
        for col_idx in 0..map_cols {
            let col_x = (col_idx + 1) as u8;
            let screen_col = map_start_col + row_label_cols + col_idx * 3;
            let is_crosshair_col = col_x == app.crosshair_x;

            // Find what's at this sector.
            let planet = app.game_data.planets.records.iter().find(|p| {
                p.coords_raw() == [col_x, row_y] && p.owner_empire_slot_raw() != 0
            });

            let (sym, style) = if let Some(p) = planet {
                let owner = p.owner_empire_slot_raw();
                if owner == player_empire {
                    (SYM_OWNED, theme::friendly_style())
                } else {
                    (SYM_ENEMY, theme::enemy_style())
                }
            } else {
                (SYM_EMPTY, theme::dim_style())
            };

            // Crosshair styling.
            if is_crosshair_row && is_crosshair_col {
                // Intersection: show the symbol bright.
                let cx_style = nc_ui::CellStyle::new(GameColor::BrightWhite, GameColor::BrightBlack, true);
                buf.write_text(screen_col, screen_row, &format!(" {sym} "), cx_style);
            } else if is_crosshair_row {
                let h_style = nc_ui::CellStyle::new(GameColor::BrightRed, GameColor::Black, false);
                buf.write_text(screen_row, screen_col, &format!("─{sym}─"), h_style);
            } else if is_crosshair_col {
                let v_style = nc_ui::CellStyle::new(GameColor::BrightRed, GameColor::Black, false);
                buf.write_text(screen_row, screen_col, &format!(" {sym} "), v_style);
                buf.set_cell(screen_row, screen_col + 1, sym, v_style);
            } else {
                buf.write_text(screen_row, screen_col, &format!(" {sym} "), style);
            }
        }
    }

    // Status line below grid.
    let status_row = grid_start_row + map_rows;
    let status = format!(
        " Sector ({:02},{:02})",
        app.crosshair_x, app.crosshair_y
    );
    buf.write_text(status_row, map_start_col, &status, theme::value_style());
}
