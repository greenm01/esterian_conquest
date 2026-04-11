use crossterm::event::{KeyCode, KeyEvent};
use nc_ui::table_filter::{TableFilterClause, is_filter_column_char};

use crate::app::Action;
use crate::app::helpers::is_coordinate_input_char;
use crate::domains::planet::PlanetAction;
use crate::screen::layout::{new_playfield_for, stacked_table_visible_rows_for};
use crate::screen::table::{
    HorizontalAlign, LayoutRect, TableColumn, TableFooter, TableWidthMode, VerticalAlign,
    draw_table_footer, draw_table_title, layout_stacked_table_block,
    resolve_table_columns_for_widget, write_stacked_table_window_with_states_at,
};
use crate::screen::{
    COMMAND_LABEL, CommandMenu, PlayfieldBuffer, ScreenGeometry, SortDirection,
    format_sector_coords_default, format_sector_coords_table,
};
use crate::theme::classic;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetDatabaseFilterMode {
    All,
    Range,
    Empire,
    MaxProduction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetDatabasePromptMode {
    FilterMenu,
    FilterValueInput,
    SortMenu,
    SortRangeInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetDatabaseFilter {
    All,
    Range { anchor: [u8; 2], radius: u8 },
    Empire(u8),
    MaxProduction(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetDatabaseSortMode {
    Location,
    Range,
    PlanetName,
    Owner,
    MaxProduction,
    YearSeen,
    Armies,
    Batteries,
    Starbases,
    CurrentProduction,
    Treasury,
    ScoutYear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetDatabaseSort {
    Location,
    Range([u8; 2]),
    PlanetName,
    Owner,
    MaxProduction,
    YearSeen,
    Armies,
    Batteries,
    Starbases,
    CurrentProduction,
    Treasury,
    ScoutYear,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetDatabaseRow {
    pub planet_record_index_1_based: usize,
    pub coords: [u8; 2],
    pub known_name: Option<String>,
    pub known_owner_empire_id: Option<u8>,
    pub known_owner_name: Option<String>,
    pub known_max_production: Option<u16>,
    pub known_year_seen: Option<u16>,
    pub known_armies: Option<u8>,
    pub known_batteries: Option<u8>,
    pub known_starbase_count: Option<u8>,
    pub known_current_production: Option<u8>,
    pub known_stored_points: Option<u16>,
    pub known_scout_year: Option<u16>,
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

struct RenderedPlanetDatabase {
    buffer: PlayfieldBuffer,
}

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
    "Coord", "", "", "Max", "Year", "", "", "", "Curr", "Trsry", "Year",
];

fn filter_prompt_dismiss_prompt(message: &str) -> String {
    format!("{message} (slap a key)")
}

fn database_title(
    _sort: PlanetDatabaseSort,
    direction: SortDirection,
    filter: PlanetDatabaseFilter,
    filter_clause: Option<&TableFilterClause>,
) -> String {
    format!(
        "TOTAL PLANET DATABASE: {} {}",
        direction.title_label(),
        filter_clause
            .map(|clause| clause.summary.as_str())
            .unwrap_or(filter_label(filter))
    )
}

fn filter_label(filter: PlanetDatabaseFilter) -> &'static str {
    match filter {
        PlanetDatabaseFilter::All => "ALL",
        PlanetDatabaseFilter::Range { .. } => "RNG",
        PlanetDatabaseFilter::Empire(_) => "EMP",
        PlanetDatabaseFilter::MaxProduction(_) => "MAX",
    }
}

impl PlanetDatabaseScreen {
    pub fn new() -> Self {
        Self
    }

    fn render_table(
        &mut self,
        geometry: ScreenGeometry,
        rows: &[PlanetDatabaseRow],
        sort: PlanetDatabaseSort,
        direction: SortDirection,
        filter: PlanetDatabaseFilter,
        filter_clause: Option<&TableFilterClause>,
        scroll_offset: usize,
        cursor: usize,
        footer: TableFooter<'_>,
    ) -> Result<RenderedPlanetDatabase, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield_for(geometry);
        let visible_rows = stacked_table_visible_rows_for(geometry, 1);
        let title = database_title(sort, direction, filter, filter_clause);

        let table_rows = database_table_rows(rows);
        let displayed_rows = table_rows
            .len()
            .saturating_sub(scroll_offset)
            .min(visible_rows);
        let scrollable = table_rows.len() > visible_rows;
        let columns = resolve_table_columns_for_widget(
            &DATABASE_COLUMNS,
            &table_rows,
            buffer.width(),
            scrollable,
            TableWidthMode::Compact,
            Some(&title),
            Some(footer),
        );
        let layout = layout_stacked_table_block(
            LayoutRect::new(0, 0, buffer.width(), buffer.height()),
            &columns,
            displayed_rows,
            Some(&title),
            Some(footer),
            scrollable,
            HorizontalAlign::Center,
            VerticalAlign::Top,
        );
        let _ = layout.title_row;
        draw_table_title(&mut buffer, layout.table_row, layout.table_col, &title);
        let selected = if table_rows.is_empty() {
            None
        } else {
            Some(cursor)
        };
        let metrics = write_stacked_table_window_with_states_at(
            &mut buffer,
            layout.table_row,
            layout.table_col,
            &DATABASE_TOP_HEADER_CELLS,
            &columns,
            &table_rows,
            scroll_offset,
            visible_rows,
            classic::status_value_style(),
            classic::status_value_style(),
            selected,
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
        Ok(RenderedPlanetDatabase { buffer })
    }

    pub fn render_list(
        &mut self,
        geometry: ScreenGeometry,
        rows: &[PlanetDatabaseRow],
        sort: PlanetDatabaseSort,
        direction: SortDirection,
        filter: PlanetDatabaseFilter,
        scroll_offset: usize,
        cursor: usize,
        default_coords: [u8; 2],
        input: &str,
        status: Option<&str>,
        menu: CommandMenu,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_list_with_filter_clause(
            geometry,
            rows,
            sort,
            direction,
            filter,
            None,
            scroll_offset,
            cursor,
            default_coords,
            input,
            status,
            menu,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_list_with_filter_clause(
        &mut self,
        geometry: ScreenGeometry,
        rows: &[PlanetDatabaseRow],
        sort: PlanetDatabaseSort,
        direction: SortDirection,
        filter: PlanetDatabaseFilter,
        filter_clause: Option<&TableFilterClause>,
        scroll_offset: usize,
        cursor: usize,
        _default_coords: [u8; 2],
        input: &str,
        _status: Option<&str>,
        _menu: CommandMenu,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let default = rows
            .get(cursor)
            .map(|row| format_sector_coords_default(row.coords))
            .unwrap_or_else(|| "00,00".to_string());
        let footer = if rows.is_empty() {
            TableFooter::CommandText {
                label: COMMAND_LABEL,
                text: if filter_clause.is_some() {
                    "No worlds match current filter."
                } else {
                    "No planets are in your database. Q quits."
                },
            }
        } else {
            TableFooter::CommandBar {
                hotkeys_markup: "? F S <Q>",
                default: Some(&default),
                input,
            }
        };
        Ok(self
            .render_table(
                geometry,
                rows,
                sort,
                direction,
                filter,
                filter_clause,
                scroll_offset,
                cursor,
                footer,
            )?
            .buffer)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_filter_prompt(
        &mut self,
        geometry: ScreenGeometry,
        rows: &[PlanetDatabaseRow],
        sort: PlanetDatabaseSort,
        direction: SortDirection,
        filter: PlanetDatabaseFilter,
        scroll_offset: usize,
        cursor: usize,
        prompt_mode: PlanetDatabasePromptMode,
        prompt_default: &str,
        input: &str,
        status: Option<&str>,
        menu: CommandMenu,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_filter_prompt_with_filter_clause(
            geometry,
            rows,
            sort,
            direction,
            filter,
            None,
            scroll_offset,
            cursor,
            prompt_mode,
            prompt_default,
            input,
            status,
            menu,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_filter_prompt_with_filter_clause(
        &mut self,
        geometry: ScreenGeometry,
        rows: &[PlanetDatabaseRow],
        sort: PlanetDatabaseSort,
        direction: SortDirection,
        filter: PlanetDatabaseFilter,
        filter_clause: Option<&TableFilterClause>,
        scroll_offset: usize,
        cursor: usize,
        prompt_mode: PlanetDatabasePromptMode,
        prompt_default: &str,
        input: &str,
        status: Option<&str>,
        menu: CommandMenu,
        dismiss_message: Option<&str>,
        pending_column_code: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let prompt_text;
        let footer = match prompt_mode {
            PlanetDatabasePromptMode::FilterMenu => {
                if let Some(message) = dismiss_message {
                    prompt_text = filter_prompt_dismiss_prompt(message);
                    TableFooter::CommandPrompt {
                        label: COMMAND_LABEL,
                        prompt: &prompt_text,
                    }
                } else {
                    prompt_text = status
                        .filter(|value| value.trim_start().starts_with("Ambiguous:"))
                        .map(|value| value.trim_start().to_string())
                        .unwrap_or_else(|| "Filter column [?] ".to_string());
                    TableFooter::CommandInput {
                        label: COMMAND_LABEL,
                        prompt: &prompt_text,
                        default: prompt_default,
                        input,
                    }
                }
            }
            PlanetDatabasePromptMode::FilterValueInput => {
                prompt_text = {
                    let mut prompt = format!("Filter {} ", pending_column_code.unwrap_or("value"));
                    if let Some(status) = status {
                        prompt.push_str(status);
                    }
                    prompt
                };
                TableFooter::CommandInput {
                    label: COMMAND_LABEL,
                    prompt: &prompt_text,
                    default: prompt_default,
                    input,
                }
            }
            PlanetDatabasePromptMode::SortMenu => {
                if let Some(message) = dismiss_message {
                    prompt_text = filter_prompt_dismiss_prompt(message);
                    TableFooter::CommandPrompt {
                        label: COMMAND_LABEL,
                        prompt: &prompt_text,
                    }
                } else {
                    prompt_text = status
                        .filter(|value| value.trim_start().starts_with("Ambiguous:"))
                        .map(|value| value.trim_start().to_string())
                        .unwrap_or_else(|| "Sort column [?] ".to_string());
                    TableFooter::CommandInput {
                        label: COMMAND_LABEL,
                        prompt: &prompt_text,
                        default: prompt_default,
                        input,
                    }
                }
            }
            PlanetDatabasePromptMode::SortRangeInput => TableFooter::CommandInput {
                label: COMMAND_LABEL,
                prompt: "Sort range from ",
                default: prompt_default,
                input,
            },
        };
        let _ = menu;
        Ok(self
            .render_table(
                geometry,
                rows,
                sort,
                direction,
                filter,
                filter_clause,
                scroll_offset,
                cursor,
                footer,
            )?
            .buffer)
    }

    pub fn handle_list_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::Planet(PlanetAction::MoveDatabaseList(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::Planet(PlanetAction::MoveDatabaseList(1))
            }
            KeyCode::PageUp => Action::Planet(PlanetAction::PageDatabaseList(1)),
            KeyCode::PageDown => Action::Planet(PlanetAction::PageDatabaseList(-1)),
            KeyCode::Char(ch) if is_coordinate_input_char(ch) => {
                Action::Planet(PlanetAction::AppendDatabaseChar(ch))
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                Action::Planet(PlanetAction::OpenDatabaseFilterPrompt)
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                Action::Planet(PlanetAction::OpenDatabaseSortPrompt)
            }
            KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceDatabaseInput),
            KeyCode::Enter => Action::Planet(PlanetAction::SubmitDatabaseLookup),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::ReturnToCommandMenu,
            _ => Action::Noop,
        }
    }

    pub fn handle_filter_prompt_key(&self, key: KeyEvent) -> Action {
        self.handle_filter_prompt_key_for_mode(key, PlanetDatabasePromptMode::SortMenu)
    }

    pub fn handle_filter_prompt_key_for_mode(
        &self,
        key: KeyEvent,
        prompt_mode: PlanetDatabasePromptMode,
    ) -> Action {
        match prompt_mode {
            PlanetDatabasePromptMode::FilterMenu => match key.code {
                KeyCode::Char('?') => Action::OpenPopupHelp,
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    Action::Planet(PlanetAction::OpenDatabase)
                }
                KeyCode::Enter => Action::Planet(PlanetAction::SubmitDatabaseFilterPrompt),
                KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceDatabaseInput),
                KeyCode::Char(ch) if is_filter_column_char(ch) => {
                    Action::Planet(PlanetAction::AppendDatabaseChar(ch))
                }
                _ => Action::Noop,
            },
            PlanetDatabasePromptMode::FilterValueInput => match key.code {
                KeyCode::Char('?') => Action::OpenPopupHelp,
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    Action::Planet(PlanetAction::OpenDatabase)
                }
                KeyCode::Enter => Action::Planet(PlanetAction::SubmitDatabaseFilterPrompt),
                KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceDatabaseInput),
                KeyCode::Char(ch)
                    if matches!(
                        ch,
                        ' ' | '-' | '#' | '*' | '/' | '?' | '=' | '!' | '>' | '<' | '+' | ','
                    ) || ch.is_ascii_alphanumeric() =>
                {
                    Action::Planet(PlanetAction::AppendDatabaseChar(ch))
                }
                _ => Action::Noop,
            },
            PlanetDatabasePromptMode::SortMenu => match key.code {
                KeyCode::Char('?') => Action::OpenPopupHelp,
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    Action::Planet(PlanetAction::OpenDatabase)
                }
                KeyCode::Enter => Action::Planet(PlanetAction::SubmitDatabaseSortPrompt),
                KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceDatabaseInput),
                KeyCode::Char(ch) if is_filter_column_char(ch) => {
                    Action::Planet(PlanetAction::AppendDatabaseChar(ch))
                }
                _ => Action::Noop,
            },
            PlanetDatabasePromptMode::SortRangeInput => match key.code {
                KeyCode::Char('?') => Action::OpenPopupHelp,
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    Action::Planet(PlanetAction::OpenDatabase)
                }
                KeyCode::Enter => Action::Planet(PlanetAction::SubmitDatabaseSort(
                    PlanetDatabaseSortMode::Range,
                )),
                KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceDatabaseInput),
                KeyCode::Char(ch) if is_coordinate_input_char(ch) => {
                    Action::Planet(PlanetAction::AppendDatabaseChar(ch))
                }
                _ => Action::Noop,
            },
        }
    }
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
