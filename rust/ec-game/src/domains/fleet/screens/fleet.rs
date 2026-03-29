use crossterm::event::{KeyCode, KeyEvent};
use std::collections::BTreeSet;

use crate::app::Action;
use crate::domains::fleet::FleetAction;
use crate::domains::fleet::missions::FLEET_MISSION_OPTIONS;
use crate::domains::fleet::state::FleetMenuPromptMode;
use crate::domains::planet::PlanetAction;
use crate::domains::starbase::StarbaseAction;
use crate::domains::starmap::StarmapAction;
use crate::screen::layout::{
    EXPERT_MENU_PROMPT_ROW, MenuEntry, PromptFeedback, dismiss_prompt_row,
    draw_command_line_default_input_at, draw_command_prompt_at, draw_dismiss_prompt,
    draw_expert_menu, draw_inline_planet_info_prompt, draw_menu_entry, draw_menu_notice,
    draw_prompt_error_after, draw_prompt_feedback_after, draw_status_line, draw_title_bar,
    draw_wrapped_message, last_body_row, menu_prompt_row, new_playfield,
    standard_table_visible_rows, standard_table_visible_rows_for,
};
use crate::screen::table::{
    HorizontalAlign, LayoutRect, TableColumn, TableFooter, TableRowState, TableWidthMode,
    VerticalAlign, draw_table_footer, draw_table_title, fit_table_columns,
    fit_table_columns_for_widget, fleet_id_column_width, format_fleet_number,
    layout_standard_table_block, resolve_table_columns_for_widget, write_table_window_with_cursor,
    write_table_window_with_states_at,
};
use crate::screen::{
    COMMAND_LABEL, PlanetTransportMode, PlayfieldBuffer, Screen, ScreenFrame, ScreenGeometry,
    StyledSpan, format_sector_coords, format_sector_coords_table,
};
use crate::theme::classic;

pub const FLEET_VISIBLE_ROWS: usize = standard_table_visible_rows(3);
pub const FLEET_LIST_VISIBLE_ROWS: usize = standard_table_visible_rows(1);

pub fn fleet_visible_rows(geometry: ScreenGeometry) -> usize {
    standard_table_visible_rows_for(geometry, 3)
}

pub fn fleet_list_visible_rows(geometry: ScreenGeometry) -> usize {
    standard_table_visible_rows_for(geometry, 1)
}

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
    pub list_eta_label: String,
    pub rules_of_engagement: u8,
    pub order_label: String,
    pub composition_label: String,
    pub table_composition_label: String,
}

pub struct FleetMenuScreen;
pub struct FleetListScreen;
pub struct FleetReviewScreen;
pub struct FleetSingleOrderScreen;
pub struct FleetGroupScreen;
pub struct FleetMissionPickerScreen;
pub struct FleetTransferScreen;
pub struct FleetEtaScreen;
pub struct FleetDetachScreen;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetSingleOrderMode {
    EnteringTarget,
    EnteringTargetX,
    EnteringTargetY,
    ConfirmingTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetEtaMode {
    EnteringDestination,
    ConfirmingSystemEntry,
    ShowingResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetDetachClass {
    Battleships,
    Cruisers,
    Destroyers,
    FullTransports,
    EmptyTransports,
    Scouts,
    Etacs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetDetachMode {
    ChoosingClass,
    EnteringQuantity(FleetDetachClass),
    AdjustingDonorSpeed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetTransferMode {
    ChoosingClass,
    EnteringQuantity(FleetDetachClass),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetGroupOrderMode {
    SelectingFleets,
    EnteringTarget,
    EnteringTargetX,
    EnteringTargetY,
    ConfirmingTarget,
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

fn mission_picker_columns() -> Vec<TableColumn<'static>> {
    let columns = [
        TableColumn::right("No.", 3),
        TableColumn::left("Mission", "Mission".len()),
        TableColumn::left("Requirements (if any)", "Requirements (if any)".len()),
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
    fit_table_columns(&columns, &rows)
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
        menu_prompt_mode: Option<FleetMenuPromptMode>,
        menu_prompt_label: Option<&str>,
        menu_prompt_default: &str,
        menu_prompt_input: &str,
        menu_prompt_status: Option<&PromptFeedback>,
        inline_transport_mode: Option<PlanetTransportMode>,
        inline_transport_summary: Option<&str>,
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
            } else if menu_prompt_mode.is_some() {
                draw_command_line_default_input_at(
                    &mut buffer,
                    EXPERT_MENU_PROMPT_ROW,
                    "FLEET COMMAND",
                    menu_prompt_label.unwrap_or("Command "),
                    menu_prompt_default,
                    menu_prompt_input,
                );
                if let Some(status) = menu_prompt_status {
                    draw_prompt_feedback_after(&mut buffer, EXPERT_MENU_PROMPT_ROW, status);
                }
            } else {
                draw_expert_menu(
                    &mut buffer,
                    "FLEET COMMAND",
                    "H X V S F R E C I D T O G M L U <Q>",
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
        } else if let Some(mode) = inline_transport_mode {
            draw_title_bar(&mut buffer, 6, mode.title());
            if let Some(summary) = inline_transport_summary {
                buffer.write_text(8, 0, summary, classic::status_value_style());
            }
            const TRANSPORT_COMMAND_ROW: usize = 10;
            draw_command_line_default_input_at(
                &mut buffer,
                TRANSPORT_COMMAND_ROW,
                "FLEET COMMAND",
                menu_prompt_label.unwrap_or("How many armies? "),
                menu_prompt_default,
                menu_prompt_input,
            );
            if let Some(status) = menu_prompt_status {
                draw_prompt_feedback_after(&mut buffer, TRANSPORT_COMMAND_ROW, status);
            }
        } else if menu_prompt_mode.is_some() {
            draw_command_line_default_input_at(
                &mut buffer,
                command_row,
                "FLEET COMMAND",
                menu_prompt_label.unwrap_or("Command "),
                menu_prompt_default,
                menu_prompt_input,
            );
            if let Some(status) = menu_prompt_status {
                draw_prompt_feedback_after(&mut buffer, command_row, status);
            }
        } else if let Some(notice) = notice {
            draw_menu_notice(&mut buffer, command_row, notice);
        }
        if !inline_planet_info && menu_prompt_mode.is_none() {
            draw_command_prompt_at(
                &mut buffer,
                command_row,
                "FLEET COMMAND",
                "H X V S F R E C I D T O G M L U <Q>",
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
        self.render_with_notice(
            None,
            false,
            false,
            None,
            None,
            "",
            "",
            None,
            None,
            None,
            [0, 0],
            "",
            None,
        )
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('f') | KeyCode::Char('F') => Action::Fleet(FleetAction::OpenList),
            KeyCode::Char('r') | KeyCode::Char('R') => Action::Fleet(FleetAction::OpenReviewPrompt),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenMainMenu,
            KeyCode::Char('h') | KeyCode::Char('H') => Action::Fleet(FleetAction::OpenHelp),
            KeyCode::Char('s') | KeyCode::Char('S') => Action::Starbase(StarbaseAction::OpenMenu),
            KeyCode::Char('d') | KeyCode::Char('D') => Action::Fleet(FleetAction::OpenDetach),
            KeyCode::Char('m') | KeyCode::Char('M') => Action::Fleet(FleetAction::OpenMerge),
            KeyCode::Char('o') | KeyCode::Char('O') => Action::Fleet(FleetAction::OpenOrder),
            KeyCode::Char('t') | KeyCode::Char('T') => Action::Fleet(FleetAction::OpenTransfer),
            KeyCode::Char('c') | KeyCode::Char('C') => Action::Fleet(FleetAction::OpenChangePrompt),
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
        geometry: ScreenGeometry,
        rows: &[FleetRow],
        scroll_offset: usize,
        cursor: usize,
        input: &str,
        _status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = crate::screen::layout::new_playfield_for(geometry);
        let max_fleet_number = max_fleet_number(rows);
        draw_table_title(&mut buffer, 1, 0, "FLEET LIST:");
        let table_rows = rows
            .iter()
            .map(|row| {
                vec![
                    format_fleet_number(row.fleet_number, max_fleet_number),
                    format_sector_coords_table(row.coords),
                    fleet_table_order_label(row.order_code).to_string(),
                    fleet_list_target_label(row.target_coords),
                    row.current_speed.to_string(),
                    row.list_eta_label.clone(),
                    row.rules_of_engagement.to_string(),
                    row.table_composition_label.clone(),
                ]
            })
            .collect::<Vec<_>>();
        let default_fleet_number = rows
            .get(cursor)
            .map(|row| format_fleet_number(row.fleet_number, max_fleet_number))
            .unwrap_or_else(|| {
                rows.first()
                    .map(|row| format_fleet_number(row.fleet_number, max_fleet_number))
                    .unwrap_or_else(|| format_fleet_number(1, max_fleet_number))
            });
        let footer = if table_rows.is_empty() {
            TableFooter::CommandText {
                label: COMMAND_LABEL,
                text: "You have no active fleets.",
            }
        } else {
            TableFooter::CommandBar {
                hotkeys_markup: "J K ^U ^D <Q>",
                default: Some(&default_fleet_number),
                input,
            }
        };
        let columns = fit_table_columns_for_widget(
            &full_columns(max_fleet_number),
            &table_rows,
            Some("FLEET LIST:"),
            Some(footer),
        );
        let metrics = write_table_window_with_cursor(
            &mut buffer,
            1,
            &columns,
            &table_rows,
            scroll_offset,
            fleet_list_visible_rows(geometry),
            classic::status_value_style(),
            classic::status_value_style(),
            if table_rows.is_empty() {
                None
            } else {
                Some(cursor)
            },
            0,
        );
        draw_table_footer(&mut buffer, geometry, 0, metrics.bottom_row, footer);
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
            KeyCode::Backspace => Action::Fleet(FleetAction::BackspaceListInput),
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                Action::Fleet(FleetAction::AppendListChar(ch))
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Fleet(FleetAction::OpenMenu)
            }
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
        return_to_list: bool,
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
        if return_to_list {
            draw_command_prompt_at(&mut buffer, menu_prompt_row(12), COMMAND_LABEL, "HJKL <Q>");
        } else {
            draw_dismiss_prompt(&mut buffer, dismiss_prompt_row(12));
        }
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
                Action::Fleet(FleetAction::CloseReview)
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
        row: &FleetRow,
        current_order_label: &str,
        new_order_label: &str,
        mode: FleetSingleOrderMode,
        header_text: &str,
        target_prompt: &str,
        target_default: &str,
        input: &str,
        target_x_default: &str,
        target_x_input: &str,
        target_y_default: &str,
        target_y_input: &str,
        confirm_input: &str,
        current_year: u16,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        if mode == FleetSingleOrderMode::ConfirmingTarget {
            return self.render_confirm_target(
                row,
                new_order_label,
                header_text,
                confirm_input,
                current_year,
                status,
            );
        }
        if mode == FleetSingleOrderMode::EnteringTarget {
            return self.render_named_target_entry(
                row,
                current_order_label,
                header_text,
                target_prompt,
                target_default,
                input,
                status,
            );
        }
        if matches!(
            mode,
            FleetSingleOrderMode::EnteringTargetX | FleetSingleOrderMode::EnteringTargetY
        ) {
            return self.render_coordinate_target_entry(
                row,
                current_order_label,
                new_order_label,
                mode,
                target_x_default,
                target_x_input,
                target_y_default,
                target_y_input,
                status,
            );
        }
        unreachable!("target-entry render path handled above")
    }

    fn render_coordinate_target_entry(
        &mut self,
        row: &FleetRow,
        current_order_label: &str,
        new_order_label: &str,
        mode: FleetSingleOrderMode,
        target_x_default: &str,
        target_x_input: &str,
        target_y_default: &str,
        target_y_input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(
            0,
            0,
            &format!("ORDER FLEET #{}:", row.fleet_number),
            classic::title_style(),
        );
        draw_status_line(
            &mut buffer,
            2,
            "Location: ",
            &format_sector_coords_table(row.coords),
        );
        draw_status_line(
            &mut buffer,
            3,
            "Current / Max Speed: ",
            &format!("{}/{}", row.current_speed, row.max_speed),
        );
        draw_status_line(
            &mut buffer,
            4,
            "ROE: ",
            &row.rules_of_engagement.to_string(),
        );
        draw_status_line(&mut buffer, 5, "Order: ", current_order_label);
        draw_status_line(&mut buffer, 7, "Ships: ", &row.composition_label);
        draw_status_line(
            &mut buffer,
            9,
            "Enter target coordinates for new order: ",
            new_order_label,
        );
        let command_row = menu_prompt_row(9);
        let active_row = match mode {
            FleetSingleOrderMode::EnteringTargetX => {
                draw_command_line_default_input_at(
                    &mut buffer,
                    command_row,
                    COMMAND_LABEL,
                    "Target XX ",
                    target_x_default,
                    target_x_input,
                );
                command_row
            }
            FleetSingleOrderMode::EnteringTargetY => {
                draw_command_line_default_input_at(
                    &mut buffer,
                    command_row,
                    COMMAND_LABEL,
                    "Target XX ",
                    target_x_default,
                    target_x_input,
                );
                draw_command_line_default_input_at(
                    &mut buffer,
                    command_row + 2,
                    COMMAND_LABEL,
                    "Target YY ",
                    target_y_default,
                    target_y_input,
                );
                command_row + 2
            }
            _ => command_row,
        };
        if let Some(status) = status {
            draw_prompt_error_after(&mut buffer, active_row, status);
        }
        Ok(buffer)
    }

    fn render_named_target_entry(
        &mut self,
        row: &FleetRow,
        current_order_label: &str,
        header_text: &str,
        target_prompt: &str,
        target_default: &str,
        input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(
            0,
            0,
            &format!("ORDER FLEET #{}:", row.fleet_number),
            classic::title_style(),
        );
        draw_status_line(
            &mut buffer,
            2,
            "Location: ",
            &format_sector_coords_table(row.coords),
        );
        draw_status_line(
            &mut buffer,
            3,
            "Current / Max Speed: ",
            &format!("{}/{}", row.current_speed, row.max_speed),
        );
        draw_status_line(
            &mut buffer,
            4,
            "ROE: ",
            &row.rules_of_engagement.to_string(),
        );
        draw_status_line(&mut buffer, 5, "Order: ", current_order_label);
        draw_status_line(&mut buffer, 7, "Ships: ", &row.composition_label);
        draw_status_line(&mut buffer, 9, "", header_text);
        let command_row = menu_prompt_row(9);
        draw_command_line_default_input_at(
            &mut buffer,
            command_row,
            COMMAND_LABEL,
            target_prompt,
            target_default,
            input,
        );
        if let Some(status) = status {
            draw_prompt_error_after(&mut buffer, command_row, status);
        }
        Ok(buffer)
    }

    fn render_confirm_target(
        &mut self,
        row: &FleetRow,
        new_order_label: &str,
        header_text: &str,
        confirm_input: &str,
        current_year: u16,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(
            0,
            0,
            &format!("ORDER FLEET #{}:", row.fleet_number),
            classic::title_style(),
        );
        draw_status_line(&mut buffer, 2, "Stardate: ", &current_year.to_string());
        draw_status_line(&mut buffer, 4, "", header_text);
        draw_status_line(&mut buffer, 6, "New Orders: ", new_order_label);
        let command_row = menu_prompt_row(6);
        draw_confirm_prompt_at(&mut buffer, command_row, COMMAND_LABEL, confirm_input);
        if let Some(status) = status {
            draw_prompt_error_after(&mut buffer, command_row, status);
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
        row: &FleetRow,
        mode: FleetEtaMode,
        destination_default: [u8; 2],
        destination_input: &str,
        include_system_input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(0, 0, "CALCULATE FLEET ETA:", classic::title_style());
        // row 1: blank
        draw_status_line(&mut buffer, 2, "Fleet ID: ", &row.fleet_number.to_string());
        // row 3: blank
        draw_status_line(
            &mut buffer,
            4,
            "Location: ",
            &format_sector_coords_table(row.coords),
        );
        draw_status_line(&mut buffer, 5, "Speed: ", &row.current_speed.to_string());
        // row 6: blank
        draw_status_line(&mut buffer, 7, "Orders: ", &row.order_label);
        draw_status_line(
            &mut buffer,
            8,
            "Target: ",
            &format_sector_coords_table(row.target_coords),
        );
        // row 9: blank
        draw_status_line(&mut buffer, 10, "Ships: ", &row.composition_label);
        const LAST_CONTENT_ROW: usize = 10;
        match mode {
            FleetEtaMode::EnteringDestination => {
                let command_row = menu_prompt_row(LAST_CONTENT_ROW);
                draw_command_line_default_input_at(
                    &mut buffer,
                    command_row,
                    COMMAND_LABEL,
                    "Destination ",
                    &format!("{},{}", destination_default[0], destination_default[1]),
                    destination_input,
                );
                if let Some(err) = status {
                    draw_prompt_error_after(&mut buffer, command_row, err);
                }
            }
            FleetEtaMode::ConfirmingSystemEntry => {
                let command_row = menu_prompt_row(LAST_CONTENT_ROW);
                draw_command_line_default_input_at(
                    &mut buffer,
                    command_row,
                    COMMAND_LABEL,
                    "Include time to enter system? ",
                    "N",
                    include_system_input,
                );
                if let Some(err) = status {
                    draw_prompt_error_after(&mut buffer, command_row, err);
                }
            }
            FleetEtaMode::ShowingResult => {
                let result_row = dismiss_prompt_row(LAST_CONTENT_ROW);
                draw_status_line(&mut buffer, result_row, "", status.unwrap_or(""));
                draw_dismiss_prompt(&mut buffer, dismiss_prompt_row(result_row));
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
        geometry: ScreenGeometry,
        rows: &[FleetRow],
        scroll_offset: usize,
        cursor: usize,
        selected_fleet_record_indexes: &BTreeSet<usize>,
        mode: FleetGroupOrderMode,
        target_status_line: &str,
        new_order_label: &str,
        target_prompt: &str,
        target_default: &str,
        input: &str,
        target_x_default: &str,
        target_x_input: &str,
        target_y_default: &str,
        target_y_input: &str,
        confirm_input: &str,
        current_year: u16,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        if mode == FleetGroupOrderMode::SelectingFleets {
            return self.render_selection_table(
                geometry,
                rows,
                scroll_offset,
                cursor,
                selected_fleet_record_indexes,
                status,
            );
        }
        let selected_fleet_label =
            format_selected_fleet_numbers(rows, selected_fleet_record_indexes);
        if mode == FleetGroupOrderMode::ConfirmingTarget {
            return self.render_confirm_target(
                &selected_fleet_label,
                target_status_line,
                new_order_label,
                confirm_input,
                current_year,
                status,
            );
        }
        return self.render_target_entry(
            &selected_fleet_label,
            mode,
            target_status_line,
            new_order_label,
            target_prompt,
            target_default,
            input,
            target_x_default,
            target_x_input,
            target_y_default,
            target_y_input,
            status,
        );
    }

    fn render_selection_table(
        &mut self,
        geometry: ScreenGeometry,
        rows: &[FleetRow],
        scroll_offset: usize,
        cursor: usize,
        selected_fleet_record_indexes: &BTreeSet<usize>,
        _status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = crate::screen::layout::new_playfield_for(geometry);
        buffer.fill_row(0, classic::menu_style());
        let max_fleet_number = max_fleet_number(rows);
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
                    format_sector_coords_table(row.coords),
                    fleet_table_order_label(row.order_code).to_string(),
                    fleet_list_target_label(row.target_coords),
                    row.current_speed.to_string(),
                    row.list_eta_label.clone(),
                    row.rules_of_engagement.to_string(),
                    row.table_composition_label.clone(),
                ]
            })
            .collect::<Vec<_>>();
        let visible_rows = fleet_visible_rows(geometry);
        let scrollable = table_rows.len() > visible_rows;
        let footer = if table_rows.is_empty() {
            TableFooter::CommandBar {
                hotkeys_markup: "<Q>",
                default: None,
                input: "",
            }
        } else {
            TableFooter::CommandBar {
                hotkeys_markup: "J K ^U ^D SPACE <Q>",
                default: None,
                input: "",
            }
        };
        let columns = resolve_table_columns_for_widget(
            &group_selection_columns(max_fleet_number),
            &table_rows,
            buffer.width(),
            scrollable,
            TableWidthMode::Compact,
            Some("GROUP FLEET ORDER:"),
            Some(footer),
        );
        let layout = layout_standard_table_block(
            LayoutRect::new(0, 0, buffer.width(), buffer.height()),
            &columns,
            visible_rows,
            Some("GROUP FLEET ORDER:"),
            Some(footer),
            scrollable,
            HorizontalAlign::Center,
            VerticalAlign::Top,
        );
        draw_table_title(
            &mut buffer,
            layout.table_row,
            layout.table_col,
            "GROUP FLEET ORDER:",
        );
        let metrics = crate::screen::table::write_table_window_with_states_at(
            &mut buffer,
            layout.table_row,
            layout.table_col,
            &columns,
            &table_rows,
            scroll_offset,
            visible_rows,
            classic::status_value_style(),
            classic::status_value_style(),
            if table_rows.is_empty() {
                None
            } else {
                Some(cursor)
            },
            0,
            None,
        );
        draw_table_footer(
            &mut buffer,
            geometry,
            layout.command_col,
            metrics.bottom_row,
            footer,
        );
        Ok(buffer)
    }

    fn render_target_entry(
        &mut self,
        selected_fleet_label: &str,
        mode: FleetGroupOrderMode,
        header_text: &str,
        new_order_label: &str,
        target_prompt: &str,
        target_default: &str,
        input: &str,
        target_x_default: &str,
        target_x_input: &str,
        target_y_default: &str,
        target_y_input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(0, 0, "GROUP FLEET ORDER:", classic::title_style());
        draw_status_line(&mut buffer, 2, "Selected fleets: ", selected_fleet_label);
        if mode == FleetGroupOrderMode::EnteringTarget {
            draw_status_line(&mut buffer, 4, "", header_text);
        } else {
            draw_status_line(
                &mut buffer,
                4,
                "Enter target coordinates for new order: ",
                new_order_label,
            );
        }
        let command_row = menu_prompt_row(4);
        let active_row = match mode {
            FleetGroupOrderMode::EnteringTarget => {
                draw_command_line_default_input_at(
                    &mut buffer,
                    command_row,
                    COMMAND_LABEL,
                    target_prompt,
                    target_default,
                    input,
                );
                command_row
            }
            FleetGroupOrderMode::EnteringTargetX => {
                draw_command_line_default_input_at(
                    &mut buffer,
                    command_row,
                    COMMAND_LABEL,
                    "Target XX ",
                    target_x_default,
                    target_x_input,
                );
                command_row
            }
            FleetGroupOrderMode::EnteringTargetY => {
                draw_command_line_default_input_at(
                    &mut buffer,
                    command_row,
                    COMMAND_LABEL,
                    "Target XX ",
                    target_x_default,
                    target_x_input,
                );
                draw_command_line_default_input_at(
                    &mut buffer,
                    command_row + 2,
                    COMMAND_LABEL,
                    "Target YY ",
                    target_y_default,
                    target_y_input,
                );
                command_row + 2
            }
            FleetGroupOrderMode::SelectingFleets | FleetGroupOrderMode::ConfirmingTarget => {
                command_row
            }
        };
        if let Some(status) = status {
            draw_prompt_error_after(&mut buffer, active_row, status);
        }
        Ok(buffer)
    }

    fn render_confirm_target(
        &mut self,
        selected_fleet_label: &str,
        header_text: &str,
        new_order_label: &str,
        confirm_input: &str,
        current_year: u16,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(0, 0, "GROUP FLEET ORDER:", classic::title_style());
        draw_status_line(&mut buffer, 2, "Stardate: ", &current_year.to_string());
        draw_status_line(&mut buffer, 3, "Selected fleets: ", selected_fleet_label);
        draw_status_line(&mut buffer, 5, "", header_text);
        draw_status_line(&mut buffer, 7, "New Orders: ", new_order_label);
        let command_row = menu_prompt_row(7);
        draw_confirm_prompt_at(&mut buffer, command_row, COMMAND_LABEL, confirm_input);
        if let Some(status) = status {
            draw_prompt_error_after(&mut buffer, command_row, status);
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
        donor_row: &FleetRow,
        host_row: &FleetRow,
        _mode: FleetTransferMode,
        input: &str,
        status: Option<&str>,
        prompt: &str,
        default: &str,
        source_ships: &str,
        destination_ships: &str,
        staged_summary: &str,
        remaining_summary: &str,
        destination_summary: &str,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        const SOURCE_FLEET_ROW: usize = 2;
        const SOURCE_LOCATION_ROW: usize = 3;
        const SOURCE_ORDERS_ROW: usize = 4;
        const SOURCE_TARGET_ROW: usize = 5;
        const SOURCE_SPEED_ROE_ROW: usize = 6;
        const SOURCE_SHIPS_ROW: usize = 7;
        const DEST_FLEET_ROW: usize = 9;
        const DEST_LOCATION_ROW: usize = 10;
        const DEST_ORDERS_ROW: usize = 11;
        const DEST_TARGET_ROW: usize = 12;
        const DEST_SPEED_ROE_ROW: usize = 13;
        const DEST_SHIPS_ROW: usize = 14;
        const ACTION_ROW: usize = 16;
        const COMMAND_ROW: usize = 18;
        const STAGED_ROW: usize = 20;

        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(
            0,
            0,
            "TRANSFER SHIPS BETWEEN FLEETS:",
            classic::title_style(),
        );
        draw_status_line(
            &mut buffer,
            SOURCE_FLEET_ROW,
            "Source Fleet: ",
            &format!("Fleet #{}", donor_row.fleet_number),
        );
        draw_status_line(
            &mut buffer,
            SOURCE_LOCATION_ROW,
            "Location: ",
            &format_sector_coords_table(donor_row.coords),
        );
        draw_status_line(
            &mut buffer,
            SOURCE_ORDERS_ROW,
            "Orders: ",
            &donor_row.order_label,
        );
        draw_status_line(
            &mut buffer,
            SOURCE_TARGET_ROW,
            "Target: ",
            &fleet_list_target_label(donor_row.target_coords),
        );
        draw_status_line(
            &mut buffer,
            SOURCE_SPEED_ROE_ROW,
            "Speed / ROE: ",
            &format!(
                "{} / {}",
                donor_row.current_speed, donor_row.rules_of_engagement
            ),
        );
        draw_status_line(&mut buffer, SOURCE_SHIPS_ROW, "Ships: ", source_ships);

        draw_status_line(
            &mut buffer,
            DEST_FLEET_ROW,
            "Destination Fleet: ",
            &format!("Fleet #{}", host_row.fleet_number),
        );
        draw_status_line(
            &mut buffer,
            DEST_LOCATION_ROW,
            "Location: ",
            &format_sector_coords_table(host_row.coords),
        );
        draw_status_line(
            &mut buffer,
            DEST_ORDERS_ROW,
            "Orders: ",
            &host_row.order_label,
        );
        draw_status_line(
            &mut buffer,
            DEST_TARGET_ROW,
            "Target: ",
            &fleet_list_target_label(host_row.target_coords),
        );
        draw_status_line(
            &mut buffer,
            DEST_SPEED_ROE_ROW,
            "Speed / ROE: ",
            &format!(
                "{} / {}",
                host_row.current_speed, host_row.rules_of_engagement
            ),
        );
        draw_status_line(&mut buffer, DEST_SHIPS_ROW, "Ships: ", destination_ships);
        buffer.write_spans(
            ACTION_ROW,
            0,
            &[
                StyledSpan::new("<", classic::prompt_style()),
                StyledSpan::new("C", classic::prompt_hotkey_style()),
                StyledSpan::new(">ommit, <", classic::prompt_style()),
                StyledSpan::new("X", classic::prompt_hotkey_style()),
                StyledSpan::new("> Cancel", classic::prompt_style()),
            ],
        );
        draw_command_line_default_input_at(
            &mut buffer,
            COMMAND_ROW,
            COMMAND_LABEL,
            prompt,
            default,
            input,
        );
        let staged_end_row = if staged_summary != "none" {
            draw_status_line(
                &mut buffer,
                STAGED_ROW,
                "Staged to Transfer: ",
                staged_summary,
            );
            draw_status_line(
                &mut buffer,
                STAGED_ROW + 1,
                "Remaining on Source: ",
                remaining_summary,
            );
            draw_status_line(
                &mut buffer,
                STAGED_ROW + 2,
                "Destination After Transfer: ",
                destination_summary,
            );
            STAGED_ROW + 2
        } else {
            draw_status_line(
                &mut buffer,
                STAGED_ROW,
                "Staged to Transfer: ",
                staged_summary,
            );
            STAGED_ROW
        };
        if let Some(status) = status {
            let status_row = (staged_end_row + 2).min(last_body_row());
            let max_rows = last_body_row().saturating_sub(status_row) + 1;
            if max_rows > 0 {
                draw_wrapped_message(&mut buffer, status_row, max_rows, "", status);
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
        _status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        let rows = FLEET_MISSION_OPTIONS
            .iter()
            .map(|option| {
                vec![
                    format!("{:02}", option.code),
                    option.mission.to_string(),
                    option.requirements.to_string(),
                ]
            })
            .collect::<Vec<_>>();
        let default = FLEET_MISSION_OPTIONS
            .get(cursor)
            .map(|option| option.code.to_string())
            .unwrap_or_else(|| "1".to_string());
        let footer = TableFooter::CommandBar {
            hotkeys_markup: "J K ^U ^D <Q>",
            default: Some(&default),
            input,
        };
        let columns = resolve_table_columns_for_widget(
            &mission_picker_columns(),
            &rows,
            buffer.width(),
            false,
            TableWidthMode::Compact,
            Some("FLEET MISSION ORDERS:"),
            Some(footer),
        );
        let layout = layout_standard_table_block(
            LayoutRect::new(0, 0, buffer.width(), buffer.height()),
            &columns,
            rows.len(),
            Some("FLEET MISSION ORDERS:"),
            Some(footer),
            false,
            HorizontalAlign::Center,
            VerticalAlign::Top,
        );
        let _ = layout.title_row;
        draw_table_title(
            &mut buffer,
            layout.table_row,
            layout.table_col,
            "FLEET MISSION ORDERS:",
        );
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
        let metrics = write_table_window_with_states_at(
            &mut buffer,
            layout.table_row,
            layout.table_col,
            &columns,
            &rows,
            0,
            rows.len(),
            classic::status_value_style(),
            classic::status_value_style(),
            Some(cursor),
            0,
            Some(&row_states),
        );
        draw_table_footer(
            &mut buffer,
            ScreenGeometry::local_default(),
            layout.command_col,
            metrics.bottom_row,
            footer,
        );
        Ok(buffer)
    }
}

impl FleetDetachScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        donor_row: &FleetRow,
        prompt: &str,
        default: &str,
        input: &str,
        staged_summary: &str,
        remaining_summary: &str,
        status: Option<&str>,
        last_commissioned: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        const FLEET_ROW: usize = 2;
        const LOCATION_ROW: usize = 4;
        const ORDERS_ROW: usize = 5;
        const TARGET_ROW: usize = 6;
        const SPEED_ROW: usize = 7;
        const ROE_ROW: usize = 8;
        const SHIPS_ROW: usize = 10;
        const ACTION_ROW: usize = 12;
        const COMMAND_ROW: usize = 14;
        const STAGED_ROW: usize = 16;

        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(0, 0, "DETACH FLEET SHIPS:", classic::title_style());
        draw_status_line(
            &mut buffer,
            FLEET_ROW,
            "Fleet: ",
            &format!("Fleet #{}", donor_row.fleet_number),
        );
        draw_status_line(
            &mut buffer,
            LOCATION_ROW,
            "Location: ",
            &format_sector_coords_table(donor_row.coords),
        );
        draw_status_line(&mut buffer, ORDERS_ROW, "Orders: ", &donor_row.order_label);
        draw_status_line(
            &mut buffer,
            TARGET_ROW,
            "Target: ",
            &fleet_list_target_label(donor_row.target_coords),
        );
        draw_status_line(
            &mut buffer,
            SPEED_ROW,
            "Speed: ",
            &donor_row.current_speed.to_string(),
        );
        draw_status_line(
            &mut buffer,
            ROE_ROW,
            "ROE: ",
            &donor_row.rules_of_engagement.to_string(),
        );
        draw_status_line(
            &mut buffer,
            SHIPS_ROW,
            "Ships: ",
            &donor_row.composition_label,
        );
        buffer.write_spans(
            ACTION_ROW,
            0,
            &[
                StyledSpan::new("<", classic::prompt_style()),
                StyledSpan::new("C", classic::prompt_hotkey_style()),
                StyledSpan::new(">ommission, <", classic::prompt_style()),
                StyledSpan::new("X", classic::prompt_hotkey_style()),
                StyledSpan::new("> Cancel", classic::prompt_style()),
            ],
        );
        draw_command_line_default_input_at(
            &mut buffer,
            COMMAND_ROW,
            COMMAND_LABEL,
            prompt,
            default,
            input,
        );
        let staged_rows = draw_wrapped_message(
            &mut buffer,
            STAGED_ROW,
            last_body_row().saturating_sub(STAGED_ROW) + 1,
            "Staged for New Fleet: ",
            staged_summary,
        );
        let staged_end_row = if staged_summary != "none" {
            let remaining_row = STAGED_ROW + staged_rows;
            let remaining_rows = draw_wrapped_message(
                &mut buffer,
                remaining_row,
                last_body_row().saturating_sub(remaining_row) + 1,
                "Remaining on Donor: ",
                remaining_summary,
            );
            remaining_row + remaining_rows.saturating_sub(1)
        } else {
            STAGED_ROW + staged_rows.saturating_sub(1)
        };
        if let Some(status) = status {
            let status_row = (staged_end_row + 2).min(last_body_row().saturating_sub(1));
            let max_rows = last_body_row().saturating_sub(status_row);
            if max_rows > 0 {
                draw_wrapped_message(&mut buffer, status_row, max_rows, "", status);
            }
        }
        if let Some(last_commissioned) = last_commissioned {
            draw_wrapped_message(&mut buffer, last_body_row(), 1, "", last_commissioned);
        }
        Ok(buffer)
    }
}

fn max_fleet_number(rows: &[FleetRow]) -> u16 {
    rows.iter().map(|row| row.fleet_number).max().unwrap_or(1)
}

fn format_selected_fleet_numbers(
    rows: &[FleetRow],
    selected_fleet_record_indexes: &BTreeSet<usize>,
) -> String {
    let mut fleet_numbers = rows
        .iter()
        .filter(|row| selected_fleet_record_indexes.contains(&row.fleet_record_index_1_based))
        .map(|row| row.fleet_number)
        .collect::<Vec<_>>();
    fleet_numbers.sort_unstable();
    if fleet_numbers.is_empty() {
        return "0".to_string();
    }
    fleet_numbers
        .into_iter()
        .map(|fleet_number| format!("{fleet_number:02}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn full_columns(max_fleet_number: u16) -> [TableColumn<'static>; 8] {
    let id_width = fleet_id_column_width(max_fleet_number);
    let ships_width = 71usize.saturating_sub(id_width + 8 + 15 + 8 + 3 + 4 + 3);
    [
        TableColumn::right("ID", id_width),
        TableColumn::left("Location", 8),
        TableColumn::left("Order", 15),
        TableColumn::left("Target", 8),
        TableColumn::right("Spd", 3),
        TableColumn::right("ETA", 4),
        TableColumn::right("ROE", 3),
        TableColumn::left("Ships", ships_width),
    ]
}

fn group_selection_columns(max_fleet_number: u16) -> [TableColumn<'static>; 9] {
    let id_width = fleet_id_column_width(max_fleet_number);
    let ships_width = 70usize.saturating_sub(id_width + 3 + 8 + 15 + 8 + 3 + 4 + 3);
    [
        TableColumn::right("ID", id_width),
        TableColumn::center("Sel", 3),
        TableColumn::left("Location", 8),
        TableColumn::left("Order", 15),
        TableColumn::left("Target", 8),
        TableColumn::right("Spd", 3),
        TableColumn::right("ETA", 4),
        TableColumn::right("ROE", 3),
        TableColumn::left("Ships", ships_width),
    ]
}

pub(crate) fn fleet_order_label(order_code: u8) -> &'static str {
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

pub(crate) fn fleet_table_order_label(order_code: u8) -> &'static str {
    match ec_data::Order::from_raw(order_code) {
        ec_data::Order::HoldPosition => "Hold",
        ec_data::Order::MoveOnly => "Move",
        ec_data::Order::SeekHome => "Seek",
        ec_data::Order::PatrolSector => "Patrol",
        ec_data::Order::GuardStarbase => "Grd SB",
        ec_data::Order::GuardBlockadeWorld => "Grd/Blkd",
        ec_data::Order::BombardWorld => "Bomb",
        ec_data::Order::InvadeWorld => "Invade",
        ec_data::Order::BlitzWorld => "Blitz",
        ec_data::Order::ViewWorld => "View",
        ec_data::Order::ScoutSector => "SC Sctr",
        ec_data::Order::ScoutSolarSystem => "SC Sys",
        ec_data::Order::ColonizeWorld => "Col",
        ec_data::Order::JoinAnotherFleet => "Join",
        ec_data::Order::RendezvousSector => "Rendez",
        ec_data::Order::Salvage => "Salvage",
        ec_data::Order::Unknown(_) => "Unknown",
    }
}

fn fleet_list_target_label(target_coords: [u8; 2]) -> String {
    if target_coords[0] == 0 || target_coords[1] == 0 {
        String::new()
    } else {
        format_sector_coords_table(target_coords)
    }
}

fn draw_confirm_prompt_at(buffer: &mut PlayfieldBuffer, row: usize, label: &str, input: &str) {
    buffer.fill_row(row, classic::prompt_style());
    let prefix = buffer.write_spans(
        row,
        0,
        &[
            StyledSpan::new(label, classic::title_style()),
            StyledSpan::new(" <- Confirm ", classic::prompt_style()),
            StyledSpan::new("[", classic::prompt_square_delimiter_style()),
            StyledSpan::new("Y", classic::prompt_hotkey_style()),
            StyledSpan::new("]", classic::prompt_square_delimiter_style()),
            StyledSpan::new("/N ", classic::prompt_style()),
            StyledSpan::new("<", classic::prompt_angle_delimiter_style()),
            StyledSpan::new("Q", classic::prompt_hotkey_style()),
            StyledSpan::new(">", classic::prompt_angle_delimiter_style()),
            StyledSpan::new(" -> ", classic::prompt_style()),
        ],
    );
    let written = buffer.write_text(row, prefix, input, classic::prompt_hotkey_style());
    buffer.set_cursor((prefix + written) as u16, row as u16);
}
