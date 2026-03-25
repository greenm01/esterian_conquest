use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::domains::planet::PlanetAction;
use crate::screen::layout::{
    draw_command_line_text_at_col, draw_inline_status_after, draw_table_command_bar_at_col,
    draw_table_command_prompt_at_col, new_playfield, stacked_table_visible_rows, table_prompt_row,
};
use crate::screen::table::{
    TableColumn, centered_table_start_col, fit_table_columns,
    write_stacked_table_window_with_states_at,
};
use crate::screen::{
    CommandMenu, PlayfieldBuffer, format_sector_coords_default, format_sector_coords_table,
};
use crate::theme::classic;

pub const PLANET_DATABASE_VISIBLE_ROWS: usize = stacked_table_visible_rows(1);

const DATABASE_FILTER_PROMPT: &str =
    "Filter by <L>ocation, <R>ange, <E>mpire, <M>ax Prod, or <Q>uit? [L] ->";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetDatabaseFilterMode {
    Range,
    Empire,
    MaxProduction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetDatabaseRow {
    pub planet_record_index_1_based: usize,
    pub coords: [u8; 2],
    pub name_label: String,
    pub owner_label: String,
    pub max_prod_label: String,
    pub year_seen_label: String,
    pub armies_label: String,
    pub batteries_label: String,
    pub starbase_count_label: String,
    pub current_prod_label: String,
    pub stored_points_label: String,
    pub year_scout_label: String,
}

pub struct PlanetDatabaseScreen;

const DATABASE_COLUMNS: [TableColumn<'static>; 11] = [
    TableColumn::left("(XX,YY)", 7),
    TableColumn::left("Planet Name", 11),
    TableColumn::left("Owner", 5),
    TableColumn::right("Prod", 4),
    TableColumn::right("Seen", 4),
    TableColumn::right("ARs", 3),
    TableColumn::right("GBs", 3),
    TableColumn::right("SBs", 3),
    TableColumn::right("Prod", 4),
    TableColumn::right("Points", 6),
    TableColumn::right("Scout", 5),
];

const DATABASE_TOP_HEADER_CELLS: [&str; 11] = [
    "Coord", "", "", "Max", "Year", "", "", "", "Curr", "Stored", "Year",
];

impl PlanetDatabaseScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_list(
        &mut self,
        rows: &[PlanetDatabaseRow],
        scroll_offset: usize,
        cursor: usize,
        _default_coords: [u8; 2],
        input: &str,
        status: Option<&str>,
        _menu: CommandMenu,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();

        let table_rows = database_table_rows(rows);
        let columns = fit_table_columns(&DATABASE_COLUMNS, &table_rows);
        let start_col = centered_table_start_col(buffer.width(), &columns);
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(
            0,
            start_col,
            "TOTAL PLANET DATABASE:",
            classic::title_style(),
        );
        let selected = if table_rows.is_empty() {
            None
        } else {
            Some(cursor)
        };
        let metrics = write_stacked_table_window_with_states_at(
            &mut buffer,
            1,
            start_col,
            &DATABASE_TOP_HEADER_CELLS,
            &columns,
            &table_rows,
            scroll_offset,
            PLANET_DATABASE_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
            selected,
            None,
        );

        let command_row = table_prompt_row(metrics.bottom_row);
        if rows.is_empty() {
            draw_command_line_text_at_col(
                &mut buffer,
                command_row,
                start_col,
                "COMMANDS",
                "No planets are in your database. Q quits.",
            );
        } else {
            let default = rows
                .get(cursor)
                .map(|row| format_sector_coords_default(row.coords))
                .unwrap_or_else(|| "00,00".to_string());
            draw_table_command_bar_at_col(
                &mut buffer,
                command_row,
                start_col,
                "<ARROWS J K F Q>",
                Some(&default),
                input,
            );
            if let Some(status) = status {
                draw_inline_status_after(&mut buffer, command_row, status);
            }
        }
        Ok(buffer)
    }

    pub fn render_filter_prompt(
        &mut self,
        rows: &[PlanetDatabaseRow],
        scroll_offset: usize,
        cursor: usize,
        default_coords: [u8; 2],
        input: &str,
        status: Option<&str>,
        menu: CommandMenu,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = self.render_list(
            rows,
            scroll_offset,
            cursor,
            default_coords,
            input,
            status,
            menu,
        )?;
        let columns = fit_table_columns(&DATABASE_COLUMNS, &database_table_rows(rows));
        let start_col = centered_table_start_col(buffer.width(), &columns);
        draw_table_command_prompt_at_col(
            &mut buffer,
            database_command_row(rows.len(), scroll_offset),
            start_col,
            DATABASE_FILTER_PROMPT,
        );
        Ok(buffer)
    }

    pub fn handle_list_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::Planet(PlanetAction::MoveDatabaseList(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::Planet(PlanetAction::MoveDatabaseList(1))
            }
            KeyCode::PageUp => Action::Planet(PlanetAction::MoveDatabaseList(-8)),
            KeyCode::PageDown => Action::Planet(PlanetAction::MoveDatabaseList(8)),
            KeyCode::Char(ch) if ch.is_ascii_digit() || ch == ',' || ch == ' ' => {
                Action::Planet(PlanetAction::AppendDatabaseChar(ch))
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                Action::Planet(PlanetAction::OpenDatabaseFilterPrompt)
            }
            KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceDatabaseInput),
            KeyCode::Enter => Action::Planet(PlanetAction::SubmitDatabaseLookup),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::ReturnToCommandMenu,
            _ => Action::Noop,
        }
    }

    pub fn handle_filter_prompt_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Enter | KeyCode::Char('l') | KeyCode::Char('L') => {
                Action::Planet(PlanetAction::OpenDatabase)
            }
            KeyCode::Char('r') | KeyCode::Char('R') => Action::Planet(
                PlanetAction::SubmitDatabaseFilter(PlanetDatabaseFilterMode::Range),
            ),
            KeyCode::Char('e') | KeyCode::Char('E') => Action::Planet(
                PlanetAction::SubmitDatabaseFilter(PlanetDatabaseFilterMode::Empire),
            ),
            KeyCode::Char('m') | KeyCode::Char('M') => Action::Planet(
                PlanetAction::SubmitDatabaseFilter(PlanetDatabaseFilterMode::MaxProduction),
            ),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Planet(PlanetAction::OpenDatabase)
            }
            _ => Action::Noop,
        }
    }
}

fn database_command_row(total_rows: usize, scroll_offset: usize) -> usize {
    let displayed_rows = total_rows
        .saturating_sub(scroll_offset)
        .min(PLANET_DATABASE_VISIBLE_ROWS);
    table_prompt_row(1 + 4 + displayed_rows)
}

fn database_table_rows(rows: &[PlanetDatabaseRow]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| {
            vec![
                format_sector_coords_table(row.coords),
                row.name_label.clone(),
                row.owner_label.clone(),
                row.max_prod_label.clone(),
                row.year_seen_label.clone(),
                row.armies_label.clone(),
                row.batteries_label.clone(),
                row.starbase_count_label.clone(),
                row.current_prod_label.clone(),
                row.stored_points_label.clone(),
                row.year_scout_label.clone(),
            ]
        })
        .collect()
}
