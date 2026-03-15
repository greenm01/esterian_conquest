use crossterm::event::{KeyCode, KeyEvent};
use std::collections::BTreeSet;

use crate::app::Action;
use crate::screen::layout::{draw_command_prompt, draw_status_line, draw_title_bar, new_playfield};
use crate::screen::table::{TableColumn, write_table_window_with_cursor};
use crate::screen::{PlayfieldBuffer, Screen, ScreenFrame};
use crate::theme::classic;

pub struct PlanetCommissionScreen;

pub(crate) const PLANET_COMMISSION_VISIBLE_ROWS: usize = 10;

const COMMISSION_COLUMNS: [TableColumn<'static>; 4] = [
    TableColumn::right("#", 2),
    TableColumn::left("Sel", 3),
    TableColumn::left("Unit", 24),
    TableColumn::right("Qty", 4),
];

#[derive(Debug, Clone)]
pub struct PlanetCommissionRow {
    pub slot_0_based: usize,
    pub unit_label: String,
    pub qty: u32,
}

#[derive(Debug, Clone)]
pub struct PlanetCommissionView {
    pub planet_name: String,
    pub coords: [u8; 2],
    pub rows: Vec<PlanetCommissionRow>,
}

impl PlanetCommissionScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_menu(
        &mut self,
        view: &PlanetCommissionView,
        scroll_offset: usize,
        cursor: usize,
        selected_slots: &BTreeSet<usize>,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(
            &mut buffer,
            0,
            &format!(
                "COMMISSION SHIPS: \"{}\" IN SYSTEM ({},{}):",
                view.planet_name, view.coords[0], view.coords[1]
            ),
        );
        buffer.write_text(
            2,
            0,
            "UP/DOWN or J/K nav rows.  H/L or LEFT/RIGHT change planet.",
            classic::status_value_style(),
        );
        buffer.write_text(
            3,
            0,
            "SPACE selects rows.  ENTER commissions the current selection.",
            classic::status_value_style(),
        );

        let table_rows: Vec<Vec<String>> = view
            .rows
            .iter()
            .map(|row| {
                vec![
                    (row.slot_0_based + 1).to_string(),
                    if selected_slots.contains(&row.slot_0_based) {
                        "X".to_string()
                    } else {
                        "".to_string()
                    },
                    row.unit_label.clone(),
                    row.qty.to_string(),
                ]
            })
            .collect();

        let selected = if view.rows.is_empty() {
            None
        } else {
            Some(cursor)
        };
        write_table_window_with_cursor(
            &mut buffer,
            5,
            &COMMISSION_COLUMNS,
            &table_rows,
            scroll_offset,
            PLANET_COMMISSION_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
            selected,
        );

        if view.rows.is_empty() {
            buffer.write_text(
                7,
                0,
                "This planet has no units waiting in stardock.",
                classic::status_value_style(),
            );
        }

        if let Some(status) = status {
            draw_status_line(&mut buffer, 17, "", status);
        }
        draw_command_prompt(
            &mut buffer,
            19,
            "PLANET COMMAND",
            "J K H L SPACE ENTER ARROWS Q",
        );
        Ok(buffer)
    }
}

impl Screen for PlanetCommissionScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        Ok(new_playfield())
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::MovePlanetCommissionRow(-1)
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::MovePlanetCommissionRow(1)
            }
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => {
                Action::MovePlanetCommissionPlanet(-1)
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
                Action::MovePlanetCommissionPlanet(1)
            }
            KeyCode::PageUp => Action::MovePlanetCommissionRow(-8),
            KeyCode::PageDown => Action::MovePlanetCommissionRow(8),
            KeyCode::Char(' ') => Action::TogglePlanetCommissionSelection,
            KeyCode::Enter => Action::CommissionPlanetStardockSelection,
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenPlanetMenu,
            _ => Action::Noop,
        }
    }
}
