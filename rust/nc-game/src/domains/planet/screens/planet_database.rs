use crossterm::event::{KeyCode, KeyEvent};

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

const DATABASE_FILTER_HOTKEYS: &str = "? A R E M <Q>";
const DATABASE_SORT_HOTKEYS: &str = "? L R E M <Q>";

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
    FilterRangeCoords,
    FilterRangeDistance,
    FilterEmpireInput,
    FilterMaxProductionInput,
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
    Empire,
    MaxProduction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetDatabaseSort {
    Location,
    Range([u8; 2]),
    Empire,
    MaxProduction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetDatabaseRow {
    pub planet_record_index_1_based: usize,
    pub coords: [u8; 2],
    pub known_owner_empire_id: Option<u8>,
    pub known_max_production: Option<u16>,
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

fn database_title(
    sort: PlanetDatabaseSort,
    direction: SortDirection,
    filter: PlanetDatabaseFilter,
) -> String {
    let key = match sort {
        PlanetDatabaseSort::Location => "LOC",
        PlanetDatabaseSort::Range(_) => "RNG",
        PlanetDatabaseSort::Empire => "EMP",
        PlanetDatabaseSort::MaxProduction => "MAX",
    };
    format!(
        "TOTAL PLANET DATABASE: {key} {} {}",
        direction.label(),
        filter_label(filter)
    )
}

fn sort_footer_label(direction: SortDirection) -> String {
    format!("SORT {}", direction.label())
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
        scroll_offset: usize,
        cursor: usize,
        footer: TableFooter<'_>,
    ) -> Result<RenderedPlanetDatabase, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield_for(geometry);
        let visible_rows = stacked_table_visible_rows_for(geometry, 1);
        let title = database_title(sort, direction, filter);

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
                text: "No planets are in your database. Q quits.",
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
                scroll_offset,
                cursor,
                footer,
            )?
            .buffer)
    }

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
        let footer_label = sort_footer_label(direction);
        let footer = match prompt_mode {
            PlanetDatabasePromptMode::FilterMenu => TableFooter::LabeledCommandBar {
                label: "FILTER",
                hotkeys_markup: DATABASE_FILTER_HOTKEYS,
                default: None,
                input: "",
            },
            PlanetDatabasePromptMode::FilterRangeCoords => TableFooter::CommandInput {
                label: COMMAND_LABEL,
                prompt: "Range from ",
                default: prompt_default,
                input,
            },
            PlanetDatabasePromptMode::FilterRangeDistance => TableFooter::CommandInput {
                label: COMMAND_LABEL,
                prompt: "Range radius ",
                default: prompt_default,
                input,
            },
            PlanetDatabasePromptMode::FilterEmpireInput => TableFooter::CommandInput {
                label: COMMAND_LABEL,
                prompt: "Empire ",
                default: prompt_default,
                input,
            },
            PlanetDatabasePromptMode::FilterMaxProductionInput => TableFooter::CommandInput {
                label: COMMAND_LABEL,
                prompt: "Max production at least ",
                default: prompt_default,
                input,
            },
            PlanetDatabasePromptMode::SortMenu => TableFooter::LabeledCommandBar {
                label: &footer_label,
                hotkeys_markup: DATABASE_SORT_HOTKEYS,
                default: None,
                input: "",
            },
            PlanetDatabasePromptMode::SortRangeInput => TableFooter::CommandInput {
                label: COMMAND_LABEL,
                prompt: "Sort range from ",
                default: prompt_default,
                input,
            },
        };
        let _ = status;
        let _ = menu;
        Ok(self
            .render_table(
                geometry,
                rows,
                sort,
                direction,
                filter,
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
                KeyCode::Enter | KeyCode::Char('a') | KeyCode::Char('A') => Action::Planet(
                    PlanetAction::SubmitDatabaseFilter(PlanetDatabaseFilterMode::All),
                ),
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
            },
            PlanetDatabasePromptMode::FilterRangeCoords => match key.code {
                KeyCode::Char('?') => Action::OpenPopupHelp,
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    Action::Planet(PlanetAction::OpenDatabase)
                }
                KeyCode::Enter => Action::Planet(PlanetAction::SubmitDatabaseFilter(
                    PlanetDatabaseFilterMode::Range,
                )),
                KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceDatabaseInput),
                KeyCode::Char(ch) if is_coordinate_input_char(ch) => {
                    Action::Planet(PlanetAction::AppendDatabaseChar(ch))
                }
                _ => Action::Noop,
            },
            PlanetDatabasePromptMode::FilterRangeDistance => match key.code {
                KeyCode::Char('?') => Action::OpenPopupHelp,
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    Action::Planet(PlanetAction::OpenDatabase)
                }
                KeyCode::Enter => Action::Planet(PlanetAction::SubmitDatabaseFilter(
                    PlanetDatabaseFilterMode::Range,
                )),
                KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceDatabaseInput),
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    Action::Planet(PlanetAction::AppendDatabaseChar(ch))
                }
                _ => Action::Noop,
            },
            PlanetDatabasePromptMode::FilterEmpireInput => match key.code {
                KeyCode::Char('?') => Action::OpenPopupHelp,
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    Action::Planet(PlanetAction::OpenDatabase)
                }
                KeyCode::Enter => Action::Planet(PlanetAction::SubmitDatabaseFilter(
                    PlanetDatabaseFilterMode::Empire,
                )),
                KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceDatabaseInput),
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    Action::Planet(PlanetAction::AppendDatabaseChar(ch))
                }
                _ => Action::Noop,
            },
            PlanetDatabasePromptMode::FilterMaxProductionInput => match key.code {
                KeyCode::Char('?') => Action::OpenPopupHelp,
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    Action::Planet(PlanetAction::OpenDatabase)
                }
                KeyCode::Enter => Action::Planet(PlanetAction::SubmitDatabaseFilter(
                    PlanetDatabaseFilterMode::MaxProduction,
                )),
                KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceDatabaseInput),
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    Action::Planet(PlanetAction::AppendDatabaseChar(ch))
                }
                _ => Action::Noop,
            },
            PlanetDatabasePromptMode::SortMenu => match key.code {
                KeyCode::Char('?') => Action::OpenPopupHelp,
                KeyCode::Enter | KeyCode::Char('l') | KeyCode::Char('L') => Action::Planet(
                    PlanetAction::SubmitDatabaseSort(PlanetDatabaseSortMode::Location),
                ),
                KeyCode::Char('r') | KeyCode::Char('R') => Action::Planet(
                    PlanetAction::SubmitDatabaseSort(PlanetDatabaseSortMode::Range),
                ),
                KeyCode::Char('e') | KeyCode::Char('E') => Action::Planet(
                    PlanetAction::SubmitDatabaseSort(PlanetDatabaseSortMode::Empire),
                ),
                KeyCode::Char('m') | KeyCode::Char('M') => Action::Planet(
                    PlanetAction::SubmitDatabaseSort(PlanetDatabaseSortMode::MaxProduction),
                ),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    Action::Planet(PlanetAction::OpenDatabase)
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
