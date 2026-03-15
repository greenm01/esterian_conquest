use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::screen::layout::{
    draw_command_line_default_input, draw_command_prompt, draw_status_line, draw_title_bar,
    new_playfield,
};
use crate::screen::table::{TableColumn, write_table_window_with_cursor};
use crate::screen::{CommandMenu, PlayfieldBuffer, command_menu_label};
use crate::theme::classic;

pub const PLANET_DATABASE_VISIBLE_ROWS: usize = 11;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetDatabaseRow {
    pub planet_record_index_1_based: usize,
    pub coords: [u8; 2],
    pub name_label: String,
    pub owner_label: String,
    pub potential_label: String,
    pub armies_label: String,
    pub batteries_label: String,
    pub last_intel_year_label: String,
    pub intel_label: String,
}

pub struct PlanetDatabaseScreen;

const DATABASE_COLUMNS: [TableColumn<'static>; 8] = [
    TableColumn::left("Planet Name", 15),
    TableColumn::left("Location", 9),
    TableColumn::left("Owner", 14),
    TableColumn::right("Prod", 4),
    TableColumn::right("Arm", 3),
    TableColumn::right("GB", 2),
    TableColumn::left("Year", 5),
    TableColumn::left("Intel", 15),
];

impl PlanetDatabaseScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_list(
        &mut self,
        rows: &[PlanetDatabaseRow],
        scroll_offset: usize,
        cursor: usize,
        default_coords: [u8; 2],
        input: &str,
        status: Option<&str>,
        menu: CommandMenu,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "TOTAL PLANET DATABASE:");
        buffer.write_text(
            2,
            0,
            "Planets your empire has encountered, sorted by location.",
            classic::status_value_style(),
        );
        let table_rows = rows
            .iter()
            .map(|row| {
                vec![
                    row.name_label.clone(),
                    format!("({:>2},{:>2})", row.coords[0], row.coords[1]),
                    row.owner_label.clone(),
                    row.potential_label.clone(),
                    row.armies_label.clone(),
                    row.batteries_label.clone(),
                    row.last_intel_year_label.clone(),
                    row.intel_label.clone(),
                ]
            })
            .collect::<Vec<_>>();
        let selected = if table_rows.is_empty() {
            None
        } else {
            Some(cursor)
        };
        write_table_window_with_cursor(
            &mut buffer,
            4,
            &DATABASE_COLUMNS,
            &table_rows,
            scroll_offset,
            PLANET_DATABASE_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
            selected,
        );
        if table_rows.is_empty() {
            draw_status_line(
                &mut buffer,
                16,
                "Notice: ",
                "No planets are currently recorded in your database.",
            );
        }
        if let Some(status) = status {
            draw_status_line(&mut buffer, 16, "Error: ", status);
        }
        draw_command_line_default_input(
            &mut buffer,
            command_menu_label(menu),
            "",
            &format!("{},{}", default_coords[0], default_coords[1]),
            input,
        );
        Ok(buffer)
    }

    pub fn render_detail(
        &mut self,
        row: &PlanetDatabaseRow,
        selected_index: usize,
        total: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(
            &mut buffer,
            0,
            &format!("TOTAL PLANET DATABASE {}/{}:", selected_index + 1, total),
        );
        draw_status_line(
            &mut buffer,
            2,
            "Coordinates: ",
            &format!("X={}, Y={}", row.coords[0], row.coords[1]),
        );
        draw_status_line(&mut buffer, 3, "Planet Name: ", &row.name_label);
        draw_status_line(&mut buffer, 4, "Known Owner: ", &row.owner_label);
        draw_status_line(
            &mut buffer,
            6,
            "Potential Production: ",
            &row.potential_label,
        );
        draw_status_line(&mut buffer, 7, "Armies: ", &row.armies_label);
        draw_status_line(&mut buffer, 8, "Ground Batteries: ", &row.batteries_label);
        draw_status_line(&mut buffer, 10, "Last Intel: ", &row.last_intel_year_label);
        draw_status_line(&mut buffer, 11, "Known Intel: ", &row.intel_label);
        buffer.write_text(
            13,
            0,
            "Use arrows or HJKL to browse other known planets in the database.",
            classic::body_style(),
        );
        draw_command_prompt(&mut buffer, 19, "PLANET DATABASE", "ARROWS H J K L Q");
        Ok(buffer)
    }

    pub fn handle_list_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::MovePlanetDatabaseList(-1)
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::MovePlanetDatabaseList(1)
            }
            KeyCode::PageUp => Action::MovePlanetDatabaseList(-8),
            KeyCode::PageDown => Action::MovePlanetDatabaseList(8),
            KeyCode::Char(ch) if ch.is_ascii_digit() || ch == ',' || ch == ' ' => {
                Action::AppendPlanetDatabaseChar(ch)
            }
            KeyCode::Backspace => Action::BackspacePlanetDatabaseInput,
            KeyCode::Enter => Action::SubmitPlanetDatabaseLookup,
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::ReturnToCommandMenu,
            _ => Action::Noop,
        }
    }

    pub fn handle_detail_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Left => Action::MovePlanetDatabaseDetail(-1),
            KeyCode::Down | KeyCode::Right => Action::MovePlanetDatabaseDetail(1),
            KeyCode::Home => Action::MovePlanetDatabaseDetail(i8::MIN),
            KeyCode::End => Action::MovePlanetDatabaseDetail(i8::MAX),
            KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Char('h') | KeyCode::Char('H') => {
                Action::MovePlanetDatabaseDetail(-1)
            }
            KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Char('l') | KeyCode::Char('L') => {
                Action::MovePlanetDatabaseDetail(1)
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenPlanetDatabase,
            _ => Action::Noop,
        }
    }
}
