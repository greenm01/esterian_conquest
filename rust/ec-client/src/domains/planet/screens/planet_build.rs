use crossterm::event::{KeyCode, KeyEvent};
use ec_data::{EmpirePlanetEconomyRow, ProductionItemKind};

use crate::app::Action;
use crate::domains::planet::PlanetAction;
use crate::domains::starmap::StarmapAction;
use crate::screen::layout::{
    CMD_COL_1, MenuEntry, centered_row, draw_command_line_default_input_at, draw_command_prompt,
    draw_command_prompt_at, draw_menu_row, draw_status_line, draw_title_bar, last_body_row,
    menu_prompt_row, new_playfield, standard_table_visible_rows, table_prompt_row,
};
use crate::screen::table::{
    SplitTableRow, TableColumn, write_split_table, write_table_window_with_cursor,
};
use crate::screen::{
    CommandMenu, PlayfieldBuffer, Screen, ScreenFrame, format_sector_coords,
    format_sector_coords_padded,
};
use crate::theme::classic;

pub struct PlanetBuildScreen;

pub(crate) const PLANET_BUILD_LIST_VISIBLE_ROWS: usize = standard_table_visible_rows(4);
pub(crate) const PLANET_BUILD_CHANGE_VISIBLE_ROWS: usize = standard_table_visible_rows(4);

const CHANGE_COLUMNS: [TableColumn<'static>; 5] = [
    TableColumn::left("Planet Name", 20),
    TableColumn::left("Location", 9),
    TableColumn::left("Production", 16),
    TableColumn::right("PP", 4),
    TableColumn::right("Spent", 5),
];

const BUILD_LIST_COLUMNS: [TableColumn<'static>; 4] = [
    TableColumn::left("Unit", 24),
    TableColumn::right("Points", 6),
    TableColumn::right("Queue", 5),
    TableColumn::right("Dock", 4),
];

const BUILD_HALF_COLUMNS: [TableColumn<'static>; 4] = [
    TableColumn::left("NO.", 4),
    TableColumn::left("UNIT TYPE", 19),
    TableColumn::right("COST", 4),
    TableColumn::right("QTY.", 5),
];

const ROW_1: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "H", "elp with commands"),
    MenuEntry::new(29, "P", "lanets, List your"),
    MenuEntry::new(57, "S", "pecify Build Orders"),
];

const ROW_2: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "Q", "uit to Planet Menu"),
    MenuEntry::new(29, "R", "eview current planet"),
    MenuEntry::new(57, "A", "bort planet's builds"),
];

const ROW_3: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "X", "pert mode ON/OFF"),
    MenuEntry::new(29, "C", "hange current planet"),
    MenuEntry::new(57, "L", "ist builds"),
];

const ROW_4: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "V", "iew partial star map"),
    MenuEntry::new(29, "N", "ext planet"),
    MenuEntry::new(57, "I", "nfo about a Planet"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanetBuildOrder {
    pub kind: ProductionItemKind,
    pub points_remaining: u8,
}

#[derive(Debug, Clone)]
pub struct PlanetBuildMenuView {
    pub row: EmpirePlanetEconomyRow,
    pub committed_points: u32,
    pub available_points: u32,
    pub points_left: u32,
    /// Number of occupied build-queue slots (0..=10).
    pub queue_used: usize,
    /// Total build-queue capacity (always 10).
    pub queue_capacity: usize,
    /// Number of stardock slots currently occupied by built or pending
    /// ships/starbases (0..=10). Armies and batteries excluded.
    pub stardock_used: usize,
    /// Total stardock capacity (always 10).
    pub stardock_capacity: usize,
}

#[derive(Debug, Clone)]
pub struct PlanetBuildListRow {
    pub kind: ProductionItemKind,
    pub unit_label: String,
    pub points: u32,
    pub queue_qty: u32,
    pub stardock_qty: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct PlanetBuildChangeRow {
    pub planet_name: String,
    pub coords: [u8; 2],
    pub present_production: u16,
    pub potential_production: u16,
    pub available_points: u32,
    pub committed_points: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildUnitSpec {
    pub number: u8,
    pub kind: ProductionItemKind,
    pub label: &'static str,
    pub singular_label: &'static str,
    pub cost: u32,
}

const BUILD_UNITS: [BuildUnitSpec; 9] = [
    BuildUnitSpec {
        number: 1,
        kind: ProductionItemKind::Destroyer,
        label: "Destroyers",
        singular_label: "destroyers",
        cost: 5,
    },
    BuildUnitSpec {
        number: 2,
        kind: ProductionItemKind::Cruiser,
        label: "Cruisers",
        singular_label: "cruisers",
        cost: 15,
    },
    BuildUnitSpec {
        number: 3,
        kind: ProductionItemKind::Battleship,
        label: "Battleships",
        singular_label: "battleships",
        cost: 45,
    },
    BuildUnitSpec {
        number: 4,
        kind: ProductionItemKind::Scout,
        label: "Scouts",
        singular_label: "scouts",
        cost: 15,
    },
    BuildUnitSpec {
        number: 5,
        kind: ProductionItemKind::Transport,
        label: "Troop transports",
        singular_label: "troop transports",
        cost: 5,
    },
    BuildUnitSpec {
        number: 6,
        kind: ProductionItemKind::Etac,
        label: "ETACs",
        singular_label: "ETACs",
        cost: 20,
    },
    BuildUnitSpec {
        number: 7,
        kind: ProductionItemKind::Starbase,
        label: "Starbases",
        singular_label: "starbases",
        cost: 50,
    },
    BuildUnitSpec {
        number: 9,
        kind: ProductionItemKind::Army,
        label: "Armies",
        singular_label: "armies",
        cost: 2,
    },
    BuildUnitSpec {
        number: 10,
        kind: ProductionItemKind::GroundBattery,
        label: "Ground batteries",
        singular_label: "ground batteries",
        cost: 20,
    },
];

impl PlanetBuildScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_menu(
        &mut self,
        view: &PlanetBuildMenuView,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(
            &mut buffer,
            0,
            &format!(
                "BUILD ON CURRENT PLANET: \"{}\" IN SYSTEM {}:",
                view.row.planet_name,
                format_sector_coords(view.row.coords)
            ),
        );

        let spent = view.committed_points.min(view.available_points);

        draw_menu_row(&mut buffer, 2, &ROW_1);
        draw_menu_row(&mut buffer, 3, &ROW_2);
        draw_menu_row(&mut buffer, 4, &ROW_3);
        draw_menu_row(&mut buffer, 5, &ROW_4);

        let command_row = menu_prompt_row(5);
        let lower_block_height = if status.is_some() { 7 } else { 5 };
        let lower_block_row = centered_row(command_row + 1, last_body_row(), lower_block_height);

        let starbase_line = if view.row.has_friendly_starbase {
            format!(
                "There is a starbase orbiting planet \"{}\".",
                view.row.planet_name
            )
        } else {
            format!(
                "There are no starbases orbiting planet \"{}\".",
                view.row.planet_name
            )
        };
        let restrictions_line = if view.row.has_friendly_starbase {
            "Standard building restrictions do not apply.".to_string()
        } else {
            "Standard building restrictions apply.".to_string()
        };
        buffer.write_text(
            lower_block_row,
            0,
            &starbase_line,
            classic::status_value_style(),
        );
        buffer.write_text(
            lower_block_row + 1,
            0,
            &restrictions_line,
            classic::status_value_style(),
        );
        buffer.write_text(
            lower_block_row + 2,
            0,
            &format!(
                "You have spent {} out of {} points.  You have {} points left to spend.",
                spent, view.available_points, view.points_left
            ),
            classic::status_value_style(),
        );
        buffer.write_text(
            lower_block_row + 4,
            0,
            &format!(
                "Build queue: [{}/{}]   Stardock: [{}/{}]",
                view.queue_used, view.queue_capacity, view.stardock_used, view.stardock_capacity,
            ),
            classic::status_value_style(),
        );
        if let Some(status) = status {
            draw_status_line(&mut buffer, lower_block_row + 6, "", status);
        }
        draw_command_prompt_at(
            &mut buffer,
            command_row,
            "BUILD COMMAND",
            "H,Q,X,V,P,R,C,N,S,A,L,I",
        );
        Ok(buffer)
    }

    pub fn render_review(
        &mut self,
        view: &PlanetBuildMenuView,
        orders: &[PlanetBuildOrder],
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "BUILD REVIEW:");
        draw_status_line(&mut buffer, 2, "Planet Name: ", &view.row.planet_name);
        draw_status_line(
            &mut buffer,
            3,
            "Location: ",
            &format_sector_coords(view.row.coords),
        );
        draw_status_line(
            &mut buffer,
            4,
            "Production: ",
            &format!(
                "{} of {}",
                view.row.present_production, view.row.potential_production
            ),
        );
        draw_status_line(
            &mut buffer,
            5,
            "Stored Production Points: ",
            &view.row.stored_production_points.to_string(),
        );
        draw_status_line(
            &mut buffer,
            6,
            "Build Capacity: ",
            &view.row.build_capacity.to_string(),
        );
        draw_status_line(
            &mut buffer,
            7,
            "Available To Spend: ",
            &view.available_points.to_string(),
        );
        draw_status_line(
            &mut buffer,
            8,
            "Queued Points: ",
            &view.committed_points.to_string(),
        );
        draw_status_line(
            &mut buffer,
            9,
            "Points Left: ",
            &view.points_left.to_string(),
        );
        draw_status_line(
            &mut buffer,
            10,
            "Starbase In Orbit: ",
            if view.row.has_friendly_starbase {
                "YES"
            } else {
                "NO"
            },
        );
        let queue_summary = if orders.is_empty() {
            "<none>".to_string()
        } else {
            orders
                .iter()
                .map(|o| build_order_summary(*o))
                .collect::<Vec<_>>()
                .join(", ")
        };
        draw_status_line(&mut buffer, 12, "Queued Build: ", &queue_summary);
        draw_command_prompt(&mut buffer, 19, "BUILD COMMAND", "SLAP A KEY");
        Ok(buffer)
    }

    pub fn render_list(
        &mut self,
        view: &PlanetBuildMenuView,
        rows: &[PlanetBuildListRow],
        scroll_offset: usize,
        cursor: usize,
        confirming: bool,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(
            &mut buffer,
            0,
            &format!(
                "BUILD LIST: \"{}\" AT {}:",
                view.row.planet_name,
                format_sector_coords(view.row.coords)
            ),
        );

        buffer.write_text(
            2,
            0,
            &format!(
                "You have spent {} out of {} points.  You have {} points left to spend.",
                view.committed_points.min(view.available_points),
                view.available_points,
                view.points_left
            ),
            classic::status_value_style(),
        );

        let table_rows: Vec<Vec<String>> = rows
            .iter()
            .map(|row| {
                vec![
                    row.unit_label.clone(),
                    row.points.to_string(),
                    row.queue_qty.to_string(),
                    row.stardock_qty
                        .map(|q| q.to_string())
                        .unwrap_or_else(|| "N/A".to_string()),
                ]
            })
            .collect();

        let selected = if rows.is_empty() { None } else { Some(cursor) };
        let metrics = write_table_window_with_cursor(
            &mut buffer,
            4,
            &BUILD_LIST_COLUMNS,
            &table_rows,
            scroll_offset,
            PLANET_BUILD_LIST_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
            selected,
        );
        let command_row = table_prompt_row(metrics.bottom_row);

        if rows.is_empty() {
            buffer.write_text(
                6,
                0,
                "No build orders are queued.",
                classic::status_value_style(),
            );
        }

        if confirming {
            buffer.write_text(
                17,
                0,
                "Delete queued build(s) for this unit? Y/[N]",
                classic::alert_style(),
            );
            draw_command_prompt_at(&mut buffer, command_row, "BUILD COMMAND", "Y N");
        } else {
            draw_command_prompt_at(
                &mut buffer,
                command_row,
                "BUILD COMMAND",
                "ARROWS D(elete queued) Q",
            );
        }
        Ok(buffer)
    }

    pub fn render_abort_confirm(
        &mut self,
        view: &PlanetBuildMenuView,
        orders: &[PlanetBuildOrder],
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "BUILD COMMAND:");
        let style = classic::status_value_style();

        buffer.write_text(
            2,
            0,
            &format!(
                "Abort all build orders for \"{}\" at {}.",
                view.row.planet_name,
                format_sector_coords(view.row.coords)
            ),
            style,
        );

        if orders.is_empty() {
            buffer.write_text(4, 0, "No build orders are queued.", style);
        } else {
            buffer.write_text(4, 0, "Queued orders to be cancelled:", style);
            for (i, order) in orders.iter().enumerate() {
                buffer.write_text(
                    5 + i,
                    2,
                    &format!("- {}", build_order_summary(*order)),
                    style,
                );
            }
        }

        buffer.write_text(
            12,
            0,
            &format!(
                "All {} committed points will be fully refunded.",
                view.committed_points
            ),
            classic::prompt_hotkey_style(),
        );

        draw_command_line_default_input_at(
            &mut buffer,
            14,
            "BUILD COMMAND",
            "Cancel these orders? ",
            "N",
            "",
        );
        Ok(buffer)
    }

    pub fn render_specify(
        &mut self,
        view: &PlanetBuildMenuView,
        orders: &[PlanetBuildOrder],
        input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        let table_metrics = draw_specify_table(&mut buffer, view, orders);

        let max_unit_num = BUILD_UNITS
            .iter()
            .filter(|u| max_quantity(view.points_left, u.cost) > 0)
            .map(|u| u.number)
            .max()
            .unwrap_or(0);
        if let Some(status) = status {
            draw_status_line(&mut buffer, 14, "", status);
        }
        draw_command_line_default_input_at(
            &mut buffer,
            table_prompt_row(table_metrics.bottom_row),
            "BUILD COMMAND",
            &format!("Unit number or 0 if done (0 - {}) ", max_unit_num),
            "0",
            input,
        );
        Ok(buffer)
    }

    pub fn render_quantity_prompt(
        &mut self,
        view: &PlanetBuildMenuView,
        orders: &[PlanetBuildOrder],
        unit: BuildUnitSpec,
        max_qty: u32,
        input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        let table_metrics = draw_specify_table(&mut buffer, view, orders);

        if let Some(status) = status {
            draw_status_line(&mut buffer, 14, "Error: ", status);
        }
        draw_command_line_default_input_at(
            &mut buffer,
            table_prompt_row(table_metrics.bottom_row),
            "BUILD COMMAND",
            &format!(
                "How many new {} to build (0 - {}) ",
                unit.singular_label, max_qty
            ),
            &max_qty.to_string(),
            input,
        );
        Ok(buffer)
    }

    pub fn render_change(
        &mut self,
        rows: &[PlanetBuildChangeRow],
        scroll_offset: usize,
        cursor: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "CHANGE CURRENT PLANET:");
        buffer.write_text(
            2,
            0,
            "Select a planet with ARROWS then press ENTER, or press Q to cancel.",
            classic::body_style(),
        );

        let table_rows: Vec<Vec<String>> = rows
            .iter()
            .map(|row| {
                vec![
                    row.planet_name.clone(),
                    format_sector_coords_padded(row.coords),
                    format!(
                        "{:>3} of {:>3}",
                        row.present_production, row.potential_production
                    ),
                    row.available_points.to_string(),
                    row.committed_points.to_string(),
                ]
            })
            .collect();

        let selected = if rows.is_empty() { None } else { Some(cursor) };
        let metrics = write_table_window_with_cursor(
            &mut buffer,
            4,
            &CHANGE_COLUMNS,
            &table_rows,
            scroll_offset,
            PLANET_BUILD_CHANGE_VISIBLE_ROWS,
            classic::status_value_style(),
            classic::status_value_style(),
            selected,
        );
        let command_row = table_prompt_row(metrics.bottom_row);

        if rows.is_empty() {
            buffer.write_text(
                6,
                0,
                "No owned planets available.",
                classic::status_value_style(),
            );
        }

        draw_command_prompt_at(&mut buffer, command_row, "BUILD COMMAND", "ARROWS ENTER Q");
        Ok(buffer)
    }

    pub fn handle_change_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::Planet(PlanetAction::MoveBuildChange(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::Planet(PlanetAction::MoveBuildChange(1))
            }
            KeyCode::PageUp => Action::Planet(PlanetAction::MoveBuildChange(-8)),
            KeyCode::PageDown => Action::Planet(PlanetAction::MoveBuildChange(8)),
            KeyCode::Enter => Action::Planet(PlanetAction::ConfirmBuildChange),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Planet(PlanetAction::OpenBuildMenu)
            }
            _ => Action::Noop,
        }
    }

    pub fn handle_menu_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Planet(PlanetAction::OpenMenu)
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                Action::Starmap(StarmapAction::OpenPartialView(CommandMenu::PlanetBuild))
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                Action::Planet(PlanetAction::OpenInfoPrompt(CommandMenu::PlanetBuild))
            }
            KeyCode::Char('h') | KeyCode::Char('H') => Action::Planet(PlanetAction::OpenBuildHelp),
            KeyCode::Char('c') | KeyCode::Char('C') => {
                Action::Planet(PlanetAction::OpenBuildChange)
            }
            KeyCode::Char('n') | KeyCode::Char('N') => Action::Planet(PlanetAction::MoveBuild(1)),
            KeyCode::Char('r') | KeyCode::Char('R') => {
                Action::Planet(PlanetAction::OpenBuildReview)
            }
            KeyCode::Char('l') | KeyCode::Char('L') => Action::Planet(PlanetAction::OpenBuildList),
            KeyCode::Char('a') | KeyCode::Char('A') => {
                Action::Planet(PlanetAction::OpenBuildAbortConfirm)
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                Action::Planet(PlanetAction::OpenBuildSpecify)
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                Action::Planet(PlanetAction::SubmitListSort(
                    crate::screen::PlanetListMode::Brief,
                    crate::screen::PlanetListSort::CurrentProduction,
                ))
            }
            _ => Action::Noop,
        }
    }

    pub fn handle_review_key(&self, _key: KeyEvent) -> Action {
        Action::Planet(PlanetAction::OpenBuildMenu)
    }

    pub fn handle_list_key(&self, key: KeyEvent, confirming: bool) -> Action {
        if confirming {
            return match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    Action::Planet(PlanetAction::ConfirmDeleteBuildSlot)
                }
                _ => Action::Planet(PlanetAction::CancelDeleteBuildSlot),
            };
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::Planet(PlanetAction::MoveBuildList(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::Planet(PlanetAction::MoveBuildList(1))
            }
            KeyCode::PageUp => Action::Planet(PlanetAction::MoveBuildList(-8)),
            KeyCode::PageDown => Action::Planet(PlanetAction::MoveBuildList(8)),
            KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete => {
                Action::Planet(PlanetAction::DeleteBuildSlotRequest)
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Planet(PlanetAction::OpenBuildMenu)
            }
            _ => Action::Noop,
        }
    }

    pub fn handle_abort_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                Action::Planet(PlanetAction::ConfirmBuildAbort)
            }
            _ => Action::Planet(PlanetAction::OpenBuildMenu),
        }
    }

    pub fn handle_specify_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Planet(PlanetAction::OpenBuildMenu)
            }
            KeyCode::Enter => Action::Planet(PlanetAction::SubmitBuildUnit),
            KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceBuildUnitInput),
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                Action::Planet(PlanetAction::AppendBuildUnitChar(ch))
            }
            _ => Action::Noop,
        }
    }

    pub fn handle_quantity_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Planet(PlanetAction::OpenBuildSpecify)
            }
            KeyCode::Enter => Action::Planet(PlanetAction::SubmitBuildQuantity),
            KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceBuildQuantityInput),
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                Action::Planet(PlanetAction::AppendBuildQuantityChar(ch))
            }
            _ => Action::Noop,
        }
    }
}

impl Screen for PlanetBuildScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "BUILD COMMAND:");
        draw_command_prompt(&mut buffer, 19, "BUILD COMMAND", "SLAP A KEY");
        Ok(buffer)
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        self.handle_menu_key(key)
    }
}

// Draw the shared header, two-column unit table, and points line used by both
// render_specify and render_quantity_prompt.
fn draw_specify_table(
    buffer: &mut PlayfieldBuffer,
    view: &PlanetBuildMenuView,
    orders: &[PlanetBuildOrder],
) -> crate::screen::table::TableRenderMetrics {
    draw_title_bar(buffer, 0, "SPECIFY BUILD ORDERS:");
    buffer.write_text(
        2,
        0,
        &format!(
            "You have spent {} out of {} points.  You have {} points left to spend.",
            view.committed_points.min(view.available_points),
            view.available_points,
            view.points_left
        ),
        classic::status_value_style(),
    );

    let style = classic::status_value_style();

    struct HalfEntry {
        tag: String,
        name: &'static str,
        cost: u32,
        qty: u32,
    }

    let entry = |unit: &BuildUnitSpec| -> HalfEntry {
        let max_qty = max_quantity(view.points_left, unit.cost);
        // Sum quantities across all queued orders for this unit kind.
        let order_qty = if unit.cost == 0 {
            0
        } else {
            orders
                .iter()
                .filter(|o| o.kind == unit.kind)
                .map(|o| u32::from(o.points_remaining) / unit.cost)
                .sum()
        };
        let tag = if max_qty > 0 {
            format!("<{}>", unit.number)
        } else {
            String::new()
        };
        HalfEntry {
            tag,
            name: unit.label,
            cost: unit.cost,
            qty: order_qty,
        }
    };

    let done_tag = "<0>".to_string();
    let right_units = [4usize, 5, 6, 7, 8];
    let left_units = [0usize, 1, 2, 3];

    let mut rows = Vec::with_capacity(5);
    let first_right = entry(&BUILD_UNITS[right_units[0]]);
    rows.push(SplitTableRow {
        left_cells: vec![done_tag, "DONE".to_string(), String::new(), String::new()],
        right_cells: vec![
            first_right.tag,
            first_right.name.to_string(),
            first_right.cost.to_string(),
            format!("({})", first_right.qty),
        ],
    });

    for i in 0..4 {
        let left = entry(&BUILD_UNITS[left_units[i]]);
        let right = entry(&BUILD_UNITS[right_units[i + 1]]);
        rows.push(SplitTableRow {
            left_cells: vec![
                left.tag,
                left.name.to_string(),
                left.cost.to_string(),
                format!("({})", left.qty),
            ],
            right_cells: vec![
                right.tag,
                right.name.to_string(),
                right.cost.to_string(),
                format!("({})", right.qty),
            ],
        });
    }

    write_split_table(
        buffer,
        5,
        &BUILD_HALF_COLUMNS,
        &BUILD_HALF_COLUMNS,
        &rows,
        style,
    )
}

pub fn build_unit_spec(number: u8) -> Option<BuildUnitSpec> {
    BUILD_UNITS
        .iter()
        .copied()
        .find(|unit| unit.number == number)
}

pub fn build_unit_spec_by_kind(kind: ProductionItemKind) -> Option<BuildUnitSpec> {
    BUILD_UNITS.iter().copied().find(|unit| unit.kind == kind)
}

pub fn build_order_summary(order: PlanetBuildOrder) -> String {
    let kind = build_kind_name(order.kind);
    format!("{kind} ({:>2} pts)", order.points_remaining)
}

pub fn build_kind_name(kind: ProductionItemKind) -> &'static str {
    match kind {
        ProductionItemKind::Destroyer => "Destroyers",
        ProductionItemKind::Cruiser => "Cruisers",
        ProductionItemKind::Battleship => "Battleships",
        ProductionItemKind::Scout => "Scouts",
        ProductionItemKind::Transport => "Troop transports",
        ProductionItemKind::Etac => "ETACs",
        ProductionItemKind::GroundBattery => "Ground batteries",
        ProductionItemKind::Army => "Armies",
        ProductionItemKind::Starbase => "Starbases",
        ProductionItemKind::Unknown(_) => "Unknown",
    }
}

pub fn infer_quantity(order: PlanetBuildOrder, cost: u32) -> Option<u32> {
    if cost == 0 {
        return None;
    }
    let points = u32::from(order.points_remaining);
    if points % cost == 0 {
        Some(points / cost)
    } else {
        None
    }
}

pub fn max_quantity(points_left: u32, cost: u32) -> u32 {
    if cost == 0 { 0 } else { points_left / cost }
}
