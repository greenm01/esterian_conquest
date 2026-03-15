use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::screen::layout::{
    draw_command_line_default_input, draw_command_line_text, draw_command_prompt, draw_title_bar,
    new_playfield,
};
use crate::screen::table::{
    TableColumn, fleet_id_column_width, format_fleet_number, write_table_window_with_cursor,
};
use crate::screen::{PlayfieldBuffer, Screen};
use crate::theme::classic;

pub struct PlanetTransportScreen;

pub const PLANET_TRANSPORT_VISIBLE_ROWS: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetTransportMode {
    Load,
    Unload,
}

impl PlanetTransportMode {
    pub fn title(self) -> &'static str {
        match self {
            Self::Load => "LOAD ARMIES ONTO TROOP TRANSPORTS:",
            Self::Unload => "UNLOAD ARMIES FROM TROOP TRANSPORTS:",
        }
    }

    pub fn verb(self) -> &'static str {
        match self {
            Self::Load => "load",
            Self::Unload => "unload",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlanetTransportPlanetRow {
    pub planet_record_index_1_based: usize,
    pub planet_name: String,
    pub coords: [u8; 2],
    pub planet_armies: u8,
    pub transport_capacity: u16,
}

#[derive(Debug, Clone)]
pub struct PlanetTransportFleetRow {
    pub fleet_record_index_1_based: usize,
    pub fleet_number: u16,
    pub troop_transports: u16,
    pub loaded_armies: u16,
    pub available_qty: u16,
}

const PLANET_COLUMNS: [TableColumn<'static>; 4] = [
    TableColumn::left("Planet Name", 20),
    TableColumn::left("Location", 10),
    TableColumn::right("Armies", 6),
    TableColumn::right("Avail", 6),
];

const FLEET_COLUMNS: [TableColumn<'static>; 4] = [
    TableColumn::right("ID", 2),
    TableColumn::right("TTs", 4),
    TableColumn::right("Loaded", 6),
    TableColumn::right("Avail", 6),
];

impl PlanetTransportScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_planet_select(
        &mut self,
        mode: PlanetTransportMode,
        rows: &[PlanetTransportPlanetRow],
        scroll_offset: usize,
        cursor: usize,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, mode.title());
        buffer.write_text(
            2,
            0,
            &format!(
                "Select a planet, then press ENTER to {} armies.",
                mode.verb()
            ),
            classic::status_value_style(),
        );
        let table_rows = rows
            .iter()
            .map(|row| {
                vec![
                    row.planet_name.clone(),
                    format!("({:>2},{:>2})", row.coords[0], row.coords[1]),
                    row.planet_armies.to_string(),
                    row.transport_capacity.to_string(),
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
            &PLANET_COLUMNS,
            &table_rows,
            scroll_offset,
            PLANET_TRANSPORT_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
            selected,
        );
        if let Some(status) = status {
            buffer.write_text(17, 0, status, classic::status_value_style());
        }
        let prompt_keys = if table_rows.is_empty() {
            "Q"
        } else {
            "ARROWS J K ENTER Q"
        };
        draw_command_prompt(&mut buffer, 19, "PLANET COMMAND", prompt_keys);
        Ok(buffer)
    }

    pub fn render_fleet_select(
        &mut self,
        mode: PlanetTransportMode,
        planet: &PlanetTransportPlanetRow,
        fleets: &[PlanetTransportFleetRow],
        scroll_offset: usize,
        cursor: usize,
        input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, mode.title());
        let max_fleet_number = max_fleet_number(fleets);
        let fleet_columns = fleet_columns(max_fleet_number);
        buffer.write_text(
            2,
            0,
            &format!(
                "Select a fleet at {} ({},{}), then press ENTER.",
                planet.planet_name, planet.coords[0], planet.coords[1]
            ),
            classic::status_value_style(),
        );
        let table_rows = fleets
            .iter()
            .map(|row| {
                vec![
                    format_fleet_number(row.fleet_number, max_fleet_number),
                    row.troop_transports.to_string(),
                    row.loaded_armies.to_string(),
                    row.available_qty.to_string(),
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
            &fleet_columns,
            &table_rows,
            scroll_offset,
            PLANET_TRANSPORT_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
            selected,
        );
        let max_qty = fleets.get(cursor).map(|row| row.available_qty).unwrap_or(0);
        if table_rows.is_empty() {
            draw_command_line_text(
                &mut buffer,
                "PLANET COMMAND",
                "No eligible fleets remain here. Q quits.",
            );
        } else if let Some(status) = status {
            draw_command_line_text(&mut buffer, "PLANET COMMAND", status);
        } else {
            draw_command_line_default_input(
                &mut buffer,
                "PLANET COMMAND",
                &format!("How many armies to {}? ", mode.verb()),
                &max_qty.to_string(),
                input,
            );
        }
        Ok(buffer)
    }

    pub fn render_quantity_prompt(
        &mut self,
        mode: PlanetTransportMode,
        planet: &PlanetTransportPlanetRow,
        fleet: &PlanetTransportFleetRow,
        input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, mode.title());
        buffer.write_text(
            2,
            0,
            &format!(
                "Planet: {} ({},{})   Fleet {:02}",
                planet.planet_name, planet.coords[0], planet.coords[1], fleet.fleet_number
            ),
            classic::status_value_style(),
        );
        if let Some(status) = status {
            buffer.write_text(6, 0, status, classic::status_value_style());
        }
        draw_command_line_default_input(
            &mut buffer,
            "PLANET COMMAND",
            &format!("How many armies to {}? ", mode.verb()),
            &fleet.available_qty.to_string(),
            input,
        );
        Ok(buffer)
    }

    pub fn render_done(
        &mut self,
        mode: PlanetTransportMode,
        status: &str,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, mode.title());
        buffer.write_text(3, 0, status, classic::status_value_style());
        draw_command_prompt(&mut buffer, 19, "PLANET COMMAND", "SLAP A KEY");
        Ok(buffer)
    }
}

fn max_fleet_number(rows: &[PlanetTransportFleetRow]) -> u16 {
    rows.iter().map(|row| row.fleet_number).max().unwrap_or(1)
}

fn fleet_columns(max_fleet_number: u16) -> [TableColumn<'static>; 4] {
    [
        TableColumn::right("ID", fleet_id_column_width(max_fleet_number)),
        FLEET_COLUMNS[1],
        FLEET_COLUMNS[2],
        FLEET_COLUMNS[3],
    ]
}

impl Screen for PlanetTransportScreen {
    fn render(
        &mut self,
        _frame: &crate::screen::ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        Ok(new_playfield())
    }

    fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::Noop
    }
}

impl PlanetTransportScreen {
    pub fn handle_planet_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::MovePlanetTransportPlanet(-1)
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::MovePlanetTransportPlanet(1)
            }
            KeyCode::PageUp => Action::MovePlanetTransportPlanet(-8),
            KeyCode::PageDown => Action::MovePlanetTransportPlanet(8),
            KeyCode::Enter => Action::ConfirmPlanetTransportPlanet,
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenPlanetMenu,
            _ => Action::Noop,
        }
    }

    pub fn handle_fleet_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::MovePlanetTransportFleet(-1)
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::MovePlanetTransportFleet(1)
            }
            KeyCode::PageUp => Action::MovePlanetTransportFleet(-8),
            KeyCode::PageDown => Action::MovePlanetTransportFleet(8),
            KeyCode::Char(ch) if ch.is_ascii_digit() => Action::AppendPlanetTransportQtyChar(ch),
            KeyCode::Backspace => Action::BackspacePlanetTransportQty,
            KeyCode::Enter => Action::SubmitPlanetTransportQty,
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenPlanetMenu,
            _ => Action::Noop,
        }
    }

    pub fn handle_quantity_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char(ch) if ch.is_ascii_digit() => Action::AppendPlanetTransportQtyChar(ch),
            KeyCode::Backspace => Action::BackspacePlanetTransportQty,
            KeyCode::Enter => Action::SubmitPlanetTransportQty,
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenPlanetMenu,
            _ => Action::Noop,
        }
    }
}
