use crate::app::helpers::{center_scroll_to_cursor, sync_scroll_to_cursor};
use crate::app::state::App;
use crate::domains::fleet::FleetAction;
use crate::screen::{
    CommandMenu, FleetDetachMode, FleetMergeMode, FleetRow, FleetTransferMode, ScreenId,
};
use ec_data::{CoreGameData, FleetDetachSelection};
use ec_engine::{FleetEtaEstimate, estimate_fleet_eta, estimate_fleet_eta_to_destination};

impl App {
    pub fn open_fleet_merge(&mut self) {
        let total = self.fleet_rows().len();
        if total < 2 {
            self.show_command_menu_notice(
                CommandMenu::Fleet,
                "You need at least two fleets to merge.",
            );
            return;
        }
        self.clear_command_menu_notice();
        self.fleet.merge_status = None;
        self.fleet.merge_source_input.clear();
        self.fleet.merge_host_input.clear();
        self.fleet.merge_source_record_index_1_based = None;
        self.fleet.merge_mode = FleetMergeMode::SelectingSource;
        self.fleet.merge_cursor = self.fleet.merge_cursor.min(total - 1);
        center_scroll_to_cursor(
            &mut self.fleet.merge_scroll_offset,
            self.fleet.merge_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
            total,
        );
        self.current_screen = ScreenId::FleetMerge;
    }

    pub fn open_fleet_transfer(&mut self) {
        let total = self.fleet_rows().len();
        if total < 2 {
            self.show_command_menu_notice(
                CommandMenu::Fleet,
                "You need at least two fleets to transfer ships.",
            );
            return;
        }
        self.clear_command_menu_notice();
        self.fleet.transfer_status = None;
        self.fleet.transfer_select_input.clear();
        self.fleet.transfer_input.clear();
        self.fleet.transfer_selected_fleets.clear();
        self.fleet.transfer_donor_record_index_1_based = None;
        self.fleet.transfer_host_record_index_1_based = None;
        self.fleet.transfer_selection = FleetDetachSelection::default();
        self.fleet.transfer_mode = FleetTransferMode::SelectingFleets;
        self.fleet.transfer_cursor = self.fleet.transfer_cursor.min(total - 1);
        center_scroll_to_cursor(
            &mut self.fleet.transfer_scroll_offset,
            self.fleet.transfer_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
            total,
        );
        self.current_screen = ScreenId::FleetTransfer;
    }

    pub fn open_fleet_detach(&mut self) {
        if self.current_screen == ScreenId::FleetDetach {
            self.fleet.detach_mode = FleetDetachMode::SelectingFleet;
            self.fleet.detach_input.clear();
            self.fleet.detach_status = None;
            self.fleet.detach_selection = FleetDetachSelection::default();
            self.fleet.detach_donor_speed = None;
            return;
        }
        if self.fleet_rows().is_empty() {
            self.show_command_menu_notice(CommandMenu::Fleet, "You have no active fleets.");
            return;
        }
        self.clear_command_menu_notice();
        self.fleet.detach_status = None;
        self.fleet.detach_select_input.clear();
        self.fleet.detach_input.clear();
        let total = self.fleet_rows().len();
        self.fleet.detach_cursor = self.fleet.detach_cursor.min(total - 1);
        center_scroll_to_cursor(
            &mut self.fleet.detach_scroll_offset,
            self.fleet.detach_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
            total,
        );
        self.fleet.detach_mode = FleetDetachMode::SelectingFleet;
        self.fleet.detach_selection = FleetDetachSelection::default();
        self.fleet.detach_donor_speed = None;
        self.current_screen = ScreenId::FleetDetach;
    }

    pub fn move_fleet_merge_select(&mut self, delta: i8) {
        if self.current_screen != ScreenId::FleetMerge {
            return;
        }
        let total = self.current_fleet_merge_rows().len();
        if total == 0 {
            self.fleet.merge_cursor = 0;
            return;
        }
        let next = self.fleet.merge_cursor as isize + delta as isize;
        self.fleet.merge_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.fleet.merge_scroll_offset,
            self.fleet.merge_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
        match self.fleet.merge_mode {
            FleetMergeMode::SelectingSource => self.fleet.merge_source_input.clear(),
            FleetMergeMode::SelectingHost => self.fleet.merge_host_input.clear(),
        }
        self.fleet.merge_status = None;
    }

    pub fn move_fleet_transfer_select(&mut self, delta: i8) {
        if self.current_screen != ScreenId::FleetTransfer {
            return;
        }
        if self.fleet.transfer_mode != FleetTransferMode::SelectingFleets {
            return;
        }
        let total = self.current_fleet_transfer_rows().len();
        if total == 0 {
            self.fleet.transfer_cursor = 0;
            return;
        }
        let next = self.fleet.transfer_cursor as isize + delta as isize;
        self.fleet.transfer_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.fleet.transfer_scroll_offset,
            self.fleet.transfer_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
        self.fleet.transfer_select_input.clear();
        self.fleet.transfer_status = None;
    }

    pub fn toggle_fleet_transfer_selection(&mut self) {
        if self.current_screen != ScreenId::FleetTransfer
            || self.fleet.transfer_mode != FleetTransferMode::SelectingFleets
        {
            return;
        }
        let rows = self.current_fleet_transfer_rows();
        let Some(row) = rows.get(self.fleet.transfer_cursor) else {
            return;
        };
        if !self
            .fleet
            .transfer_selected_fleets
            .insert(row.fleet_record_index_1_based)
        {
            self.fleet
                .transfer_selected_fleets
                .remove(&row.fleet_record_index_1_based);
        }
        self.fleet.transfer_status = None;
    }

    pub fn move_fleet_detach_select(&mut self, delta: i8) {
        if self.current_screen != ScreenId::FleetDetach
            || self.fleet.detach_mode != FleetDetachMode::SelectingFleet
        {
            return;
        }
        let total = self.fleet_rows().len();
        if total == 0 {
            self.fleet.detach_cursor = 0;
            return;
        }
        let next = self.fleet.detach_cursor as isize + delta as isize;
        self.fleet.detach_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.fleet.detach_scroll_offset,
            self.fleet.detach_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
        self.fleet.detach_select_input.clear();
        self.fleet.detach_status = None;
    }

    pub fn append_fleet_merge_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::FleetMerge || !ch.is_ascii_digit() {
            return;
        }
        let input = match self.fleet.merge_mode {
            FleetMergeMode::SelectingSource => &mut self.fleet.merge_source_input,
            FleetMergeMode::SelectingHost => &mut self.fleet.merge_host_input,
        };
        if input.len() >= 4 {
            return;
        }
        input.push(ch);
        self.sync_fleet_merge_cursor_to_input();
        self.fleet.merge_status = None;
    }

    pub fn append_fleet_transfer_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::FleetTransfer || !ch.is_ascii_digit() {
            return;
        }
        let input = if self.fleet.transfer_mode == FleetTransferMode::SelectingFleets {
            &mut self.fleet.transfer_select_input
        } else {
            &mut self.fleet.transfer_input
        };
        let limit = if self.fleet.transfer_mode == FleetTransferMode::SelectingFleets {
            4
        } else {
            3
        };
        if input.len() >= limit {
            return;
        }
        input.push(ch);
        if self.fleet.transfer_mode == FleetTransferMode::SelectingFleets {
            self.sync_fleet_transfer_cursor_to_input();
        }
        self.fleet.transfer_status = None;
    }

    pub fn append_fleet_detach_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::FleetDetach || !ch.is_ascii_digit() {
            return;
        }
        let limit = if self.fleet.detach_mode == FleetDetachMode::SelectingFleet {
            4
        } else {
            3
        };
        let input = if self.fleet.detach_mode == FleetDetachMode::SelectingFleet {
            &mut self.fleet.detach_select_input
        } else {
            &mut self.fleet.detach_input
        };
        if input.len() >= limit {
            return;
        }
        input.push(ch);
        if self.fleet.detach_mode == FleetDetachMode::SelectingFleet {
            self.sync_fleet_detach_cursor_to_input();
        }
        self.fleet.detach_status = None;
    }

    pub fn backspace_fleet_merge_input(&mut self) {
        if self.current_screen != ScreenId::FleetMerge {
            return;
        }
        match self.fleet.merge_mode {
            FleetMergeMode::SelectingSource => self.fleet.merge_source_input.pop(),
            FleetMergeMode::SelectingHost => self.fleet.merge_host_input.pop(),
        };
        self.sync_fleet_merge_cursor_to_input();
        self.fleet.merge_status = None;
    }

    pub fn backspace_fleet_transfer_input(&mut self) {
        if self.current_screen != ScreenId::FleetTransfer {
            return;
        }
        if self.fleet.transfer_mode == FleetTransferMode::SelectingFleets {
            self.fleet.transfer_select_input.pop();
            self.sync_fleet_transfer_cursor_to_input();
        } else {
            self.fleet.transfer_input.pop();
        }
        self.fleet.transfer_status = None;
    }

    pub fn backspace_fleet_detach_input(&mut self) {
        if self.current_screen != ScreenId::FleetDetach {
            return;
        }
        if self.fleet.detach_mode == FleetDetachMode::SelectingFleet {
            self.fleet.detach_select_input.pop();
            self.sync_fleet_detach_cursor_to_input();
        } else {
            self.fleet.detach_input.pop();
        }
        self.fleet.detach_status = None;
    }

    pub fn submit_fleet_merge(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::FleetMerge {
            return Ok(());
        }
        let rows = self.current_fleet_merge_rows();
        let Some(_) = rows.get(self.fleet.merge_cursor) else {
            self.current_screen = ScreenId::FleetMenu;
            return Ok(());
        };

        let target_fleet_id = match self.current_fleet_merge_input().trim() {
            "" => rows[self.fleet.merge_cursor].fleet_number,
            raw => match raw.parse::<u16>() {
                Ok(value) => value,
                Err(_) => {
                    self.fleet.merge_status =
                        Some("Enter a fleet number from the table.".to_string());
                    return Ok(());
                }
            },
        };
        let Some(index) = rows
            .iter()
            .position(|row| row.fleet_number == target_fleet_id)
        else {
            self.fleet.merge_status = Some(format!(
                "Fleet #{target_fleet_id} is not in your fleet list."
            ));
            return Ok(());
        };
        self.fleet.merge_cursor = index;
        sync_scroll_to_cursor(
            &mut self.fleet.merge_scroll_offset,
            self.fleet.merge_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );

        match self.fleet.merge_mode {
            FleetMergeMode::SelectingSource => {
                let source = &rows[self.fleet.merge_cursor];
                self.fleet.merge_source_record_index_1_based =
                    Some(source.fleet_record_index_1_based);
                self.fleet.merge_source_input.clear();
                self.fleet.merge_host_input.clear();
                self.fleet.merge_status = None;
                self.fleet.merge_mode = FleetMergeMode::SelectingHost;
                self.fleet.merge_cursor = 0;
                self.fleet.merge_scroll_offset = 0;
                let host_total = self.current_fleet_merge_rows().len();
                if host_total == 0 {
                    self.show_command_menu_notice(
                        CommandMenu::Fleet,
                        "You need at least two fleets to merge.",
                    );
                    return Ok(());
                }
                center_scroll_to_cursor(
                    &mut self.fleet.merge_scroll_offset,
                    self.fleet.merge_cursor,
                    crate::screen::FLEET_VISIBLE_ROWS,
                    host_total,
                );
            }
            FleetMergeMode::SelectingHost => {
                let host = &rows[self.fleet.merge_cursor];
                let source_record_index_1_based = self
                    .fleet
                    .merge_source_record_index_1_based
                    .ok_or("fleet merge source missing")?;
                let source_fleet_number = self
                    .fleet_rows()
                    .into_iter()
                    .find(|row| row.fleet_record_index_1_based == source_record_index_1_based)
                    .map(|row| row.fleet_number)
                    .ok_or("fleet merge source row missing")?;
                self.game_data.set_join_fleet_order(
                    self.player.record_index_1_based,
                    source_record_index_1_based,
                    host.fleet_record_index_1_based,
                )?;
                self.save_game_data()?;
                self.fleet.merge_source_record_index_1_based = None;
                self.fleet.merge_source_input.clear();
                self.fleet.merge_host_input.clear();
                self.fleet.merge_status = None;
                self.fleet.merge_mode = FleetMergeMode::SelectingSource;
                self.show_command_menu_notice(
                    CommandMenu::Fleet,
                    format!(
                        "Fleet #{} ordered to join Fleet #{}.",
                        source_fleet_number, host.fleet_number
                    ),
                );
            }
        }
        Ok(())
    }

    pub fn submit_fleet_transfer(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::FleetTransfer {
            return Ok(());
        }
        let rows = self.current_fleet_transfer_rows();
        match self.fleet.transfer_mode {
            FleetTransferMode::SelectingFleets => {
                if self.fleet.transfer_selected_fleets.len() != 2 {
                    self.fleet.transfer_status =
                        Some("Select exactly two fleets for transfer.".to_string());
                    return Ok(());
                }
                let Some(host_row) = rows.get(self.fleet.transfer_cursor) else {
                    return Ok(());
                };
                if !self
                    .fleet
                    .transfer_selected_fleets
                    .contains(&host_row.fleet_record_index_1_based)
                {
                    self.fleet.transfer_status =
                        Some("Highlight one selected fleet as the host.".to_string());
                    return Ok(());
                }
                let selected_rows = rows
                    .iter()
                    .filter(|row| {
                        self.fleet
                            .transfer_selected_fleets
                            .contains(&row.fleet_record_index_1_based)
                    })
                    .collect::<Vec<_>>();
                if selected_rows.len() != 2 {
                    self.fleet.transfer_status =
                        Some("Select exactly two fleets for transfer.".to_string());
                    return Ok(());
                }
                if selected_rows[0].coords != selected_rows[1].coords {
                    self.fleet.transfer_status =
                        Some("Transfer fleets must share one sector.".to_string());
                    return Ok(());
                }
                self.fleet.transfer_host_record_index_1_based =
                    Some(host_row.fleet_record_index_1_based);
                self.fleet.transfer_donor_record_index_1_based = selected_rows
                    .iter()
                    .find(|row| {
                        row.fleet_record_index_1_based != host_row.fleet_record_index_1_based
                    })
                    .map(|row| row.fleet_record_index_1_based);
                self.fleet.transfer_mode = FleetTransferMode::EnteringBattleships;
                self.fleet.transfer_input.clear();
                self.fleet.transfer_status = None;
                self.fleet.transfer_selection = FleetDetachSelection::default();
            }
            _ => {
                let value = if self.fleet.transfer_input.trim().is_empty() {
                    0
                } else {
                    match self.fleet.transfer_input.trim().parse::<u16>() {
                        Ok(value) => value,
                        Err(_) => {
                            self.fleet.transfer_status =
                                Some("Enter a number from 0 up.".to_string());
                            return Ok(());
                        }
                    }
                };
                match self.fleet.transfer_mode {
                    FleetTransferMode::EnteringBattleships => {
                        self.fleet.transfer_selection.battleships = value;
                        self.fleet.transfer_mode = FleetTransferMode::EnteringCruisers;
                    }
                    FleetTransferMode::EnteringCruisers => {
                        self.fleet.transfer_selection.cruisers = value;
                        self.fleet.transfer_mode = FleetTransferMode::EnteringDestroyers;
                    }
                    FleetTransferMode::EnteringDestroyers => {
                        self.fleet.transfer_selection.destroyers = value;
                        self.fleet.transfer_mode = FleetTransferMode::EnteringFullTransports;
                    }
                    FleetTransferMode::EnteringFullTransports => {
                        self.fleet.transfer_selection.full_transports = value;
                        self.fleet.transfer_mode = FleetTransferMode::EnteringEmptyTransports;
                    }
                    FleetTransferMode::EnteringEmptyTransports => {
                        self.fleet.transfer_selection.empty_transports = value;
                        self.fleet.transfer_mode = FleetTransferMode::EnteringScouts;
                    }
                    FleetTransferMode::EnteringScouts => {
                        self.fleet.transfer_selection.scouts = value.min(u16::from(u8::MAX)) as u8;
                        self.fleet.transfer_mode = FleetTransferMode::EnteringEtacs;
                    }
                    FleetTransferMode::EnteringEtacs => {
                        self.fleet.transfer_selection.etacs = value;
                        self.finish_fleet_transfer()?;
                    }
                    FleetTransferMode::SelectingFleets => {}
                }
                self.fleet.transfer_input.clear();
            }
        }
        Ok(())
    }

    pub fn submit_fleet_detach(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::FleetDetach {
            return Ok(());
        }
        let rows = self.fleet_rows();
        let Some(selected_row) = rows.get(self.fleet.detach_cursor) else {
            self.current_screen = ScreenId::FleetMenu;
            return Ok(());
        };

        if self.fleet.detach_mode == FleetDetachMode::SelectingFleet {
            if !self.fleet.detach_select_input.trim().is_empty() {
                let target_fleet_id = match self.fleet.detach_select_input.trim().parse::<u16>() {
                    Ok(value) => value,
                    Err(_) => {
                        self.fleet.detach_status =
                            Some("Enter a fleet number from the table.".to_string());
                        return Ok(());
                    }
                };
                let Some(index) = rows
                    .iter()
                    .position(|row| row.fleet_number == target_fleet_id)
                else {
                    self.fleet.detach_status = Some(format!(
                        "Fleet #{target_fleet_id} is not in your fleet list."
                    ));
                    return Ok(());
                };
                self.fleet.detach_cursor = index;
                sync_scroll_to_cursor(
                    &mut self.fleet.detach_scroll_offset,
                    self.fleet.detach_cursor,
                    crate::screen::FLEET_VISIBLE_ROWS,
                );
            }
            if self.current_fleet_detach_ship_total() <= 1 {
                self.fleet.detach_status =
                    Some("A fleet must contain at least two ships to detach.".to_string());
                return Ok(());
            }
            self.fleet.detach_select_input.clear();
            self.fleet.detach_input.clear();
            self.fleet.detach_status = None;
            self.fleet.detach_selection = FleetDetachSelection::default();
            self.fleet.detach_donor_speed = None;
            self.fleet.detach_mode = self
                .next_fleet_detach_prompt_mode(FleetDetachMode::SelectingFleet)
                .unwrap_or(FleetDetachMode::SettingNewFleetRoe);
            return Ok(());
        }

        let Some(record) = self
            .game_data
            .fleets
            .records
            .get(selected_row.fleet_record_index_1_based - 1)
            .cloned()
        else {
            self.current_screen = ScreenId::FleetMenu;
            return Ok(());
        };

        match self.fleet.detach_mode {
            FleetDetachMode::EnteringBattleships
            | FleetDetachMode::EnteringCruisers
            | FleetDetachMode::EnteringDestroyers
            | FleetDetachMode::EnteringFullTransports
            | FleetDetachMode::EnteringEmptyTransports
            | FleetDetachMode::EnteringScouts
            | FleetDetachMode::EnteringEtacs => {
                let value = self.resolve_fleet_detach_numeric_input(0)?;
                match self.fleet.detach_mode {
                    FleetDetachMode::EnteringBattleships => {
                        if value > record.battleship_count() {
                            self.fleet.detach_status =
                                Some("Enter a value from 0 to the table limit.".to_string());
                            return Ok(());
                        }
                        self.fleet.detach_selection.battleships = value;
                    }
                    FleetDetachMode::EnteringCruisers => {
                        if value > record.cruiser_count() {
                            self.fleet.detach_status =
                                Some("Enter a value from 0 to the table limit.".to_string());
                            return Ok(());
                        }
                        self.fleet.detach_selection.cruisers = value;
                    }
                    FleetDetachMode::EnteringDestroyers => {
                        if value > record.destroyer_count() {
                            self.fleet.detach_status =
                                Some("Enter a value from 0 to the table limit.".to_string());
                            return Ok(());
                        }
                        self.fleet.detach_selection.destroyers = value;
                    }
                    FleetDetachMode::EnteringFullTransports => {
                        if value > record.army_count() {
                            self.fleet.detach_status =
                                Some("Enter a value from 0 to the table limit.".to_string());
                            return Ok(());
                        }
                        self.fleet.detach_selection.full_transports = value;
                    }
                    FleetDetachMode::EnteringEmptyTransports => {
                        let available = record
                            .troop_transport_count()
                            .saturating_sub(record.army_count());
                        if value > available {
                            self.fleet.detach_status =
                                Some("Enter a value from 0 to the table limit.".to_string());
                            return Ok(());
                        }
                        self.fleet.detach_selection.empty_transports = value;
                    }
                    FleetDetachMode::EnteringScouts => {
                        if value > u16::from(record.scout_count()) {
                            self.fleet.detach_status =
                                Some("Enter a value from 0 to the table limit.".to_string());
                            return Ok(());
                        }
                        self.fleet.detach_selection.scouts = value as u8;
                    }
                    FleetDetachMode::EnteringEtacs => {
                        if value > record.etac_count() {
                            self.fleet.detach_status =
                                Some("Enter a value from 0 to the table limit.".to_string());
                            return Ok(());
                        }
                        self.fleet.detach_selection.etacs = value;
                    }
                    _ => {}
                }
                self.fleet.detach_input.clear();
                self.fleet.detach_status = None;
                if let Some(next_mode) = self.next_fleet_detach_prompt_mode(self.fleet.detach_mode)
                {
                    self.fleet.detach_mode = next_mode;
                } else if self.fleet.detach_selection.total_ships() == 0 {
                    self.open_fleet_detach();
                } else if self.fleet_detach_requires_speed_prompt() {
                    self.fleet.detach_donor_speed = None;
                    self.fleet.detach_mode = FleetDetachMode::AdjustingDonorSpeed;
                } else {
                    self.fleet.detach_donor_speed = None;
                    self.fleet.detach_mode = FleetDetachMode::SettingNewFleetRoe;
                }
            }
            FleetDetachMode::AdjustingDonorSpeed => {
                let default_speed = self.fleet_detach_donor_default_speed().max(1);
                let speed = self.resolve_fleet_detach_numeric_input(default_speed as u16)? as u8;
                let max_speed = self.fleet_detach_donor_default_speed();
                if speed == 0 || speed > max_speed {
                    self.fleet.detach_status =
                        Some(format!("Enter a speed from 1 to {max_speed}."));
                    return Ok(());
                }
                self.fleet.detach_donor_speed = Some(speed);
                self.fleet.detach_input.clear();
                self.fleet.detach_status = None;
                self.fleet.detach_mode = FleetDetachMode::SettingNewFleetRoe;
            }
            FleetDetachMode::SettingNewFleetRoe => {
                let new_roe = self.resolve_fleet_detach_numeric_input(6)? as u8;
                if new_roe > 10 {
                    self.fleet.detach_status = Some("Enter an ROE from 0 to 10.".to_string());
                    return Ok(());
                }
                let detached_has_combat_ships = self.fleet.detach_selection.battleships > 0
                    || self.fleet.detach_selection.cruisers > 0
                    || self.fleet.detach_selection.destroyers > 0;
                if !detached_has_combat_ships && new_roe != 0 {
                    self.fleet.detach_status =
                        Some("Non-combat fleets must use ROE 0.".to_string());
                    return Ok(());
                }
                let donor_speed = if self.fleet_detach_requires_speed_prompt() {
                    Some(
                        self.fleet
                            .detach_donor_speed
                            .unwrap_or(self.fleet_detach_donor_default_speed()),
                    )
                } else {
                    None
                };
                self.game_data.detach_ships_to_new_fleet(
                    self.player.record_index_1_based,
                    selected_row.fleet_record_index_1_based,
                    self.fleet.detach_selection,
                    donor_speed,
                    new_roe,
                )?;
                self.save_game_data()?;
                self.fleet.detach_mode = FleetDetachMode::SelectingFleet;
                self.fleet.detach_input.clear();
                self.fleet.detach_select_input.clear();
                self.fleet.detach_status = None;
                self.fleet.detach_selection = FleetDetachSelection::default();
                self.fleet.detach_donor_speed = None;
            }
            FleetDetachMode::SelectingFleet => {}
        }
        Ok(())
    }

    pub(crate) fn handle_fleet_detach_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match self.fleet.detach_mode {
            FleetDetachMode::SelectingFleet => match key.code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    crate::app::Action::Fleet(FleetAction::MoveDetachSelect(-1))
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    crate::app::Action::Fleet(FleetAction::MoveDetachSelect(1))
                }
                KeyCode::PageUp => crate::app::Action::Fleet(FleetAction::MoveDetachSelect(-8)),
                KeyCode::PageDown => crate::app::Action::Fleet(FleetAction::MoveDetachSelect(8)),
                KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitDetach),
                KeyCode::Backspace => crate::app::Action::Fleet(FleetAction::BackspaceDetachInput),
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    crate::app::Action::Fleet(FleetAction::AppendDetachChar(ch))
                }
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::Fleet(FleetAction::OpenMenu)
                }
                _ => crate::app::Action::Noop,
            },
            _ => match key.code {
                KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitDetach),
                KeyCode::Backspace => crate::app::Action::Fleet(FleetAction::BackspaceDetachInput),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::Fleet(FleetAction::OpenDetach)
                }
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    crate::app::Action::Fleet(FleetAction::AppendDetachChar(ch))
                }
                _ => crate::app::Action::Noop,
            },
        }
    }

    pub(crate) fn handle_fleet_merge_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                crate::app::Action::Fleet(FleetAction::MoveMergeSelect(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                crate::app::Action::Fleet(FleetAction::MoveMergeSelect(1))
            }
            KeyCode::PageUp => crate::app::Action::Fleet(FleetAction::MoveMergeSelect(-8)),
            KeyCode::PageDown => crate::app::Action::Fleet(FleetAction::MoveMergeSelect(8)),
            KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitMerge),
            KeyCode::Backspace => crate::app::Action::Fleet(FleetAction::BackspaceMergeInput),
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                crate::app::Action::Fleet(FleetAction::AppendMergeChar(ch))
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                crate::app::Action::Fleet(FleetAction::OpenMenu)
            }
            _ => crate::app::Action::Noop,
        }
    }

    pub(crate) fn handle_fleet_transfer_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match self.fleet.transfer_mode {
            FleetTransferMode::SelectingFleets => match key.code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    crate::app::Action::Fleet(FleetAction::MoveTransferSelect(-1))
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    crate::app::Action::Fleet(FleetAction::MoveTransferSelect(1))
                }
                KeyCode::PageUp => crate::app::Action::Fleet(FleetAction::MoveTransferSelect(-8)),
                KeyCode::PageDown => crate::app::Action::Fleet(FleetAction::MoveTransferSelect(8)),
                KeyCode::Char(' ') => {
                    crate::app::Action::Fleet(FleetAction::ToggleTransferSelection)
                }
                KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitTransfer),
                KeyCode::Backspace => {
                    crate::app::Action::Fleet(FleetAction::BackspaceTransferInput)
                }
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    crate::app::Action::Fleet(FleetAction::AppendTransferChar(ch))
                }
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::Fleet(FleetAction::OpenMenu)
                }
                _ => crate::app::Action::Noop,
            },
            _ => match key.code {
                KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitTransfer),
                KeyCode::Backspace => {
                    crate::app::Action::Fleet(FleetAction::BackspaceTransferInput)
                }
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::Fleet(FleetAction::OpenTransfer)
                }
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    crate::app::Action::Fleet(FleetAction::AppendTransferChar(ch))
                }
                _ => crate::app::Action::Noop,
            },
        }
    }

    fn sync_fleet_merge_cursor_to_input(&mut self) {
        if self.current_screen != ScreenId::FleetMerge {
            return;
        }
        let Ok(target_fleet_id) = self.current_fleet_merge_input().trim().parse::<u16>() else {
            return;
        };
        let rows = self.current_fleet_merge_rows();
        let Some(index) = rows
            .iter()
            .position(|row| row.fleet_number == target_fleet_id)
        else {
            return;
        };
        self.fleet.merge_cursor = index;
        sync_scroll_to_cursor(
            &mut self.fleet.merge_scroll_offset,
            self.fleet.merge_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
    }

    fn sync_fleet_transfer_cursor_to_input(&mut self) {
        if self.current_screen != ScreenId::FleetTransfer
            || self.fleet.transfer_mode != FleetTransferMode::SelectingFleets
        {
            return;
        }
        let Ok(target_fleet_id) = self.fleet.transfer_select_input.trim().parse::<u16>() else {
            return;
        };
        let rows = self.current_fleet_transfer_rows();
        let Some(index) = rows
            .iter()
            .position(|row| row.fleet_number == target_fleet_id)
        else {
            return;
        };
        self.fleet.transfer_cursor = index;
        sync_scroll_to_cursor(
            &mut self.fleet.transfer_scroll_offset,
            self.fleet.transfer_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
    }

    fn sync_fleet_detach_cursor_to_input(&mut self) {
        if self.current_screen != ScreenId::FleetDetach
            || self.fleet.detach_mode != FleetDetachMode::SelectingFleet
        {
            return;
        }
        let Ok(target_fleet_id) = self.fleet.detach_select_input.trim().parse::<u16>() else {
            return;
        };
        let rows = self.fleet_rows();
        let Some(index) = rows
            .iter()
            .position(|row| row.fleet_number == target_fleet_id)
        else {
            return;
        };
        self.fleet.detach_cursor = index;
        sync_scroll_to_cursor(
            &mut self.fleet.detach_scroll_offset,
            self.fleet.detach_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
    }

    pub(crate) fn fleet_detach_prompt_and_default(&self, rows: &[FleetRow]) -> (String, String) {
        let fleet_number = rows
            .get(self.fleet.detach_cursor)
            .map(|row| row.fleet_number)
            .unwrap_or(1);
        match self.fleet.detach_mode {
            FleetDetachMode::SelectingFleet => (
                "Detach ships from fleet # ".to_string(),
                format_fleet_number_for_rows(fleet_number, rows),
            ),
            FleetDetachMode::EnteringBattleships => {
                ("Battleships to detach ".to_string(), "0".to_string())
            }
            FleetDetachMode::EnteringCruisers => {
                ("Cruisers to detach ".to_string(), "0".to_string())
            }
            FleetDetachMode::EnteringDestroyers => {
                ("Destroyers to detach ".to_string(), "0".to_string())
            }
            FleetDetachMode::EnteringFullTransports => {
                ("FULL transports to detach ".to_string(), "0".to_string())
            }
            FleetDetachMode::EnteringEmptyTransports => {
                ("EMPTY transports to detach ".to_string(), "0".to_string())
            }
            FleetDetachMode::EnteringScouts => {
                ("Scout ships to detach ".to_string(), "0".to_string())
            }
            FleetDetachMode::EnteringEtacs => ("ET ships to detach ".to_string(), "0".to_string()),
            FleetDetachMode::AdjustingDonorSpeed => (
                format!(
                    "Fleet #{} new speed ",
                    format_fleet_number_for_rows(fleet_number, rows)
                ),
                self.fleet_detach_donor_default_speed().to_string(),
            ),
            FleetDetachMode::SettingNewFleetRoe => ("New fleet ROE ".to_string(), "6".to_string()),
        }
    }

    pub(crate) fn fleet_detach_current_input(&self) -> &str {
        if self.fleet.detach_mode == FleetDetachMode::SelectingFleet {
            &self.fleet.detach_select_input
        } else {
            &self.fleet.detach_input
        }
    }

    fn current_fleet_detach_ship_total(&self) -> u32 {
        let rows = self.fleet_rows();
        let Some(selected_row) = rows.get(self.fleet.detach_cursor) else {
            return 0;
        };
        self.game_data
            .fleets
            .records
            .get(selected_row.fleet_record_index_1_based - 1)
            .map(|fleet| {
                u32::from(fleet.battleship_count())
                    + u32::from(fleet.cruiser_count())
                    + u32::from(fleet.destroyer_count())
                    + u32::from(fleet.troop_transport_count())
                    + u32::from(fleet.scout_count())
                    + u32::from(fleet.etac_count())
            })
            .unwrap_or(0)
    }

    fn next_fleet_detach_prompt_mode(&self, current: FleetDetachMode) -> Option<FleetDetachMode> {
        let rows = self.fleet_rows();
        let selected_row = rows.get(self.fleet.detach_cursor)?;
        let fleet = self
            .game_data
            .fleets
            .records
            .get(selected_row.fleet_record_index_1_based - 1)?;
        let modes = [
            (
                FleetDetachMode::EnteringBattleships,
                fleet.battleship_count() > 0,
            ),
            (FleetDetachMode::EnteringCruisers, fleet.cruiser_count() > 0),
            (
                FleetDetachMode::EnteringDestroyers,
                fleet.destroyer_count() > 0,
            ),
            (
                FleetDetachMode::EnteringFullTransports,
                fleet.army_count() > 0,
            ),
            (
                FleetDetachMode::EnteringEmptyTransports,
                fleet.troop_transport_count() > fleet.army_count(),
            ),
            (FleetDetachMode::EnteringScouts, fleet.scout_count() > 0),
            (FleetDetachMode::EnteringEtacs, fleet.etac_count() > 0),
        ];
        let start_idx = match current {
            FleetDetachMode::SelectingFleet => 0,
            FleetDetachMode::EnteringBattleships => 1,
            FleetDetachMode::EnteringCruisers => 2,
            FleetDetachMode::EnteringDestroyers => 3,
            FleetDetachMode::EnteringFullTransports => 4,
            FleetDetachMode::EnteringEmptyTransports => 5,
            FleetDetachMode::EnteringScouts => 6,
            FleetDetachMode::EnteringEtacs
            | FleetDetachMode::AdjustingDonorSpeed
            | FleetDetachMode::SettingNewFleetRoe => modes.len(),
        };
        modes
            .iter()
            .skip(start_idx)
            .find_map(|(mode, include)| (*include).then_some(*mode))
    }

    fn fleet_detach_requires_speed_prompt(&self) -> bool {
        let rows = self.fleet_rows();
        let Some(selected_row) = rows.get(self.fleet.detach_cursor) else {
            return false;
        };
        let Some(fleet) = self
            .game_data
            .fleets
            .records
            .get(selected_row.fleet_record_index_1_based - 1)
        else {
            return false;
        };
        let mut donor_after = fleet.clone();
        donor_after.set_battleship_count(
            donor_after
                .battleship_count()
                .saturating_sub(self.fleet.detach_selection.battleships),
        );
        donor_after.set_cruiser_count(
            donor_after
                .cruiser_count()
                .saturating_sub(self.fleet.detach_selection.cruisers),
        );
        donor_after.set_destroyer_count(
            donor_after
                .destroyer_count()
                .saturating_sub(self.fleet.detach_selection.destroyers),
        );
        donor_after.set_troop_transport_count(donor_after.troop_transport_count().saturating_sub(
            self.fleet.detach_selection.full_transports
                + self.fleet.detach_selection.empty_transports,
        ));
        donor_after.set_army_count(
            donor_after
                .army_count()
                .saturating_sub(self.fleet.detach_selection.full_transports),
        );
        donor_after.set_scout_count(
            donor_after
                .scout_count()
                .saturating_sub(self.fleet.detach_selection.scouts),
        );
        donor_after.set_etac_count(
            donor_after
                .etac_count()
                .saturating_sub(self.fleet.detach_selection.etacs),
        );
        donor_after.recompute_max_speed_from_composition();
        donor_after.max_speed() > 0 && fleet.current_speed() > donor_after.max_speed()
    }

    fn fleet_detach_donor_default_speed(&self) -> u8 {
        let rows = self.fleet_rows();
        let Some(selected_row) = rows.get(self.fleet.detach_cursor) else {
            return 1;
        };
        let Some(fleet) = self
            .game_data
            .fleets
            .records
            .get(selected_row.fleet_record_index_1_based - 1)
        else {
            return 1;
        };
        let mut donor_after = fleet.clone();
        donor_after.set_battleship_count(
            donor_after
                .battleship_count()
                .saturating_sub(self.fleet.detach_selection.battleships),
        );
        donor_after.set_cruiser_count(
            donor_after
                .cruiser_count()
                .saturating_sub(self.fleet.detach_selection.cruisers),
        );
        donor_after.set_destroyer_count(
            donor_after
                .destroyer_count()
                .saturating_sub(self.fleet.detach_selection.destroyers),
        );
        donor_after.set_troop_transport_count(donor_after.troop_transport_count().saturating_sub(
            self.fleet.detach_selection.full_transports
                + self.fleet.detach_selection.empty_transports,
        ));
        donor_after.set_army_count(
            donor_after
                .army_count()
                .saturating_sub(self.fleet.detach_selection.full_transports),
        );
        donor_after.set_scout_count(
            donor_after
                .scout_count()
                .saturating_sub(self.fleet.detach_selection.scouts),
        );
        donor_after.set_etac_count(
            donor_after
                .etac_count()
                .saturating_sub(self.fleet.detach_selection.etacs),
        );
        donor_after.recompute_max_speed_from_composition();
        donor_after.max_speed().max(1)
    }

    pub(crate) fn current_fleet_merge_rows(&self) -> Vec<FleetRow> {
        let rows = self.fleet_rows();
        match self.fleet.merge_mode {
            FleetMergeMode::SelectingSource => rows,
            FleetMergeMode::SelectingHost => {
                let Some(source_record_index_1_based) =
                    self.fleet.merge_source_record_index_1_based
                else {
                    return rows;
                };
                rows.into_iter()
                    .filter(|row| row.fleet_record_index_1_based != source_record_index_1_based)
                    .collect()
            }
        }
    }

    pub(crate) fn current_fleet_merge_input(&self) -> &str {
        match self.fleet.merge_mode {
            FleetMergeMode::SelectingSource => &self.fleet.merge_source_input,
            FleetMergeMode::SelectingHost => &self.fleet.merge_host_input,
        }
    }

    pub(crate) fn current_fleet_transfer_rows(&self) -> Vec<FleetRow> {
        let mut rows = match self.fleet.transfer_mode {
            FleetTransferMode::SelectingFleets => self.fleet_rows(),
            _ => self
                .fleet
                .transfer_donor_record_index_1_based
                .and_then(|idx| {
                    self.fleet_rows()
                        .into_iter()
                        .find(|row| row.fleet_record_index_1_based == idx)
                })
                .into_iter()
                .collect(),
        };
        rows.sort_by(|left, right| {
            left.coords
                .cmp(&right.coords)
                .then_with(|| left.order_code.cmp(&right.order_code))
                .then_with(|| right.fleet_number.cmp(&left.fleet_number))
        });
        rows
    }

    pub(crate) fn current_fleet_transfer_input(&self) -> &str {
        match self.fleet.transfer_mode {
            FleetTransferMode::SelectingFleets => &self.fleet.transfer_select_input,
            _ => &self.fleet.transfer_input,
        }
    }

    pub(crate) fn fleet_number_for_record_index(&self, record_index_1_based: usize) -> Option<u16> {
        self.game_data
            .fleets
            .records
            .get(record_index_1_based - 1)
            .map(|fleet| fleet.local_slot_word_raw())
    }

    pub(crate) fn fleet_transfer_prompt_and_default(&self, rows: &[FleetRow]) -> (String, String) {
        match self.fleet.transfer_mode {
            FleetTransferMode::SelectingFleets => (
                "Fleet # ".to_string(),
                rows.get(self.fleet.transfer_cursor)
                    .map(|row| row.fleet_number.to_string())
                    .unwrap_or_else(|| "1".to_string()),
            ),
            FleetTransferMode::EnteringBattleships => ("Battleships ".to_string(), "0".to_string()),
            FleetTransferMode::EnteringCruisers => ("Cruisers ".to_string(), "0".to_string()),
            FleetTransferMode::EnteringDestroyers => ("Destroyers ".to_string(), "0".to_string()),
            FleetTransferMode::EnteringFullTransports => {
                ("Full transports ".to_string(), "0".to_string())
            }
            FleetTransferMode::EnteringEmptyTransports => {
                ("Empty transports ".to_string(), "0".to_string())
            }
            FleetTransferMode::EnteringScouts => ("Scouts ".to_string(), "0".to_string()),
            FleetTransferMode::EnteringEtacs => ("ET ships ".to_string(), "0".to_string()),
        }
    }

    fn finish_fleet_transfer(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(donor_record_index_1_based) = self.fleet.transfer_donor_record_index_1_based
        else {
            self.fleet.transfer_status = Some("Select two fleets for transfer.".to_string());
            return Ok(());
        };
        let Some(host_record_index_1_based) = self.fleet.transfer_host_record_index_1_based else {
            self.fleet.transfer_status = Some("Select two fleets for transfer.".to_string());
            return Ok(());
        };
        self.game_data.transfer_ships_between_fleets(
            self.player.record_index_1_based,
            donor_record_index_1_based,
            host_record_index_1_based,
            self.fleet.transfer_selection.clone(),
        )?;
        self.save_game_data()?;
        let donor_fleet_number = self
            .fleet_number_for_record_index(donor_record_index_1_based)
            .unwrap_or(0);
        let host_fleet_number = self
            .fleet_number_for_record_index(host_record_index_1_based)
            .unwrap_or(0);
        self.fleet.transfer_mode = FleetTransferMode::SelectingFleets;
        self.fleet.transfer_selected_fleets.clear();
        self.fleet.transfer_donor_record_index_1_based = None;
        self.fleet.transfer_host_record_index_1_based = None;
        self.fleet.transfer_select_input.clear();
        self.fleet.transfer_input.clear();
        self.fleet.transfer_selection = FleetDetachSelection::default();
        self.current_screen = ScreenId::FleetTransfer;
        self.fleet.transfer_status = Some(format!(
            "Transferred ships from fleet {} to fleet {}.",
            donor_fleet_number, host_fleet_number
        ));
        Ok(())
    }

    fn resolve_fleet_detach_numeric_input(
        &mut self,
        default: u16,
    ) -> Result<u16, Box<dyn std::error::Error>> {
        let raw = self.fleet.detach_input.trim();
        if raw.is_empty() {
            return Ok(default);
        }
        match raw.parse::<u16>() {
            Ok(value) => Ok(value),
            Err(_) => {
                self.fleet.detach_status = Some("Enter an integer value.".to_string());
                Err("invalid detach numeric input".into())
            }
        }
    }

    pub(super) fn calculate_fleet_eta_message(
        &self,
        row: &FleetRow,
        destination: [u8; 2],
        include_system: bool,
    ) -> String {
        match estimate_fleet_eta_to_destination(
            &self.game_data,
            row.fleet_record_index_1_based - 1,
            destination,
            include_system,
            true,
        ) {
            FleetEtaEstimate::Arrived => format!(
                "Fleet {} reaches [{},{}] in 0 year(s), arriving in {}.",
                row.fleet_number,
                destination[0],
                destination[1],
                self.game_data.conquest.game_year()
            ),
            FleetEtaEstimate::Years(years) => {
                let arrival_year = self.game_data.conquest.game_year() + years;
                format!(
                    "Fleet {} reaches [{},{}] in {} year(s), arriving in {}.",
                    row.fleet_number, destination[0], destination[1], years, arrival_year
                )
            }
            FleetEtaEstimate::Stopped => format!(
                "Fleet {} is stopped and cannot reach [{},{}].",
                row.fleet_number, destination[0], destination[1]
            ),
            FleetEtaEstimate::Unreachable => {
                format!("No route found to [{},{}].", destination[0], destination[1])
            }
        }
    }
}

fn format_fleet_number_for_rows(fleet_number: u16, rows: &[FleetRow]) -> String {
    let max_fleet_number = rows.iter().map(|row| row.fleet_number).max().unwrap_or(1);
    crate::screen::format_fleet_number(fleet_number, max_fleet_number)
}

pub(super) fn fleet_eta_label(game_data: &CoreGameData, fleet_idx: usize) -> String {
    match estimate_fleet_eta(game_data, fleet_idx) {
        FleetEtaEstimate::Arrived => game_data.conquest.game_year().to_string(),
        FleetEtaEstimate::Stopped => "STOP".to_string(),
        FleetEtaEstimate::Unreachable => "N/A".to_string(),
        FleetEtaEstimate::Years(years) => game_data
            .conquest
            .game_year()
            .saturating_add(years)
            .to_string(),
    }
}

pub(super) fn fleet_list_eta_label(game_data: &CoreGameData, fleet_idx: usize) -> String {
    match estimate_fleet_eta(game_data, fleet_idx) {
        FleetEtaEstimate::Arrived => "0".to_string(),
        FleetEtaEstimate::Stopped => "0".to_string(),
        FleetEtaEstimate::Unreachable => "0".to_string(),
        FleetEtaEstimate::Years(years) => years.to_string(),
    }
}
