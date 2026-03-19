use super::helpers::sync_scroll_to_cursor;
use crate::app::state::App;
use crate::screen::{
    CommandMenu, PlanetBuildChangeRow, PlanetBuildListRow, PlanetBuildMenuView, PlanetBuildOrder,
    PlanetCommissionRow, PlanetCommissionView, PlanetListSort, ScreenId, build_unit_spec,
    build_unit_spec_by_kind, max_quantity,
};
use ec_data::{
    AutoCommissionSummary, CommissionResult, GameStateMutationError, ProductionItemKind,
};
use std::collections::BTreeMap;

impl App {
    pub fn open_planet_auto_commission_confirm(&mut self) {
        self.planet.auto_commission_status = None;
        if self.commission_planet_rows().is_empty() {
            self.show_command_menu_notice(
                CommandMenu::Planet,
                "No ships or starbases are waiting in stardock.",
            );
        } else {
            self.clear_command_menu_notice();
            self.current_screen = ScreenId::PlanetAutoCommissionConfirm;
        }
    }

    pub fn open_planet_commission_menu(&mut self) {
        self.command_return_menu = CommandMenu::Planet;
        self.planet.commission_status = None;
        self.planet.commission_cursor = 0;
        self.planet.commission_scroll_offset = 0;
        self.planet.commission_selected_slots.clear();
        let total = self.commission_planet_rows().len();
        if total == 0 {
            self.planet.commission_index = 0;
            self.show_command_menu_notice(
                CommandMenu::Planet,
                "No owned planets have units waiting in stardock.",
            );
            return;
        } else {
            self.clear_command_menu_notice();
            self.planet.commission_index = self.planet.commission_index.min(total - 1);
        }
        self.current_screen = ScreenId::PlanetCommissionMenu;
    }

    pub fn open_planet_build_help(&mut self) {
        self.clear_command_menu_notice();
        self.current_screen = ScreenId::PlanetBuildHelp;
    }

    pub fn open_planet_build_menu(&mut self) {
        self.command_return_menu = CommandMenu::PlanetBuild;
        self.planet.build_status = None;
        self.planet.build_unit_input.clear();
        self.planet.build_unit_status = None;
        self.planet.build_quantity_input.clear();
        self.planet.build_quantity_status = None;
        self.planet.build_selected_kind = None;
        self.planet.build_list_scroll_offset = 0;
        let total = self.build_planet_rows().len();
        if total == 0 {
            self.planet.build_index = 0;
            self.show_command_menu_notice(
                CommandMenu::Planet,
                "No owned planets available for building.",
            );
            return;
        }
        self.clear_command_menu_notice();
        self.planet.build_index = self.planet.build_index.min(total - 1);
        self.current_screen = ScreenId::PlanetBuildMenu;
    }

    pub fn open_planet_build_review(&mut self) {
        if self.build_planet_rows().is_empty() {
            self.open_planet_build_menu();
            return;
        }
        self.current_screen = ScreenId::PlanetBuildReview;
    }

    pub fn open_planet_build_list(&mut self) {
        if self.build_planet_rows().is_empty() {
            self.open_planet_build_menu();
            return;
        }
        self.planet.build_list_scroll_offset = 0;
        self.planet.build_list_cursor = 0;
        self.planet.build_list_confirming = false;
        self.current_screen = ScreenId::PlanetBuildList;
    }

    pub fn open_planet_build_change(&mut self) {
        // Pre-position cursor on the current planet so it's already highlighted.
        self.planet.build_change_cursor = self.planet.build_index;
        self.planet.build_change_scroll_offset = 0;
        sync_scroll_to_cursor(
            &mut self.planet.build_change_scroll_offset,
            self.planet.build_change_cursor,
            crate::screen::PLANET_BUILD_CHANGE_VISIBLE_ROWS,
        );
        self.current_screen = ScreenId::PlanetBuildChange;
    }

    pub fn move_planet_build_change_cursor(&mut self, delta: i8) {
        if self.current_screen != ScreenId::PlanetBuildChange {
            return;
        }
        let total = self.build_planet_rows().len();
        if total == 0 {
            return;
        }
        let next = self.planet.build_change_cursor as isize + delta as isize;
        self.planet.build_change_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.planet.build_change_scroll_offset,
            self.planet.build_change_cursor,
            crate::screen::PLANET_BUILD_CHANGE_VISIBLE_ROWS,
        );
    }

    pub fn confirm_planet_build_change(&mut self) {
        let total = self.build_planet_rows().len();
        if total == 0 {
            self.current_screen = ScreenId::PlanetBuildMenu;
            return;
        }
        self.planet.build_index = self.planet.build_change_cursor.min(total - 1);
        self.planet.build_status = None;
        self.current_screen = ScreenId::PlanetBuildMenu;
    }

    pub fn open_planet_build_abort_confirm(&mut self) {
        if self.build_planet_rows().is_empty() {
            self.open_planet_build_menu();
            return;
        }
        self.current_screen = ScreenId::PlanetBuildAbortConfirm;
    }

    pub fn open_planet_build_specify(&mut self) {
        if self.build_planet_rows().is_empty() {
            self.open_planet_build_menu();
            return;
        }
        self.planet.build_unit_input.clear();
        self.planet.build_unit_status = None;
        self.planet.build_quantity_input.clear();
        self.planet.build_quantity_status = None;
        self.planet.build_selected_kind = None;
        self.current_screen = ScreenId::PlanetBuildSpecify;
    }

    pub fn move_planet_build(&mut self, delta: i8) {
        let total = self.build_planet_rows().len();
        if total == 0 {
            self.planet.build_index = 0;
            return;
        }
        // Wrap around so N on the last planet returns to the first.
        let next = self.planet.build_index as isize + delta as isize;
        self.planet.build_index = next.rem_euclid(total as isize) as usize;
        self.planet.build_status = None;
    }

    pub fn move_planet_commission_planet(&mut self, delta: i8) {
        if self.current_screen != ScreenId::PlanetCommissionMenu {
            return;
        }
        let total = self.commission_planet_rows().len();
        if total == 0 {
            self.planet.commission_index = 0;
            return;
        }
        let next = self.planet.commission_index as isize + delta as isize;
        self.planet.commission_index = next.rem_euclid(total as isize) as usize;
        self.planet.commission_cursor = 0;
        self.planet.commission_scroll_offset = 0;
        self.planet.commission_selected_slots.clear();
        self.planet.commission_status = None;
    }

    pub fn move_planet_commission_row(&mut self, delta: i8) {
        if self.current_screen != ScreenId::PlanetCommissionMenu {
            return;
        }
        let total = self.current_planet_commission_rows().len();
        if total == 0 {
            self.planet.commission_cursor = 0;
            return;
        }
        let next = self.planet.commission_cursor as isize + delta as isize;
        self.planet.commission_cursor = next.rem_euclid(total as isize) as usize;
        if self.planet.commission_cursor < self.planet.commission_scroll_offset {
            self.planet.commission_scroll_offset = self.planet.commission_cursor;
        } else if self.planet.commission_cursor
            >= self.planet.commission_scroll_offset + crate::screen::PLANET_COMMISSION_VISIBLE_ROWS
        {
            self.planet.commission_scroll_offset =
                self.planet.commission_cursor + 1 - crate::screen::PLANET_COMMISSION_VISIBLE_ROWS;
        }
    }

    pub fn commission_selected_stardock_row(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::PlanetCommissionMenu {
            return Ok(());
        }
        let rows = self.current_planet_commission_rows();
        let Some(current_row) = rows.get(self.planet.commission_cursor) else {
            self.planet.commission_status = Some("No stardock units are available.".to_string());
            return Ok(());
        };
        let selected_slots: Vec<usize> = if self.planet.commission_selected_slots.is_empty() {
            vec![current_row.slot_0_based]
        } else {
            rows.iter()
                .filter(|row| {
                    self.planet
                        .commission_selected_slots
                        .contains(&row.slot_0_based)
                })
                .map(|row| row.slot_0_based)
                .collect()
        };
        let planet_record = self
            .current_commission_planet_row()?
            .planet_record_index_1_based;
        let result = match self.game_data.commission_planet_stardock_slots(
            self.player.record_index_1_based,
            planet_record,
            &selected_slots,
        ) {
            Ok(result) => result,
            Err(GameStateMutationError::InvalidCommissionSelection) => {
                self.planet.commission_status = Some(
                    "Select either ships for one fleet or one starbase by itself.".to_string(),
                );
                return Ok(());
            }
            Err(err) => return Err(err.into()),
        };
        self.save_game_data()?;
        match result {
            CommissionResult::Fleet {
                fleet_record_index_1_based,
            } => {
                let _ = self
                    .game_data
                    .fleets
                    .records
                    .get(fleet_record_index_1_based - 1)
                    .map(|fleet| fleet.local_slot_word_raw())
                    .ok_or("commissioned fleet record missing")?;
            }
            CommissionResult::Starbase {
                base_record_index_1_based: _,
            } => {}
        }
        self.planet.commission_status = None;

        let planet_rows = self.commission_planet_rows();
        if planet_rows.is_empty() {
            self.planet.commission_index = 0;
            self.planet.commission_cursor = 0;
            self.planet.commission_scroll_offset = 0;
        } else {
            self.planet.commission_index = self.planet.commission_index.min(planet_rows.len() - 1);
            let current_rows = self.current_planet_commission_rows();
            if current_rows.is_empty() {
                self.move_planet_commission_planet(1);
            } else {
                self.planet.commission_cursor =
                    self.planet.commission_cursor.min(current_rows.len() - 1);
            }
        }
        self.planet.commission_selected_slots.clear();
        Ok(())
    }

    pub fn confirm_planet_auto_commission(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::PlanetAutoCommissionConfirm {
            return Ok(());
        }
        let summary = self
            .game_data
            .auto_commission_all_stardock_units(self.player.record_index_1_based)?;
        self.save_game_data()?;
        self.planet.auto_commission_status = Some(format_auto_commission_status(summary));
        self.current_screen = ScreenId::PlanetAutoCommissionDone;
        Ok(())
    }

    pub fn toggle_planet_commission_selection(&mut self) {
        if self.current_screen != ScreenId::PlanetCommissionMenu {
            return;
        }
        let rows = self.current_planet_commission_rows();
        let Some(row) = rows.get(self.planet.commission_cursor) else {
            return;
        };
        if self
            .planet
            .commission_selected_slots
            .contains(&row.slot_0_based)
        {
            self.planet
                .commission_selected_slots
                .remove(&row.slot_0_based);
        } else {
            self.planet
                .commission_selected_slots
                .insert(row.slot_0_based);
        }
        self.planet.commission_status = None;
    }

    pub fn scroll_planet_build_list(&mut self, delta: i8) {
        if self.current_screen != ScreenId::PlanetBuildList {
            return;
        }
        let total = self.planet_build_list_rows().len();
        let max_offset = total.saturating_sub(crate::screen::PLANET_BUILD_LIST_VISIBLE_ROWS);
        self.planet.build_list_scroll_offset = self
            .planet
            .build_list_scroll_offset
            .saturating_add_signed(delta as isize)
            .min(max_offset);
    }

    pub fn move_planet_build_list_cursor(&mut self, delta: i8) {
        if self.current_screen != ScreenId::PlanetBuildList {
            return;
        }
        let total = self.planet_build_list_rows().len();
        if total == 0 {
            self.planet.build_list_cursor = 0;
            return;
        }
        let next = self.planet.build_list_cursor as isize + delta as isize;
        self.planet.build_list_cursor = next.rem_euclid(total as isize) as usize;
        // Keep scroll window in sync: ensure cursor is visible.
        if self.planet.build_list_cursor < self.planet.build_list_scroll_offset {
            self.planet.build_list_scroll_offset = self.planet.build_list_cursor;
        } else if self.planet.build_list_cursor
            >= self.planet.build_list_scroll_offset + crate::screen::PLANET_BUILD_LIST_VISIBLE_ROWS
        {
            self.planet.build_list_scroll_offset =
                self.planet.build_list_cursor + 1 - crate::screen::PLANET_BUILD_LIST_VISIBLE_ROWS;
        }
    }

    pub fn delete_planet_build_slot_request(&mut self) {
        if self.current_screen != ScreenId::PlanetBuildList {
            return;
        }
        let rows = self.planet_build_list_rows();
        let Some(row) = rows.get(self.planet.build_list_cursor) else {
            return;
        };
        if row.queue_qty == 0 {
            return;
        }
        self.planet.build_list_confirming = true;
    }

    pub fn confirm_delete_planet_build_slot(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.planet.build_list_confirming {
            return Ok(());
        }
        let rows = self.planet_build_list_rows();
        let Some(row) = rows.get(self.planet.build_list_cursor) else {
            self.planet.build_list_confirming = false;
            return Ok(());
        };
        let planet_record = self.current_build_planet_row()?.planet_record_index_1_based;
        self.game_data
            .clear_planet_build_orders_by_kind(planet_record, row.kind)?;
        self.save_game_data()?;
        self.planet.build_list_confirming = false;
        // Clamp cursor after deletion.
        let new_total = self.planet_build_list_rows().len();
        if new_total == 0 {
            self.planet.build_list_cursor = 0;
        } else {
            self.planet.build_list_cursor = self.planet.build_list_cursor.min(new_total - 1);
        }
        Ok(())
    }

    pub fn cancel_delete_planet_build_slot(&mut self) {
        self.planet.build_list_confirming = false;
    }

    pub fn append_planet_build_unit_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::PlanetBuildSpecify
            && self.planet.build_unit_input.len() < 2
        {
            self.planet.build_unit_input.push(ch);
            self.planet.build_unit_status = None;
        }
    }

    pub fn backspace_planet_build_unit_input(&mut self) {
        if self.current_screen == ScreenId::PlanetBuildSpecify {
            self.planet.build_unit_input.pop();
            self.planet.build_unit_status = None;
        }
    }

    pub fn submit_planet_build_unit(&mut self) {
        let raw = self.planet.build_unit_input.trim();
        let number = if raw.is_empty() {
            0
        } else if let Ok(value) = raw.parse::<u8>() {
            value
        } else {
            self.planet.build_unit_status = Some("Enter a valid unit number.".to_string());
            return;
        };

        if number == 0 {
            self.current_screen = ScreenId::PlanetBuildMenu;
            return;
        }

        let Some(unit) = build_unit_spec(number) else {
            self.planet.build_unit_status = Some("That unit is not available.".to_string());
            return;
        };

        let Ok(max_qty) = self.current_planet_build_max_quantity_for(unit.kind) else {
            self.planet.build_unit_status = Some("No points are available to spend.".to_string());
            return;
        };
        if max_qty == 0 {
            self.planet.build_unit_status = Some(
                self.planet_build_unavailable_message(unit.kind)
                    .unwrap_or_else(|_| "No points are available to spend.".to_string()),
            );
            return;
        }

        self.planet.build_selected_kind = Some(unit.kind);
        self.planet.build_quantity_input.clear();
        self.planet.build_quantity_status = None;
        self.current_screen = ScreenId::PlanetBuildQuantity;
    }

    pub fn append_planet_build_quantity_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::PlanetBuildQuantity
            && self.planet.build_quantity_input.len() < 3
        {
            self.planet.build_quantity_input.push(ch);
            self.planet.build_quantity_status = None;
        }
    }

    pub fn backspace_planet_build_quantity_input(&mut self) {
        if self.current_screen == ScreenId::PlanetBuildQuantity {
            self.planet.build_quantity_input.pop();
            self.planet.build_quantity_status = None;
        }
    }

    pub fn submit_planet_build_quantity(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(kind) = self.planet.build_selected_kind else {
            self.current_screen = ScreenId::PlanetBuildSpecify;
            return Ok(());
        };
        let Some(unit) = build_unit_spec_by_kind(kind) else {
            self.current_screen = ScreenId::PlanetBuildSpecify;
            return Ok(());
        };
        let max_qty = self.current_planet_build_max_quantity_for(kind)?;
        if max_qty == 0 {
            self.planet.build_quantity_status = Some(self.planet_build_unavailable_message(kind)?);
            return Ok(());
        }

        let qty = if self.planet.build_quantity_input.trim().is_empty() {
            max_qty
        } else {
            match self.planet.build_quantity_input.trim().parse::<u32>() {
                Ok(value) => value,
                Err(_) => {
                    self.planet.build_quantity_status = Some("Enter a valid quantity.".to_string());
                    return Ok(());
                }
            }
        };

        if qty == 0 {
            self.current_screen = ScreenId::PlanetBuildSpecify;
            self.planet.build_quantity_input.clear();
            return Ok(());
        }
        if qty > max_qty {
            self.planet.build_quantity_status =
                Some(format!("Enter a quantity from 0 to {}.", max_qty));
            return Ok(());
        }

        let planet_record = self.current_build_planet_row()?.planet_record_index_1_based;

        // Armies and ground batteries go directly to the planet — no stardock needed.
        // For all other kinds (ships, starbases), each queued order will need one
        // stardock slot on completion. Warn and cap if the stardock is full.
        let needs_stardock = !matches!(
            kind,
            ProductionItemKind::Army | ProductionItemKind::GroundBattery
        );
        if needs_stardock {
            let free = self.game_data.planet_free_stardock_slots(planet_record)?;
            if free == 0 {
                self.planet.build_quantity_status =
                    Some("Stardock is full — commission ships first to free space.".to_string());
                return Ok(());
            }
        }

        let points = qty.saturating_mul(unit.cost);
        match self.game_data.append_planet_build_order(
            planet_record,
            points.min(u32::from(u8::MAX)) as u8,
            production_item_kind_raw(kind),
        ) {
            Ok(()) => {}
            Err(GameStateMutationError::PlanetBuildQueueFull { .. }) => {
                self.planet.build_quantity_status =
                    Some("Build queue is full (10 orders maximum).".to_string());
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        }
        self.save_game_data()?;
        self.planet.build_unit_input.clear();
        self.planet.build_unit_status = Some(format!("Queued {} {}.", qty, unit.label));
        self.planet.build_quantity_input.clear();
        self.planet.build_quantity_status = None;
        self.planet.build_selected_kind = None;
        self.current_screen = ScreenId::PlanetBuildSpecify;
        Ok(())
    }

    pub fn abort_current_planet_builds(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let row = self.current_build_planet_row()?;
        self.game_data
            .clear_planet_build_queue(row.planet_record_index_1_based)?;
        self.save_game_data()?;
        self.planet.build_status = Some("Build orders aborted.".to_string());
        self.current_screen = ScreenId::PlanetBuildMenu;
        Ok(())
    }

    pub(super) fn build_planet_rows(&self) -> Vec<ec_data::EmpirePlanetEconomyRow> {
        self.sorted_planet_rows(PlanetListSort::CurrentProduction)
    }

    pub(super) fn commission_planet_rows(&self) -> Vec<ec_data::EmpirePlanetEconomyRow> {
        self.build_planet_rows()
            .into_iter()
            .filter(|row| {
                self.game_data
                    .planets
                    .records
                    .get(row.planet_record_index_1_based - 1)
                    .map(|record| (0..10).any(|slot| record.stardock_kind_raw(slot) != 0))
                    .unwrap_or(false)
            })
            .collect()
    }

    pub(crate) fn current_planet_commission_view(
        &self,
    ) -> Result<PlanetCommissionView, Box<dyn std::error::Error>> {
        let row = match self.current_commission_planet_row() {
            Ok(row) => row,
            Err(_) => {
                return Ok(PlanetCommissionView {
                    planet_name: "No commissionable planets".to_string(),
                    coords: self.default_planet_prompt_coords(),
                    rows: vec![],
                });
            }
        };
        Ok(PlanetCommissionView {
            planet_name: row.planet_name,
            coords: row.coords,
            rows: self.current_planet_commission_rows(),
        })
    }

    fn current_planet_commission_rows(&self) -> Vec<PlanetCommissionRow> {
        let Ok(row) = self.current_commission_planet_row() else {
            return vec![];
        };
        let Some(record) = self
            .game_data
            .planets
            .records
            .get(row.planet_record_index_1_based - 1)
        else {
            return vec![];
        };
        (0..10)
            .filter_map(|slot| {
                let kind_raw = record.stardock_kind_raw(slot);
                let qty = u32::from(record.stardock_count_raw(slot));
                if kind_raw == 0 || qty == 0 {
                    return None;
                }
                let kind = ProductionItemKind::from_raw(kind_raw);
                let unit_label = build_unit_spec_by_kind(kind)
                    .map(|spec| spec.label.to_string())
                    .unwrap_or_else(|| format!("Unknown (kind {})", kind_raw));
                Some(PlanetCommissionRow {
                    slot_0_based: slot,
                    unit_label,
                    qty,
                })
            })
            .collect()
    }

    pub(crate) fn build_change_rows(&self) -> Vec<PlanetBuildChangeRow> {
        self.build_planet_rows()
            .into_iter()
            .map(|row| {
                let available_points = u32::from(row.build_capacity)
                    .min(row.stored_production_points.min(u32::from(u16::MAX)));
                let committed_points = self
                    .current_build_committed_points(row.planet_record_index_1_based)
                    .unwrap_or(0);
                PlanetBuildChangeRow {
                    planet_name: row.planet_name,
                    coords: row.coords,
                    present_production: row.present_production,
                    potential_production: row.potential_production,
                    available_points,
                    committed_points,
                }
            })
            .collect()
    }

    fn current_build_planet_row(
        &self,
    ) -> Result<ec_data::EmpirePlanetEconomyRow, Box<dyn std::error::Error>> {
        self.build_planet_rows()
            .get(self.planet.build_index)
            .cloned()
            .ok_or_else(|| "current build planet missing".into())
    }

    pub(crate) fn current_planet_build_orders(&self) -> Vec<PlanetBuildOrder> {
        let Ok(row) = self.current_build_planet_row() else {
            return vec![];
        };
        let Some(record) = self
            .game_data
            .planets
            .records
            .get(row.planet_record_index_1_based - 1)
        else {
            return vec![];
        };
        (0..10)
            .filter_map(|slot| {
                let points = record.build_count_raw(slot);
                let kind_raw = record.build_kind_raw(slot);
                if points == 0 || kind_raw == 0 {
                    None
                } else {
                    Some(PlanetBuildOrder {
                        kind: ProductionItemKind::from_raw(kind_raw),
                        points_remaining: points,
                    })
                }
            })
            .collect()
    }

    pub(crate) fn current_planet_build_view(
        &self,
    ) -> Result<PlanetBuildMenuView, Box<dyn std::error::Error>> {
        let row = match self.current_build_planet_row() {
            Ok(row) => row,
            Err(_) => {
                return Ok(PlanetBuildMenuView {
                    row: ec_data::EmpirePlanetEconomyRow {
                        planet_record_index_1_based: 0,
                        coords: self.default_planet_prompt_coords(),
                        planet_name: "No owned planets".to_string(),
                        present_production: 0,
                        potential_production: 0,
                        stored_production_points: 0,
                        yearly_tax_revenue: 0,
                        yearly_growth_delta: 0,
                        build_capacity: 0,
                        has_friendly_starbase: false,
                        armies: 0,
                        ground_batteries: 0,
                        is_homeworld_seed: false,
                    },
                    committed_points: 0,
                    available_points: 0,
                    points_left: 0,
                    queue_used: 0,
                    queue_capacity: 10,
                    stardock_used: 0,
                    stardock_capacity: 10,
                });
            }
        };
        let committed_points =
            self.current_build_committed_points(row.planet_record_index_1_based)?;
        let available_points = u32::from(row.build_capacity)
            .min(row.stored_production_points.min(u32::from(u16::MAX)));
        let points_left = available_points.saturating_sub(committed_points);
        let record = self
            .game_data
            .planets
            .records
            .get(row.planet_record_index_1_based - 1)
            .ok_or("planet record missing")?;
        let queue_capacity: usize = 10;
        let queue_used = (0..queue_capacity)
            .filter(|&s| record.build_count_raw(s) != 0 || record.build_kind_raw(s) != 0)
            .count();
        let stardock_capacity: usize = 10;
        let stardock_open_now = self
            .game_data
            .planet_open_stardock_slots_now(row.planet_record_index_1_based)?;
        let stardock_used = stardock_capacity.saturating_sub(stardock_open_now);
        Ok(PlanetBuildMenuView {
            row,
            committed_points,
            available_points,
            points_left,
            queue_used,
            queue_capacity,
            stardock_used,
            stardock_capacity,
        })
    }

    fn current_build_committed_points(
        &self,
        planet_record_index_1_based: usize,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        let record = self
            .game_data
            .planets
            .records
            .get(planet_record_index_1_based - 1)
            .ok_or("planet record missing")?;
        Ok((0..10)
            .map(|slot| u32::from(record.build_count_raw(slot)))
            .sum::<u32>())
    }

    pub(crate) fn current_planet_build_max_quantity(
        &self,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        let kind = self
            .planet
            .build_selected_kind
            .ok_or("planet build kind missing")?;
        self.current_planet_build_max_quantity_for(kind)
    }

    fn current_planet_build_max_quantity_for(
        &self,
        kind: ProductionItemKind,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        let view = self.current_planet_build_view()?;
        let unit = build_unit_spec_by_kind(kind).ok_or("unit spec missing")?;
        let mut max_qty = max_quantity(view.points_left, unit.cost);
        match kind {
            ProductionItemKind::Army => {
                let free = self
                    .game_data
                    .planet_free_army_capacity(view.row.planet_record_index_1_based)?;
                max_qty = max_qty.min(u32::from(free));
            }
            ProductionItemKind::GroundBattery => {
                let free = self
                    .game_data
                    .planet_free_ground_battery_capacity(view.row.planet_record_index_1_based)?;
                max_qty = max_qty.min(u32::from(free));
            }
            _ => {}
        }
        Ok(max_qty)
    }

    fn planet_build_unavailable_message(
        &self,
        kind: ProductionItemKind,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let view = self.current_planet_build_view()?;
        if view.points_left == 0 {
            return Ok("No points are available to spend.".to_string());
        }
        Ok(match kind {
            ProductionItemKind::Army => "Planet already has the maximum 255 armies.",
            ProductionItemKind::GroundBattery => {
                "Planet already has the maximum 255 ground batteries."
            }
            _ => "No points are available to spend.",
        }
        .to_string())
    }

    pub(crate) fn planet_build_list_rows(&self) -> Vec<PlanetBuildListRow> {
        let Ok(row) = self.current_build_planet_row() else {
            return vec![];
        };
        let Some(record) = self
            .game_data
            .planets
            .records
            .get(row.planet_record_index_1_based - 1)
        else {
            return vec![];
        };
        let mut queue_qty_by_kind: BTreeMap<u8, u32> = BTreeMap::new();
        let mut stardock_qty_by_kind: BTreeMap<u8, u32> = BTreeMap::new();

        for slot in 0..10 {
            let points = u32::from(record.build_count_raw(slot));
            let kind_raw = record.build_kind_raw(slot);
            if points == 0 || kind_raw == 0 {
                continue;
            }
            let kind = ProductionItemKind::from_raw(kind_raw);
            let cost = u32::from(build_unit_spec_by_kind(kind).map(|u| u.cost).unwrap_or(1));
            let qty = if cost > 0 { points / cost } else { 0 };
            *queue_qty_by_kind.entry(kind_raw).or_default() += qty.max(1);
        }

        for slot in 0..10 {
            let qty = u32::from(record.stardock_count_raw(slot));
            let kind_raw = record.stardock_kind_raw(slot);
            if qty == 0 || kind_raw == 0 {
                continue;
            }
            *stardock_qty_by_kind.entry(kind_raw).or_default() += qty;
        }

        let mut ordered_kind_raws = vec![1, 2, 3, 4, 5, 6, 9, 8, 7];
        for kind_raw in queue_qty_by_kind.keys().chain(stardock_qty_by_kind.keys()) {
            if !ordered_kind_raws.contains(kind_raw) {
                ordered_kind_raws.push(*kind_raw);
            }
        }

        ordered_kind_raws
            .into_iter()
            .filter_map(|kind_raw| {
                let queue_qty = queue_qty_by_kind.get(&kind_raw).copied().unwrap_or(0);
                let stardock_qty = stardock_qty_by_kind.get(&kind_raw).copied().unwrap_or(0);
                if queue_qty == 0 && stardock_qty == 0 {
                    return None;
                }
                let kind = ProductionItemKind::from_raw(kind_raw);
                let (unit_label, cost) = build_unit_spec_by_kind(kind)
                    .map(|u| (u.label.to_string(), u.cost))
                    .unwrap_or_else(|| (format!("Unknown (kind {})", kind_raw), 0));
                Some(PlanetBuildListRow {
                    kind,
                    unit_label,
                    points: u32::from(cost),
                    queue_qty,
                    stardock_qty: if kind.requires_stardock() {
                        Some(stardock_qty)
                    } else {
                        None
                    },
                })
            })
            .collect()
    }
}

fn production_item_kind_raw(kind: ProductionItemKind) -> u8 {
    match kind {
        ProductionItemKind::Destroyer => 1,
        ProductionItemKind::Cruiser => 2,
        ProductionItemKind::Battleship => 3,
        ProductionItemKind::Scout => 4,
        ProductionItemKind::Transport => 5,
        ProductionItemKind::Etac => 6,
        ProductionItemKind::GroundBattery => 7,
        ProductionItemKind::Army => 8,
        ProductionItemKind::Starbase => 9,
        ProductionItemKind::Unknown(raw) => raw,
    }
}

fn format_auto_commission_status(summary: AutoCommissionSummary) -> String {
    format!(
        "Commissioned {} ships into {} new fleets and {} starbases from {} planets.",
        summary.ships_commissioned,
        summary.fleets_created,
        summary.starbases_commissioned,
        summary.planets_used
    )
}
