use crossterm::event::{KeyCode, KeyEvent};
use ec_data::EmpirePlanetEconomyRow;

use crate::app::Action;
use crate::screen::layout::{
    draw_command_prompt, draw_plain_prompt, draw_status_line, draw_title_bar, new_playfield,
};
use crate::screen::table::{write_table_window_with_cursor, TableColumn};
use crate::screen::{PlayfieldBuffer, ScreenFrame};
use crate::theme::classic;

pub const PLANET_BRIEF_VISIBLE_ROWS: usize = 11;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetListMode {
    Brief,
    Detail,
    Stub(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetListSort {
    CurrentProduction,
    Location,
    PotentialProduction,
}

pub struct PlanetListScreen;

const BRIEF_COLUMNS: [TableColumn<'static>; 6] = [
    TableColumn::left("Planet Name", 20),
    TableColumn::left("Location", 10),
    TableColumn::left("Production", 16),
    TableColumn::right("Stored Pts", 10),
    TableColumn::right("Armies", 6),
    TableColumn::right("GBs", 4),
];

impl PlanetListScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_sort_prompt(
        &mut self,
        mode: PlanetListMode,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "PLANET COMMAND:");
        if let PlanetListMode::Stub(message) = mode {
            draw_status_line(&mut buffer, 3, "Notice: ", message);
            draw_command_prompt(&mut buffer, 19, "PLANET COMMAND", "SLAP A KEY");
            return Ok(buffer);
        }

        let cursor_col = draw_plain_prompt(
            &mut buffer,
            3,
            "List by <C>urrent Production, <L>ocation, <P>otential or <A>bort? [C] -> ",
        );
        if let Some(status) = status {
            draw_status_line(&mut buffer, 5, "Error: ", status);
        }
        draw_command_prompt(&mut buffer, 19, "PLANET COMMAND", "C L P A");
        buffer.set_cursor(cursor_col as u16, 3);
        Ok(buffer)
    }

    pub fn render_brief_list(
        &mut self,
        rows: &[EmpirePlanetEconomyRow],
        sort: PlanetListSort,
        scroll_offset: usize,
        cursor: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "PLANET COMMAND:");
        buffer.write_text(
            2,
            0,
            &format!("Listing planets by {}:", sort_label(sort)),
            classic::status_value_style(),
        );
        let table_rows = rows
            .iter()
            .map(|row| {
                vec![
                    row.planet_name.clone(),
                    format!("({:>2},{:>2})", row.coords[0], row.coords[1]),
                    format!(
                        "{:>3} of {:>3}",
                        row.present_production, row.potential_production
                    ),
                    row.stored_production_points.to_string(),
                    row.armies.to_string(),
                    row.ground_batteries.to_string(),
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
            &BRIEF_COLUMNS,
            &table_rows,
            scroll_offset,
            PLANET_BRIEF_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
            selected,
        );
        draw_command_prompt(&mut buffer, 19, "PLANET COMMAND", "ARROWS J K Q");
        Ok(buffer)
    }

    pub fn render_detail(
        &mut self,
        frame: &ScreenFrame<'_>,
        rows: &[EmpirePlanetEconomyRow],
        selected_index: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let row = rows
            .get(selected_index)
            .ok_or("planet detail row missing")?;
        let mut buffer = new_playfield();
        draw_title_bar(
            &mut buffer,
            0,
            &format!("PLANET DETAIL {}/{}:", selected_index + 1, rows.len()),
        );
        let efficiency = if row.potential_production == 0 {
            0.0
        } else {
            (row.present_production as f64 / row.potential_production as f64) * 100.0
        };
        let fleet_count = frame
            .game_data
            .fleets
            .records
            .iter()
            .filter(|fleet| {
                fleet.owner_empire_raw() as usize == frame.player.record_index_1_based
                    && fleet.current_location_coords_raw() == row.coords
            })
            .count();
        let status = frame
            .game_data
            .planets
            .records
            .get(row.planet_record_index_1_based - 1)
            .map(|planet| planet.status_or_name_summary())
            .unwrap_or_else(|| row.planet_name.clone());

        draw_status_line(&mut buffer, 2, "Planet Name: ", &row.planet_name);
        draw_status_line(
            &mut buffer,
            3,
            "Location: ",
            &format!("({},{})", row.coords[0], row.coords[1]),
        );
        draw_status_line(
            &mut buffer,
            4,
            "Current Production: ",
            &row.present_production.to_string(),
        );
        draw_status_line(
            &mut buffer,
            5,
            "Maximum Production: ",
            &row.potential_production.to_string(),
        );
        draw_status_line(
            &mut buffer,
            6,
            "Stored Production Points: ",
            &row.stored_production_points.to_string(),
        );
        draw_status_line(
            &mut buffer,
            7,
            "Expected Revenue: ",
            &format!("{} points", row.yearly_tax_revenue),
        );
        draw_status_line(
            &mut buffer,
            8,
            "Yearly Growth: ",
            &format!("+{} production", row.yearly_growth_delta),
        );
        draw_status_line(
            &mut buffer,
            9,
            "Build Capacity: ",
            &row.build_capacity.to_string(),
        );
        draw_status_line(
            &mut buffer,
            10,
            "Efficiency: ",
            &format!("{efficiency:.3}%"),
        );
        draw_status_line(&mut buffer, 12, "Armies: ", &row.armies.to_string());
        draw_status_line(
            &mut buffer,
            13,
            "Ground Batteries: ",
            &row.ground_batteries.to_string(),
        );
        draw_status_line(
            &mut buffer,
            14,
            "Friendly Fleets Here: ",
            &fleet_count.to_string(),
        );
        draw_status_line(
            &mut buffer,
            15,
            "Friendly Starbase: ",
            if row.has_friendly_starbase {
                "YES"
            } else {
                "NO"
            },
        );
        draw_status_line(&mut buffer, 16, "Status: ", &planet_status(row, &status));
        draw_command_prompt(&mut buffer, 19, "PLANET COMMAND", "ARROWS J K Q");
        Ok(buffer)
    }

    pub fn handle_sort_prompt_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Enter => {
                Action::SubmitPlanetListSort(
                    PlanetListMode::Brief,
                    PlanetListSort::CurrentProduction,
                )
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                Action::SubmitPlanetListSort(PlanetListMode::Brief, PlanetListSort::Location)
            }
            KeyCode::Char('p') | KeyCode::Char('P') => Action::SubmitPlanetListSort(
                PlanetListMode::Brief,
                PlanetListSort::PotentialProduction,
            ),
            KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Esc => Action::OpenPlanetMenu,
            _ => Action::Noop,
        }
    }

    pub fn handle_brief_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up => Action::MovePlanetBrief(-1),
            KeyCode::Down => Action::MovePlanetBrief(1),
            KeyCode::PageUp => Action::MovePlanetBrief(-5),
            KeyCode::PageDown => Action::MovePlanetBrief(5),
            KeyCode::Char('k') | KeyCode::Char('K') => Action::MovePlanetBrief(-1),
            KeyCode::Char('j') | KeyCode::Char('J') => Action::MovePlanetBrief(1),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenPlanetMenu,
            _ => Action::Noop,
        }
    }

    pub fn handle_detail_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Left => Action::MovePlanetDetail(-1),
            KeyCode::Down | KeyCode::Right => Action::MovePlanetDetail(1),
            KeyCode::Home => Action::MovePlanetDetail(i8::MIN),
            KeyCode::End => Action::MovePlanetDetail(i8::MAX),
            KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Char('h') | KeyCode::Char('H') => {
                Action::MovePlanetDetail(-1)
            }
            KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Char('l') | KeyCode::Char('L') => {
                Action::MovePlanetDetail(1)
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenPlanetMenu,
            _ => Action::Noop,
        }
    }
}

fn sort_label(sort: PlanetListSort) -> &'static str {
    match sort {
        PlanetListSort::CurrentProduction => "current production",
        PlanetListSort::Location => "location",
        PlanetListSort::PotentialProduction => "potential production",
    }
}

fn planet_status(row: &EmpirePlanetEconomyRow, fallback: &str) -> String {
    if row.is_homeworld_seed {
        return "Homeworld - fully developed".to_string();
    }
    if row.has_friendly_starbase {
        return "Regular planet - starbase present".to_string();
    }
    if fallback.starts_with("Regular planet") || fallback.starts_with("Homeworld") {
        return fallback.to_string();
    }
    "Regular planet - factories fully functional".to_string()
}
