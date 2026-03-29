use crossterm::event::{KeyCode, KeyEvent};
use ec_data::build_player_starmap_projection_from_snapshots;

use crate::app::Action;
use crate::domains::starmap::StarmapAction;
use crate::screen::layout::{
    PLAYFIELD_WIDTH, centered_row, command_line_row_for, draw_command_prompt_at_col,
    draw_status_line, new_playfield_for, table_prompt_row_for,
};
use crate::screen::{PlayfieldBuffer, ScreenFrame, format_sector_coords};
use crate::theme::classic;

pub struct PartialStarmapScreen;

const MAP_TOP_ROW: usize = 1;
const SEPARATOR_COL: usize = 3;
const AXIS_LABEL_COL: usize = 4;
const MAP_CELL_START_COL: usize = 5;
const MAP_CELL_STEP: usize = 3;
const VISIBLE_MAP_COLUMNS: usize = 25;

/// Width of the dot/symbol grid alone: first cell to last cell inclusive.
const fn grid_width(visible_columns: usize) -> usize {
    (visible_columns - 1) * MAP_CELL_STEP + 1
}

fn oversized_viewport_start(center: u8, visible: usize, map_size: usize) -> usize {
    let centered = center as isize - (visible / 2) as isize;
    centered.clamp(1, map_size as isize - visible as isize + 1) as usize
}

impl PartialStarmapScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_view(
        &mut self,
        frame: &ScreenFrame<'_>,
        center: [u8; 2],
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let projection = build_player_starmap_projection_from_snapshots(
            frame.game_data,
            frame.planet_intel_snapshots,
            frame.player.record_index_1_based as u8,
        );
        let mut buffer = new_playfield_for(frame.geometry);
        let map_top_frame_row = if let Some(status) = status {
            draw_status_line(&mut buffer, 1, "Status: ", status);
            MAP_TOP_ROW + 1
        } else {
            MAP_TOP_ROW
        };
        let map_bottom_frame_row = command_line_row_for(frame.geometry).saturating_sub(1);

        let map_width = projection.map_width as usize;
        let map_height = projection.map_height as usize;
        let available_map_rows = map_bottom_frame_row.saturating_sub(map_top_frame_row);
        let horizontal_overflow = map_width > VISIBLE_MAP_COLUMNS;
        let vertical_overflow = map_height > available_map_rows;
        let visible_columns = if horizontal_overflow {
            VISIBLE_MAP_COLUMNS
        } else {
            map_width.max(1)
        };
        let visible_rows = if vertical_overflow {
            available_map_rows
        } else {
            map_height.max(1)
        };
        let fits_entirely = !horizontal_overflow && !vertical_overflow;
        let (map_cell_start_col, map_top_row, map_bottom_row, x_axis_row) = if fits_entirely {
            // Center the grid of cells in the playfield; axes sit just outside.
            let cell_start = (PLAYFIELD_WIDTH - grid_width(visible_columns)) / 2;
            let top = centered_row(map_top_frame_row, map_bottom_frame_row, visible_rows);
            let bottom = top + visible_rows - 1;
            let x_axis = bottom + 1;
            (cell_start, top, bottom, x_axis)
        } else {
            // Anchor axes at col 0 / row 23; fill all available space.
            let x_axis = map_bottom_frame_row;
            let bottom = x_axis - 1;
            let top = bottom + 1 - visible_rows;
            (MAP_CELL_START_COL, top, bottom, x_axis)
        };
        let map_left_col = map_cell_start_col - MAP_CELL_START_COL;
        let separator_col = map_left_col + SEPARATOR_COL;
        let axis_label_col = map_left_col + AXIS_LABEL_COL;
        let title = format!("Map Center at Sector {}", format_sector_coords(center));
        let title_row = map_top_row.saturating_sub(1);
        buffer.fill_row(title_row, classic::menu_style());
        buffer.write_text(title_row, map_left_col, &title, classic::title_style());
        let start_x = if horizontal_overflow {
            oversized_viewport_start(center[0], visible_columns, map_width)
        } else {
            1
        };
        let start_y = if vertical_overflow {
            oversized_viewport_start(center[1], visible_rows, map_height)
        } else {
            1
        };
        let end_x = start_x + visible_columns - 1;
        let end_y = start_y + visible_rows - 1;

        for row_offset in 0..visible_rows {
            let world_y = end_y - row_offset;
            let screen_row = map_top_row + row_offset;
            buffer.write_text(
                screen_row,
                map_left_col,
                &format!("{world_y:02} "),
                classic::status_value_style(),
            );
            buffer.write_text(
                screen_row,
                separator_col,
                "|",
                classic::status_value_style(),
            );
            for column_offset in 0..visible_columns {
                let screen_col = map_cell_start_col + column_offset * MAP_CELL_STEP;
                buffer.write_text(screen_row, screen_col, ".", classic::map_dot_style());
            }
        }

        for column_offset in 0..visible_columns {
            let world_x = start_x + column_offset;
            let col = axis_label_col + column_offset * MAP_CELL_STEP;
            buffer.write_text(
                x_axis_row,
                col,
                &format!("{world_x:02}"),
                classic::status_value_style(),
            );
        }

        let center_col = map_cell_start_col + (center[0] as usize - start_x) * MAP_CELL_STEP;
        let center_row = map_bottom_row - (center[1] as usize - start_y);
        buffer.write_text(
            center_row,
            map_cell_start_col,
            &"-".repeat((visible_columns - 1) * MAP_CELL_STEP + 1),
            classic::map_crosshair_style(),
        );
        for row_offset in 0..visible_rows {
            buffer.write_text(
                map_top_row + row_offset,
                center_col,
                "|",
                classic::map_crosshair_style(),
            );
        }
        buffer.write_text(center_row, center_col, "+", classic::map_crosshair_style());

        for world in &projection.worlds {
            let x = world.coords[0] as usize;
            let y = world.coords[1] as usize;
            if x < start_x || x > end_x || y < start_y || y > end_y {
                continue;
            }
            let screen_col = map_cell_start_col + (x - start_x) * MAP_CELL_STEP;
            let screen_row = map_bottom_row - (y - start_y);
            let symbol = match world.known_owner_empire_id {
                Some(empire_id) if empire_id as usize == frame.player.record_index_1_based => 'O',
                Some(_) => '#',
                None if world.known_name.is_some()
                    || world.known_potential_production.is_some()
                    || world.known_armies.is_some()
                    || world.known_ground_batteries.is_some() =>
                {
                    '*'
                }
                None => '?',
            };
            buffer.write_text(
                screen_row,
                screen_col,
                &symbol.to_string(),
                classic::bright_style(),
            );
        }
        buffer.write_text(center_row, center_col, "+", classic::map_crosshair_style());

        draw_command_prompt_at_col(
            &mut buffer,
            table_prompt_row_for(frame.geometry, x_axis_row),
            map_left_col,
            "MAP COMMAND",
            "? HJKL 1 2 3 4 6 7 8 9 <Q>",
        );
        Ok(buffer)
    }

    pub fn handle_view_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Char('8') => {
                Action::Starmap(StarmapAction::MovePartial(0, 1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Char('2') => {
                Action::Starmap(StarmapAction::MovePartial(0, -1))
            }
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('4') => {
                Action::Starmap(StarmapAction::MovePartial(-1, 0))
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') | KeyCode::Char('6') => {
                Action::Starmap(StarmapAction::MovePartial(1, 0))
            }
            KeyCode::Char('7') => Action::Starmap(StarmapAction::MovePartial(-1, 1)),
            KeyCode::Char('9') => Action::Starmap(StarmapAction::MovePartial(1, 1)),
            KeyCode::Char('1') => Action::Starmap(StarmapAction::MovePartial(-1, -1)),
            KeyCode::Char('3') => Action::Starmap(StarmapAction::MovePartial(1, -1)),
            KeyCode::Enter => Action::Starmap(StarmapAction::OpenPlanetInfoAtCenter),
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => Action::ReturnToCommandMenu,
            _ => Action::Noop,
        }
    }
}
