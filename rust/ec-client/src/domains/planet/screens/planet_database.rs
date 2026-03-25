use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::domains::planet::PlanetAction;
use crate::screen::layout::{
    draw_command_line_text_at, draw_inline_status_after, draw_table_command_bar_at,
    draw_table_command_prompt_at, draw_title_bar, new_playfield, standard_table_visible_rows,
    table_prompt_row,
};
use crate::screen::{
    CommandMenu, PlayfieldBuffer, format_sector_coords_default, format_sector_coords_table,
};

pub const PLANET_DATABASE_VISIBLE_ROWS: usize = standard_table_visible_rows(1);

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
    pub current_prod_label: String,
    pub stored_points_label: String,
    pub year_scout_label: String,
    pub intel_label: String,
}

pub struct PlanetDatabaseScreen;

use crate::screen::table::{TableColumn, write_table_window_with_states};

const DATABASE_COLUMNS: [TableColumn<'static>; 11] = [
    TableColumn::left("(X,Y)", 8),
    TableColumn::left("Planet Name", 19),
    TableColumn::left("Owner", 5),
    TableColumn::right("Max", 4),
    TableColumn::right("Seen", 4),
    TableColumn::right("AR", 3),
    TableColumn::right("GB", 3),
    TableColumn::right("Curr", 4),
    TableColumn::right("Points", 6),
    TableColumn::right("Scout", 5),
    TableColumn::left("Intel", 6),
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
        draw_title_bar(&mut buffer, 0, "TOTAL PLANET DATABASE:");

        let table_rows = rows
            .iter()
            .map(|row| {
                vec![
                    format_sector_coords_table(row.coords),
                    row.name_label.clone(),
                    row.owner_label.clone(),
                    row.max_prod_label.clone(),
                    row.year_seen_label.clone(),
                    row.armies_label.clone(),
                    row.batteries_label.clone(),
                    row.current_prod_label.clone(),
                    row.stored_points_label.clone(),
                    row.year_scout_label.clone(),
                    row.intel_label.clone(),
                ]
            })
            .collect::<Vec<_>>();
        let selected = if table_rows.is_empty() {
            None
        } else {
            Some(cursor)
        };
        let metrics = write_table_window_with_states(
            &mut buffer,
            1,
            &DATABASE_COLUMNS,
            &table_rows,
            scroll_offset,
            PLANET_DATABASE_VISIBLE_ROWS,
            crate::theme::classic::status_value_style(),
            crate::theme::classic::status_value_style(),
            selected,
            None,
        );

        let command_row = table_prompt_row(metrics.bottom_row);
        if rows.is_empty() {
            draw_command_line_text_at(
                &mut buffer,
                command_row,
                "COMMANDS",
                "No planets are in your database. Q quits.",
            );
        } else {
            let default = rows
                .get(cursor)
                .map(|row| format_sector_coords_default(row.coords))
                .unwrap_or_else(|| "00,00".to_string());
            draw_table_command_bar_at(
                &mut buffer,
                command_row,
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
        draw_table_command_prompt_at(
            &mut buffer,
            database_command_row(rows.len(), scroll_offset),
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
    table_prompt_row(1 + 3 + displayed_rows)
}
