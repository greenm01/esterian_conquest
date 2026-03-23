use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::domains::fleet::FleetAction;
use crate::domains::planet::PlanetAction;
use crate::domains::starbase::StarbaseAction;
use crate::domains::starmap::StarmapAction;
use crate::screen::layout::{
    MenuEntry, draw_command_line_default_input, draw_command_prompt, draw_help_panel,
    draw_menu_entry, draw_status_line, draw_title_bar, new_playfield,
};
use crate::screen::table::{TableColumn, write_table_window_with_cursor};
use crate::screen::{PlayfieldBuffer, Screen, ScreenFrame, format_sector_coords_padded};
use crate::theme::classic;

pub const STARBASE_VISIBLE_ROWS: usize = 19;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StarbaseRow {
    pub base_record_index_1_based: usize,
    pub base_id: u8,
    pub escort_label: String,
    pub coords: [u8; 2],
    pub destination_coords: [u8; 2],
    pub eta_label: String,
    pub operation_label: String,
}

pub struct StarbaseMenuScreen;
pub struct StarbaseHelpScreen;
pub struct StarbaseListScreen;
pub struct StarbaseReviewScreen;

const TOP_ROW: [MenuEntry<'static>; 2] = [
    MenuEntry::new(29, "X", "pert mode ON/OFF"),
    MenuEntry::new(52, "V", "iew Partial Star Map"),
];

const ROW_1: [MenuEntry<'static>; 3] = [
    MenuEntry::new(2, "H", "elp with commands"),
    MenuEntry::new(29, "S", "tarbases-List"),
    MenuEntry::new(52, "I", "nfo about a Planet"),
];

const ROW_2: [MenuEntry<'static>; 3] = [
    MenuEntry::new(2, "Q", "uit to Fleet Command"),
    MenuEntry::new(29, "R", "eview a Starbase"),
    MenuEntry::new(52, "M", "ove/Halt Starbase"),
];

const STARBASE_COLUMNS: [TableColumn<'static>; 6] = [
    TableColumn::right("ID", 2),
    TableColumn::left("Escort/Guard", 16),
    TableColumn::left("Location", 14),
    TableColumn::left("Destination", 14),
    TableColumn::right("ETA", 4),
    TableColumn::left("Present Operation", 23),
];

impl StarbaseMenuScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_with_notice(
        &mut self,
        notice: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "STARBASE CONTROL:");
        for entry in TOP_ROW {
            draw_menu_entry(&mut buffer, 0, entry.col, entry.hotkey, entry.label);
        }
        for (row_idx, row) in [ROW_1.as_slice(), ROW_2.as_slice()].into_iter().enumerate() {
            buffer.fill_row(row_idx + 1, classic::menu_style());
            for entry in row {
                draw_menu_entry(
                    &mut buffer,
                    row_idx + 1,
                    entry.col,
                    entry.hotkey,
                    entry.label,
                );
            }
        }
        if let Some(notice) = notice {
            draw_status_line(&mut buffer, 16, "Notice: ", notice);
        }
        draw_command_prompt(&mut buffer, 19, "STARBASE COMMAND", "H,Q,X,S,R,V,I,M");
        Ok(buffer)
    }
}

impl Screen for StarbaseMenuScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_with_notice(None)
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('h') | KeyCode::Char('H') => Action::Starbase(StarbaseAction::OpenHelp),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Fleet(FleetAction::OpenMenu)
            }
            KeyCode::Char('x') | KeyCode::Char('X') => {
                Action::Starbase(StarbaseAction::ShowExpertModeNotice)
            }
            KeyCode::Char('s') | KeyCode::Char('S') => Action::Starbase(StarbaseAction::OpenList),
            KeyCode::Char('r') | KeyCode::Char('R') => {
                Action::Starbase(StarbaseAction::OpenReviewSelect)
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                Action::Starbase(StarbaseAction::ShowMoveNotice)
            }
            KeyCode::Char('v') | KeyCode::Char('V') => Action::Starmap(
                StarmapAction::OpenPartialPrompt(crate::screen::CommandMenu::Starbase),
            ),
            KeyCode::Char('i') | KeyCode::Char('I') => Action::Planet(
                PlanetAction::OpenInfoPrompt(crate::screen::CommandMenu::Starbase),
            ),
            _ => Action::Noop,
        }
    }
}

impl StarbaseHelpScreen {
    pub fn new() -> Self {
        Self
    }
}

impl Screen for StarbaseHelpScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        let lines = [
            "<H> - describe Starbase Control commands",
            "<I> - show Intelligence on what you know about any planet",
            "<M> - order a starbase to move (ie, to be hauled) to a new location",
            "<Q> - quit the Starbase Control menu & returns you to the Fleet Command Center",
            "<R> - display all game information regarding a specified starbase",
            "<S> - display all of your starbases with their locations, destinations etc.",
            "<V> - display a portion of the map (goto GENERAL MENU for entire map)",
            "<X> - hide/show menus",
        ];
        draw_help_panel(
            &mut buffer,
            "STARBASE HELP:",
            "Help - Starbase Control command descriptions:",
            &lines,
            "STARBASE COMMAND",
        );
        Ok(buffer)
    }

    fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::Starbase(StarbaseAction::OpenMenu)
    }
}

impl StarbaseListScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        rows: &[StarbaseRow],
        scroll_offset: usize,
        cursor: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "STARBASE LIST:");
        draw_status_line(
            &mut buffer,
            1,
            "",
            "Use arrows or J/K to move through the list. ENTER reviews a starbase.",
        );
        let table_rows = rows
            .iter()
            .map(|row| {
                vec![
                    row.base_id.to_string(),
                    row.escort_label.clone(),
                    format!("System{}", format_sector_coords_padded(row.coords)),
                    format!(
                        "System{}",
                        format_sector_coords_padded(row.destination_coords)
                    ),
                    row.eta_label.clone(),
                    starbase_list_operation_label(&row.operation_label),
                ]
            })
            .collect::<Vec<_>>();
        write_table_window_with_cursor(
            &mut buffer,
            3,
            &STARBASE_COLUMNS,
            &table_rows,
            scroll_offset,
            STARBASE_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
            if table_rows.is_empty() {
                None
            } else {
                Some(cursor)
            },
        );
        draw_command_prompt(&mut buffer, 19, "STARBASE COMMAND", "ARROWS J K ENTER Q");
        Ok(buffer)
    }

    pub fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::Starbase(StarbaseAction::MoveSelect(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::Starbase(StarbaseAction::MoveSelect(1))
            }
            KeyCode::PageUp => Action::Starbase(StarbaseAction::MoveSelect(-8)),
            KeyCode::PageDown => Action::Starbase(StarbaseAction::MoveSelect(8)),
            KeyCode::Enter => Action::Starbase(StarbaseAction::OpenReview),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Starbase(StarbaseAction::OpenMenu)
            }
            _ => Action::Noop,
        }
    }
}

impl StarbaseReviewScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_select(
        &mut self,
        rows: &[StarbaseRow],
        scroll_offset: usize,
        cursor: usize,
        input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "REVIEW A STARBASE:");
        draw_status_line(
            &mut buffer,
            1,
            "",
            "Select a starbase, then press ENTER to review it.",
        );
        let table_rows = rows
            .iter()
            .map(|row| {
                vec![
                    row.base_id.to_string(),
                    row.escort_label.clone(),
                    format!("System{}", format_sector_coords_padded(row.coords)),
                    format!(
                        "System{}",
                        format_sector_coords_padded(row.destination_coords)
                    ),
                    row.eta_label.clone(),
                    starbase_list_operation_label(&row.operation_label),
                ]
            })
            .collect::<Vec<_>>();
        write_table_window_with_cursor(
            &mut buffer,
            3,
            &STARBASE_COLUMNS,
            &table_rows,
            scroll_offset,
            STARBASE_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
            if table_rows.is_empty() {
                None
            } else {
                Some(cursor)
            },
        );
        if let Some(status) = status {
            draw_status_line(&mut buffer, 17, "Notice: ", status);
            draw_command_prompt(&mut buffer, 19, "STARBASE COMMAND", "ARROWS J K ENTER Q");
        } else {
            let default_base = rows
                .get(cursor)
                .map(|row| row.base_id.to_string())
                .unwrap_or_else(|| "1".to_string());
            draw_command_line_default_input(
                &mut buffer,
                "STARBASE COMMAND",
                "Starbase # ",
                &default_base,
                input,
            );
        }
        Ok(buffer)
    }

    pub fn render_detail(
        &mut self,
        row: &StarbaseRow,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, &format!("REVIEW STARBASE {}:", row.base_id));
        draw_status_line(
            &mut buffer,
            3,
            "Starbase ID: ",
            &format!("Starbase {}", row.base_id),
        );
        draw_status_line(
            &mut buffer,
            4,
            "Location:    ",
            &format!(
                "World in Solar System {}",
                format_sector_coords_padded(row.coords)
            ),
        );
        draw_status_line(
            &mut buffer,
            5,
            "Destination: ",
            &format!(
                "World in Solar System {}",
                format_sector_coords_padded(row.destination_coords)
            ),
        );
        draw_status_line(&mut buffer, 6, "Operation:   ", &row.operation_label);
        let eta_text = if row.coords == row.destination_coords {
            format!(
                "Starbase {} has already arrived and is in operation.",
                row.base_id
            )
        } else {
            format!(
                "Starbase {} is in transit with ETA {} years.",
                row.base_id, row.eta_label
            )
        };
        draw_status_line(&mut buffer, 7, "ETA:         ", &eta_text);
        draw_status_line(&mut buffer, 8, "Escort:      ", &row.escort_label);
        buffer.write_text(10, 0, &"-".repeat(79), classic::help_panel_style());
        draw_command_prompt(&mut buffer, 19, "STARBASE COMMAND", "SLAP A KEY");
        Ok(buffer)
    }
}

fn starbase_list_operation_label(operation: &str) -> String {
    match operation {
        "Protection & Enhancement" => "Protect & Enhance".to_string(),
        other => other.to_string(),
    }
}

impl Screen for StarbaseReviewScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        Err("use starbase review render helpers".into())
    }

    fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::Noop
    }
}
