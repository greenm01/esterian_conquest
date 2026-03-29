use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::domains::planet::PlanetAction;
use crate::screen::layout::{
    ScreenGeometry, dismiss_prompt_row, draw_command_line_default_input_at,
    draw_dismiss_prompt, draw_prompt_error_after, draw_title_bar, menu_prompt_row, new_playfield,
    new_playfield_for, standard_table_visible_rows_for,
};
use crate::screen::table::{
    TableColumn, TableFooter, draw_table_footer, draw_table_title, fleet_id_column_width,
    format_fleet_number, write_table_window_with_cursor,
};
use crate::screen::{PlayfieldBuffer, Screen, format_sector_coords, format_sector_coords_table};
use crate::theme::classic;

pub struct PlanetTransportScreen;

pub fn planet_transport_visible_rows(geometry: ScreenGeometry) -> usize {
    standard_table_visible_rows_for(geometry, 1)
}

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
        geometry: ScreenGeometry,
        prompt_label: &str,
        mode: PlanetTransportMode,
        rows: &[PlanetTransportPlanetRow],
        scroll_offset: usize,
        cursor: usize,
        input: &str,
        default_coords: [u8; 2],
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield_for(geometry);
        draw_table_title(&mut buffer, 1, 0, mode.title());
        let table_rows = rows
            .iter()
            .map(|row| {
                vec![
                    row.planet_name.clone(),
                    format_sector_coords_table(row.coords),
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
        let metrics = write_table_window_with_cursor(
            &mut buffer,
            1,
            &PLANET_COLUMNS,
            &table_rows,
            scroll_offset,
            planet_transport_visible_rows(geometry),
            classic::status_value_style(),
            classic::status_value_style(),
            selected,
            0,
        );
        if table_rows.is_empty() {
            draw_table_footer(
                &mut buffer,
                geometry,
                0,
                metrics.bottom_row,
                TableFooter::CommandText {
                    label: prompt_label,
                    text: "No eligible planets remain. Q quits.",
                },
            );
        } else {
            draw_table_footer(
                &mut buffer,
                geometry,
                0,
                metrics.bottom_row,
                TableFooter::CommandInput {
                    label: prompt_label,
                    prompt: "",
                    default: &format!("{},{}", default_coords[0], default_coords[1]),
                    input,
                },
            );
        }
        let _ = status;
        Ok(buffer)
    }

    pub fn render_fleet_select(
        &mut self,
        geometry: ScreenGeometry,
        prompt_label: &str,
        mode: PlanetTransportMode,
        planet: &PlanetTransportPlanetRow,
        fleets: &[PlanetTransportFleetRow],
        scroll_offset: usize,
        cursor: usize,
        input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield_for(geometry);
        draw_table_title(&mut buffer, 1, 0, mode.title());
        let max_fleet_number = max_fleet_number(fleets);
        let fleet_columns = fleet_columns(max_fleet_number);
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
        let metrics = write_table_window_with_cursor(
            &mut buffer,
            1,
            &fleet_columns,
            &table_rows,
            scroll_offset,
            planet_transport_visible_rows(geometry),
            classic::status_value_style(),
            classic::status_value_style(),
            selected,
            0,
        );
        let max_qty = fleets.get(cursor).map(|row| row.available_qty).unwrap_or(0);
        if table_rows.is_empty() {
            draw_table_footer(
                &mut buffer,
                geometry,
                0,
                metrics.bottom_row,
                TableFooter::CommandText {
                    label: prompt_label,
                    text: "No eligible fleets remain here. Q quits.",
                },
            );
        } else {
            let default_qty = max_qty.to_string();
            let prompt = format!("How many armies to {}? ", mode.verb());
            draw_table_footer(
                &mut buffer,
                geometry,
                0,
                metrics.bottom_row,
                TableFooter::CommandInput {
                    label: prompt_label,
                    prompt: &prompt,
                    default: &default_qty,
                    input,
                },
            );
        }
        let _ = (planet, status);
        Ok(buffer)
    }

    pub fn render_quantity_prompt(
        &mut self,
        prompt_label: &str,
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
                "Planet: {} {}   Fleet {:02}",
                planet.planet_name,
                format_sector_coords(planet.coords),
                fleet.fleet_number
            ),
            classic::status_value_style(),
        );
        let command_row = menu_prompt_row(2);
        draw_command_line_default_input_at(
            &mut buffer,
            command_row,
            prompt_label,
            &format!("How many armies to {}? ", mode.verb()),
            &fleet.available_qty.to_string(),
            input,
        );
        if let Some(status) = status {
            draw_prompt_error_after(&mut buffer, command_row, status);
        }
        Ok(buffer)
    }

    pub fn render_done(
        &mut self,
        prompt_label: &str,
        mode: PlanetTransportMode,
        status: &str,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, mode.title());
        buffer.write_text(3, 0, status, classic::status_value_style());
        let _ = prompt_label;
        draw_dismiss_prompt(&mut buffer, dismiss_prompt_row(3));
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
                Action::Planet(PlanetAction::MoveTransportPlanet(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::Planet(PlanetAction::MoveTransportPlanet(1))
            }
            KeyCode::PageUp => Action::Planet(PlanetAction::MoveTransportPlanet(-8)),
            KeyCode::PageDown => Action::Planet(PlanetAction::MoveTransportPlanet(8)),
            KeyCode::Enter => Action::Planet(PlanetAction::SubmitTransportPlanet),
            KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceTransportPlanetInput),
            KeyCode::Char(ch) if ch.is_ascii_digit() || matches!(ch, ',' | '[' | ']' | ' ') => {
                Action::Planet(PlanetAction::AppendTransportPlanetChar(ch))
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::ReturnToCommandMenu,
            _ => Action::Noop,
        }
    }

    pub fn handle_fleet_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::Planet(PlanetAction::MoveTransportFleet(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::Planet(PlanetAction::MoveTransportFleet(1))
            }
            KeyCode::PageUp => Action::Planet(PlanetAction::MoveTransportFleet(-8)),
            KeyCode::PageDown => Action::Planet(PlanetAction::MoveTransportFleet(8)),
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                Action::Planet(PlanetAction::AppendTransportQtyChar(ch))
            }
            KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceTransportQty),
            KeyCode::Enter => Action::Planet(PlanetAction::SubmitTransportQty),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::ReturnToCommandMenu,
            _ => Action::Noop,
        }
    }

    pub fn handle_quantity_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                Action::Planet(PlanetAction::AppendTransportQtyChar(ch))
            }
            KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceTransportQty),
            KeyCode::Enter => Action::Planet(PlanetAction::SubmitTransportQty),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::ReturnToCommandMenu,
            _ => Action::Noop,
        }
    }
}
