use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::screen::layout::{
    draw_command_center, draw_command_prompt, draw_status_line, new_playfield, MenuEntry,
    CMD_COL_1, CMD_COL_2, CMD_COL_3,
};
use crate::screen::table::{format_empire_id, write_table_window_with_cursor, TableColumn};
use crate::screen::{PlayfieldBuffer, Screen, ScreenFrame};
use crate::theme::classic;

pub const FLEET_VISIBLE_ROWS: usize = 11;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetRow {
    pub fleet_record_index_1_based: usize,
    pub fleet_id: u8,
    pub coords: [u8; 2],
    pub current_speed: u8,
    pub max_speed: u8,
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

const TOP_ROW: [MenuEntry<'static>; 2] = [
    MenuEntry::new(CMD_COL_2, "B", "rief List of Fleets"),
    MenuEntry::new(CMD_COL_3, "E", "TA calculation"),
];

const ROW_1: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "H", "elp with commands"),
    MenuEntry::new(CMD_COL_2, "F", "leet-Detailed List"),
    MenuEntry::new(CMD_COL_3, "D", "etach a Fleet"),
];

const ROW_2: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "Q", "uit to main menu"),
    MenuEntry::new(CMD_COL_2, "R", "eview a Fleet"),
    MenuEntry::new(CMD_COL_3, "M", "erge a Fleet"),
];

const ROW_3: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "X", "pert mode ON/OFF"),
    MenuEntry::new(CMD_COL_2, "O", "rder fleet on mission"),
    MenuEntry::new(CMD_COL_3, "T", "ransfer (reassign) ships"),
];

const ROW_4: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "S", "tarbase INFO..."),
    MenuEntry::new(CMD_COL_2, "C", "hange a fleet's ROE"),
    MenuEntry::new(CMD_COL_3, "L", "oad Armies to Transports"),
];

const ROW_5: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "V", "iew partial Starmap"),
    MenuEntry::new(CMD_COL_2, "A", "lter a fleet's ID"),
    MenuEntry::new(CMD_COL_3, "U", "nload Armies from Transport"),
];

const BRIEF_COLUMNS: [TableColumn<'static>; 5] = [
    TableColumn::right("ID", 3),
    TableColumn::left("Location", 10),
    TableColumn::right("Spd", 7),
    TableColumn::right("ROE", 3),
    TableColumn::left("Ships", 52),
];

const FULL_COLUMNS: [TableColumn<'static>; 6] = [
    TableColumn::right("ID", 3),
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
            &[&ROW_1, &ROW_2, &ROW_3, &ROW_4, &ROW_5],
            "FLEET COMMAND",
            "H Q X S V B F R O C A E D M T L U",
        );
        Ok(buffer)
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('b') | KeyCode::Char('B') => Action::OpenFleetList(FleetListMode::Brief),
            KeyCode::Char('f') | KeyCode::Char('F') => Action::OpenFleetList(FleetListMode::Full),
            KeyCode::Char('r') | KeyCode::Char('R') => Action::OpenFleetReview,
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenMainMenu,
            KeyCode::Char('s') | KeyCode::Char('S') => Action::Noop, // Starbase INFO - TODO
            KeyCode::Char('d') | KeyCode::Char('D') => Action::Noop, // Detach - TODO
            KeyCode::Char('m') | KeyCode::Char('M') => Action::Noop, // Merge - TODO
            KeyCode::Char('o') | KeyCode::Char('O') => Action::Noop, // Order - TODO
            KeyCode::Char('t') | KeyCode::Char('T') => Action::Noop, // Transfer - TODO
            KeyCode::Char('c') | KeyCode::Char('C') => Action::Noop, // Change ROE - TODO
            KeyCode::Char('l') | KeyCode::Char('L') => Action::Noop, // Load - TODO
            KeyCode::Char('a') | KeyCode::Char('A') => Action::Noop, // Alter ID - TODO
            KeyCode::Char('u') | KeyCode::Char('U') => Action::Noop, // Unload - TODO
            KeyCode::Char('e') | KeyCode::Char('E') => Action::Noop, // ETA - TODO
            KeyCode::Char('v') | KeyCode::Char('V') => Action::Noop, // View map - TODO
            KeyCode::Char('h') | KeyCode::Char('H') => Action::Noop, // Help - TODO
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
                    format_empire_id(row.fleet_id),
                    format!("({:>2},{:>2})", row.coords[0], row.coords[1]),
                    format!("{}/{}", row.current_speed, row.max_speed),
                    row.rules_of_engagement.to_string(),
                    row.composition_label.clone(),
                ],
                FleetListMode::Full => vec![
                    format_empire_id(row.fleet_id),
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
                FleetListMode::Brief => &BRIEF_COLUMNS,
                FleetListMode::Full => &FULL_COLUMNS,
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
        draw_status_line(&mut buffer, 2, "Fleet ID: ", &format_empire_id(row.fleet_id));
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
