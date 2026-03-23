use crossterm::event::{KeyCode, KeyEvent};
use ec_data::{EmpirePlanetEconomyRow, STARDOCK_SLOT_COUNT};

use crate::app::Action;
use crate::domains::planet::PlanetAction;
use crate::screen::layout::{
    dismiss_prompt_row, draw_command_prompt, draw_dismiss_prompt, draw_status_line,
    draw_table_command_bar, draw_table_command_bar_at, draw_table_command_prompt_at,
    draw_title_bar, new_playfield, standard_table_visible_rows, table_prompt_row,
};
use crate::screen::table::{TableColumn, write_table_window_with_states};
use crate::screen::{
    PlayfieldBuffer, ScreenFrame, format_sector_coords, format_sector_coords_default,
    format_sector_coords_table,
};

pub const PLANET_BRIEF_VISIBLE_ROWS: usize = standard_table_visible_rows(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetListMode {
    Brief,
    Detail,
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
    TableColumn::left("(X,Y)", 8),
    TableColumn::left("Name", 22),
    TableColumn::right("Curr", 4),
    TableColumn::right("Max", 4),
    TableColumn::right("Points", 6),
    TableColumn::right("Rev", 4),
    TableColumn::right("Grow", 5),
    TableColumn::right("Docked", 6),
    TableColumn::left("SB", 2),
    TableColumn::right("ARs", 3),
    TableColumn::right("GBs", 3),
];

const BRIEF_SORT_PROMPT: &str = "Sort by <C>urrent Prod, <L>ocation, <M>ax, or <Q>uit? [C] ->";

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
            let mut buffer = new_playfield();
            draw_title_bar(&mut buffer, 0, "PLANET COMMAND:");
            draw_status_line(&mut buffer, 3, "Notice: ", message);
            draw_dismiss_prompt(&mut buffer, dismiss_prompt_row(3));
            return Ok(buffer);
        }

        let mut buffer = self.render_brief_list(frame, rows, sort, scroll_offset, cursor, input)?;
        if let Some(status) = status {
            draw_status_line(&mut buffer, 21, "Notice: ", status);
        }
        draw_table_command_prompt_at(
            &mut buffer,
            brief_list_command_row(rows.len(), scroll_offset),
            BRIEF_SORT_PROMPT,
        );
        Ok(buffer)
    }

    pub fn render_brief_list(
        &mut self,
        frame: &ScreenFrame<'_>,
        rows: &[EmpirePlanetEconomyRow],
        _sort: PlanetListSort,
        scroll_offset: usize,
        cursor: usize,
        input: &str,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "PLANET COMMAND:");

        let table_rows = rows
            .iter()
            .map(|row| {
                vec![
                    format_sector_coords_table(row.coords),
                    row.planet_name.clone(),
                    row.present_production.to_string(),
                    row.potential_production.to_string(),
                    row.stored_production_points.to_string(),
                    row.yearly_tax_revenue.to_string(),
                    format_signed_growth(row.yearly_growth_delta),
                    docked_units(frame, row).to_string(),
                    if row.has_friendly_starbase {
                        "Y".to_string()
                    } else {
                        "N".to_string()
                    },
                    row.armies.to_string(),
                    row.ground_batteries.to_string(),
                ]
            })
            .collect::<Vec<_>>();
        let metrics = write_table_window_with_states(
            &mut buffer,
            1,
            &BRIEF_COLUMNS,
            &table_rows,
            scroll_offset,
            PLANET_BRIEF_VISIBLE_ROWS,
            crate::theme::classic::status_value_style(),
            crate::theme::classic::status_value_style(),
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
        draw_table_command_bar_at(
            &mut buffer,
            table_prompt_row(metrics.bottom_row),
            "<ARROWS J K S Q>",
            Some(&default_coords),
            input,
        );
        Ok(buffer)
    }

    pub fn render_detail(
        &mut self,
        frame: &ScreenFrame<'_>,
        rows: &[EmpirePlanetEconomyRow],
        selected_index: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        if rows.is_empty() {
            draw_title_bar(&mut buffer, 0, "PLANET DETAIL 0/0:");
            draw_status_line(
                &mut buffer,
                3,
                "Notice: ",
                "You do not currently control any planets.",
            );
            draw_command_prompt(&mut buffer, 19, "PLANET COMMAND", "Q");
            return Ok(buffer);
        }
        let row = rows
            .get(selected_index)
            .ok_or("planet detail row missing")?;
        draw_title_bar(
            &mut buffer,
            0,
            &format!("PLANET DETAIL {}/{}:", selected_index + 1, rows.len()),
        );
        let efficiency = if row.potential_production == 0 {
            0.0
        } else {
            (row.present_production as f64 / row.potential_production as f64) * 100.0
        };
        let fleet_count = frame
            .game_data
            .fleets
            .records
            .iter()
            .filter(|fleet| {
                fleet.owner_empire_raw() as usize == frame.player.record_index_1_based
                    && fleet.current_location_coords_raw() == row.coords
            })
            .count();
        let status = frame
            .game_data
            .planets
            .records
            .get(row.planet_record_index_1_based - 1)
            .map(|planet| planet.status_or_name_summary())
            .unwrap_or_else(|| row.planet_name.clone());

        draw_status_line(&mut buffer, 2, "Planet Name: ", &row.planet_name);
        draw_status_line(
            &mut buffer,
            3,
            "Location: ",
            &format_sector_coords(row.coords),
        );
        draw_status_line(
            &mut buffer,
            4,
            "Current Production: ",
            &row.present_production.to_string(),
        );
        draw_status_line(
            &mut buffer,
            5,
            "Maximum Production: ",
            &row.potential_production.to_string(),
        );
        draw_status_line(
            &mut buffer,
            6,
            "Stored Production Points: ",
            &row.stored_production_points.to_string(),
        );
        draw_status_line(
            &mut buffer,
            7,
            "Expected Revenue: ",
            &format!("{} points", row.yearly_tax_revenue),
        );
        draw_status_line(
            &mut buffer,
            8,
            "Yearly Growth: ",
            &format!("+{} production", row.yearly_growth_delta),
        );
        draw_status_line(
            &mut buffer,
            9,
            "Build Capacity: ",
            &row.build_capacity.to_string(),
        );
        draw_status_line(
            &mut buffer,
            10,
            "Efficiency: ",
            &format!("{efficiency:.3}%"),
        );
        draw_status_line(&mut buffer, 12, "Armies: ", &row.armies.to_string());
        draw_status_line(
            &mut buffer,
            13,
            "Ground Batteries: ",
            &row.ground_batteries.to_string(),
        );
        draw_status_line(
            &mut buffer,
            14,
            "Friendly Fleets Here: ",
            &fleet_count.to_string(),
        );
        draw_status_line(
            &mut buffer,
            15,
            "Friendly Starbase: ",
            if row.has_friendly_starbase {
                "YES"
            } else {
                "NO"
            },
        );
        draw_status_line(&mut buffer, 16, "Status: ", &planet_status(row, &status));
        draw_table_command_bar(&mut buffer, "<ARROWS J K Q>", None, "");
        Ok(buffer)
    }

    pub fn handle_sort_prompt_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Enter => {
                Action::Planet(PlanetAction::SubmitListSort(
                    PlanetListMode::Brief,
                    PlanetListSort::CurrentProduction,
                ))
            }
            KeyCode::Char('l') | KeyCode::Char('L') => Action::Planet(
                PlanetAction::SubmitListSort(PlanetListMode::Brief, PlanetListSort::Location),
            ),
            KeyCode::Char('m') | KeyCode::Char('M') => {
                Action::Planet(PlanetAction::SubmitListSort(
                    PlanetListMode::Brief,
                    PlanetListSort::PotentialProduction,
                ))
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Planet(PlanetAction::CloseListSortPrompt(PlanetListMode::Brief))
            }
            _ => Action::Noop,
        }
    }

    pub fn handle_brief_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up => Action::Planet(PlanetAction::MoveBrief(-1)),
            KeyCode::Down => Action::Planet(PlanetAction::MoveBrief(1)),
            KeyCode::PageUp => Action::Planet(PlanetAction::MoveBrief(-5)),
            KeyCode::PageDown => Action::Planet(PlanetAction::MoveBrief(5)),
            KeyCode::Char('k') | KeyCode::Char('K') => Action::Planet(PlanetAction::MoveBrief(-1)),
            KeyCode::Char('j') | KeyCode::Char('J') => Action::Planet(PlanetAction::MoveBrief(1)),
            KeyCode::Char('s') | KeyCode::Char('S') => {
                Action::Planet(PlanetAction::OpenListSortPrompt(PlanetListMode::Brief))
            }
            KeyCode::Enter => Action::Planet(PlanetAction::SubmitBriefInput),
            KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceBriefInput),
            KeyCode::Char(ch) if ch.is_ascii_digit() || ch == ',' || ch == ' ' => {
                Action::Planet(PlanetAction::AppendBriefChar(ch))
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Planet(PlanetAction::OpenMenu)
            }
            _ => Action::Noop,
        }
    }

    pub fn handle_detail_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Left => Action::Planet(PlanetAction::MoveDetail(-1)),
            KeyCode::Down | KeyCode::Right => Action::Planet(PlanetAction::MoveDetail(1)),
            KeyCode::Home => Action::Planet(PlanetAction::MoveDetail(i8::MIN)),
            KeyCode::End => Action::Planet(PlanetAction::MoveDetail(i8::MAX)),
            KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Char('h') | KeyCode::Char('H') => {
                Action::Planet(PlanetAction::MoveDetail(-1))
            }
            KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Char('l') | KeyCode::Char('L') => {
                Action::Planet(PlanetAction::MoveDetail(1))
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Planet(PlanetAction::OpenMenu)
            }
            _ => Action::Noop,
        }
    }
}

fn brief_list_command_row(total_rows: usize, scroll_offset: usize) -> usize {
    let displayed_rows = total_rows
        .saturating_sub(scroll_offset)
        .min(PLANET_BRIEF_VISIBLE_ROWS);
    table_prompt_row(1 + 3 + displayed_rows)
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

fn planet_status(row: &EmpirePlanetEconomyRow, fallback: &str) -> String {
    if row.is_homeworld_seed {
        return "Homeworld - fully developed".to_string();
    }
    if row.has_friendly_starbase {
        return "Regular planet - starbase present".to_string();
    }
    if fallback.starts_with("Regular planet") || fallback.starts_with("Homeworld") {
        return fallback.to_string();
    }
    "Regular planet - factories fully functional".to_string()
}
