use crossterm::event::{KeyCode, KeyEvent};
use ec_data::{build_player_starmap_projection, DatabaseDat};

use crate::app::Action;
use crate::screen::layout::{
    draw_command_prompt, draw_plain_prompt, draw_status_line, draw_title_bar, new_playfield,
};
use crate::screen::{CommandMenu, PlayfieldBuffer, ScreenFrame, StyledSpan};
use crate::theme::classic;

pub struct PartialStarmapScreen;

impl PartialStarmapScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_prompt(
        &mut self,
        input: &str,
        error: Option<&str>,
        menu: CommandMenu,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "VIEW PARTIAL STARMAP:");
        let prompt = format!("Enter coordinates for center of partial map (x,y): {input}");
        let cursor_col = draw_plain_prompt(&mut buffer, 2, &prompt);
        if let Some(error) = error {
            draw_status_line(&mut buffer, 4, "Error: ", error);
        }
        draw_command_prompt(&mut buffer, 6, command_label(menu), "Q");
        buffer.set_cursor(cursor_col as u16, 2);
        Ok(buffer)
    }

    pub fn render_view(
        &mut self,
        frame: &ScreenFrame<'_>,
        database: &DatabaseDat,
        center: [u8; 2],
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let projection = build_player_starmap_projection(
            frame.game_data,
            database,
            frame.player.record_index_1_based as u8,
        );
        let mut buffer = new_playfield();
        let title = format!("Map Center at Sector ({},{})", center[0], center[1]);
        draw_title_bar(&mut buffer, 0, &title);
        buffer.write_text(0, 36, "Col: 8, Row: 2 in red", classic::alert_style());

        let map_width = projection.map_width as usize;
        let map_height = projection.map_height as usize;
        let mut start_x = center[0].saturating_sub(8).max(1) as usize;
        let mut start_y = center[1].saturating_sub(8).max(1) as usize;
        let mut end_x = usize::min(start_x + 16, map_width);
        let mut end_y = usize::min(start_y + 16, map_height);
        if end_x.saturating_sub(start_x) < 16 && map_width > 17 {
            start_x = end_x.saturating_sub(16).max(1);
        }
        if end_y.saturating_sub(start_y) < 16 && map_height > 17 {
            start_y = end_y.saturating_sub(16).max(1);
        }
        end_x = usize::min(start_x + 16, map_width);
        end_y = usize::min(start_y + 16, map_height);

        for y in start_y..=end_y {
            let screen_row = 17 - (y - start_y);
            buffer.write_text(
                screen_row,
                0,
                &format!("{y:>2} "),
                classic::status_value_style(),
            );
            buffer.write_text(screen_row, 3, "|", classic::status_value_style());
            buffer.write_text(screen_row, 55, "|", classic::status_value_style());
            for x in start_x..=end_x {
                let screen_col = 5 + (x - start_x) * 3;
                buffer.write_text(screen_row, screen_col, ".", classic::map_dot_style());
            }
        }

        for x in start_x..=end_x {
            let col = 4 + (x - start_x) * 3;
            buffer.write_text(18, col, &format!("{x:>2}"), classic::status_value_style());
        }

        let center_col = 5 + (center[0] as usize - start_x) * 3;
        let center_row = 17 - (center[1] as usize - start_y);
        buffer.write_text(
            center_row,
            4,
            &"-".repeat(51),
            classic::map_crosshair_style(),
        );
        for row in 1..=17 {
            buffer.write_text(row, center_col, "|", classic::map_crosshair_style());
        }
        buffer.write_text(center_row, center_col, "+", classic::map_crosshair_style());

        for world in &projection.worlds {
            let x = world.coords[0] as usize;
            let y = world.coords[1] as usize;
            if x < start_x || x > end_x || y < start_y || y > end_y {
                continue;
            }
            let screen_col = 5 + (x - start_x) * 3;
            let screen_row = 17 - (y - start_y);
            let actual_owner = frame
                .game_data
                .planet_record_index_at_coords(world.coords)
                .and_then(|idx| frame.game_data.planets.records.get(idx))
                .map(|planet| planet.owner_empire_slot_raw() as usize)
                .unwrap_or(0);
            let symbol = match world.known_owner_empire_id {
                Some(empire_id) if empire_id as usize == frame.player.record_index_1_based => 'O',
                Some(_) => '#',
                None if actual_owner == frame.player.record_index_1_based => 'O',
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

        buffer.write_text(1, 58, "\".\"= Empty Sector", classic::body_style());
        buffer.write_text(2, 58, "\"*\"= Unowned Planet", classic::body_style());
        buffer.write_text(3, 58, "\"#\"= Owned by Emp #", classic::body_style());
        buffer.write_text(4, 58, "\"O\"= Planet You Own", classic::body_style());
        buffer.write_text(5, 58, "\"?\"= Unexplored", classic::body_style());
        buffer.write_text(7, 58, &" ".repeat(22), classic::menu_style());
        buffer.write_text(7, 59, "STARMAP MENU", classic::title_style());
        buffer.write_text(8, 60, "Arrows / HJKL", classic::prompt_hotkey_style());
        buffer.write_text(9, 60, "7 8 9  = up", classic::body_style());
        buffer.write_text(10, 60, "4   6  = left/right", classic::body_style());
        buffer.write_text(11, 60, "1 2 3  = down", classic::body_style());
        buffer.write_text(12, 60, "Enter/Q = quit", classic::body_style());
        let cursor_col = buffer.write_spans(
            19,
            0,
            &[
                StyledSpan::new("Map Command:[", classic::prompt_style()),
                StyledSpan::new("( 1 2 3 4 5 6 7 8 9 )", classic::prompt_hotkey_style()),
                StyledSpan::new(" [Enter]=quit -> ", classic::prompt_style()),
            ],
        );
        buffer.set_cursor(cursor_col as u16, 19);
        Ok(buffer)
    }

    pub fn handle_prompt_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::ReturnToCommandMenu,
            KeyCode::Enter => Action::SubmitPartialStarmapPrompt,
            KeyCode::Backspace => Action::BackspacePartialStarmapInput,
            KeyCode::Char(ch)
                if ch.is_ascii_digit() || matches!(ch, ',' | ' ' | ':' | '/' | '-') =>
            {
                Action::AppendPartialStarmapChar(ch)
            }
            _ => Action::Noop,
        }
    }

    pub fn handle_view_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Char('8') => {
                Action::MovePartialStarmap(0, 1)
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Char('2') => {
                Action::MovePartialStarmap(0, -1)
            }
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('4') => {
                Action::MovePartialStarmap(-1, 0)
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') | KeyCode::Char('6') => {
                Action::MovePartialStarmap(1, 0)
            }
            KeyCode::Char('7') => Action::MovePartialStarmap(-1, 1),
            KeyCode::Char('9') => Action::MovePartialStarmap(1, 1),
            KeyCode::Char('1') => Action::MovePartialStarmap(-1, -1),
            KeyCode::Char('3') => Action::MovePartialStarmap(1, -1),
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                Action::ReturnToCommandMenu
            }
            _ => Action::Noop,
        }
    }
}

fn command_label(menu: CommandMenu) -> &'static str {
    match menu {
        CommandMenu::General => "GENERAL COMMAND",
        CommandMenu::Planet => "PLANET COMMAND",
        CommandMenu::PlanetBuild => "BUILD COMMAND",
    }
}
