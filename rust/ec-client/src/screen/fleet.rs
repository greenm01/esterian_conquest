use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::screen::layout::{
    draw_command_center, draw_command_line_default_input, draw_command_line_text,
    draw_command_prompt, draw_status_line, new_playfield,
    MenuEntry,
    CMD_COL_1, CMD_COL_2, CMD_COL_3,
};
use crate::screen::table::{
    fleet_id_column_width, format_fleet_number, write_table_window_with_cursor, TableColumn,
};
use crate::screen::{PlayfieldBuffer, Screen, ScreenFrame};
use crate::theme::classic;

pub const FLEET_VISIBLE_ROWS: usize = 11;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetRow {
    pub fleet_record_index_1_based: usize,
    pub fleet_number: u16,
    pub coords: [u8; 2],
    pub target_coords: [u8; 2],
    pub current_speed: u8,
    pub max_speed: u8,
    pub eta_label: String,
    pub rules_of_engagement: u8,
    pub order_label: String,
    pub composition_label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetListMode {
    Brief,
    Full,
}

pub struct FleetMenuScreen;
pub struct FleetListScreen;
pub struct FleetReviewScreen;
pub struct FleetRoeScreen;
pub struct FleetEtaScreen;
pub struct FleetDetachScreen;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetEtaMode {
    SelectingFleet,
    EnteringDestination,
    ConfirmingSystemEntry,
    ShowingResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetDetachMode {
    SelectingFleet,
    EnteringBattleships,
    EnteringCruisers,
    EnteringDestroyers,
    EnteringFullTransports,
    EnteringEmptyTransports,
    EnteringScouts,
    EnteringEtacs,
    AdjustingDonorSpeed,
    SettingNewFleetRoe,
}

const TOP_ROW: [MenuEntry<'static>; 2] = [
    MenuEntry::new(CMD_COL_2, "E", "TA Calculation"),
    MenuEntry::new(CMD_COL_3, "O", "rder a Fleet"),
];

const ROW_1: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "H", "elp on Options"),
    MenuEntry::new(CMD_COL_2, "S", "TARBASE MENU..."),
    MenuEntry::new(CMD_COL_3, "C", "hg ROE,ID,Speed"),
];

const ROW_2: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "Q", "uit: Main Menu"),
    MenuEntry::new(CMD_COL_2, "B", "rief Fleet List"),
    MenuEntry::new(CMD_COL_3, "I", "nfo about Planet"),
];

const ROW_3: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "X", "pert Mode"),
    MenuEntry::new(CMD_COL_2, "F", "ull Fleet List"),
    MenuEntry::new(CMD_COL_3, "D", "etach Ships"),
];

const ROW_4: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "V", "iew Partial Map"),
    MenuEntry::new(CMD_COL_2, "R", "eview a Fleet"),
    MenuEntry::new(CMD_COL_3, "T", "ransfer Ships"),
];

const ROW_5: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "G", "ROUP FLEET ORDER"),
    MenuEntry::new(CMD_COL_2, "M", "erge a Fleet"),
    MenuEntry::new(CMD_COL_3, "L", "oad TTs w/Armies"),
];

const ROW_6: [MenuEntry<'static>; 1] = [
    MenuEntry::new(CMD_COL_3, "U", "nload TT Armies"),
];

const BRIEF_COLUMNS: [TableColumn<'static>; 5] = [
    TableColumn::right("ID", 2),
    TableColumn::left("Location", 10),
    TableColumn::right("Spd", 7),
    TableColumn::right("ROE", 3),
    TableColumn::left("Ships", 52),
];

const FULL_COLUMNS: [TableColumn<'static>; 6] = [
    TableColumn::right("ID", 2),
    TableColumn::left("Location", 10),
    TableColumn::right("Spd", 7),
    TableColumn::right("ROE", 3),
    TableColumn::left("Order", 26),
    TableColumn::left("Ships", 26),
];

impl FleetMenuScreen {
    pub fn new() -> Self {
        Self
    }
}

impl Screen for FleetMenuScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_command_center(
            &mut buffer,
            "FLEET COMMAND CENTER:",
            &TOP_ROW,
            &[&ROW_1, &ROW_2, &ROW_3, &ROW_4, &ROW_5, &ROW_6],
            "FLEET COMMAND",
            "H,Q,X,V,S,B,F,R,E,C,I,D,T,O,G,M,L,U",
        );
        Ok(buffer)
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('b') | KeyCode::Char('B') => Action::OpenFleetList(FleetListMode::Brief),
            KeyCode::Char('f') | KeyCode::Char('F') => Action::OpenFleetList(FleetListMode::Full),
            KeyCode::Char('r') | KeyCode::Char('R') => Action::OpenFleetReview,
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenMainMenu,
            KeyCode::Char('h') | KeyCode::Char('H') => Action::OpenFleetHelp,
            KeyCode::Char('s') | KeyCode::Char('S') => Action::Noop, // Starbase menu - TODO
            KeyCode::Char('d') | KeyCode::Char('D') => Action::OpenFleetDetach,
            KeyCode::Char('m') | KeyCode::Char('M') => Action::Noop, // Merge - TODO
            KeyCode::Char('o') | KeyCode::Char('O') => Action::Noop, // Order - TODO
            KeyCode::Char('t') | KeyCode::Char('T') => Action::Noop, // Transfer - TODO
            KeyCode::Char('c') | KeyCode::Char('C') => Action::OpenFleetRoeSelect,
            KeyCode::Char('l') | KeyCode::Char('L') => Action::Noop, // Load - TODO
            KeyCode::Char('u') | KeyCode::Char('U') => Action::Noop, // Unload - TODO
            KeyCode::Char('e') | KeyCode::Char('E') => Action::OpenFleetEta,
            KeyCode::Char('v') | KeyCode::Char('V') => {
                Action::OpenPartialStarmapPrompt(crate::screen::CommandMenu::Fleet)
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                Action::OpenPlanetInfoPrompt(crate::screen::CommandMenu::Fleet)
            }
            KeyCode::Char('g') | KeyCode::Char('G') => Action::Noop, // Group order - TODO
            KeyCode::Char('x') | KeyCode::Char('X') => Action::Noop, // Expert mode - TODO
            _ => Action::Noop,
        }
    }
}

impl FleetListScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        mode: FleetListMode,
        rows: &[FleetRow],
        scroll_offset: usize,
        cursor: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        let max_fleet_number = max_fleet_number(rows);
        let brief_columns = brief_columns(max_fleet_number);
        let full_columns = full_columns(max_fleet_number);
        let title = match mode {
            FleetListMode::Brief => "BRIEF FLEET LIST:",
            FleetListMode::Full => "FULL FLEET LIST:",
        };
        draw_status_line(
            &mut buffer,
            1,
            "",
            "ENTER reviews a fleet. Use arrows or J/K to move through the list.",
        );
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(0, 0, title, classic::title_style());
        let table_rows = rows
            .iter()
            .map(|row| match mode {
                FleetListMode::Brief => vec![
                    format_fleet_number(row.fleet_number, max_fleet_number),
                    format!("({:>2},{:>2})", row.coords[0], row.coords[1]),
                    format!("{}/{}", row.current_speed, row.max_speed),
                    row.rules_of_engagement.to_string(),
                    row.composition_label.clone(),
                ],
                FleetListMode::Full => vec![
                    format_fleet_number(row.fleet_number, max_fleet_number),
                    format!("({:>2},{:>2})", row.coords[0], row.coords[1]),
                    format!("{}/{}", row.current_speed, row.max_speed),
                    row.rules_of_engagement.to_string(),
                    row.order_label.clone(),
                    row.composition_label.clone(),
                ],
            })
            .collect::<Vec<_>>();
        write_table_window_with_cursor(
            &mut buffer,
            3,
            match mode {
                FleetListMode::Brief => &brief_columns,
                FleetListMode::Full => &full_columns,
            },
            &table_rows,
            scroll_offset,
            FLEET_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
            if table_rows.is_empty() { None } else { Some(cursor) },
        );
        if table_rows.is_empty() {
            draw_status_line(&mut buffer, 17, "Notice: ", "You have no active fleets.");
        }
        draw_command_prompt(&mut buffer, 19, "FLEET COMMAND", "ARROWS J K ENTER Q");
        Ok(buffer)
    }

    pub fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => Action::MoveFleetList(-1),
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => Action::MoveFleetList(1),
            KeyCode::PageUp => Action::MoveFleetList(-8),
            KeyCode::PageDown => Action::MoveFleetList(8),
            KeyCode::Enter => Action::OpenFleetReview,
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenFleetMenu,
            _ => Action::Noop,
        }
    }
}

impl FleetReviewScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        row: &FleetRow,
        selected_index: usize,
        total: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(
            0,
            0,
            &format!("REVIEW FLEET {}/{}:", selected_index + 1, total),
            classic::title_style(),
        );
        draw_status_line(&mut buffer, 2, "Fleet ID: ", &row.fleet_number.to_string());
        draw_status_line(
            &mut buffer,
            3,
            "Location: ",
            &format!("({},{})", row.coords[0], row.coords[1]),
        );
        draw_status_line(
            &mut buffer,
            4,
            "Current / Max Speed: ",
            &format!("{}/{}", row.current_speed, row.max_speed),
        );
        draw_status_line(
            &mut buffer,
            5,
            "Rules of Engagement: ",
            &row.rules_of_engagement.to_string(),
        );
        draw_status_line(&mut buffer, 7, "Standing Order: ", &row.order_label);
        draw_status_line(&mut buffer, 9, "Composition: ", &row.composition_label);
        draw_status_line(
            &mut buffer,
            12,
            "Fleet Record #: ",
            &row.fleet_record_index_1_based.to_string(),
        );
        draw_command_prompt(&mut buffer, 19, "FLEET COMMAND", "ARROWS H J K L Q");
        Ok(buffer)
    }

    pub fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Left => Action::MoveFleetReview(-1),
            KeyCode::Down | KeyCode::Right => Action::MoveFleetReview(1),
            KeyCode::Home => Action::MoveFleetReview(i8::MIN),
            KeyCode::End => Action::MoveFleetReview(i8::MAX),
            KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Char('h') | KeyCode::Char('H') => {
                Action::MoveFleetReview(-1)
            }
            KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Char('l') | KeyCode::Char('L') => {
                Action::MoveFleetReview(1)
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenFleetMenu,
            _ => Action::Noop,
        }
    }
}

impl FleetRoeScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_select(
        &mut self,
        rows: &[FleetRow],
        scroll_offset: usize,
        cursor: usize,
        editing: bool,
        select_input: &str,
        input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(0, 0, "CHANGE FLEET ROE:", classic::title_style());
        let max_fleet_number = max_fleet_number(rows);
        let brief_columns = brief_columns(max_fleet_number);
        draw_status_line(
            &mut buffer,
            1,
            "",
            "Select a fleet, then press ENTER to change its rules of engagement.",
        );
        let table_rows = rows
            .iter()
            .map(|row| {
                vec![
                    format_fleet_number(row.fleet_number, max_fleet_number),
                    format!("({:>2},{:>2})", row.coords[0], row.coords[1]),
                    format!("{}/{}", row.current_speed, row.max_speed),
                    row.rules_of_engagement.to_string(),
                    row.composition_label.clone(),
                ]
            })
            .collect::<Vec<_>>();
        write_table_window_with_cursor(
            &mut buffer,
            3,
            &brief_columns,
            &table_rows,
            scroll_offset,
            FLEET_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
            if table_rows.is_empty() { None } else { Some(cursor) },
        );
        if table_rows.is_empty() {
            draw_command_line_text(&mut buffer, "FLEET COMMAND", "You have no active fleets. Q quits.");
        } else if editing && status.is_some() {
            draw_command_line_text(
                &mut buffer,
                "FLEET COMMAND",
                status.unwrap_or(""),
            );
        } else if editing {
            let row = &rows[cursor];
            draw_command_line_default_input(
                &mut buffer,
                "FLEET COMMAND",
                &format!("Fleet #{} new ROE ", format_fleet_number(row.fleet_number, max_fleet_number)),
                &row.rules_of_engagement.to_string(),
                input,
            );
        } else if let Some(status) = status {
            draw_command_line_text(&mut buffer, "FLEET COMMAND", status);
        } else {
            draw_command_line_default_input(
                &mut buffer,
                "FLEET COMMAND",
                "Fleet # ",
                &format_fleet_number(rows[cursor].fleet_number, max_fleet_number),
                select_input,
            );
        }
        Ok(buffer)
    }

    pub fn handle_select_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => Action::MoveFleetRoeSelect(-1),
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => Action::MoveFleetRoeSelect(1),
            KeyCode::PageUp => Action::MoveFleetRoeSelect(-8),
            KeyCode::PageDown => Action::MoveFleetRoeSelect(8),
            KeyCode::Enter => Action::Noop,
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenFleetMenu,
            _ => Action::Noop,
        }
    }
}

impl FleetEtaScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        rows: &[FleetRow],
        scroll_offset: usize,
        cursor: usize,
        mode: FleetEtaMode,
        select_input: &str,
        destination_default: [u8; 2],
        destination_input: &str,
        include_system_input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(0, 0, "CALCULATE FLEET ETA:", classic::title_style());
        let max_fleet_number = max_fleet_number(rows);
        let brief_columns = brief_columns(max_fleet_number);
        draw_status_line(
            &mut buffer,
            1,
            "",
            "Select a fleet, then enter a destination to calculate arrival time.",
        );
        let table_rows = rows
            .iter()
            .map(|row| {
                vec![
                    format_fleet_number(row.fleet_number, max_fleet_number),
                    format!("({:>2},{:>2})", row.coords[0], row.coords[1]),
                    format!("{}/{}", row.current_speed, row.max_speed),
                    row.rules_of_engagement.to_string(),
                    row.composition_label.clone(),
                ]
            })
            .collect::<Vec<_>>();
        write_table_window_with_cursor(
            &mut buffer,
            3,
            &brief_columns,
            &table_rows,
            scroll_offset,
            FLEET_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
            if table_rows.is_empty() { None } else { Some(cursor) },
        );
        if table_rows.is_empty() {
            draw_command_line_text(&mut buffer, "FLEET COMMAND", "You have no active fleets. Q quits.");
            return Ok(buffer);
        }
        match mode {
            FleetEtaMode::SelectingFleet => {
                if let Some(status) = status {
                    draw_command_line_text(&mut buffer, "FLEET COMMAND", status);
                } else {
                    draw_command_line_default_input(
                        &mut buffer,
                        "FLEET COMMAND",
                        "Calculate time for fleet # ",
                        &format_fleet_number(rows[cursor].fleet_number, max_fleet_number),
                        select_input,
                    );
                }
            }
            FleetEtaMode::EnteringDestination => {
                draw_command_line_default_input(
                    &mut buffer,
                    "FLEET COMMAND",
                    "Destination ",
                    &format!("{},{}", destination_default[0], destination_default[1]),
                    destination_input,
                );
            }
            FleetEtaMode::ConfirmingSystemEntry => {
                draw_command_line_default_input(
                    &mut buffer,
                    "FLEET COMMAND",
                    "Include time to enter system? ",
                    "N",
                    include_system_input,
                );
            }
            FleetEtaMode::ShowingResult => {
                draw_command_line_text(
                    &mut buffer,
                    "FLEET COMMAND",
                    status.unwrap_or("Press ENTER to continue."),
                );
            }
        }
        Ok(buffer)
    }
}

impl FleetDetachScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        rows: &[FleetRow],
        scroll_offset: usize,
        cursor: usize,
        prompt: &str,
        default: &str,
        input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(0, 0, "DETACH FLEET SHIPS:", classic::title_style());
        let max_fleet_number = max_fleet_number(rows);
        let brief_columns = brief_columns(max_fleet_number);
        draw_status_line(
            &mut buffer,
            1,
            "",
            "Select a fleet, then detach ships to create a new fleet.",
        );
        let table_rows = rows
            .iter()
            .map(|row| {
                vec![
                    format_fleet_number(row.fleet_number, max_fleet_number),
                    format!("({:>2},{:>2})", row.coords[0], row.coords[1]),
                    format!("{}/{}", row.current_speed, row.max_speed),
                    row.rules_of_engagement.to_string(),
                    row.composition_label.clone(),
                ]
            })
            .collect::<Vec<_>>();
        write_table_window_with_cursor(
            &mut buffer,
            3,
            &brief_columns,
            &table_rows,
            scroll_offset,
            FLEET_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
            if table_rows.is_empty() { None } else { Some(cursor) },
        );
        if table_rows.is_empty() {
            draw_command_line_text(&mut buffer, "FLEET COMMAND", "You have no active fleets. Q quits.");
        } else if let Some(status) = status {
            draw_command_line_text(&mut buffer, "FLEET COMMAND", status);
        } else {
            draw_command_line_default_input(
                &mut buffer,
                "FLEET COMMAND",
                prompt,
                default,
                input,
            );
        }
        Ok(buffer)
    }
}

fn max_fleet_number(rows: &[FleetRow]) -> u16 {
    rows.iter().map(|row| row.fleet_number).max().unwrap_or(1)
}

fn brief_columns(max_fleet_number: u16) -> [TableColumn<'static>; 5] {
    [
        TableColumn::right("ID", fleet_id_column_width(max_fleet_number)),
        BRIEF_COLUMNS[1],
        BRIEF_COLUMNS[2],
        BRIEF_COLUMNS[3],
        BRIEF_COLUMNS[4],
    ]
}

fn full_columns(max_fleet_number: u16) -> [TableColumn<'static>; 6] {
    [
        TableColumn::right("ID", fleet_id_column_width(max_fleet_number)),
        FULL_COLUMNS[1],
        FULL_COLUMNS[2],
        FULL_COLUMNS[3],
        FULL_COLUMNS[4],
        FULL_COLUMNS[5],
    ]
}
