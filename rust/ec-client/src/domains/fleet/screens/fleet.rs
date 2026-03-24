use crossterm::event::{KeyCode, KeyEvent};
use std::collections::BTreeSet;

use crate::app::Action;
use crate::domains::fleet::FleetAction;
use crate::domains::planet::PlanetAction;
use crate::domains::starbase::StarbaseAction;
use crate::domains::starmap::StarmapAction;
use crate::screen::layout::{
    CommandMessage, EXPERT_MENU_PROMPT_ROW, MenuEntry, draw_command_line_default_input_at,
    draw_command_line_text_at, draw_command_message_stack, draw_command_prompt_at,
    draw_expert_menu, draw_inline_planet_info_prompt, draw_inline_status_after, draw_menu_entry,
    draw_menu_notice, draw_status_line, draw_table_command_bar_at, draw_title_bar, menu_prompt_row,
    new_playfield, standard_table_visible_rows, table_prompt_row,
};
use crate::screen::table::{
    TableColumn, TableRowState, fleet_id_column_width, format_fleet_number,
    write_table_window_with_cursor, write_table_window_with_states,
};
use crate::screen::{
    PlayfieldBuffer, Screen, ScreenFrame, format_sector_coords, format_sector_coords_padded,
};
use crate::theme::classic;

pub const FLEET_VISIBLE_ROWS: usize = standard_table_visible_rows(3);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetRow {
    pub fleet_record_index_1_based: usize,
    pub fleet_number: u16,
    pub coords: [u8; 2],
    pub target_coords: [u8; 2],
    pub order_code: u8,
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
pub struct FleetSingleOrderScreen;
pub struct FleetGroupScreen;
pub struct FleetMissionPickerScreen;
pub struct FleetMergeScreen;
pub struct FleetTransferScreen;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetMergeMode {
    SelectingSource,
    SelectingHost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetTransferMode {
    SelectingFleets,
    EnteringBattleships,
    EnteringCruisers,
    EnteringDestroyers,
    EnteringFullTransports,
    EnteringEmptyTransports,
    EnteringScouts,
    EnteringEtacs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetGroupOrderMode {
    SelectingFleets,
    EnteringTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetSingleOrderMode {
    SelectingFleet,
    EnteringTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FleetMissionOption {
    pub code: u8,
    pub mission: &'static str,
    pub requirements: &'static str,
}

const FLEET_COL_1: usize = 2;
const FLEET_COL_2: usize = 21;
const FLEET_COL_3: usize = 41;
const FLEET_COL_4: usize = 61;

const TOP_ROW: [MenuEntry<'static>; 1] = [MenuEntry::new(FLEET_COL_4, "O", "rder a Fleet")];

const ROW_1: [MenuEntry<'static>; 4] = [
    MenuEntry::new(FLEET_COL_1, "H", "elp on Options"),
    MenuEntry::new(FLEET_COL_2, "S", "TARBASE MENU..."),
    MenuEntry::new(FLEET_COL_3, "C", "hg ROE,ID,Speed"),
    MenuEntry::new(FLEET_COL_4, "G", "ROUP FLEET ORDER"),
];

const ROW_2: [MenuEntry<'static>; 4] = [
    MenuEntry::new(FLEET_COL_1, "Q", "uit: Main Menu"),
    MenuEntry::new(FLEET_COL_2, "E", "TA Calc"),
    MenuEntry::new(FLEET_COL_3, "I", "nfo about Planet"),
    MenuEntry::new(FLEET_COL_4, "M", "erge a Fleet"),
];

const ROW_3: [MenuEntry<'static>; 4] = [
    MenuEntry::new(FLEET_COL_1, "X", "pert Mode"),
    MenuEntry::new(FLEET_COL_2, "F", "leet List"),
    MenuEntry::new(FLEET_COL_3, "D", "etach Ships"),
    MenuEntry::new(FLEET_COL_4, "L", "oad TTs w/Armies"),
];

const ROW_4: [MenuEntry<'static>; 4] = [
    MenuEntry::new(FLEET_COL_1, "V", "iew Partial Map"),
    MenuEntry::new(FLEET_COL_2, "R", "eview a Fleet"),
    MenuEntry::new(FLEET_COL_3, "T", "ransfer Ships"),
    MenuEntry::new(FLEET_COL_4, "U", "nload TT Armies"),
];

const BRIEF_COLUMNS: [TableColumn<'static>; 5] = [
    TableColumn::right("ID", 2),
    TableColumn::left("Location", 10),
    TableColumn::right("Spd", 7),
    TableColumn::right("ROE", 3),
    TableColumn::left("Ships", 52),
];

pub const FLEET_MISSION_OPTIONS: [FleetMissionOption; 16] = [
    FleetMissionOption {
        code: 0,
        mission: "None (hold position)",
        requirements: "None. All ships can do this.",
    },
    FleetMissionOption {
        code: 1,
        mission: "Move Fleet (only)",
        requirements: "None. All ships can do this.",
    },
    FleetMissionOption {
        code: 2,
        mission: "Seek Home",
        requirements: "None. All ships can do this.",
    },
    FleetMissionOption {
        code: 3,
        mission: "Patrol a Sector",
        requirements: "None. All ships can do this.",
    },
    FleetMissionOption {
        code: 4,
        mission: "Guard a Starbase",
        requirements: "Combat ship(s).",
    },
    FleetMissionOption {
        code: 5,
        mission: "Guard/Blockade a World",
        requirements: "Combat ship(s).",
    },
    FleetMissionOption {
        code: 6,
        mission: "Bombard a World",
        requirements: "Combat ship(s).",
    },
    FleetMissionOption {
        code: 7,
        mission: "Invade a World",
        requirements: "Combat ship(s) & Loaded TTs.",
    },
    FleetMissionOption {
        code: 8,
        mission: "Blitz a World",
        requirements: "Loaded troop transports.",
    },
    FleetMissionOption {
        code: 9,
        mission: "View a World",
        requirements: "None. All ships can do this.",
    },
    FleetMissionOption {
        code: 10,
        mission: "Scout a Sector",
        requirements: "At least one scout ship.",
    },
    FleetMissionOption {
        code: 11,
        mission: "Scout a Solar System",
        requirements: "At least one scout ship.",
    },
    FleetMissionOption {
        code: 12,
        mission: "Colonize a World",
        requirements: "At least one ETAC.",
    },
    FleetMissionOption {
        code: 13,
        mission: "Join another fleet",
        requirements: "None. All ships can do this.",
    },
    FleetMissionOption {
        code: 14,
        mission: "Rendezvous at Sector",
        requirements: "None. All ships can do this.",
    },
    FleetMissionOption {
        code: 15,
        mission: "Salvage",
        requirements: "None. All ships can do this.",
    },
];

fn fleet_selector_columns(max_fleet_number: u16) -> [TableColumn<'static>; 7] {
    [
        TableColumn::right("ID", fleet_id_column_width(max_fleet_number)),
        TableColumn::left("Location", 10),
        TableColumn::right("Spd", 7),
        TableColumn::right("ROE", 3),
        TableColumn::right("Ord", 3),
        TableColumn::left("Target", 10),
        TableColumn::left("Ships", 31),
    ]
}

impl FleetMenuScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_with_notice(
        &mut self,
        notice: Option<&str>,
        expert_mode: bool,
        inline_planet_info: bool,
        info_default_coords: [u8; 2],
        info_input: &str,
        info_notice: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        if expert_mode {
            if inline_planet_info {
                draw_inline_planet_info_prompt(
                    &mut buffer,
                    EXPERT_MENU_PROMPT_ROW,
                    info_default_coords,
                    info_input,
                    info_notice,
                    notice,
                );
            } else {
                draw_expert_menu(
                    &mut buffer,
                    "FLEET COMMAND",
                    "H,Q,X,V,S,F,R,E,C,I,D,T,O,G,M,L,U",
                    notice,
                );
            }
            return Ok(buffer);
        }
        buffer.fill_row(0, classic::menu_style());
        draw_title_bar(&mut buffer, 0, "FLEET COMMAND CENTER:");
        for entry in TOP_ROW {
            draw_menu_entry(&mut buffer, 0, entry.col, entry.hotkey, entry.label);
        }
        for (row_idx, row) in [
            ROW_1.as_slice(),
            ROW_2.as_slice(),
            ROW_3.as_slice(),
            ROW_4.as_slice(),
        ]
        .into_iter()
        .enumerate()
        {
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
        let command_row = menu_prompt_row(4);
        if inline_planet_info {
            draw_inline_planet_info_prompt(
                &mut buffer,
                command_row,
                info_default_coords,
                info_input,
                info_notice,
                notice,
            );
        } else if let Some(notice) = notice {
            draw_menu_notice(&mut buffer, command_row, notice);
        }
        if !inline_planet_info {
            draw_command_prompt_at(
                &mut buffer,
                command_row,
                "FLEET COMMAND",
                "H,Q,X,V,S,F,R,E,C,I,D,T,O,G,M,L,U",
            );
        }
        Ok(buffer)
    }
}

impl Screen for FleetMenuScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_with_notice(None, false, false, [0, 0], "", None)
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('f') | KeyCode::Char('F') => {
                Action::Fleet(FleetAction::OpenList(FleetListMode::Full))
            }
            KeyCode::Char('r') | KeyCode::Char('R') => Action::Fleet(FleetAction::OpenReviewSelect),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenMainMenu,
            KeyCode::Char('h') | KeyCode::Char('H') => Action::Fleet(FleetAction::OpenHelp),
            KeyCode::Char('s') | KeyCode::Char('S') => Action::Starbase(StarbaseAction::OpenMenu),
            KeyCode::Char('d') | KeyCode::Char('D') => Action::Fleet(FleetAction::OpenDetach),
            KeyCode::Char('m') | KeyCode::Char('M') => Action::Fleet(FleetAction::OpenMerge),
            KeyCode::Char('o') | KeyCode::Char('O') => Action::Fleet(FleetAction::OpenOrder),
            KeyCode::Char('t') | KeyCode::Char('T') => Action::Fleet(FleetAction::OpenTransfer),
            KeyCode::Char('c') | KeyCode::Char('C') => Action::Fleet(FleetAction::OpenRoeSelect),
            KeyCode::Char('l') | KeyCode::Char('L') => {
                Action::Fleet(FleetAction::OpenTransportLoad)
            }
            KeyCode::Char('u') | KeyCode::Char('U') => {
                Action::Fleet(FleetAction::OpenTransportUnload)
            }
            KeyCode::Char('e') | KeyCode::Char('E') => Action::Fleet(FleetAction::OpenEta),
            KeyCode::Char('v') | KeyCode::Char('V') => Action::Starmap(
                StarmapAction::OpenPartialView(crate::screen::CommandMenu::Fleet),
            ),
            KeyCode::Char('i') | KeyCode::Char('I') => Action::Planet(
                PlanetAction::OpenInfoPrompt(crate::screen::CommandMenu::Fleet),
            ),
            KeyCode::Char('g') | KeyCode::Char('G') => Action::Fleet(FleetAction::OpenGroupOrder),
            KeyCode::Char('x') | KeyCode::Char('X') => Action::ToggleExpertMode,
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
            FleetListMode::Full => "FLEET LIST:",
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
                    format_sector_coords_padded(row.coords),
                    format!("{}/{}", row.current_speed, row.max_speed),
                    row.rules_of_engagement.to_string(),
                    row.composition_label.clone(),
                ],
                FleetListMode::Full => vec![
                    format_fleet_number(row.fleet_number, max_fleet_number),
                    format_sector_coords_padded(row.coords),
                    fleet_list_order_label(row.order_code).to_string(),
                    fleet_list_target_label(row.target_coords),
                    format!("{}/{}", row.current_speed, row.max_speed),
                    row.eta_label.clone(),
                    row.rules_of_engagement.to_string(),
                    row.composition_label.clone(),
                ],
            })
            .collect::<Vec<_>>();
        let metrics = write_table_window_with_cursor(
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
            if table_rows.is_empty() {
                None
            } else {
                Some(cursor)
            },
        );
        let command_row = table_prompt_row(metrics.bottom_row);
        draw_table_command_bar_at(&mut buffer, command_row, "<ARROWS J K Q>", None, "");
        if table_rows.is_empty() {
            draw_command_message_stack(
                &mut buffer,
                command_row,
                &[CommandMessage::Notice("You have no active fleets.")],
            );
        }
        Ok(buffer)
    }

    pub fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::Fleet(FleetAction::MoveList(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::Fleet(FleetAction::MoveList(1))
            }
            KeyCode::PageUp => Action::Fleet(FleetAction::MoveList(-8)),
            KeyCode::PageDown => Action::Fleet(FleetAction::MoveList(8)),
            KeyCode::Enter => Action::Fleet(FleetAction::OpenReview),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Fleet(FleetAction::OpenReviewSelect)
            }
            _ => Action::Noop,
        }
    }
}

impl FleetReviewScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_select(
        &mut self,
        rows: &[FleetRow],
        scroll_offset: usize,
        cursor: usize,
        input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(0, 0, "REVIEW A FLEET:", classic::title_style());
        let max_fleet_number = max_fleet_number(rows);
        let brief_columns = brief_columns(max_fleet_number);
        draw_status_line(
            &mut buffer,
            1,
            "",
            "Select a fleet, then press ENTER to review its status and composition.",
        );
        let table_rows = rows
            .iter()
            .map(|row| {
                vec![
                    format_fleet_number(row.fleet_number, max_fleet_number),
                    format_sector_coords_padded(row.coords),
                    format!("{}/{}", row.current_speed, row.max_speed),
                    row.rules_of_engagement.to_string(),
                    row.composition_label.clone(),
                ]
            })
            .collect::<Vec<_>>();
        let metrics = write_table_window_with_cursor(
            &mut buffer,
            3,
            &brief_columns,
            &table_rows,
            scroll_offset,
            FLEET_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
            if table_rows.is_empty() {
                None
            } else {
                Some(cursor)
            },
        );
        let command_row = table_prompt_row(metrics.bottom_row);
        if table_rows.is_empty() {
            draw_command_line_text_at(
                &mut buffer,
                command_row,
                "COMMANDS",
                "You have no active fleets. Q quits.",
            );
        } else {
            draw_table_command_bar_at(
                &mut buffer,
                command_row,
                "<ARROWS J K Q>",
                Some(&format_fleet_number(
                    rows[cursor].fleet_number,
                    max_fleet_number,
                )),
                input,
            );
            if let Some(status) = status {
                draw_inline_status_after(&mut buffer, command_row, status);
            }
        }
        Ok(buffer)
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
            &format_sector_coords(row.coords),
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
        draw_command_prompt_at(
            &mut buffer,
            menu_prompt_row(12),
            "FLEET COMMAND",
            "ARROWS H J K L Q",
        );
        Ok(buffer)
    }

    pub fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Left => Action::Fleet(FleetAction::MoveReview(-1)),
            KeyCode::Down | KeyCode::Right => Action::Fleet(FleetAction::MoveReview(1)),
            KeyCode::Home => Action::Fleet(FleetAction::MoveReview(i8::MIN)),
            KeyCode::End => Action::Fleet(FleetAction::MoveReview(i8::MAX)),
            KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Char('h') | KeyCode::Char('H') => {
                Action::Fleet(FleetAction::MoveReview(-1))
            }
            KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Char('l') | KeyCode::Char('L') => {
                Action::Fleet(FleetAction::MoveReview(1))
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Fleet(FleetAction::OpenReviewSelect)
            }
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
                    format_sector_coords_padded(row.coords),
                    format!("{}/{}", row.current_speed, row.max_speed),
                    row.rules_of_engagement.to_string(),
                    row.composition_label.clone(),
                ]
            })
            .collect::<Vec<_>>();
        let metrics = write_table_window_with_cursor(
            &mut buffer,
            3,
            &brief_columns,
            &table_rows,
            scroll_offset,
            FLEET_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
            if table_rows.is_empty() {
                None
            } else {
                Some(cursor)
            },
        );
        let command_row = table_prompt_row(metrics.bottom_row);
        if table_rows.is_empty() {
            draw_command_line_text_at(
                &mut buffer,
                command_row,
                "COMMANDS",
                "You have no active fleets. Q quits.",
            );
        } else if editing {
            let row = &rows[cursor];
            draw_command_line_default_input_at(
                &mut buffer,
                command_row,
                "FLEET COMMAND",
                &format!(
                    "Fleet #{} new ROE ",
                    format_fleet_number(row.fleet_number, max_fleet_number)
                ),
                &row.rules_of_engagement.to_string(),
                input,
            );
        } else {
            draw_table_command_bar_at(
                &mut buffer,
                command_row,
                "<ARROWS J K Q>",
                Some(&format_fleet_number(
                    rows[cursor].fleet_number,
                    max_fleet_number,
                )),
                select_input,
            );
        }
        if !table_rows.is_empty() {
            if let Some(status) = status {
                draw_inline_status_after(&mut buffer, command_row, status);
            }
        }
        Ok(buffer)
    }

    pub fn handle_select_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::Fleet(FleetAction::MoveRoeSelect(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::Fleet(FleetAction::MoveRoeSelect(1))
            }
            KeyCode::PageUp => Action::Fleet(FleetAction::MoveRoeSelect(-8)),
            KeyCode::PageDown => Action::Fleet(FleetAction::MoveRoeSelect(8)),
            KeyCode::Enter => Action::Noop,
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Fleet(FleetAction::OpenMenu)
            }
            _ => Action::Noop,
        }
    }
}

impl FleetSingleOrderScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        rows: &[FleetRow],
        scroll_offset: usize,
        cursor: usize,
        mode: FleetSingleOrderMode,
        target_status_line: &str,
        target_prompt: &str,
        target_default: &str,
        input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(0, 0, "ORDER A FLEET:", classic::title_style());
        let max_fleet_number = max_fleet_number(rows);
        let columns = [
            TableColumn::right("ID", fleet_id_column_width(max_fleet_number)),
            TableColumn::left("Location", 10),
            TableColumn::right("Spd", 7),
            TableColumn::right("ROE", 3),
            TableColumn::right("Ord", 3),
            TableColumn::left("Target", 10),
            TableColumn::left("Ships", 31),
        ];
        draw_status_line(
            &mut buffer,
            1,
            "",
            match mode {
                FleetSingleOrderMode::SelectingFleet => {
                    "Select a fleet, then press ENTER to give it a mission."
                }
                FleetSingleOrderMode::EnteringTarget => target_status_line,
            },
        );
        let selected_fleet_label = rows
            .get(cursor)
            .map(|row| row.fleet_number.to_string())
            .unwrap_or_else(|| "?".to_string());
        draw_status_line(&mut buffer, 2, "Selected fleet: ", &selected_fleet_label);
        let table_rows = rows
            .iter()
            .map(|row| {
                vec![
                    format_fleet_number(row.fleet_number, max_fleet_number),
                    format_sector_coords_padded(row.coords),
                    format!("{}/{}", row.current_speed, row.max_speed),
                    row.rules_of_engagement.to_string(),
                    row.order_code.to_string(),
                    format_sector_coords_padded(row.target_coords),
                    row.composition_label.clone(),
                ]
            })
            .collect::<Vec<_>>();
        let metrics = write_table_window_with_cursor(
            &mut buffer,
            3,
            &columns,
            &table_rows,
            scroll_offset,
            FLEET_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
            if table_rows.is_empty() {
                None
            } else {
                Some(cursor)
            },
        );
        let command_row = table_prompt_row(metrics.bottom_row);
        if table_rows.is_empty() {
            draw_command_line_text_at(
                &mut buffer,
                command_row,
                "COMMANDS",
                "You have no active fleets. Q quits.",
            );
        } else {
            match mode {
                FleetSingleOrderMode::SelectingFleet => {
                    draw_table_command_bar_at(
                        &mut buffer,
                        command_row,
                        "<ARROWS J K Q>",
                        Some(&format_fleet_number(
                            rows[cursor].fleet_number,
                            max_fleet_number,
                        )),
                        input,
                    );
                }
                FleetSingleOrderMode::EnteringTarget => {
                    draw_command_line_default_input_at(
                        &mut buffer,
                        command_row,
                        "FLEET COMMAND",
                        target_prompt,
                        target_default,
                        input,
                    );
                }
            }
            if let Some(status) = status {
                draw_inline_status_after(&mut buffer, command_row, status);
            }
        }
        Ok(buffer)
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
        let eta_columns = fleet_selector_columns(max_fleet_number);
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
                    format_sector_coords_padded(row.coords),
                    format!("{}/{}", row.current_speed, row.max_speed),
                    row.rules_of_engagement.to_string(),
                    row.order_code.to_string(),
                    format_sector_coords_padded(row.target_coords),
                    row.composition_label.clone(),
                ]
            })
            .collect::<Vec<_>>();
        let metrics = write_table_window_with_cursor(
            &mut buffer,
            3,
            &eta_columns,
            &table_rows,
            scroll_offset,
            FLEET_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
            if table_rows.is_empty() {
                None
            } else {
                Some(cursor)
            },
        );
        let command_row = table_prompt_row(metrics.bottom_row);
        if table_rows.is_empty() {
            draw_command_line_text_at(
                &mut buffer,
                command_row,
                "COMMANDS",
                "You have no active fleets. Q quits.",
            );
            return Ok(buffer);
        }
        match mode {
            FleetEtaMode::SelectingFleet => {
                draw_table_command_bar_at(
                    &mut buffer,
                    command_row,
                    "<ARROWS J K Q>",
                    Some(&format_fleet_number(
                        rows[cursor].fleet_number,
                        max_fleet_number,
                    )),
                    select_input,
                );
            }
            FleetEtaMode::EnteringDestination => {
                draw_command_line_default_input_at(
                    &mut buffer,
                    command_row,
                    "FLEET COMMAND",
                    "Destination ",
                    &format!("{},{}", destination_default[0], destination_default[1]),
                    destination_input,
                );
            }
            FleetEtaMode::ConfirmingSystemEntry => {
                draw_command_line_default_input_at(
                    &mut buffer,
                    command_row,
                    "FLEET COMMAND",
                    "Include time to enter system? ",
                    "N",
                    include_system_input,
                );
            }
            FleetEtaMode::ShowingResult => {
                draw_command_line_text_at(
                    &mut buffer,
                    command_row,
                    "FLEET COMMAND",
                    status.unwrap_or("Press ENTER to continue."),
                );
            }
        }
        if mode != FleetEtaMode::ShowingResult {
            if let Some(status) = status {
                draw_inline_status_after(&mut buffer, command_row, status);
            }
        }
        Ok(buffer)
    }
}

impl FleetMergeScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        rows: &[FleetRow],
        scroll_offset: usize,
        cursor: usize,
        mode: FleetMergeMode,
        input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(0, 0, "MERGE A FLEET:", classic::title_style());
        let max_fleet_number = max_fleet_number(rows);
        let brief_columns = brief_columns(max_fleet_number);
        draw_status_line(
            &mut buffer,
            1,
            "",
            match mode {
                FleetMergeMode::SelectingSource => "Select the fleet that will join another fleet.",
                FleetMergeMode::SelectingHost => {
                    "Select the host fleet that will absorb the joining fleet."
                }
            },
        );
        let table_rows = rows
            .iter()
            .map(|row| {
                vec![
                    format_fleet_number(row.fleet_number, max_fleet_number),
                    format_sector_coords_padded(row.coords),
                    format!("{}/{}", row.current_speed, row.max_speed),
                    row.rules_of_engagement.to_string(),
                    row.composition_label.clone(),
                ]
            })
            .collect::<Vec<_>>();
        let metrics = write_table_window_with_cursor(
            &mut buffer,
            3,
            &brief_columns,
            &table_rows,
            scroll_offset,
            FLEET_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
            if table_rows.is_empty() {
                None
            } else {
                Some(cursor)
            },
        );
        let command_row = table_prompt_row(metrics.bottom_row);
        if table_rows.is_empty() {
            draw_command_line_text_at(
                &mut buffer,
                command_row,
                "COMMANDS",
                "At least two fleets are required. Q quits.",
            );
        } else {
            draw_table_command_bar_at(
                &mut buffer,
                command_row,
                "<ARROWS J K Q>",
                Some(&format_fleet_number(
                    rows[cursor].fleet_number,
                    max_fleet_number,
                )),
                input,
            );
            if let Some(status) = status {
                draw_inline_status_after(&mut buffer, command_row, status);
            }
        }
        Ok(buffer)
    }
}

impl FleetGroupScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        rows: &[FleetRow],
        scroll_offset: usize,
        cursor: usize,
        selected_fleet_record_indexes: &BTreeSet<usize>,
        mode: FleetGroupOrderMode,
        target_status_line: &str,
        target_prompt: &str,
        target_default: &str,
        input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(0, 0, "GROUP FLEET ORDER:", classic::title_style());
        let max_fleet_number = max_fleet_number(rows);
        let columns = [
            TableColumn::right("ID", fleet_id_column_width(max_fleet_number)),
            TableColumn::center("Sel", 3),
            TableColumn::left("Location", 10),
            TableColumn::right("Spd", 7),
            TableColumn::right("ROE", 3),
            TableColumn::right("Ord", 3),
            TableColumn::left("Target", 10),
            TableColumn::left("Ships", 27),
        ];
        draw_status_line(
            &mut buffer,
            1,
            "",
            match mode {
                FleetGroupOrderMode::SelectingFleets => {
                    "Select fleets with SPACE, then press ENTER to give them the same mission."
                }
                FleetGroupOrderMode::EnteringTarget => target_status_line,
            },
        );
        draw_status_line(
            &mut buffer,
            2,
            "Selected fleets: ",
            &selected_fleet_record_indexes.len().to_string(),
        );
        let table_rows = rows
            .iter()
            .map(|row| {
                vec![
                    format_fleet_number(row.fleet_number, max_fleet_number),
                    if selected_fleet_record_indexes.contains(&row.fleet_record_index_1_based) {
                        "X".to_string()
                    } else {
                        "".to_string()
                    },
                    format_sector_coords_padded(row.coords),
                    format!("{}/{}", row.current_speed, row.max_speed),
                    row.rules_of_engagement.to_string(),
                    row.order_code.to_string(),
                    format_sector_coords_padded(row.target_coords),
                    row.composition_label.clone(),
                ]
            })
            .collect::<Vec<_>>();
        let metrics = write_table_window_with_cursor(
            &mut buffer,
            3,
            &columns,
            &table_rows,
            scroll_offset,
            FLEET_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
            if table_rows.is_empty() {
                None
            } else {
                Some(cursor)
            },
        );
        let command_row = table_prompt_row(metrics.bottom_row);
        if table_rows.is_empty() {
            draw_command_line_text_at(
                &mut buffer,
                command_row,
                "COMMANDS",
                "You have no active fleets. Q quits.",
            );
        } else {
            match mode {
                FleetGroupOrderMode::SelectingFleets => {
                    draw_table_command_bar_at(
                        &mut buffer,
                        command_row,
                        "<ARROWS J K SPACE Q>",
                        None,
                        "",
                    );
                }
                FleetGroupOrderMode::EnteringTarget => {
                    draw_command_line_default_input_at(
                        &mut buffer,
                        command_row,
                        "FLEET COMMAND",
                        target_prompt,
                        target_default,
                        input,
                    );
                }
            }
            if let Some(status) = status {
                draw_inline_status_after(&mut buffer, command_row, status);
            }
        }
        Ok(buffer)
    }
}

impl FleetTransferScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        rows: &[FleetRow],
        scroll_offset: usize,
        cursor: usize,
        mode: FleetTransferMode,
        selected_fleet_record_indexes: &BTreeSet<usize>,
        donor_fleet_number: Option<u16>,
        host_fleet_number: Option<u16>,
        input: &str,
        status: Option<&str>,
        prompt: &str,
        default: &str,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        let max_fleet_number = max_fleet_number(rows);
        let columns = [
            TableColumn::right("ID", fleet_id_column_width(max_fleet_number)),
            TableColumn::center("Sel", 3),
            TableColumn::left("Location", 10),
            TableColumn::right("Spd", 7),
            TableColumn::right("ROE", 3),
            TableColumn::right("Ord", 3),
            TableColumn::left("Target", 10),
            TableColumn::left("Ships", 27),
        ];
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(0, 0, "TRANSFER SHIPS:", classic::title_style());
        draw_status_line(
            &mut buffer,
            1,
            "",
            match mode {
                FleetTransferMode::SelectingFleets => {
                    "Select two fleets in one sector. Highlight the host fleet, then press ENTER."
                }
                _ => "Enter ship counts to transfer from the donor fleet to the host fleet.",
            },
        );
        match mode {
            FleetTransferMode::SelectingFleets => {
                draw_status_line(
                    &mut buffer,
                    2,
                    "Selected fleets: ",
                    &selected_fleet_record_indexes.len().to_string(),
                );
            }
            _ => {
                if let Some(donor) = donor_fleet_number {
                    draw_status_line(&mut buffer, 2, "Donor: ", &format!("Fleet #{donor}"));
                }
                if let Some(host) = host_fleet_number {
                    draw_status_line(&mut buffer, 2, "Host: ", &format!("Fleet #{host}"));
                }
            }
        }
        let table_rows = rows
            .iter()
            .map(|row| {
                vec![
                    format_fleet_number(row.fleet_number, max_fleet_number),
                    if selected_fleet_record_indexes.contains(&row.fleet_record_index_1_based) {
                        "X".to_string()
                    } else {
                        "".to_string()
                    },
                    format_sector_coords_padded(row.coords),
                    format!("{}/{}", row.current_speed, row.max_speed),
                    row.rules_of_engagement.to_string(),
                    row.order_code.to_string(),
                    format_sector_coords_padded(row.target_coords),
                    row.composition_label.clone(),
                ]
            })
            .collect::<Vec<_>>();
        let metrics = write_table_window_with_cursor(
            &mut buffer,
            3,
            &columns,
            &table_rows,
            scroll_offset,
            FLEET_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
            if !table_rows.is_empty() {
                Some(cursor)
            } else {
                None
            },
        );
        let command_row = table_prompt_row(metrics.bottom_row);
        if rows.is_empty() {
            draw_table_command_bar_at(&mut buffer, command_row, "<ARROWS J K SPACE Q>", None, "");
        } else {
            match mode {
                FleetTransferMode::SelectingFleets => {
                    if !table_rows.is_empty() {
                        draw_table_command_bar_at(
                            &mut buffer,
                            command_row,
                            "<ARROWS J K SPACE Q>",
                            None,
                            "",
                        );
                    }
                }
                _ => {
                    draw_command_line_default_input_at(
                        &mut buffer,
                        command_row,
                        "FLEET COMMAND",
                        prompt,
                        default,
                        input,
                    );
                }
            }
            if let Some(status) = status {
                draw_inline_status_after(&mut buffer, command_row, status);
            }
        }
        Ok(buffer)
    }
}

impl FleetMissionPickerScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        cursor: usize,
        input: &str,
        enabled: &[bool],
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(0, 0, "FLEET MISSION ORDERS:", classic::title_style());
        let columns = [
            TableColumn::right("No.", 3),
            TableColumn::left("Mission", 27),
            TableColumn::left("Requirements (if any)", 46),
        ];
        let rows = FLEET_MISSION_OPTIONS
            .iter()
            .map(|option| {
                vec![
                    option.code.to_string(),
                    option.mission.to_string(),
                    option.requirements.to_string(),
                ]
            })
            .collect::<Vec<_>>();
        let row_states = enabled
            .iter()
            .map(|enabled| {
                if *enabled {
                    TableRowState::Normal
                } else {
                    TableRowState::Disabled
                }
            })
            .collect::<Vec<_>>();
        let metrics = write_table_window_with_states(
            &mut buffer,
            1,
            &columns,
            &rows,
            0,
            rows.len(),
            classic::status_value_style(),
            classic::status_value_style(),
            Some(cursor),
            Some(&row_states),
        );
        let command_row = table_prompt_row(metrics.bottom_row);
        {
            let default = FLEET_MISSION_OPTIONS
                .get(cursor)
                .map(|option| option.code.to_string())
                .unwrap_or_else(|| "1".to_string());
            draw_table_command_bar_at(
                &mut buffer,
                command_row,
                "<ARROWS J K Q>",
                Some(&default),
                input,
            );
            if let Some(status) = status {
                draw_inline_status_after(&mut buffer, command_row, status);
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
                    format_sector_coords_padded(row.coords),
                    format!("{}/{}", row.current_speed, row.max_speed),
                    row.rules_of_engagement.to_string(),
                    row.composition_label.clone(),
                ]
            })
            .collect::<Vec<_>>();
        let metrics = write_table_window_with_cursor(
            &mut buffer,
            3,
            &brief_columns,
            &table_rows,
            scroll_offset,
            FLEET_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
            if table_rows.is_empty() {
                None
            } else {
                Some(cursor)
            },
        );
        let command_row = table_prompt_row(metrics.bottom_row);
        if table_rows.is_empty() {
            draw_command_line_text_at(
                &mut buffer,
                command_row,
                "FLEET COMMAND",
                "You have no active fleets. Q quits.",
            );
        } else {
            draw_command_line_default_input_at(
                &mut buffer,
                command_row,
                "FLEET COMMAND",
                prompt,
                default,
                input,
            );
            if let Some(status) = status {
                draw_inline_status_after(&mut buffer, command_row, status);
            }
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

fn full_columns(max_fleet_number: u16) -> [TableColumn<'static>; 8] {
    let id_width = fleet_id_column_width(max_fleet_number);
    let ships_width = 71usize.saturating_sub(id_width + 8 + 15 + 8 + 5 + 4 + 3);
    [
        TableColumn::right("ID", id_width),
        TableColumn::left("Location", 8),
        TableColumn::left("Order", 15),
        TableColumn::left("Target", 8),
        TableColumn::right("Spd", 5),
        TableColumn::right("ETA", 4),
        TableColumn::right("ROE", 3),
        TableColumn::left("Ships", ships_width),
    ]
}

fn fleet_list_order_label(order_code: u8) -> &'static str {
    match ec_data::Order::from_raw(order_code) {
        ec_data::Order::HoldPosition => "Hold",
        ec_data::Order::MoveOnly => "Move",
        ec_data::Order::SeekHome => "Seek home",
        ec_data::Order::PatrolSector => "Patrol",
        ec_data::Order::GuardStarbase => "Guard starbase",
        ec_data::Order::GuardBlockadeWorld => "Guard/blockade",
        ec_data::Order::BombardWorld => "Bombard",
        ec_data::Order::InvadeWorld => "Invade",
        ec_data::Order::BlitzWorld => "Blitz",
        ec_data::Order::ViewWorld => "View",
        ec_data::Order::ScoutSector => "Scout sector",
        ec_data::Order::ScoutSolarSystem => "Scout system",
        ec_data::Order::ColonizeWorld => "Colonize",
        ec_data::Order::JoinAnotherFleet => "Join fleet",
        ec_data::Order::RendezvousSector => "Rendezvous",
        ec_data::Order::Salvage => "Salvage",
        ec_data::Order::Unknown(_) => "Unknown",
    }
}

fn fleet_list_target_label(target_coords: [u8; 2]) -> String {
    if target_coords[0] == 0 || target_coords[1] == 0 {
        String::new()
    } else {
        format_sector_coords_padded(target_coords)
    }
}
