use crossterm::event::{KeyCode, KeyEvent};
use ec_data::{EmpirePlanetEconomyRow, STARDOCK_SLOT_COUNT};

use crate::app::Action;
use crate::domains::planet::PlanetAction;
use crate::screen::layout::{
    dismiss_prompt_row, draw_dismiss_prompt, draw_status_line, draw_table_command_bar_at_col,
    draw_table_command_prompt_at_col, draw_title_bar, new_playfield, stacked_table_visible_rows,
    table_prompt_row,
};
use crate::screen::table::{
    TableColumn, centered_table_start_col, fit_table_columns,
    write_stacked_table_window_with_states_at,
};
use crate::screen::{
    PlayfieldBuffer, ScreenFrame, format_sector_coords_default, format_sector_coords_table,
};
use crate::theme::classic;

pub const PLANET_BRIEF_VISIBLE_ROWS: usize = stacked_table_visible_rows(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetListMode {
    Brief,
    BuildSelect,
    Stub(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetListSort {
    CurrentProduction,
    Location,
    PotentialProduction,
}

pub struct PlanetListScreen;

const BRIEF_COLUMNS: [TableColumn<'static>; 11] = [
    TableColumn::left("(XX,YY)", 7),
    TableColumn::left("Planet Name", 11),
    TableColumn::right("Prod", 4),
    TableColumn::right("Prod", 4),
    TableColumn::right("Points", 6),
    TableColumn::right("Rev", 3),
    TableColumn::right("Grow", 4),
    TableColumn::right("Docked", 6),
    TableColumn::right("SBs", 3),
    TableColumn::right("ARs", 3),
    TableColumn::right("GBs", 3),
];

const BRIEF_TOP_HEADER_CELLS: [&str; 11] =
    ["Coord", "", "Max", "Curr", "Stored", "", "", "", "", "", ""];

const BRIEF_SORT_PROMPT: &str = "Sort <C>, <L>, <M>, or <Q>? [C] ->";

impl PlanetListScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_sort_prompt(
        &mut self,
        frame: &ScreenFrame<'_>,
        mode: PlanetListMode,
        rows: &[EmpirePlanetEconomyRow],
        sort: PlanetListSort,
        scroll_offset: usize,
        cursor: usize,
        input: &str,
        _status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        if let PlanetListMode::Stub(message) = mode {
            let mut buffer = new_playfield();
            draw_title_bar(&mut buffer, 0, "PLANET COMMAND:");
            draw_status_line(&mut buffer, 3, "Notice: ", message);
            draw_dismiss_prompt(&mut buffer, dismiss_prompt_row(3));
            return Ok(buffer);
        }

        let mut buffer =
            self.render_brief_list(frame, mode, rows, sort, scroll_offset, cursor, input)?;
        let columns = fit_table_columns(&BRIEF_COLUMNS, &planet_table_rows(frame, rows));
        let start_col = centered_table_start_col(buffer.width(), &columns);
        let command_row = brief_list_command_row(rows.len(), scroll_offset);
        draw_table_command_prompt_at_col(&mut buffer, command_row, start_col, BRIEF_SORT_PROMPT);
        Ok(buffer)
    }

    pub fn render_brief_list(
        &mut self,
        frame: &ScreenFrame<'_>,
        mode: PlanetListMode,
        rows: &[EmpirePlanetEconomyRow],
        _sort: PlanetListSort,
        scroll_offset: usize,
        cursor: usize,
        input: &str,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        let table_rows = planet_table_rows(frame, rows);
        let columns = fit_table_columns(&BRIEF_COLUMNS, &table_rows);
        let start_col = centered_table_start_col(buffer.width(), &columns);
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(0, start_col, brief_list_title(mode), classic::title_style());

        let metrics = write_stacked_table_window_with_states_at(
            &mut buffer,
            1,
            start_col,
            &BRIEF_TOP_HEADER_CELLS,
            &columns,
            &table_rows,
            scroll_offset,
            PLANET_BRIEF_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
            if table_rows.is_empty() {
                None
            } else {
                Some(cursor)
            },
            None,
        );

        let default_coords = rows
            .get(cursor)
            .map(|row| format_sector_coords_default(row.coords))
            .unwrap_or_else(|| "00,00".to_string());
        draw_table_command_bar_at_col(
            &mut buffer,
            table_prompt_row(metrics.bottom_row),
            start_col,
            "<ARROWS J K S Q>",
            Some(&default_coords),
            input,
        );
        Ok(buffer)
    }

    pub fn handle_sort_prompt_key(&self, key: KeyEvent, mode: PlanetListMode) -> Action {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Enter => Action::Planet(
                PlanetAction::SubmitListSort(mode, PlanetListSort::CurrentProduction),
            ),
            KeyCode::Char('l') | KeyCode::Char('L') => {
                Action::Planet(PlanetAction::SubmitListSort(mode, PlanetListSort::Location))
            }
            KeyCode::Char('m') | KeyCode::Char('M') => Action::Planet(
                PlanetAction::SubmitListSort(mode, PlanetListSort::PotentialProduction),
            ),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Planet(PlanetAction::CloseListSortPrompt(mode))
            }
            _ => Action::Noop,
        }
    }

    pub fn handle_brief_key(&self, key: KeyEvent, mode: PlanetListMode) -> Action {
        match key.code {
            KeyCode::Up => Action::Planet(PlanetAction::MoveBrief(-1)),
            KeyCode::Down => Action::Planet(PlanetAction::MoveBrief(1)),
            KeyCode::PageUp => Action::Planet(PlanetAction::MoveBrief(-5)),
            KeyCode::PageDown => Action::Planet(PlanetAction::MoveBrief(5)),
            KeyCode::Char('k') | KeyCode::Char('K') => Action::Planet(PlanetAction::MoveBrief(-1)),
            KeyCode::Char('j') | KeyCode::Char('J') => Action::Planet(PlanetAction::MoveBrief(1)),
            KeyCode::Char('s') | KeyCode::Char('S') => {
                Action::Planet(PlanetAction::OpenListSortPrompt(mode))
            }
            KeyCode::Enter => Action::Planet(PlanetAction::SubmitBriefInput),
            KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceBriefInput),
            KeyCode::Char(ch) if ch.is_ascii_digit() || ch == ',' || ch == ' ' => {
                Action::Planet(PlanetAction::AppendBriefChar(ch))
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => match mode {
                PlanetListMode::Brief => Action::Planet(PlanetAction::OpenMenu),
                PlanetListMode::BuildSelect => Action::Planet(PlanetAction::OpenBuildMenu),
                PlanetListMode::Stub(_) => Action::Planet(PlanetAction::OpenMenu),
            },
            _ => Action::Noop,
        }
    }
}

fn brief_list_command_row(total_rows: usize, scroll_offset: usize) -> usize {
    let displayed_rows = total_rows
        .saturating_sub(scroll_offset)
        .min(PLANET_BRIEF_VISIBLE_ROWS);
    table_prompt_row(1 + 4 + displayed_rows)
}

fn brief_list_title(mode: PlanetListMode) -> &'static str {
    match mode {
        PlanetListMode::Brief | PlanetListMode::Stub(_) => "PLANET COMMAND:",
        PlanetListMode::BuildSelect => "CHANGE CURRENT PLANET:",
    }
}

fn planet_table_rows(frame: &ScreenFrame<'_>, rows: &[EmpirePlanetEconomyRow]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| {
            vec![
                format_sector_coords_table(row.coords),
                row.planet_name.clone(),
                row.potential_production.to_string(),
                row.present_production.to_string(),
                row.stored_production_points.to_string(),
                row.yearly_tax_revenue.to_string(),
                format_signed_growth(row.yearly_growth_delta),
                docked_units(frame, row).to_string(),
                if row.has_friendly_starbase {
                    "1".to_string()
                } else {
                    "0".to_string()
                },
                row.armies.to_string(),
                row.ground_batteries.to_string(),
            ]
        })
        .collect()
}

fn format_signed_growth(growth: u16) -> String {
    format!("+{growth}")
}

fn docked_units(frame: &ScreenFrame<'_>, row: &EmpirePlanetEconomyRow) -> u32 {
    frame
        .game_data
        .planets
        .records
        .get(row.planet_record_index_1_based.saturating_sub(1))
        .map(|planet| {
            (0..STARDOCK_SLOT_COUNT)
                .map(|slot| u32::from(planet.stardock_count_raw(slot)))
                .sum()
        })
        .unwrap_or(0)
}
