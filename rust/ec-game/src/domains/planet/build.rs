use crate::app::helpers::sync_scroll_to_cursor;
use crate::app::state::App;
use crate::screen::{
    CommandMenu, PLANET_AUTO_COMMISSION_REPORT_PAGE_ROWS, PlanetBuildChangeRow, PlanetBuildListRow,
    PlanetBuildMenuView, PlanetBuildOrder, PlanetCommissionDraftRow, PlanetCommissionPickerRow,
    PlanetCommissionRow, PlanetCommissionView, PlanetListSort, ScreenId, build_unit_spec,
    build_unit_spec_by_kind, format_fleet_number, format_sector_coords, format_sector_coords_table,
    max_quantity,
};
use crossterm::event::KeyCode;
use ec_data::{
    AutoCommissionEntry, AutoCommissionFleetEntry, AutoCommissionReport,
    AutoCommissionStarbaseEntry, CommissionFleetDraft, CommissionResult, GameStateMutationError,
    ProductionItemKind, STARDOCK_SLOT_COUNT,
};
use std::collections::BTreeMap;

impl App {
    pub fn open_planet_commission_menu(&mut self) {
        self.command_return_menu = CommandMenu::Planet;
        self.close_planet_tax_prompt();
        self.close_planet_auto_commission_prompt();
        self.close_planet_info_prompt();
        self.clear_command_menu_notice();
        self.clear_planet_commission_draft_state();
        self.planet.commission_status = None;
        self.planet.commission_result_title = None;
        self.planet.commission_result_return_to_picker = false;
        self.planet.commission_result_dismiss_key = None;
        self.planet.commission_result_notice = None;
        self.planet.commission_cursor = 0;
        self.planet.commission_scroll_offset = 0;
        self.planet.commission_selected_slots.clear();
        let total = self.commission_planet_rows().len();
        if total == 0 {
            self.planet.commission_index = 0;
            self.planet.commission_picker_scroll_offset = 0;
            self.show_command_menu_notice(
                CommandMenu::Planet,
                "No owned planets have units waiting in stardock.",
            );
            return;
        }
        self.planet.commission_index = self.planet.commission_index.min(total - 1);
        sync_scroll_to_cursor(
            &mut self.planet.commission_picker_scroll_offset,
            self.planet.commission_index,
            crate::screen::PLANET_COMMISSION_PICKER_VISIBLE_ROWS,
        );
        self.current_screen = ScreenId::PlanetCommissionPicker;
    }

    pub fn open_planet_commission_planet(&mut self) {
        if self.current_screen != ScreenId::PlanetCommissionPicker {
            return;
        }
        let total = self.commission_planet_rows().len();
        if total == 0 {
            self.open_planet_commission_menu();
            return;
        }
        self.planet.commission_index = self.planet.commission_index.min(total - 1);
        self.clear_planet_commission_draft_state();
        self.planet.commission_cursor = 0;
        self.planet.commission_scroll_offset = 0;
        self.planet.commission_selected_slots.clear();
        self.planet.commission_status = None;
        self.planet.commission_result_title = None;
        self.planet.commission_result_return_to_picker = false;
        self.planet.commission_result_dismiss_key = None;
        self.planet.commission_result_notice = None;
        self.load_planet_commission_draft_for_current_planet();
        self.current_screen = ScreenId::PlanetCommissionDraft;
    }

    pub fn close_planet_commission_planet(&mut self) {
        if self.current_screen != ScreenId::PlanetCommissionMenu {
            return;
        }
        self.clear_planet_commission_draft_state();
        self.planet.commission_selected_slots.clear();
        self.planet.commission_status = None;
        self.planet.commission_result_dismiss_key = None;
        self.current_screen = ScreenId::PlanetCommissionPicker;
    }

    pub fn dismiss_planet_commission_result(&mut self, key_code: KeyCode) {
        if self.current_screen != ScreenId::PlanetCommissionResult {
            return;
        }
        let return_to_picker = self.planet.commission_result_return_to_picker;
        self.clear_planet_commission_draft_state();
        self.planet.commission_result_title = None;
        self.planet.commission_result_return_to_picker = false;
        self.planet.commission_result_notice = None;
        self.planet.commission_status = None;
        self.planet.commission_selected_slots.clear();
        if return_to_picker {
            let total = self.commission_planet_rows().len();
            if total == 0 {
                self.planet.commission_result_dismiss_key = None;
                self.open_planet_menu();
                return;
            }
            self.planet.commission_index = self.planet.commission_index.min(total - 1);
            sync_scroll_to_cursor(
                &mut self.planet.commission_picker_scroll_offset,
                self.planet.commission_index,
                crate::screen::PLANET_COMMISSION_PICKER_VISIBLE_ROWS,
            );
            self.planet.commission_result_dismiss_key = Some(key_code);
            self.current_screen = ScreenId::PlanetCommissionPicker;
            return;
        }
        let current_planet_has_rows = self
            .current_commission_planet_row()
            .ok()
            .map(|row| {
                !self
                    .current_planet_commission_rows_for(row.planet_record_index_1_based)
                    .is_empty()
            })
            .unwrap_or(false);
        if current_planet_has_rows {
            self.load_planet_commission_draft_for_current_planet();
            self.planet.commission_result_dismiss_key = Some(key_code);
            self.current_screen = ScreenId::PlanetCommissionDraft;
            return;
        }
        let total = self.commission_planet_rows().len();
        if total == 0 {
            self.planet.commission_result_dismiss_key = None;
            self.open_planet_menu();
            return;
        }
        self.planet.commission_index = self.planet.commission_index.min(total - 1);
        sync_scroll_to_cursor(
            &mut self.planet.commission_picker_scroll_offset,
            self.planet.commission_index,
            crate::screen::PLANET_COMMISSION_PICKER_VISIBLE_ROWS,
        );
        self.planet.commission_result_dismiss_key = Some(key_code);
        self.current_screen = ScreenId::PlanetCommissionPicker;
    }

    pub fn clear_planet_commission_dismiss_key(&mut self) {
        self.planet.commission_result_dismiss_key = None;
    }

    pub fn open_planet_build_help(&mut self) {
        self.clear_command_menu_notice();
        self.current_screen = ScreenId::PlanetBuildHelp;
    }

    pub fn open_planet_build_menu(&mut self) {
        self.command_return_menu = CommandMenu::PlanetBuild;
        self.close_planet_build_abort_prompt();
        self.planet.build_status = None;
        self.planet.build_unit_input.clear();
        self.planet.build_unit_status = None;
        self.planet.build_unit_notice = None;
        self.planet.build_quantity_input.clear();
        self.planet.build_quantity_status = None;
        self.planet.build_selected_kind = None;
        self.planet.build_list_scroll_offset = 0;
        self.reset_planet_build_list_delete_state();
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

    pub fn open_current_build_planet_info(&mut self) {
        if self.build_planet_rows().is_empty() {
            self.open_planet_build_menu();
            return;
        }
        let Ok(row) = self.current_build_planet_row() else {
            self.open_planet_build_menu();
            return;
        };
        self.command_return_menu = CommandMenu::PlanetBuild;
        let _ = self.open_planet_info_detail_at_coords(row.coords, Some(ScreenId::PlanetBuildMenu));
    }

    pub fn open_planet_build_list(&mut self) {
        if self.build_planet_rows().is_empty() {
            self.open_planet_build_menu();
            return;
        }
        self.planet.build_list_scroll_offset = 0;
        self.planet.build_list_cursor = 0;
        self.reset_planet_build_list_delete_state();
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

    pub fn open_planet_build_abort_prompt(&mut self) {
        if self.build_planet_rows().is_empty() {
            self.open_planet_build_menu();
            return;
        }
        if self.current_planet_build_orders().is_empty() {
            self.planet.build_status = Some("No build orders are queued.".to_string());
            self.current_screen = ScreenId::PlanetBuildMenu;
            return;
        }
        self.close_planet_info_prompt();
        self.planet.build_status = None;
        self.planet.build_abort_prompt_active = true;
        self.current_screen = ScreenId::PlanetBuildMenu;
    }

    pub fn close_planet_build_abort_prompt(&mut self) {
        self.planet.build_abort_prompt_active = false;
    }

    pub fn open_planet_build_specify(&mut self) {
        if self.build_planet_rows().is_empty() {
            self.open_planet_build_menu();
            return;
        }
        self.planet.build_unit_input.clear();
        self.planet.build_unit_status = None;
        self.planet.build_unit_notice = None;
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
        if self.current_screen != ScreenId::PlanetCommissionPicker {
            return;
        }
        let total = self.commission_planet_rows().len();
        if total == 0 {
            self.planet.commission_index = 0;
            return;
        }
        let next = self.planet.commission_index as isize + delta as isize;
        self.planet.commission_index = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.planet.commission_picker_scroll_offset,
            self.planet.commission_index,
            crate::screen::PLANET_COMMISSION_PICKER_VISIBLE_ROWS,
        );
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
        self.planet.commission_status = None;
    }

    pub fn move_planet_commission_draft_row(
        &mut self,
        delta: i8,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::PlanetCommissionDraft {
            return Ok(());
        }
        if !self.commit_current_planet_commission_draft_input()? {
            return Ok(());
        }
        let total = self.planet.commission_draft_rows.len();
        if total == 0 {
            self.planet.commission_draft_cursor = 0;
            return Ok(());
        }
        let next = self.planet.commission_draft_cursor as isize + delta as isize;
        self.planet.commission_draft_cursor = next.rem_euclid(total as isize) as usize;
        self.planet.commission_draft_input.clear();
        self.planet.commission_draft_status = None;
        Ok(())
    }

    pub fn append_planet_commission_draft_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::PlanetCommissionDraft
            || !ch.is_ascii_digit()
            || self.planet.commission_draft_input.len() >= 3
        {
            return;
        }
        let Some(row) = self
            .planet
            .commission_draft_rows
            .get(self.planet.commission_draft_cursor)
        else {
            return;
        };
        if !row.accepts_fleet_qty() {
            return;
        }
        self.planet.commission_draft_input.push(ch);
        self.planet.commission_draft_status = None;
        self.planet.commission_draft_notice = None;
    }

    pub fn backspace_planet_commission_draft_input(&mut self) {
        if self.current_screen != ScreenId::PlanetCommissionDraft {
            return;
        }
        let Some(row) = self
            .planet
            .commission_draft_rows
            .get(self.planet.commission_draft_cursor)
        else {
            return;
        };
        if !row.accepts_fleet_qty() {
            return;
        }
        self.planet.commission_draft_input.pop();
        self.planet.commission_draft_status = None;
        self.planet.commission_draft_notice = None;
    }

    pub fn close_planet_commission_draft(&mut self) {
        if self.current_screen != ScreenId::PlanetCommissionDraft {
            return;
        }
        self.clear_planet_commission_draft_state();
        self.planet.commission_status = None;
        self.planet.commission_result_dismiss_key = None;
        if self.commission_planet_rows().is_empty() {
            self.open_planet_menu();
            return;
        }
        self.current_screen = ScreenId::PlanetCommissionPicker;
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
        let selected_rows: Vec<PlanetCommissionRow> = rows
            .iter()
            .filter(|row| selected_slots.contains(&row.slot_0_based))
            .cloned()
            .collect();
        let starbase_count = selected_rows
            .iter()
            .filter(|row| row.kind == ProductionItemKind::Starbase)
            .count();
        let ship_count = selected_rows
            .iter()
            .filter(|row| is_commission_ship_kind(row.kind))
            .count();
        if starbase_count > 1 || (starbase_count == 1 && ship_count > 0) {
            self.planet.commission_status =
                Some("Select either ships for one fleet or one starbase by itself.".to_string());
            return Ok(());
        }
        if ship_count > 0 {
            self.load_planet_commission_draft_for_current_planet();
            self.current_screen = ScreenId::PlanetCommissionDraft;
            return Ok(());
        }

        let commission_title = self.current_planet_commission_title()?;
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
        let result_notice = match result {
            CommissionResult::Fleet {
                fleet_record_index_1_based,
            } => {
                let fleet = self
                    .game_data
                    .fleets
                    .records
                    .get(fleet_record_index_1_based - 1)
                    .ok_or("commissioned fleet record missing")?;
                let max_fleet_number = self
                    .game_data
                    .fleets
                    .records
                    .iter()
                    .filter(|fleet| {
                        fleet.owner_empire_raw() as usize == self.player.record_index_1_based
                    })
                    .map(|fleet| fleet.local_slot_word_raw())
                    .max()
                    .unwrap_or(fleet.local_slot_word_raw());
                format!(
                    "Commissioned selected ships into Fleet {}.",
                    format_fleet_number(fleet.local_slot_word_raw(), max_fleet_number)
                )
            }
            CommissionResult::Starbase {
                base_record_index_1_based,
            } => {
                let base = self
                    .game_data
                    .bases
                    .records
                    .get(base_record_index_1_based - 1)
                    .ok_or("commissioned starbase record missing")?;
                format!("Commissioned Starbase {:02}.", base.local_slot_raw())
            }
        };
        self.planet.commission_status = None;
        self.clear_planet_commission_draft_state();
        self.planet.commission_result_title = Some(commission_title);
        self.planet.commission_result_return_to_picker = false;
        self.planet.commission_result_notice = Some(result_notice);

        let planet_rows = self.commission_planet_rows();
        if planet_rows.is_empty() {
            self.planet.commission_index = 0;
            self.planet.commission_cursor = 0;
            self.planet.commission_scroll_offset = 0;
            self.planet.commission_picker_scroll_offset = 0;
        } else {
            self.planet.commission_index = self.planet.commission_index.min(planet_rows.len() - 1);
            sync_scroll_to_cursor(
                &mut self.planet.commission_picker_scroll_offset,
                self.planet.commission_index,
                crate::screen::PLANET_COMMISSION_PICKER_VISIBLE_ROWS,
            );
        }
        self.planet.commission_selected_slots.clear();
        self.current_screen = ScreenId::PlanetCommissionResult;
        Ok(())
    }

    fn commission_current_planet_direct_stardock_slot(
        &mut self,
        slot_0_based: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let commission_title = self.current_planet_commission_title()?;
        let planet_record = self
            .current_commission_planet_row()?
            .planet_record_index_1_based;
        let result = self.game_data.commission_planet_stardock_slots(
            self.player.record_index_1_based,
            planet_record,
            &[slot_0_based],
        )?;
        self.save_game_data()?;

        let result_notice = match result {
            CommissionResult::Fleet {
                fleet_record_index_1_based,
            } => self.commission_fleet_notice(fleet_record_index_1_based)?,
            CommissionResult::Starbase {
                base_record_index_1_based,
            } => {
                let base = self
                    .game_data
                    .bases
                    .records
                    .get(base_record_index_1_based - 1)
                    .ok_or("commissioned starbase record missing")?;
                format!("Commissioned Starbase {:02}.", base.local_slot_raw())
            }
        };

        self.clear_planet_commission_draft_state();
        self.planet.commission_status = None;
        self.planet.commission_result_title = Some(commission_title);
        self.planet.commission_result_return_to_picker = false;
        self.planet.commission_result_notice = Some(result_notice);
        self.current_screen = ScreenId::PlanetCommissionResult;
        Ok(())
    }

    pub fn submit_planet_commission_draft(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::PlanetCommissionDraft {
            return Ok(());
        }
        if !self.commit_current_planet_commission_draft_input()? {
            return Ok(());
        }

        let draft = self.current_planet_commission_draft()?;
        let current_row = self
            .planet
            .commission_draft_rows
            .get(self.planet.commission_draft_cursor)
            .cloned();
        if draft.total_ships() == 0 {
            if let Some(slot_0_based) = current_row.and_then(|row| row.direct_slot_0_based) {
                return self.commission_current_planet_direct_stardock_slot(slot_0_based);
            }
        }
        if draft.total_ships() == 0 {
            self.planet.commission_draft_status =
                Some("Select at least one ship for this fleet.".to_string());
            return Ok(());
        }

        let draft_title = self.current_planet_commission_draft_title()?;
        let planet_record = self
            .current_commission_planet_row()?
            .planet_record_index_1_based;
        let result = self.game_data.commission_planet_stardock_slots_with_draft(
            self.player.record_index_1_based,
            planet_record,
            &self.planet.commission_draft_slots,
            draft,
        )?;
        self.save_game_data()?;

        let fleet_record_index_1_based = match result {
            CommissionResult::Fleet {
                fleet_record_index_1_based,
            } => fleet_record_index_1_based,
            CommissionResult::Starbase { .. } => {
                return Err("ship draft unexpectedly commissioned a starbase".into());
            }
        };
        let notice = self.commission_fleet_notice(fleet_record_index_1_based)?;
        let remaining_rows = self.current_planet_commission_rows_for(planet_record);
        let (remaining_slots, remaining_rows) =
            build_planet_commission_draft_state(&remaining_rows);
        self.planet.commission_selected_slots.clear();
        self.planet.commission_draft_input.clear();
        self.planet.commission_draft_status = None;

        let has_remaining_ships = remaining_rows.iter().any(|row| row.accepts_fleet_qty());
        if !has_remaining_ships {
            self.clear_planet_commission_draft_state();
            self.planet.commission_result_title = Some(draft_title);
            self.planet.commission_result_return_to_picker = true;
            self.planet.commission_result_notice = Some(notice);
            self.current_screen = ScreenId::PlanetCommissionResult;
            return Ok(());
        }

        self.planet.commission_draft_slots = remaining_slots;
        self.planet.commission_draft_rows = remaining_rows;
        self.planet.commission_draft_cursor = self
            .planet
            .commission_draft_cursor
            .min(self.planet.commission_draft_rows.len() - 1);
        self.planet.commission_draft_notice = Some(notice);
        Ok(())
    }

    pub fn confirm_planet_auto_commission(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.inline_planet_auto_commission_active_on_current_screen() {
            return Ok(());
        }
        let report = self
            .game_data
            .auto_commission_all_stardock_units(self.player.record_index_1_based)?;
        self.save_game_data()?;
        self.close_planet_auto_commission_prompt();
        let rows = self.build_auto_commission_report_rows(&report);
        if rows.is_empty() {
            self.show_command_menu_notice(
                CommandMenu::Planet,
                "No ships or starbases are waiting in stardock.",
            );
            return Ok(());
        }
        self.planet.auto_commission_report_revealed_rows =
            rows.len().min(PLANET_AUTO_COMMISSION_REPORT_PAGE_ROWS);
        self.planet.auto_commission_report_rows = rows;
        self.current_screen = ScreenId::PlanetAutoCommissionReport;
        Ok(())
    }

    pub fn advance_planet_auto_commission_report(&mut self) {
        if self.current_screen != ScreenId::PlanetAutoCommissionReport {
            return;
        }
        let total_rows = self.planet.auto_commission_report_rows.len();
        if total_rows == 0 {
            self.open_planet_menu();
            return;
        }
        if self.planet.auto_commission_report_revealed_rows >= total_rows {
            self.open_planet_menu();
            return;
        }
        self.planet.auto_commission_report_revealed_rows = usize::min(
            self.planet.auto_commission_report_revealed_rows
                + PLANET_AUTO_COMMISSION_REPORT_PAGE_ROWS,
            total_rows,
        );
    }

    fn build_auto_commission_report_rows(&self, report: &AutoCommissionReport) -> Vec<String> {
        let max_fleet_number = self
            .game_data
            .fleets
            .records
            .iter()
            .filter(|fleet| fleet.owner_empire_raw() as usize == self.player.record_index_1_based)
            .map(|fleet| fleet.local_slot_word_raw())
            .max()
            .unwrap_or(1);

        let mut rows = Vec::new();
        for (idx, entry) in report.entries.iter().enumerate() {
            if idx > 0 {
                rows.push(String::new());
            }
            let line = match entry {
                AutoCommissionEntry::Fleet(entry) => {
                    format_auto_commission_fleet_entry(entry, max_fleet_number)
                }
                AutoCommissionEntry::Starbase(entry) => {
                    format_auto_commission_starbase_entry(entry)
                }
            };
            rows.extend(crate::screen::layout::wrap_text(
                &line,
                crate::screen::layout::PLAYFIELD_WIDTH,
                crate::screen::layout::PLAYFIELD_WIDTH,
            ));
        }
        rows
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
        self.reset_planet_build_list_delete_state();
        self.planet.build_list_delete_qty_prompt_active = true;
    }

    pub fn append_delete_build_qty_char(&mut self, ch: char) {
        if !self.planet.build_list_delete_qty_prompt_active || !ch.is_ascii_digit() {
            return;
        }
        let rows = self.planet_build_list_rows();
        let max_digits = rows
            .get(self.planet.build_list_cursor)
            .map(|row| row.queue_qty.to_string().len())
            .unwrap_or(1);
        if self.planet.build_list_delete_qty_input.len() >= max_digits {
            return;
        }
        self.planet.build_list_delete_qty_input.push(ch);
        self.planet.build_list_delete_qty_status = None;
    }

    pub fn backspace_delete_build_qty_input(&mut self) {
        if !self.planet.build_list_delete_qty_prompt_active {
            return;
        }
        self.planet.build_list_delete_qty_input.pop();
        self.planet.build_list_delete_qty_status = None;
    }

    pub fn submit_delete_build_qty(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.planet.build_list_delete_qty_prompt_active {
            return Ok(());
        }
        let rows = self.planet_build_list_rows();
        let Some(row) = rows.get(self.planet.build_list_cursor) else {
            self.reset_planet_build_list_delete_state();
            return Ok(());
        };
        let max_qty = row.queue_qty;
        if max_qty == 0 {
            self.reset_planet_build_list_delete_state();
            return Ok(());
        }
        let input = self.planet.build_list_delete_qty_input.trim();
        let quantity = if input.is_empty() {
            max_qty
        } else if let Ok(value) = input.parse::<u32>() {
            value
        } else {
            self.planet.build_list_delete_qty_status =
                Some(format!("Enter 1-{max_qty}, or press Enter for All."));
            return Ok(());
        };
        if quantity == 0 || quantity > max_qty {
            self.planet.build_list_delete_qty_status =
                Some(format!("Enter 1-{max_qty}, or press Enter for All."));
            return Ok(());
        }
        self.planet.build_list_delete_qty_pending = Some(quantity);
        self.planet.build_list_delete_qty_prompt_active = false;
        self.planet.build_list_delete_qty_status = None;
        self.planet.build_list_confirming = true;
        Ok(())
    }

    pub fn confirm_delete_planet_build_slot(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.planet.build_list_confirming {
            return Ok(());
        }
        let rows = self.planet_build_list_rows();
        let Some(row) = rows.get(self.planet.build_list_cursor) else {
            self.reset_planet_build_list_delete_state();
            return Ok(());
        };
        let planet_record = self.current_build_planet_row()?.planet_record_index_1_based;
        let quantity = self
            .planet
            .build_list_delete_qty_pending
            .unwrap_or(row.queue_qty);
        if let Some(unit) = build_unit_spec_by_kind(row.kind) {
            self.game_data.remove_planet_build_points_by_kind(
                planet_record,
                row.kind,
                quantity.saturating_mul(unit.cost),
            )?;
        } else {
            self.game_data
                .clear_planet_build_orders_by_kind(planet_record, row.kind)?;
        }
        self.save_game_data()?;
        self.reset_planet_build_list_delete_state();
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
        self.reset_planet_build_list_delete_state();
    }

    pub fn append_planet_build_unit_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::PlanetBuildSpecify
            && self.planet.build_unit_input.len() < 2
        {
            self.planet.build_unit_input.push(ch);
            self.planet.build_unit_status = None;
            self.planet.build_unit_notice = None;
        }
    }

    pub fn backspace_planet_build_unit_input(&mut self) {
        if self.current_screen == ScreenId::PlanetBuildSpecify {
            self.planet.build_unit_input.pop();
            self.planet.build_unit_status = None;
            self.planet.build_unit_notice = None;
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
            self.planet.build_unit_notice = None;
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
        self.planet.build_unit_notice = None;
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
            1
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
        self.planet.build_unit_status = None;
        self.planet.build_unit_notice = Some(format!("Queued {} {}.", qty, unit.label));
        self.planet.build_quantity_input.clear();
        self.planet.build_quantity_status = None;
        self.planet.build_selected_kind = None;
        self.current_screen = ScreenId::PlanetBuildSpecify;
        Ok(())
    }

    pub fn abort_current_planet_builds(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.inline_planet_build_abort_active_on_current_screen() {
            return Ok(());
        }
        let row = self.current_build_planet_row()?;
        self.game_data
            .clear_planet_build_queue(row.planet_record_index_1_based)?;
        self.save_game_data()?;
        self.close_planet_build_abort_prompt();
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
                    .map(|record| {
                        (0..STARDOCK_SLOT_COUNT).any(|slot| {
                            let count = record.stardock_count_raw(slot);
                            let kind = ProductionItemKind::from_raw(record.stardock_kind_raw(slot));
                            count > 0 && kind.requires_stardock()
                        })
                    })
                    .unwrap_or(false)
            })
            .collect()
    }

    pub(crate) fn planet_commission_picker_rows(&self) -> Vec<PlanetCommissionPickerRow> {
        self.commission_planet_rows()
            .into_iter()
            .filter_map(|row| {
                let record = self
                    .game_data
                    .planets
                    .records
                    .get(row.planet_record_index_1_based - 1)?;
                let mut destroyers = 0u32;
                let mut cruisers = 0u32;
                let mut battleships = 0u32;
                let mut scouts = 0u32;
                let mut troop_transports = 0u32;
                let mut etacs = 0u32;
                let mut starbases = 0u32;
                for slot in 0..STARDOCK_SLOT_COUNT {
                    let count = u32::from(record.stardock_count_raw(slot));
                    if count == 0 {
                        continue;
                    }
                    match ProductionItemKind::from_raw(record.stardock_kind_raw(slot)) {
                        ProductionItemKind::Destroyer => {
                            destroyers = destroyers.saturating_add(count)
                        }
                        ProductionItemKind::Cruiser => cruisers = cruisers.saturating_add(count),
                        ProductionItemKind::Battleship => {
                            battleships = battleships.saturating_add(count)
                        }
                        ProductionItemKind::Scout => scouts = scouts.saturating_add(count),
                        ProductionItemKind::Transport => {
                            troop_transports = troop_transports.saturating_add(count)
                        }
                        ProductionItemKind::Etac => etacs = etacs.saturating_add(count),
                        ProductionItemKind::Starbase => starbases = starbases.saturating_add(count),
                        ProductionItemKind::Army
                        | ProductionItemKind::GroundBattery
                        | ProductionItemKind::Unknown(_) => {}
                    }
                }
                Some(PlanetCommissionPickerRow {
                    coords: row.coords,
                    planet_name: row.planet_name,
                    destroyers,
                    cruisers,
                    battleships,
                    scouts,
                    troop_transports,
                    etacs,
                    starbases,
                })
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
            rows: self.current_planet_commission_rows_for(row.planet_record_index_1_based),
        })
    }

    fn current_planet_commission_rows(&self) -> Vec<PlanetCommissionRow> {
        let Ok(row) = self.current_commission_planet_row() else {
            return vec![];
        };
        self.current_planet_commission_rows_for(row.planet_record_index_1_based)
    }

    fn current_planet_commission_rows_for(
        &self,
        planet_record_index_1_based: usize,
    ) -> Vec<PlanetCommissionRow> {
        let Some(record) = self
            .game_data
            .planets
            .records
            .get(planet_record_index_1_based - 1)
        else {
            return vec![];
        };
        (0..STARDOCK_SLOT_COUNT)
            .filter_map(|slot| {
                let qty = u32::from(record.stardock_count_raw(slot));
                let kind = ProductionItemKind::from_raw(record.stardock_kind_raw(slot));
                if qty == 0 || !kind.requires_stardock() {
                    return None;
                }
                let unit_label = build_unit_spec_by_kind(kind)
                    .map(|spec| spec.label.to_string())
                    .unwrap_or_else(|| {
                        format!("Unknown (kind {})", record.stardock_kind_raw(slot))
                    });
                Some(PlanetCommissionRow {
                    slot_0_based: slot,
                    kind,
                    unit_label,
                    qty,
                })
            })
            .collect()
    }

    fn clear_planet_commission_draft_state(&mut self) {
        self.planet.commission_draft_slots.clear();
        self.planet.commission_draft_rows.clear();
        self.planet.commission_draft_cursor = 0;
        self.planet.commission_draft_input.clear();
        self.planet.commission_draft_status = None;
        self.planet.commission_draft_notice = None;
    }

    fn load_planet_commission_draft_for_current_planet(&mut self) {
        let rows = self.current_planet_commission_rows();
        let (draft_slots, draft_rows) = build_planet_commission_draft_state(&rows);
        self.planet.commission_draft_slots = draft_slots;
        self.planet.commission_draft_rows = draft_rows;
        self.planet.commission_draft_cursor = 0;
        self.planet.commission_draft_input.clear();
        self.planet.commission_draft_status = None;
        self.planet.commission_draft_notice = None;
        self.planet.commission_status = None;
    }

    fn commit_current_planet_commission_draft_input(
        &mut self,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::PlanetCommissionDraft {
            return Ok(false);
        }
        let Some(row) = self
            .planet
            .commission_draft_rows
            .get_mut(self.planet.commission_draft_cursor)
        else {
            self.planet.commission_draft_status =
                Some("No ships remain in this commission draft.".to_string());
            return Ok(false);
        };
        if !row.accepts_fleet_qty() {
            self.planet.commission_draft_input.clear();
            self.planet.commission_draft_status = None;
            return Ok(true);
        }
        let raw = self.planet.commission_draft_input.trim();
        if raw.is_empty() {
            row.fleet_qty = row.remaining_qty;
            self.planet.commission_draft_input.clear();
            self.planet.commission_draft_status = None;
            return Ok(true);
        }
        let qty = match raw.parse::<u16>() {
            Ok(value) => value,
            Err(_) => {
                self.planet.commission_draft_status = Some("Enter a valid quantity.".to_string());
                return Ok(false);
            }
        };
        if qty > row.remaining_qty {
            self.planet.commission_draft_status =
                Some(format!("Enter a quantity from 0 to {}.", row.remaining_qty));
            return Ok(false);
        }
        row.fleet_qty = qty;
        self.planet.commission_draft_input.clear();
        self.planet.commission_draft_status = None;
        Ok(true)
    }

    fn current_planet_commission_draft(
        &self,
    ) -> Result<CommissionFleetDraft, Box<dyn std::error::Error>> {
        let mut draft = CommissionFleetDraft::default();
        for row in &self.planet.commission_draft_rows {
            if !row.accepts_fleet_qty() {
                continue;
            }
            match row.kind {
                ProductionItemKind::Destroyer => draft.destroyers = row.fleet_qty,
                ProductionItemKind::Cruiser => draft.cruisers = row.fleet_qty,
                ProductionItemKind::Battleship => draft.battleships = row.fleet_qty,
                ProductionItemKind::Scout => draft.scouts = row.fleet_qty,
                ProductionItemKind::Transport => draft.transports = row.fleet_qty,
                ProductionItemKind::Etac => draft.etacs = row.fleet_qty,
                _ => return Err("invalid ship kind in commission draft".into()),
            }
        }
        Ok(draft)
    }

    fn current_planet_commission_title(&self) -> Result<String, Box<dyn std::error::Error>> {
        let row = self.current_commission_planet_row()?;
        Ok(format!(
            "COMMISSION SHIPS: \"{}\" IN SYSTEM {}:",
            row.planet_name,
            format_sector_coords(row.coords)
        ))
    }

    pub(crate) fn current_planet_commission_draft_title(
        &self,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let row = self.current_commission_planet_row()?;
        Ok(format!(
            "DRAFT COMMISSION FLEET: \"{}\" IN SYSTEM {}:",
            row.planet_name,
            format_sector_coords(row.coords)
        ))
    }

    fn commission_fleet_notice(
        &self,
        fleet_record_index_1_based: usize,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let fleet = self
            .game_data
            .fleets
            .records
            .get(fleet_record_index_1_based - 1)
            .ok_or("commissioned fleet record missing")?;
        let max_fleet_number = self
            .game_data
            .fleets
            .records
            .iter()
            .filter(|fleet| fleet.owner_empire_raw() as usize == self.player.record_index_1_based)
            .map(|fleet| fleet.local_slot_word_raw())
            .max()
            .unwrap_or(fleet.local_slot_word_raw());
        Ok(format!(
            "Commissioned selected ships into Fleet {}.",
            format_fleet_number(fleet.local_slot_word_raw(), max_fleet_number)
        ))
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

        let mut ordered_kind_raws = vec![1, 2, 3, 4, 5, 6, 9, 8, 7];
        for kind_raw in queue_qty_by_kind.keys() {
            if !ordered_kind_raws.contains(kind_raw) {
                ordered_kind_raws.push(*kind_raw);
            }
        }

        ordered_kind_raws
            .into_iter()
            .filter_map(|kind_raw| {
                let queue_qty = queue_qty_by_kind.get(&kind_raw).copied().unwrap_or(0);
                if queue_qty == 0 {
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
                })
            })
            .collect()
    }

    fn reset_planet_build_list_delete_state(&mut self) {
        self.planet.build_list_confirming = false;
        self.planet.build_list_delete_qty_prompt_active = false;
        self.planet.build_list_delete_qty_input.clear();
        self.planet.build_list_delete_qty_status = None;
        self.planet.build_list_delete_qty_pending = None;
    }
}

fn is_commission_ship_kind(kind: ProductionItemKind) -> bool {
    matches!(
        kind,
        ProductionItemKind::Destroyer
            | ProductionItemKind::Cruiser
            | ProductionItemKind::Battleship
            | ProductionItemKind::Scout
            | ProductionItemKind::Transport
            | ProductionItemKind::Etac
    )
}

fn build_planet_commission_draft_state(
    rows: &[PlanetCommissionRow],
) -> (Vec<usize>, Vec<PlanetCommissionDraftRow>) {
    let mut totals = BTreeMap::<u8, (ProductionItemKind, String, u16)>::new();
    let mut starbase_rows = Vec::new();
    let mut draft_slots = Vec::new();
    for row in rows {
        if row.kind == ProductionItemKind::Starbase {
            starbase_rows.push(PlanetCommissionDraftRow {
                direct_slot_0_based: Some(row.slot_0_based),
                kind: row.kind,
                unit_label: row.unit_label.clone(),
                remaining_qty: row.qty.min(u32::from(u16::MAX)) as u16,
                fleet_qty: 0,
            });
            continue;
        }
        if !is_commission_ship_kind(row.kind) {
            continue;
        }
        draft_slots.push(row.slot_0_based);
        let kind_raw = production_item_kind_raw(row.kind);
        let entry = totals
            .entry(kind_raw)
            .or_insert_with(|| (row.kind, row.unit_label.clone(), 0));
        entry.2 = entry
            .2
            .saturating_add(row.qty.min(u32::from(u16::MAX)) as u16);
    }

    let mut draft_rows = Vec::new();
    for kind in [
        ProductionItemKind::Destroyer,
        ProductionItemKind::Cruiser,
        ProductionItemKind::Battleship,
        ProductionItemKind::Scout,
        ProductionItemKind::Transport,
        ProductionItemKind::Etac,
    ] {
        let kind_raw = production_item_kind_raw(kind);
        let Some((kind, label, qty)) = totals.remove(&kind_raw) else {
            continue;
        };
        draft_rows.push(PlanetCommissionDraftRow {
            direct_slot_0_based: None,
            kind,
            unit_label: label,
            remaining_qty: qty,
            fleet_qty: 0,
        });
    }
    draft_rows.extend(starbase_rows);

    (draft_slots, draft_rows)
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

fn format_auto_commission_fleet_entry(
    entry: &AutoCommissionFleetEntry,
    max_fleet_number: u16,
) -> String {
    let mut composition = Vec::new();
    push_auto_commission_ship_code(&mut composition, "DD", entry.destroyers);
    push_auto_commission_ship_code(&mut composition, "CA", entry.cruisers);
    push_auto_commission_ship_code(&mut composition, "BB", entry.battleships);
    push_auto_commission_ship_code(&mut composition, "SC", entry.scouts);
    push_auto_commission_ship_code(&mut composition, "TT", entry.transports);
    push_auto_commission_ship_code(&mut composition, "ET", entry.etacs);
    format!(
        "Fleet {} commissioned from \"{}\" in sector {} with {}.",
        format_fleet_number(entry.fleet_number, max_fleet_number),
        entry.planet_name,
        format_sector_coords_table(entry.coords),
        composition.join(", "),
    )
}

fn format_auto_commission_starbase_entry(entry: &AutoCommissionStarbaseEntry) -> String {
    format!(
        "Starbase {:02} commissioned to \"{}\" in sector {}.",
        entry.starbase_number,
        entry.planet_name,
        format_sector_coords_table(entry.coords),
    )
}

fn push_auto_commission_ship_code(parts: &mut Vec<String>, code: &str, qty: u32) {
    if qty == 0 {
        return;
    }
    parts.push(format!("{code} {qty:02}"));
}
