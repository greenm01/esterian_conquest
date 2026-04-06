use crossterm::event::{KeyCode, KeyEvent};
use nc_data::{EmpirePlanetEconomyRow, STARDOCK_SLOT_COUNT};

use crate::app::Action;
use crate::app::helpers::is_coordinate_input_char;
use crate::domains::planet::PlanetAction;
use crate::screen::layout::{
    dismiss_prompt_row_for, draw_dismiss_prompt_padded, draw_status_line, draw_title_bar_padded,
    new_playfield_for, stacked_table_visible_rows_for,
};
use crate::screen::table::{
    HorizontalAlign, LayoutRect, TableColumn, TableFooter, TableWidthMode, VerticalAlign,
    draw_table_footer, draw_table_title, layout_stacked_table_block,
    resolve_table_columns_for_widget_with_footer_floor, table_footer_scaffold_width,
    write_stacked_table_window_with_states_at,
};
use crate::screen::{
    PlayfieldBuffer, ScreenFrame, build_quantity_from_points, format_sector_coords_default,
    format_sector_coords_table,
};
use crate::theme::classic;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetListFilterMode {
    All,
    Range,
    Starbase,
    Stardock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetListFilter {
    All,
    Range { anchor: [u8; 2], radius: u8 },
    Starbase,
    Stardock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetListFilterPromptMode {
    FilterMenu,
    RangeCoords,
    RangeDistance,
}

pub struct PlanetListScreen;

const BRIEF_COLUMNS: [TableColumn<'static>; 12] = [
    TableColumn::left("(XX,YY)", 7),
    TableColumn::left("Planet Name", 11),
    TableColumn::right("Prod", 4),
    TableColumn::right("Prod", 4),
    TableColumn::right("Points", 6),
    TableColumn::right("Rev", 3),
    TableColumn::right("Grow", 4),
    TableColumn::right("Queue", 6),
    TableColumn::right("Dock", 6),
    TableColumn::right("SBs", 3),
    TableColumn::right("ARs", 3),
    TableColumn::right("GBs", 3),
];

const BRIEF_TOP_HEADER_CELLS: [&str; 12] = [
    "Coord", "", "Max", "Curr", "Stored", "", "", "Build", "Star", "", "", "",
];

const BRIEF_BROWSE_HOTKEYS: &str = "? F S B A C L U X <Q>";
const BRIEF_SORT_HOTKEYS: &str = "? C L M <Q>";
const BRIEF_FILTER_HOTKEYS: &str = "? A R S T <Q>";
pub(crate) const PLANET_LIST_AUTO_COMMISSION_PROMPT: &str =
    "Auto-Commission: Commission all ships and starbases? [Y]/N -> ";
pub(crate) const PLANET_LIST_LOAD_FLEET_PROMPT: &str = "Load Armies: Fleet # ";
pub(crate) const PLANET_LIST_UNLOAD_FLEET_PROMPT: &str = "Unload Armies: Fleet # ";
pub(crate) const PLANET_LIST_LOAD_QTY_PROMPT: &str = "Load Armies: How many armies? ";
pub(crate) const PLANET_LIST_UNLOAD_QTY_PROMPT: &str = "Unload Armies: How many armies? ";
pub(crate) const PLANET_LIST_SCORCH_CONFIRM_PROMPT: &str = "Scorch Planet: Are you sure? Y/[N] -> ";
pub(crate) const PLANET_LIST_SCORCH_REALLY_CONFIRM_PROMPT: &str =
    "Scorch Planet: Are you really sure? Y/[N] -> ";
pub(crate) const PLANET_LIST_SCORCH_LAST_CONFIRM_PROMPT: &str =
    "Scorch Planet: Are you sure-sure? Y/[N] -> ";
const PLANET_LIST_MAX_QTY_DEFAULT: &str = "255";

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
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        if let PlanetListMode::Stub(message) = mode {
            let mut buffer = new_playfield_for(frame.geometry);
            draw_title_bar_padded(&mut buffer, 0, "PLANET COMMAND:");
            draw_status_line(&mut buffer, 3, "Notice: ", message);
            draw_dismiss_prompt_padded(&mut buffer, dismiss_prompt_row_for(frame.geometry, 3));
            return Ok(buffer);
        }

        let mut buffer = self.render_brief_list(
            frame,
            mode,
            rows,
            sort,
            scroll_offset,
            cursor,
            input,
            status,
            false,
            None,
            "",
            "",
            None,
            None,
            None,
        )?;
        let table_rows = planet_table_rows(frame, rows);
        let visible_rows = stacked_table_visible_rows_for(frame.geometry, 1);
        let displayed_rows = table_rows
            .len()
            .saturating_sub(scroll_offset)
            .min(visible_rows);
        let scrollable = table_rows.len() > visible_rows;
        let footer = TableFooter::LabeledCommandBar {
            label: "SORT",
            hotkeys_markup: BRIEF_SORT_HOTKEYS,
            default: None,
            input: "",
        };
        let columns = resolve_table_columns_for_widget_with_footer_floor(
            &BRIEF_COLUMNS,
            &table_rows,
            buffer.width(),
            scrollable,
            TableWidthMode::Compact,
            Some(brief_list_title(mode)),
            Some(footer),
            planet_list_footer_floor(frame, mode),
        );
        let layout = layout_stacked_table_block(
            LayoutRect::new(0, 0, buffer.width(), buffer.height()),
            &columns,
            displayed_rows,
            Some(brief_list_title(mode)),
            Some(footer),
            scrollable,
            HorizontalAlign::Center,
            VerticalAlign::Top,
        );
        draw_table_footer(
            &mut buffer,
            frame.geometry,
            layout.command_col,
            brief_list_table_bottom_row(frame.geometry, rows.len(), scroll_offset),
            footer,
        );
        Ok(buffer)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_filter_prompt(
        &mut self,
        frame: &ScreenFrame<'_>,
        mode: PlanetListMode,
        rows: &[EmpirePlanetEconomyRow],
        sort: PlanetListSort,
        scroll_offset: usize,
        cursor: usize,
        prompt_mode: PlanetListFilterPromptMode,
        prompt_default: &str,
        input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = self.render_brief_list(
            frame,
            mode,
            rows,
            sort,
            scroll_offset,
            cursor,
            "",
            status,
            false,
            None,
            "",
            "",
            None,
            None,
            None,
        )?;
        let table_rows = planet_table_rows(frame, rows);
        let visible_rows = stacked_table_visible_rows_for(frame.geometry, 1);
        let displayed_rows = table_rows
            .len()
            .saturating_sub(scroll_offset)
            .min(visible_rows);
        let scrollable = table_rows.len() > visible_rows;
        let footer = match prompt_mode {
            PlanetListFilterPromptMode::FilterMenu => TableFooter::LabeledCommandBar {
                label: "FILTER",
                hotkeys_markup: BRIEF_FILTER_HOTKEYS,
                default: None,
                input: "",
            },
            PlanetListFilterPromptMode::RangeCoords => TableFooter::CommandInput {
                label: "COMMAND",
                prompt: "Range from ",
                default: prompt_default,
                input,
            },
            PlanetListFilterPromptMode::RangeDistance => TableFooter::CommandInput {
                label: "COMMAND",
                prompt: "Range radius ",
                default: prompt_default,
                input,
            },
        };
        let columns = resolve_table_columns_for_widget_with_footer_floor(
            &BRIEF_COLUMNS,
            &table_rows,
            buffer.width(),
            scrollable,
            TableWidthMode::Compact,
            Some(brief_list_title(mode)),
            Some(footer),
            planet_list_footer_floor(frame, mode),
        );
        let layout = layout_stacked_table_block(
            LayoutRect::new(0, 0, buffer.width(), buffer.height()),
            &columns,
            displayed_rows,
            Some(brief_list_title(mode)),
            Some(footer),
            scrollable,
            HorizontalAlign::Center,
            VerticalAlign::Top,
        );
        draw_table_footer(
            &mut buffer,
            frame.geometry,
            layout.command_col,
            brief_list_table_bottom_row(frame.geometry, rows.len(), scroll_offset),
            footer,
        );
        Ok(buffer)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_brief_list(
        &mut self,
        frame: &ScreenFrame<'_>,
        mode: PlanetListMode,
        rows: &[EmpirePlanetEconomyRow],
        _sort: PlanetListSort,
        scroll_offset: usize,
        cursor: usize,
        input: &str,
        status: Option<&str>,
        auto_commission_prompt: bool,
        transport_prompt_label: Option<&str>,
        transport_prompt_default: &str,
        transport_prompt_input: &str,
        _transport_summary: Option<&str>,
        scorch_prompt_label: Option<&str>,
        _scorch_summary: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield_for(frame.geometry);
        let visible_rows = stacked_table_visible_rows_for(frame.geometry, 1);
        let table_rows = planet_table_rows(frame, rows);
        let scrollable = table_rows.len() > visible_rows;
        let displayed_rows = table_rows
            .len()
            .saturating_sub(scroll_offset)
            .min(visible_rows);
        let default_coords = rows
            .get(cursor)
            .map(|row| format_sector_coords_default(row.coords))
            .unwrap_or_else(|| "00,00".to_string());
        let footer = if auto_commission_prompt {
            TableFooter::CommandPrompt {
                label: "COMMAND",
                prompt: PLANET_LIST_AUTO_COMMISSION_PROMPT,
            }
        } else if let Some(prompt) = transport_prompt_label {
            TableFooter::CommandInput {
                label: "COMMAND",
                prompt,
                default: transport_prompt_default,
                input: transport_prompt_input,
            }
        } else if let Some(prompt) = scorch_prompt_label {
            TableFooter::CommandPrompt {
                label: "COMMAND",
                prompt,
            }
        } else if let Some(status) = status {
            TableFooter::CommandText {
                label: "COMMAND",
                text: status,
            }
        } else {
            TableFooter::CommandBar {
                hotkeys_markup: match mode {
                    PlanetListMode::Brief => BRIEF_BROWSE_HOTKEYS,
                    PlanetListMode::BuildSelect | PlanetListMode::Stub(_) => "? S <Q>",
                },
                default: Some(&default_coords),
                input,
            }
        };
        let columns = resolve_table_columns_for_widget_with_footer_floor(
            &BRIEF_COLUMNS,
            &table_rows,
            buffer.width(),
            scrollable,
            TableWidthMode::Compact,
            Some(brief_list_title(mode)),
            Some(footer),
            planet_list_footer_floor(frame, mode),
        );
        let layout = layout_stacked_table_block(
            LayoutRect::new(0, 0, buffer.width(), buffer.height()),
            &columns,
            displayed_rows,
            Some(brief_list_title(mode)),
            Some(footer),
            scrollable,
            HorizontalAlign::Center,
            VerticalAlign::Top,
        );
        let _ = layout.title_row;
        draw_table_title(
            &mut buffer,
            layout.table_row,
            layout.table_col,
            brief_list_title(mode),
        );

        let metrics = write_stacked_table_window_with_states_at(
            &mut buffer,
            layout.table_row,
            layout.table_col,
            &BRIEF_TOP_HEADER_CELLS,
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
            frame.geometry,
            layout.command_col,
            metrics.bottom_row,
            footer,
        );
        Ok(buffer)
    }

    pub fn handle_sort_prompt_key(&self, key: KeyEvent, mode: PlanetListMode) -> Action {
        match key.code {
            KeyCode::Char('?') => Action::OpenPopupHelp,
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

    pub fn handle_filter_prompt_key(
        &self,
        key: KeyEvent,
        mode: PlanetListMode,
        prompt_mode: PlanetListFilterPromptMode,
    ) -> Action {
        match prompt_mode {
            PlanetListFilterPromptMode::FilterMenu => match key.code {
                KeyCode::Char('?') => Action::OpenPopupHelp,
                KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Enter => Action::Planet(
                    PlanetAction::SubmitListFilter(mode, PlanetListFilterMode::All),
                ),
                KeyCode::Char('r') | KeyCode::Char('R') => Action::Planet(
                    PlanetAction::SubmitListFilter(mode, PlanetListFilterMode::Range),
                ),
                KeyCode::Char('s') | KeyCode::Char('S') => Action::Planet(
                    PlanetAction::SubmitListFilter(mode, PlanetListFilterMode::Starbase),
                ),
                KeyCode::Char('t') | KeyCode::Char('T') => Action::Planet(
                    PlanetAction::SubmitListFilter(mode, PlanetListFilterMode::Stardock),
                ),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    Action::Planet(PlanetAction::CloseListFilterPrompt(mode))
                }
                _ => Action::Noop,
            },
            PlanetListFilterPromptMode::RangeCoords | PlanetListFilterPromptMode::RangeDistance => {
                match key.code {
                    KeyCode::Char('?') => Action::OpenPopupHelp,
                    KeyCode::Enter => Action::Planet(PlanetAction::SubmitListFilter(
                        mode,
                        PlanetListFilterMode::Range,
                    )),
                    KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceListPromptInput),
                    KeyCode::Char(ch)
                        if matches!(prompt_mode, PlanetListFilterPromptMode::RangeCoords)
                            && is_coordinate_input_char(ch) =>
                    {
                        Action::Planet(PlanetAction::AppendListPromptChar(ch))
                    }
                    KeyCode::Char(ch)
                        if matches!(prompt_mode, PlanetListFilterPromptMode::RangeDistance)
                            && ch.is_ascii_digit() =>
                    {
                        Action::Planet(PlanetAction::AppendListPromptChar(ch))
                    }
                    KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                        Action::Planet(PlanetAction::CloseListFilterPrompt(mode))
                    }
                    _ => Action::Noop,
                }
            }
        }
    }

    pub fn handle_brief_key(&self, key: KeyEvent, mode: PlanetListMode) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::Planet(PlanetAction::MoveBrief(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::Planet(PlanetAction::MoveBrief(1))
            }
            KeyCode::PageUp => Action::Planet(PlanetAction::MoveBrief(-5)),
            KeyCode::PageDown => Action::Planet(PlanetAction::MoveBrief(5)),
            KeyCode::Char('f') | KeyCode::Char('F') if mode == PlanetListMode::Brief => {
                Action::Planet(PlanetAction::OpenListFilterPrompt(mode))
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                Action::Planet(PlanetAction::OpenListSortPrompt(mode))
            }
            KeyCode::Char('b') | KeyCode::Char('B') if mode == PlanetListMode::Brief => {
                Action::Planet(PlanetAction::OpenBuildSpecify)
            }
            KeyCode::Char('a') | KeyCode::Char('A') if mode == PlanetListMode::Brief => {
                Action::Planet(PlanetAction::OpenAutoCommissionPrompt)
            }
            KeyCode::Char('c') | KeyCode::Char('C') if mode == PlanetListMode::Brief => {
                Action::Planet(PlanetAction::OpenCommissionMenu)
            }
            KeyCode::Char('l') | KeyCode::Char('L') if mode == PlanetListMode::Brief => {
                Action::Planet(PlanetAction::OpenTransportPrompt(
                    crate::screen::PlanetTransportMode::Load,
                ))
            }
            KeyCode::Char('u') | KeyCode::Char('U') if mode == PlanetListMode::Brief => {
                Action::Planet(PlanetAction::OpenTransportPrompt(
                    crate::screen::PlanetTransportMode::Unload,
                ))
            }
            KeyCode::Char('x') | KeyCode::Char('X') if mode == PlanetListMode::Brief => {
                Action::Planet(PlanetAction::OpenScorchPrompt)
            }
            KeyCode::Char('i') | KeyCode::Char('I') if mode == PlanetListMode::Brief => {
                Action::Planet(PlanetAction::SubmitBriefInput)
            }
            KeyCode::Enter => Action::Planet(PlanetAction::SubmitBriefInput),
            KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceBriefInput),
            KeyCode::Char(ch) if is_coordinate_input_char(ch) => {
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

fn brief_list_table_bottom_row(
    geometry: crate::screen::ScreenGeometry,
    total_rows: usize,
    scroll_offset: usize,
) -> usize {
    let displayed_rows = total_rows
        .saturating_sub(scroll_offset)
        .min(stacked_table_visible_rows_for(geometry, 1));
    1 + 4 + displayed_rows
}

fn planet_list_footer_floor(frame: &ScreenFrame<'_>, mode: PlanetListMode) -> usize {
    match mode {
        PlanetListMode::Brief => {
            let max_fleet_default = frame
                .game_data
                .fleets
                .records
                .iter()
                .filter(|fleet| {
                    fleet.owner_empire_raw() as usize == frame.player.record_index_1_based
                })
                .map(|fleet| fleet.local_slot_word_raw())
                .max()
                .unwrap_or(1)
                .to_string();
            [
                table_footer_scaffold_width(TableFooter::CommandBar {
                    hotkeys_markup: BRIEF_BROWSE_HOTKEYS,
                    default: Some("00,00"),
                    input: "",
                }),
                table_footer_scaffold_width(TableFooter::CommandPrompt {
                    label: "COMMAND",
                    prompt: PLANET_LIST_AUTO_COMMISSION_PROMPT,
                }),
                table_footer_scaffold_width(TableFooter::CommandInput {
                    label: "COMMAND",
                    prompt: PLANET_LIST_LOAD_FLEET_PROMPT,
                    default: &max_fleet_default,
                    input: "",
                }),
                table_footer_scaffold_width(TableFooter::CommandInput {
                    label: "COMMAND",
                    prompt: PLANET_LIST_UNLOAD_FLEET_PROMPT,
                    default: &max_fleet_default,
                    input: "",
                }),
                table_footer_scaffold_width(TableFooter::CommandInput {
                    label: "COMMAND",
                    prompt: PLANET_LIST_LOAD_QTY_PROMPT,
                    default: PLANET_LIST_MAX_QTY_DEFAULT,
                    input: "",
                }),
                table_footer_scaffold_width(TableFooter::CommandInput {
                    label: "COMMAND",
                    prompt: PLANET_LIST_UNLOAD_QTY_PROMPT,
                    default: PLANET_LIST_MAX_QTY_DEFAULT,
                    input: "",
                }),
                table_footer_scaffold_width(TableFooter::CommandPrompt {
                    label: "COMMAND",
                    prompt: PLANET_LIST_SCORCH_CONFIRM_PROMPT,
                }),
                table_footer_scaffold_width(TableFooter::CommandPrompt {
                    label: "COMMAND",
                    prompt: PLANET_LIST_SCORCH_REALLY_CONFIRM_PROMPT,
                }),
                table_footer_scaffold_width(TableFooter::CommandPrompt {
                    label: "COMMAND",
                    prompt: PLANET_LIST_SCORCH_LAST_CONFIRM_PROMPT,
                }),
                table_footer_scaffold_width(TableFooter::LabeledCommandBar {
                    label: "SORT",
                    hotkeys_markup: BRIEF_SORT_HOTKEYS,
                    default: None,
                    input: "",
                }),
                table_footer_scaffold_width(TableFooter::LabeledCommandBar {
                    label: "FILTER",
                    hotkeys_markup: BRIEF_FILTER_HOTKEYS,
                    default: None,
                    input: "",
                }),
            ]
            .into_iter()
            .max()
            .unwrap_or(0)
        }
        PlanetListMode::BuildSelect | PlanetListMode::Stub(_) => [
            table_footer_scaffold_width(TableFooter::CommandBar {
                hotkeys_markup: "? S <Q>",
                default: Some("00,00"),
                input: "",
            }),
            table_footer_scaffold_width(TableFooter::LabeledCommandBar {
                label: "SORT",
                hotkeys_markup: BRIEF_SORT_HOTKEYS,
                default: None,
                input: "",
            }),
        ]
        .into_iter()
        .max()
        .unwrap_or(0),
    }
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
                queued_build_units(frame, row).to_string(),
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

fn queued_build_units(frame: &ScreenFrame<'_>, row: &EmpirePlanetEconomyRow) -> u32 {
    frame
        .game_data
        .planets
        .records
        .get(row.planet_record_index_1_based.saturating_sub(1))
        .map(|planet| {
            (0..10)
                .map(|slot| {
                    build_quantity_from_points(
                        nc_data::ProductionItemKind::from_raw(planet.build_kind_raw(slot)),
                        u32::from(planet.build_count_raw(slot)),
                    )
                })
                .sum()
        })
        .unwrap_or(0)
}
