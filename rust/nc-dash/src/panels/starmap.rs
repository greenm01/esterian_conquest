//! Center panel: sector grid, crosshair, axis labels, status line.
//!
//! Grid: 3-char-wide × 1-row-tall cells. Full map always visible.
//! Red dashed crosshair at the cursor sector.

use nc_ui::{CellStyle, GameColor, PlayfieldBuffer};

use crate::app::state::DashApp;
use crate::layout::{LEFT_WIDTH, RIGHT_WIDTH, center_width};
use crate::theme;

const ROW_LABEL_COLS: usize = 3; // "18 " — 2 digits + space
const CELL_WIDTH: usize = 3;     // 3 chars per sector

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let total_w = buf.width();
    let total_h = buf.height();
    let center_w = center_width(total_w);
    let map_start_col = LEFT_WIDTH + 2; // +2 for left border + pad

    let axis_row = 2;      // column number header row
    let grid_start = 3;    // first grid row

    // Number of map sectors visible horizontally.
    let available_cols = center_w.saturating_sub(ROW_LABEL_COLS + 2); // +2 for borders
    let map_cols = (available_cols / CELL_WIDTH).min(36).max(1);

    // Number of map sectors visible vertically.
    let available_rows = total_h.saturating_sub(grid_start + 3); // +3 for header/footer/status
    let map_rows = available_rows.min(36).max(1);

    let player_empire = app.player_record_index_1_based as u8;

    // Column axis numbers.
    for col_idx in 0..map_cols {
        let screen_col = map_start_col + ROW_LABEL_COLS + col_idx * CELL_WIDTH;
        if screen_col + 1 < total_w {
            let label = format!("{:02}", col_idx + 1);
            buf.write_text(axis_row, screen_col, &label, theme::dim_style());
        }
    }

    // Grid rows — row 1 at bottom (map_rows at top).
    for row_idx in 0..map_rows {
        let row_y = (map_rows - row_idx) as u8;
        let screen_row = grid_start + row_idx;
        if screen_row >= total_h.saturating_sub(2) {
            break;
        }
        let is_h_crosshair = row_y == app.crosshair_y;

        // Row label.
        let label = format!("{:02} ", row_y);
        buf.write_text(screen_row, map_start_col, &label, theme::dim_style());

        // Grid cells.
        for col_idx in 0..map_cols {
            let col_x = (col_idx + 1) as u8;
            let screen_col = map_start_col + ROW_LABEL_COLS + col_idx * CELL_WIDTH;
            if screen_col + CELL_WIDTH > total_w.saturating_sub(RIGHT_WIDTH + 2) {
                break;
            }
            let is_v_crosshair = col_x == app.crosshair_x;

            // Find what's at this sector.
            let planet = app.game_data.planets.records.iter().find(|p| {
                p.coords_raw() == [col_x, row_y] && p.owner_empire_slot_raw() != 0
            });

            let (sym, base_style) = if let Some(p) = planet {
                let owner = p.owner_empire_slot_raw();
                if owner == player_empire {
                    ('■', theme::friendly_style())
                } else {
                    ('●', theme::enemy_style())
                }
            } else {
                ('·', theme::dim_style())
            };

            // Render cell with crosshair overlay.
            let (left, mid, right, cell_style) = if is_h_crosshair && is_v_crosshair {
                (' ', sym, ' ', CellStyle::new(GameColor::BrightWhite, GameColor::BrightBlack, true))
            } else if is_h_crosshair {
                ('─', sym, '─', CellStyle::new(GameColor::BrightRed, GameColor::Black, false))
            } else if is_v_crosshair {
                (' ', sym, ' ', CellStyle::new(GameColor::BrightRed, GameColor::Black, false))
            } else {
                (' ', sym, ' ', base_style)
            };

            buf.set_cell(screen_row, screen_col, left, cell_style);
            buf.set_cell(screen_row, screen_col + 1, mid, if is_h_crosshair || is_v_crosshair { cell_style } else { base_style });
            buf.set_cell(screen_row, screen_col + 2, right, cell_style);
        }
    }

    // Status line below grid.
    let status_row = grid_start + map_rows;
    if status_row < total_h.saturating_sub(1) {
        // Find what's at the crosshair.
        let cx = app.crosshair_x;
        let cy = app.crosshair_y;
        let planet_info = app.game_data.planets.records.iter().find(|p| {
            p.coords_raw() == [cx, cy] && p.owner_empire_slot_raw() != 0
        });
        let status = if let Some(p) = planet_info {
            let name = p.planet_name();
            let prod = p.present_production_points().unwrap_or(0);
            let armies = p.army_count_raw();
            let batt = p.ground_batteries_raw();
            format!(
                " Sector ({:02},{:02}) {} — {} prod, {} AR, {} RB",
                cx, cy, name, prod, armies, batt
            )
        } else {
            format!(" Sector ({:02},{:02}) — empty", cx, cy)
        };
        let truncated: String = status.chars().take(center_w.saturating_sub(2)).collect();
        buf.write_text(status_row, map_start_col, &truncated, theme::value_style());
    }
}
