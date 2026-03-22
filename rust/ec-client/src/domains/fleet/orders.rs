use crate::app::helpers::{
    center_scroll_to_cursor, resolve_default_coords_input, sync_scroll_to_cursor,
};
use crate::app::state::App;
use crate::domains::fleet::FleetAction;
use crate::domains::fleet::state::FleetMissionPickerCaller;
use crate::screen::{
    CommandMenu, FLEET_MISSION_OPTIONS, FleetGroupOrderMode, FleetRow, FleetSingleOrderMode,
    ScreenId, StarbaseRow,
};
use ec_data::build_player_starmap_projection_from_snapshots;
use std::collections::BTreeSet;

impl App {
    pub fn open_fleet_order(&mut self) {
        if self.current_screen == ScreenId::FleetOrder
            && self.fleet.order_mode != FleetSingleOrderMode::SelectingFleet
        {
            self.fleet.order_mode = FleetSingleOrderMode::SelectingFleet;
            self.fleet.order_mission_code = None;
            self.fleet.order_input.clear();
            self.fleet.order_status = None;
            return;
        }
        let rows = self.fleet_rows();
        if rows.is_empty() {
            self.show_command_menu_notice(CommandMenu::Fleet, "You have no active fleets.");
            return;
        }
        self.clear_command_menu_notice();
        self.fleet.order_mode = FleetSingleOrderMode::SelectingFleet;
        self.fleet.order_status = None;
        self.fleet.order_mission_code = None;
        self.fleet.order_input.clear();
        let total = rows.len();
        self.fleet.order_cursor = self.fleet.order_cursor.min(total - 1);
        let selected_record = rows[self.fleet.order_cursor].fleet_record_index_1_based;
        self.fleet.order_fleet_record_index_1_based = Some(selected_record);
        center_scroll_to_cursor(
            &mut self.fleet.order_scroll_offset,
            self.fleet.order_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
            total,
        );
        self.current_screen = ScreenId::FleetOrder;
    }

    pub fn open_fleet_group_order(&mut self) {
        if self.current_screen == ScreenId::FleetGroupOrder
            && self.fleet.group_mode != FleetGroupOrderMode::SelectingFleets
        {
            self.fleet.group_mode = FleetGroupOrderMode::SelectingFleets;
            self.fleet.group_mission_code = None;
            self.fleet.group_input.clear();
            self.fleet.group_status = None;
            return;
        }
        let total = self.fleet_rows().len();
        if total == 0 {
            self.show_command_menu_notice(CommandMenu::Fleet, "You have no active fleets.");
            return;
        }
        self.clear_command_menu_notice();
        self.fleet.group_mode = FleetGroupOrderMode::SelectingFleets;
        self.fleet.group_status = None;
        self.fleet.group_mission_code = None;
        self.fleet.group_input.clear();
        self.fleet.group_selected_fleets.clear();
        self.fleet.group_cursor = self.fleet.group_cursor.min(total - 1);
        center_scroll_to_cursor(
            &mut self.fleet.group_scroll_offset,
            self.fleet.group_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
            total,
        );
        self.current_screen = ScreenId::FleetGroupOrder;
    }

    pub fn open_fleet_mission_picker(&mut self) {
        match self.current_screen {
            ScreenId::FleetOrder => {
                let rows = self.fleet_rows();
                if rows.is_empty() {
                    self.fleet.order_status = Some("You have no active fleets.".to_string());
                    return;
                }
                let Some(row) = rows.get(self.fleet.order_cursor) else {
                    self.fleet.order_status = Some("Select a fleet.".to_string());
                    return;
                };
                self.fleet.order_fleet_record_index_1_based = Some(row.fleet_record_index_1_based);
                self.fleet.mission_picker_caller = Some(FleetMissionPickerCaller::SingleOrder);
            }
            ScreenId::FleetGroupOrder => {
                if self.fleet.group_selected_fleets.is_empty() {
                    self.fleet.group_status = Some("Select at least one fleet.".to_string());
                    return;
                }
                self.fleet.mission_picker_caller = Some(FleetMissionPickerCaller::GroupOrder);
            }
            ScreenId::FleetMissionPicker => match self.fleet.mission_picker_caller {
                Some(FleetMissionPickerCaller::GroupOrder) => {
                    self.current_screen = ScreenId::FleetGroupOrder;
                    self.fleet.group_mode = FleetGroupOrderMode::SelectingFleets;
                    self.fleet.mission_picker_input.clear();
                    self.fleet.mission_picker_status = None;
                    self.fleet.mission_picker_caller = None;
                    return;
                }
                Some(FleetMissionPickerCaller::SingleOrder) => {
                    self.current_screen = ScreenId::FleetOrder;
                    self.fleet.order_mode = FleetSingleOrderMode::SelectingFleet;
                    self.fleet.mission_picker_input.clear();
                    self.fleet.mission_picker_status = None;
                    self.fleet.mission_picker_caller = None;
                    return;
                }
                None => {}
            },
            _ => return,
        }
        self.fleet.order_status = None;
        self.fleet.group_status = None;
        self.fleet.mission_picker_status = None;
        self.fleet.mission_picker_input.clear();
        self.fleet.mission_picker_cursor = self.first_enabled_fleet_mission_index().unwrap_or(1);
        self.current_screen = ScreenId::FleetMissionPicker;
    }

    pub fn move_fleet_group_order(&mut self, delta: i8) {
        if self.current_screen != ScreenId::FleetGroupOrder {
            return;
        }
        let total = self.fleet_rows().len();
        if total == 0 {
            self.fleet.group_cursor = 0;
            return;
        }
        let next = self.fleet.group_cursor as isize + delta as isize;
        self.fleet.group_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.fleet.group_scroll_offset,
            self.fleet.group_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
        if self.fleet.group_mode == FleetGroupOrderMode::SelectingFleets {
            self.fleet.group_input.clear();
        }
        self.fleet.group_status = None;
    }

    pub fn move_fleet_order_select(&mut self, delta: i8) {
        if self.current_screen != ScreenId::FleetOrder
            || self.fleet.order_mode != FleetSingleOrderMode::SelectingFleet
        {
            return;
        }
        let rows = self.fleet_rows();
        let total = rows.len();
        if total == 0 {
            self.fleet.order_cursor = 0;
            self.fleet.order_fleet_record_index_1_based = None;
            return;
        }
        let next = self.fleet.order_cursor as isize + delta as isize;
        self.fleet.order_cursor = next.rem_euclid(total as isize) as usize;
        self.fleet.order_fleet_record_index_1_based =
            Some(rows[self.fleet.order_cursor].fleet_record_index_1_based);
        sync_scroll_to_cursor(
            &mut self.fleet.order_scroll_offset,
            self.fleet.order_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
        self.fleet.order_input.clear();
        self.fleet.order_status = None;
    }

    pub fn move_fleet_mission_picker(&mut self, delta: i8) {
        if self.current_screen != ScreenId::FleetMissionPicker {
            return;
        }
        let total = FLEET_MISSION_OPTIONS.len();
        if total == 0 {
            self.fleet.mission_picker_cursor = 0;
            return;
        }
        let enabled = self.fleet_mission_picker_enabled_flags();
        if !enabled.iter().any(|flag| *flag) {
            self.fleet.mission_picker_status =
                Some("No missions are available for the selected fleets.".to_string());
            return;
        }
        let mut next = self.fleet.mission_picker_cursor as isize;
        let step = if delta >= 0 { 1 } else { -1 };
        let hops = delta.unsigned_abs().max(1) as usize;
        for _ in 0..hops {
            loop {
                next = (next + step).rem_euclid(total as isize);
                if enabled[next as usize] {
                    break;
                }
            }
        }
        self.fleet.mission_picker_cursor = next as usize;
        self.fleet.mission_picker_input.clear();
        self.fleet.mission_picker_status = None;
    }

    pub fn append_fleet_order_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::FleetOrder {
            return;
        }
        match self.fleet.order_mode {
            FleetSingleOrderMode::SelectingFleet => {
                if ch.is_ascii_digit() && self.fleet.order_input.len() < 4 {
                    self.fleet.order_input.push(ch);
                    self.sync_fleet_order_cursor_to_input();
                    self.fleet.order_status = None;
                }
            }
            FleetSingleOrderMode::EnteringTarget => {
                let allow_char = match fleet_target_input_kind(self.fleet.order_mission_code) {
                    FleetTargetInputKind::Coordinates | FleetTargetInputKind::None => {
                        ch.is_ascii_digit() || matches!(ch, ',' | ' ' | '(' | ')' | '[' | ']')
                    }
                    FleetTargetInputKind::StarbaseId | FleetTargetInputKind::FleetId => {
                        ch.is_ascii_digit()
                    }
                };
                if self.fleet.order_input.len() < 16 && allow_char {
                    self.fleet.order_input.push(ch);
                    self.fleet.order_status = None;
                }
            }
        }
    }

    pub fn backspace_fleet_order_input(&mut self) {
        if self.current_screen != ScreenId::FleetOrder {
            return;
        }
        self.fleet.order_input.pop();
        if self.fleet.order_mode == FleetSingleOrderMode::SelectingFleet {
            self.sync_fleet_order_cursor_to_input();
        }
        self.fleet.order_status = None;
    }

    pub fn toggle_fleet_group_order_selection(&mut self) {
        if self.current_screen != ScreenId::FleetGroupOrder {
            return;
        }
        if self.fleet.group_mode != FleetGroupOrderMode::SelectingFleets {
            return;
        }
        let rows = self.fleet_rows();
        let Some(row) = rows.get(self.fleet.group_cursor) else {
            return;
        };
        if !self
            .fleet
            .group_selected_fleets
            .insert(row.fleet_record_index_1_based)
        {
            self.fleet
                .group_selected_fleets
                .remove(&row.fleet_record_index_1_based);
        }
        self.fleet.group_status = None;
    }

    pub fn append_fleet_group_order_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::FleetGroupOrder {
            return;
        }
        match self.fleet.group_mode {
            FleetGroupOrderMode::SelectingFleets => {}
            FleetGroupOrderMode::EnteringTarget => {
                let allow_char = match fleet_target_input_kind(self.fleet.group_mission_code) {
                    FleetTargetInputKind::Coordinates | FleetTargetInputKind::None => {
                        ch.is_ascii_digit() || matches!(ch, ',' | ' ' | '(' | ')' | '[' | ']')
                    }
                    FleetTargetInputKind::StarbaseId | FleetTargetInputKind::FleetId => {
                        ch.is_ascii_digit()
                    }
                };
                if self.fleet.group_input.len() < 16 && allow_char {
                    self.fleet.group_input.push(ch);
                    self.fleet.group_status = None;
                }
            }
        }
    }

    pub fn append_fleet_mission_picker_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::FleetMissionPicker || !ch.is_ascii_digit() {
            return;
        }
        if self.fleet.mission_picker_input.len() >= 2 {
            return;
        }
        self.fleet.mission_picker_input.push(ch);
        self.sync_fleet_mission_picker_cursor_to_input();
        self.fleet.mission_picker_status = None;
    }

    pub fn backspace_fleet_group_order_input(&mut self) {
        if self.current_screen != ScreenId::FleetGroupOrder {
            return;
        }
        if self.fleet.group_mode != FleetGroupOrderMode::SelectingFleets {
            self.fleet.group_input.pop();
            self.fleet.group_status = None;
        }
    }

    pub fn backspace_fleet_mission_picker_input(&mut self) {
        if self.current_screen != ScreenId::FleetMissionPicker {
            return;
        }
        self.fleet.mission_picker_input.pop();
        self.sync_fleet_mission_picker_cursor_to_input();
        self.fleet.mission_picker_status = None;
    }

    pub fn submit_fleet_group_order(&mut self) {
        if self.current_screen != ScreenId::FleetGroupOrder {
            return;
        }
        match self.fleet.group_mode {
            FleetGroupOrderMode::SelectingFleets => {
                self.open_fleet_mission_picker();
            }
            FleetGroupOrderMode::EnteringTarget => {
                let Some(mission_code) = self.fleet.group_mission_code else {
                    self.fleet.group_mode = FleetGroupOrderMode::SelectingFleets;
                    self.fleet.group_status = Some("Choose a group mission first.".to_string());
                    return;
                };
                let (destination, aux0, aux1) = match fleet_target_input_kind(Some(mission_code)) {
                    FleetTargetInputKind::Coordinates | FleetTargetInputKind::None => {
                        let destination = match resolve_default_coords_input(
                            &self.fleet.group_input,
                            self.fleet_group_default_target(),
                        ) {
                            Some(coords) => coords,
                            None => {
                                self.fleet.group_status =
                                    Some("Enter a sector as [x,y].".to_string());
                                return;
                            }
                        };
                        (destination, 0, 0)
                    }
                    FleetTargetInputKind::StarbaseId => {
                        let Some(base) =
                            self.resolve_fleet_group_starbase_target_for_current_mission()
                        else {
                            self.fleet.group_status = Some(
                                "Enter a starbase number from your starbase list.".to_string(),
                            );
                            return;
                        };
                        (base.coords, base.base_id, 1)
                    }
                    FleetTargetInputKind::FleetId => {
                        let Some(host) = self.resolve_fleet_group_host_fleet_for_current_mission()
                        else {
                            self.fleet.group_status = Some(
                                "Enter another fleet number from your fleet list.".to_string(),
                            );
                            return;
                        };
                        if let Err(err) = self.apply_fleet_group_join_order(host) {
                            self.fleet.group_status = Some(err.to_string());
                        }
                        return;
                    }
                };
                let target_planet = self
                    .game_data
                    .planets
                    .records
                    .iter()
                    .find(|planet| planet.coords_raw() == destination);
                if fleet_order_target_requires_planet_system(mission_code)
                    && target_planet.is_none()
                {
                    self.fleet.group_input.clear();
                    self.fleet.group_status = Some(
                        "That mission requires a system with a planet at the target coordinates."
                            .to_string(),
                    );
                    return;
                }
                if fleet_order_target_rejects_owned_planet(mission_code)
                    && target_planet
                        .map(|planet| {
                            planet.owner_empire_slot_raw() as usize
                                == self.player.record_index_1_based
                        })
                        .unwrap_or(false)
                {
                    self.fleet.group_input.clear();
                    self.fleet.group_status = Some(
                        "You cannot order that combat mission against your own planet.".to_string(),
                    );
                    return;
                }
                if fleet_order_target_requires_owned_planet(mission_code)
                    && target_planet
                        .map(|planet| {
                            planet.owner_empire_slot_raw() as usize
                                != self.player.record_index_1_based
                        })
                        .unwrap_or(true)
                {
                    self.fleet.group_input.clear();
                    self.fleet.group_status =
                        Some("That mission requires one of your owned planets.".to_string());
                    return;
                }
                if let Err(err) =
                    self.apply_fleet_group_order(mission_code, destination, aux0, aux1)
                {
                    self.fleet.group_status = Some(err.to_string());
                }
            }
        }
    }

    pub fn submit_fleet_order(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::FleetOrder {
            return Ok(());
        }
        let rows = self.fleet_rows();
        if rows.is_empty() {
            self.current_screen = ScreenId::FleetMenu;
            return Ok(());
        }
        match self.fleet.order_mode {
            FleetSingleOrderMode::SelectingFleet => {
                if !self.fleet.order_input.trim().is_empty() {
                    let target_fleet_id = match self.fleet.order_input.trim().parse::<u16>() {
                        Ok(value) => value,
                        Err(_) => {
                            self.fleet.order_status =
                                Some("Enter a fleet number from the table.".to_string());
                            return Ok(());
                        }
                    };
                    let Some(index) = rows
                        .iter()
                        .position(|row| row.fleet_number == target_fleet_id)
                    else {
                        self.fleet.order_status = Some(format!(
                            "Fleet #{target_fleet_id} is not in your fleet list."
                        ));
                        return Ok(());
                    };
                    self.fleet.order_cursor = index;
                    self.fleet.order_fleet_record_index_1_based =
                        Some(rows[index].fleet_record_index_1_based);
                    sync_scroll_to_cursor(
                        &mut self.fleet.order_scroll_offset,
                        self.fleet.order_cursor,
                        crate::screen::FLEET_VISIBLE_ROWS,
                    );
                } else {
                    self.fleet.order_fleet_record_index_1_based =
                        Some(rows[self.fleet.order_cursor].fleet_record_index_1_based);
                }
                self.fleet.order_input.clear();
                self.fleet.order_status = None;
                self.open_fleet_mission_picker();
            }
            FleetSingleOrderMode::EnteringTarget => {
                let Some(mission_code) = self.fleet.order_mission_code else {
                    self.fleet.order_mode = FleetSingleOrderMode::SelectingFleet;
                    self.fleet.order_status = Some("Choose a fleet mission first.".to_string());
                    return Ok(());
                };
                let (destination, aux0, aux1) = match fleet_target_input_kind(Some(mission_code)) {
                    FleetTargetInputKind::Coordinates | FleetTargetInputKind::None => {
                        let destination = match resolve_default_coords_input(
                            &self.fleet.order_input,
                            self.fleet_order_default_target(),
                        ) {
                            Some(coords) => coords,
                            None => {
                                self.fleet.order_status =
                                    Some("Enter a sector as [x,y].".to_string());
                                return Ok(());
                            }
                        };
                        (destination, 0, 0)
                    }
                    FleetTargetInputKind::StarbaseId => {
                        let Some(base) =
                            self.resolve_fleet_order_starbase_target_for_current_mission()
                        else {
                            self.fleet.order_status = Some(
                                "Enter a starbase number from your starbase list.".to_string(),
                            );
                            return Ok(());
                        };
                        (base.coords, base.base_id, 1)
                    }
                    FleetTargetInputKind::FleetId => {
                        let Some(host) = self.resolve_fleet_order_host_fleet_for_current_mission()
                        else {
                            self.fleet.order_status = Some(
                                "Enter another fleet number from your fleet list.".to_string(),
                            );
                            return Ok(());
                        };
                        if let Err(err) = self.apply_fleet_single_join_order(host) {
                            self.fleet.order_status = Some(err.to_string());
                        }
                        return Ok(());
                    }
                };
                let target_planet = self
                    .game_data
                    .planets
                    .records
                    .iter()
                    .find(|planet| planet.coords_raw() == destination);
                if fleet_order_target_requires_planet_system(mission_code)
                    && target_planet.is_none()
                {
                    self.fleet.order_input.clear();
                    self.fleet.order_status = Some(
                        "That mission needs a system with a planet at the target.".to_string(),
                    );
                    return Ok(());
                }
                if fleet_order_target_rejects_owned_planet(mission_code)
                    && target_planet
                        .map(|planet| {
                            planet.owner_empire_slot_raw() as usize
                                == self.player.record_index_1_based
                        })
                        .unwrap_or(false)
                {
                    self.fleet.order_input.clear();
                    self.fleet.order_status =
                        Some("You cannot send that mission to your own world.".to_string());
                    return Ok(());
                }
                if fleet_order_target_requires_owned_planet(mission_code)
                    && target_planet
                        .map(|planet| {
                            planet.owner_empire_slot_raw() as usize
                                != self.player.record_index_1_based
                        })
                        .unwrap_or(true)
                {
                    self.fleet.order_input.clear();
                    self.fleet.order_status =
                        Some("That mission requires one of your owned planets.".to_string());
                    return Ok(());
                }
                if let Err(err) =
                    self.apply_fleet_single_order(mission_code, destination, aux0, aux1)
                {
                    self.fleet.order_status = Some(err.to_string());
                }
            }
        }
        Ok(())
    }

    pub fn submit_fleet_mission_picker(&mut self) {
        if self.current_screen != ScreenId::FleetMissionPicker {
            return;
        }
        let mission_code = match self.fleet.mission_picker_input.trim() {
            "" => FLEET_MISSION_OPTIONS
                .get(self.fleet.mission_picker_cursor)
                .map(|option| option.code)
                .unwrap_or(1),
            raw => match raw.parse::<u8>() {
                Ok(value) => value,
                Err(_) => {
                    self.fleet.mission_picker_status =
                        Some("Enter a mission number from 0 to 15.".to_string());
                    return;
                }
            },
        };
        if mission_code > 15 {
            self.fleet.mission_picker_status =
                Some("Enter a mission number from 0 to 15.".to_string());
            return;
        }
        let enabled = self.fleet_mission_picker_enabled_flags();
        let Some(index) = FLEET_MISSION_OPTIONS
            .iter()
            .position(|option| option.code == mission_code)
        else {
            self.fleet.mission_picker_status =
                Some("Enter a mission number from 0 to 15.".to_string());
            return;
        };
        if !enabled.get(index).copied().unwrap_or(false) {
            self.fleet.mission_picker_status =
                Some("That mission does not apply to all selected fleets.".to_string());
            return;
        }
        self.fleet.mission_picker_cursor = index;
        self.fleet.mission_picker_input.clear();
        match self.fleet.mission_picker_caller {
            Some(FleetMissionPickerCaller::SingleOrder) => {
                self.fleet.order_mission_code = Some(mission_code);
                self.fleet.mission_picker_status = None;
                self.fleet.mission_picker_caller = None;
                self.current_screen = ScreenId::FleetOrder;
                if fleet_group_order_requires_target(mission_code) {
                    if !self.fleet_order_has_target_available(mission_code) {
                        self.fleet.order_mode = FleetSingleOrderMode::SelectingFleet;
                        self.fleet.order_status = Some(match mission_code {
                            4 => "You have no starbases available to guard.".to_string(),
                            12 => "No colonize target available.".to_string(),
                            13 => "You need another fleet available to join.".to_string(),
                            _ => "No valid target available for that mission.".to_string(),
                        });
                        return;
                    }
                    self.fleet.order_mode = FleetSingleOrderMode::EnteringTarget;
                    self.fleet.order_input.clear();
                } else if let Err(err) = self.apply_fleet_single_order(mission_code, [0, 0], 0, 0) {
                    self.current_screen = ScreenId::FleetMissionPicker;
                    self.fleet.mission_picker_caller = Some(FleetMissionPickerCaller::SingleOrder);
                    self.fleet.mission_picker_status = Some(err.to_string());
                }
            }
            Some(FleetMissionPickerCaller::GroupOrder) => {
                self.fleet.group_mission_code = Some(mission_code);
                self.fleet.mission_picker_status = None;
                self.fleet.mission_picker_caller = None;
                self.current_screen = ScreenId::FleetGroupOrder;
                if fleet_group_order_requires_target(mission_code) {
                    if !self.fleet_group_has_target_available(mission_code) {
                        self.fleet.group_mode = FleetGroupOrderMode::SelectingFleets;
                        self.fleet.group_status = Some(match mission_code {
                            4 => "You have no starbases available to guard.".to_string(),
                            12 => "No colonize target available.".to_string(),
                            13 => "You need another fleet available to join.".to_string(),
                            _ => "No valid target available for that mission.".to_string(),
                        });
                        return;
                    }
                    self.fleet.group_mode = FleetGroupOrderMode::EnteringTarget;
                } else if let Err(err) = self.apply_fleet_group_order(mission_code, [0, 0], 0, 0) {
                    self.current_screen = ScreenId::FleetMissionPicker;
                    self.fleet.mission_picker_caller = Some(FleetMissionPickerCaller::GroupOrder);
                    self.fleet.mission_picker_status = Some(err.to_string());
                }
            }
            None => {
                self.fleet.mission_picker_status =
                    Some("Mission picker has no caller.".to_string());
            }
        }
    }

    pub(crate) fn handle_fleet_order_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match self.fleet.order_mode {
            FleetSingleOrderMode::SelectingFleet => match key.code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    crate::app::Action::Fleet(FleetAction::MoveOrderSelect(-1))
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    crate::app::Action::Fleet(FleetAction::MoveOrderSelect(1))
                }
                KeyCode::PageUp => crate::app::Action::Fleet(FleetAction::MoveOrderSelect(-8)),
                KeyCode::PageDown => crate::app::Action::Fleet(FleetAction::MoveOrderSelect(8)),
                KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitOrder),
                KeyCode::Backspace => crate::app::Action::Fleet(FleetAction::BackspaceOrderInput),
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    crate::app::Action::Fleet(FleetAction::AppendOrderChar(ch))
                }
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::Fleet(FleetAction::OpenMenu)
                }
                _ => crate::app::Action::Noop,
            },
            FleetSingleOrderMode::EnteringTarget => match key.code {
                KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitOrder),
                KeyCode::Backspace => crate::app::Action::Fleet(FleetAction::BackspaceOrderInput),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::Fleet(FleetAction::OpenOrder)
                }
                KeyCode::Char(ch)
                    if ch.is_ascii_digit() || matches!(ch, ',' | ' ' | '(' | ')' | '[' | ']') =>
                {
                    crate::app::Action::Fleet(FleetAction::AppendOrderChar(ch))
                }
                _ => crate::app::Action::Noop,
            },
        }
    }

    pub(crate) fn handle_fleet_group_order_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match self.fleet.group_mode {
            FleetGroupOrderMode::SelectingFleets => match key.code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    crate::app::Action::Fleet(FleetAction::MoveGroupOrder(-1))
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    crate::app::Action::Fleet(FleetAction::MoveGroupOrder(1))
                }
                KeyCode::PageUp => crate::app::Action::Fleet(FleetAction::MoveGroupOrder(-8)),
                KeyCode::PageDown => crate::app::Action::Fleet(FleetAction::MoveGroupOrder(8)),
                KeyCode::Char(' ') => {
                    crate::app::Action::Fleet(FleetAction::ToggleGroupOrderSelection)
                }
                KeyCode::Enter => crate::app::Action::Fleet(FleetAction::OpenMissionPicker),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::Fleet(FleetAction::OpenMenu)
                }
                _ => crate::app::Action::Noop,
            },
            FleetGroupOrderMode::EnteringTarget => match key.code {
                KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitGroupOrder),
                KeyCode::Backspace => {
                    crate::app::Action::Fleet(FleetAction::BackspaceGroupOrderInput)
                }
                KeyCode::Char(ch)
                    if ch.is_ascii_digit() || matches!(ch, ',' | ' ' | '(' | ')' | '[' | ']') =>
                {
                    crate::app::Action::Fleet(FleetAction::AppendGroupOrderChar(ch))
                }
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::Fleet(FleetAction::OpenGroupOrder)
                }
                _ => crate::app::Action::Noop,
            },
        }
    }

    pub(crate) fn handle_fleet_mission_picker_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                crate::app::Action::Fleet(FleetAction::MoveMissionPicker(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                crate::app::Action::Fleet(FleetAction::MoveMissionPicker(1))
            }
            KeyCode::PageUp => crate::app::Action::Fleet(FleetAction::MoveMissionPicker(-8)),
            KeyCode::PageDown => crate::app::Action::Fleet(FleetAction::MoveMissionPicker(8)),
            KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitMissionPicker),
            KeyCode::Backspace => {
                crate::app::Action::Fleet(FleetAction::BackspaceMissionPickerInput)
            }
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                crate::app::Action::Fleet(FleetAction::AppendMissionPickerChar(ch))
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                crate::app::Action::Fleet(FleetAction::OpenMissionPicker)
            }
            _ => crate::app::Action::Noop,
        }
    }

    fn sync_fleet_order_cursor_to_input(&mut self) {
        if self.current_screen != ScreenId::FleetOrder
            || self.fleet.order_mode != FleetSingleOrderMode::SelectingFleet
        {
            return;
        }
        let Ok(target_fleet_id) = self.fleet.order_input.trim().parse::<u16>() else {
            return;
        };
        let rows = self.fleet_rows();
        let Some(index) = rows
            .iter()
            .position(|row| row.fleet_number == target_fleet_id)
        else {
            return;
        };
        self.fleet.order_cursor = index;
        self.fleet.order_fleet_record_index_1_based = Some(rows[index].fleet_record_index_1_based);
        sync_scroll_to_cursor(
            &mut self.fleet.order_scroll_offset,
            self.fleet.order_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
    }

    fn sync_fleet_mission_picker_cursor_to_input(&mut self) {
        if self.current_screen != ScreenId::FleetMissionPicker {
            return;
        }
        let Ok(target_code) = self.fleet.mission_picker_input.trim().parse::<u8>() else {
            return;
        };
        let Some(index) = FLEET_MISSION_OPTIONS
            .iter()
            .position(|option| option.code == target_code)
        else {
            return;
        };
        if self
            .fleet_mission_picker_enabled_flags()
            .get(index)
            .copied()
            .unwrap_or(false)
        {
            self.fleet.mission_picker_cursor = index;
        }
    }

    fn fleet_order_selected_row(&self) -> Option<FleetRow> {
        let rows = self.fleet_rows();
        if let Some(record_index) = self.fleet.order_fleet_record_index_1_based {
            if let Some(row) = rows
                .iter()
                .find(|row| row.fleet_record_index_1_based == record_index)
            {
                return Some(row.clone());
            }
        }
        rows.get(self.fleet.order_cursor).cloned()
    }

    fn fleet_order_default_target(&self) -> [u8; 2] {
        if let Some(mission_code) = self.fleet.order_mission_code {
            if let Some(target) = self.fleet_order_default_target_for_mission(mission_code) {
                return target;
            }
        }
        let Some(row) = self.fleet_order_selected_row() else {
            return [8, 2];
        };
        if row.target_coords[0] > 0 && row.target_coords[1] > 0 {
            row.target_coords
        } else {
            row.coords
        }
    }

    pub(crate) fn fleet_order_target_status_line(&self) -> String {
        fleet_target_status_line(self.fleet.order_mission_code)
    }

    pub(crate) fn fleet_order_target_prompt(&self) -> String {
        match fleet_target_input_kind(self.fleet.order_mission_code) {
            FleetTargetInputKind::StarbaseId => "Starbase # ".to_string(),
            FleetTargetInputKind::FleetId => "Fleet # ".to_string(),
            FleetTargetInputKind::Coordinates => "Target ".to_string(),
            FleetTargetInputKind::None => "Target ".to_string(),
        }
    }

    pub(crate) fn fleet_order_target_default(&self) -> String {
        match fleet_target_input_kind(self.fleet.order_mission_code) {
            FleetTargetInputKind::StarbaseId => self
                .fleet_order_default_starbase()
                .map(|row| row.base_id.to_string())
                .unwrap_or_else(|| "1".to_string()),
            FleetTargetInputKind::FleetId => self
                .fleet_order_default_host_fleet()
                .map(|row| row.fleet_number.to_string())
                .unwrap_or_else(|| "1".to_string()),
            FleetTargetInputKind::Coordinates | FleetTargetInputKind::None => {
                let target = self.fleet_order_default_target();
                format!("{},{}", target[0], target[1])
            }
        }
    }

    fn fleet_order_default_target_for_mission(&self, mission_code: u8) -> Option<[u8; 2]> {
        let selected = self
            .fleet_order_selected_row()
            .map(|row| vec![row])
            .unwrap_or_default();
        self.recommended_fleet_target(mission_code, &selected, BTreeSet::new())
    }

    pub(super) fn fleet_group_default_target(&self) -> [u8; 2] {
        if let Some(mission_code) = self.fleet.group_mission_code {
            if let Some(target) = self.fleet_group_default_target_for_mission(mission_code) {
                return target;
            }
        }
        let rows = self.fleet_rows();
        let Some(row) = rows.get(self.fleet.group_cursor) else {
            return [8, 2];
        };
        if row.target_coords[0] > 0 && row.target_coords[1] > 0 {
            row.target_coords
        } else {
            row.coords
        }
    }

    fn fleet_group_default_target_for_mission(&self, mission_code: u8) -> Option<[u8; 2]> {
        let selected = self.fleet_group_selected_rows();
        self.recommended_fleet_target(
            mission_code,
            &selected,
            self.fleet.group_selected_fleets.clone(),
        )
    }

    fn fleet_order_has_target_available(&self, mission_code: u8) -> bool {
        match fleet_target_input_kind(Some(mission_code)) {
            FleetTargetInputKind::StarbaseId => self.fleet_order_default_starbase().is_some(),
            FleetTargetInputKind::FleetId => self.fleet_order_default_host_fleet().is_some(),
            FleetTargetInputKind::Coordinates | FleetTargetInputKind::None => {
                !fleet_mission_requires_preselected_target(mission_code)
                    || self
                        .fleet_order_default_target_for_mission(mission_code)
                        .is_some()
            }
        }
    }

    fn fleet_group_has_target_available(&self, mission_code: u8) -> bool {
        match fleet_target_input_kind(Some(mission_code)) {
            FleetTargetInputKind::StarbaseId => self.fleet_group_default_starbase().is_some(),
            FleetTargetInputKind::FleetId => self.fleet_group_default_host_fleet().is_some(),
            FleetTargetInputKind::Coordinates | FleetTargetInputKind::None => {
                !fleet_mission_requires_preselected_target(mission_code)
                    || self
                        .fleet_group_default_target_for_mission(mission_code)
                        .is_some()
            }
        }
    }

    fn friendly_colonize_target_claimed_elsewhere(
        &self,
        coords: [u8; 2],
        selected_records: &BTreeSet<usize>,
    ) -> bool {
        self.game_data
            .fleets
            .records
            .iter()
            .enumerate()
            .any(|(idx, fleet)| {
                fleet.owner_empire_raw() as usize == self.player.record_index_1_based
                    && !selected_records.contains(&(idx + 1))
                    && fleet.etac_count() > 0
                    && fleet.standing_order_kind() == ec_data::Order::ColonizeWorld
                    && fleet.standing_order_target_coords_raw() == coords
            })
    }

    fn fleet_group_selected_rows(&self) -> Vec<FleetRow> {
        self.fleet_rows()
            .into_iter()
            .filter(|row| {
                self.fleet
                    .group_selected_fleets
                    .contains(&row.fleet_record_index_1_based)
            })
            .collect()
    }

    fn recommended_fleet_target(
        &self,
        mission_code: u8,
        selected_rows: &[FleetRow],
        selected_records: BTreeSet<usize>,
    ) -> Option<[u8; 2]> {
        let anchor = selected_rows
            .first()
            .map(|row| row.coords)
            .unwrap_or(self.default_planet_prompt_coords());
        if fleet_order_target_rejects_owned_planet(mission_code) {
            return self.closest_known_non_owned_planet_target_from(anchor);
        }
        if mission_code == 4 {
            return self.closest_owned_starbase_target_from(anchor);
        }
        if mission_code == 12 {
            return self.closest_colonize_target_from(anchor, &selected_records);
        }
        if mission_code == 15 {
            return self.closest_owned_planet_target_from(anchor);
        }
        None
    }

    fn closest_owned_starbase_target_from(&self, anchor: [u8; 2]) -> Option<[u8; 2]> {
        self.closest_owned_starbase_from(anchor)
            .map(|row| row.coords)
    }

    fn closest_owned_starbase_from(&self, anchor: [u8; 2]) -> Option<StarbaseRow> {
        self.starbase_rows()
            .into_iter()
            .min_by_key(|row| sector_distance_sq(anchor, row.coords))
    }

    fn fleet_order_default_starbase(&self) -> Option<StarbaseRow> {
        let anchor = self
            .fleet_order_selected_row()
            .map(|row| row.coords)
            .unwrap_or(self.default_planet_prompt_coords());
        self.closest_owned_starbase_from(anchor)
    }

    pub(super) fn fleet_group_default_starbase(&self) -> Option<StarbaseRow> {
        let anchor = self
            .fleet_group_selected_rows()
            .first()
            .map(|row| row.coords)
            .unwrap_or(self.default_planet_prompt_coords());
        self.closest_owned_starbase_from(anchor)
    }

    fn closest_owned_fleet_from(
        &self,
        anchor: [u8; 2],
        excluded_records: &BTreeSet<usize>,
    ) -> Option<FleetRow> {
        self.fleet_rows()
            .into_iter()
            .filter(|row| !excluded_records.contains(&row.fleet_record_index_1_based))
            .min_by_key(|row| sector_distance_sq(anchor, row.coords))
    }

    fn fleet_order_default_host_fleet(&self) -> Option<FleetRow> {
        let selected = self.fleet_order_selected_row()?;
        let mut excluded = BTreeSet::new();
        excluded.insert(selected.fleet_record_index_1_based);
        self.closest_owned_fleet_from(selected.coords, &excluded)
    }

    pub(super) fn fleet_group_default_host_fleet(&self) -> Option<FleetRow> {
        let selected = self.fleet_group_selected_rows();
        let anchor = selected
            .first()
            .map(|row| row.coords)
            .unwrap_or(self.default_planet_prompt_coords());
        self.closest_owned_fleet_from(anchor, &self.fleet.group_selected_fleets)
    }

    fn resolve_fleet_order_starbase_target_for_current_mission(&self) -> Option<StarbaseRow> {
        let default_base_id = self.fleet_order_default_starbase()?.base_id;
        let base_id = resolve_default_u8_input(&self.fleet.order_input, default_base_id)?;
        self.starbase_rows()
            .into_iter()
            .find(|row| row.base_id == base_id)
    }

    fn resolve_fleet_group_starbase_target_for_current_mission(&self) -> Option<StarbaseRow> {
        let default_base_id = self.fleet_group_default_starbase()?.base_id;
        let base_id = resolve_default_u8_input(&self.fleet.group_input, default_base_id)?;
        self.starbase_rows()
            .into_iter()
            .find(|row| row.base_id == base_id)
    }

    fn resolve_fleet_order_host_fleet_for_current_mission(&self) -> Option<FleetRow> {
        let default_fleet_number = self.fleet_order_default_host_fleet()?.fleet_number;
        let fleet_number =
            resolve_default_u16_input(&self.fleet.order_input, default_fleet_number)?;
        let selected_record = self.fleet_order_selected_row()?.fleet_record_index_1_based;
        self.fleet_rows().into_iter().find(|row| {
            row.fleet_number == fleet_number && row.fleet_record_index_1_based != selected_record
        })
    }

    fn resolve_fleet_group_host_fleet_for_current_mission(&self) -> Option<FleetRow> {
        let default_fleet_number = self.fleet_group_default_host_fleet()?.fleet_number;
        let fleet_number =
            resolve_default_u16_input(&self.fleet.group_input, default_fleet_number)?;
        self.fleet_rows().into_iter().find(|row| {
            row.fleet_number == fleet_number
                && !self
                    .fleet
                    .group_selected_fleets
                    .contains(&row.fleet_record_index_1_based)
        })
    }

    fn closest_known_non_owned_planet_target_from(&self, anchor: [u8; 2]) -> Option<[u8; 2]> {
        build_player_starmap_projection_from_snapshots(
            &self.game_data,
            &self.planet_intel_snapshots,
            self.player.record_index_1_based as u8,
        )
        .worlds
        .into_iter()
        .filter(|world| {
            world.known_owner_empire_id.is_some()
                && world.known_owner_empire_id != Some(self.player.record_index_1_based as u8)
        })
        .min_by_key(|world| sector_distance_sq(anchor, world.coords))
        .map(|world| world.coords)
    }

    fn closest_owned_planet_target_from(&self, anchor: [u8; 2]) -> Option<[u8; 2]> {
        self.game_data
            .planets
            .records
            .iter()
            .filter(|planet| {
                planet.owner_empire_slot_raw() as usize == self.player.record_index_1_based
            })
            .min_by_key(|planet| sector_distance_sq(anchor, planet.coords_raw()))
            .map(|planet| planet.coords_raw())
    }

    fn closest_colonize_target_from(
        &self,
        anchor: [u8; 2],
        selected_records: &BTreeSet<usize>,
    ) -> Option<[u8; 2]> {
        build_player_starmap_projection_from_snapshots(
            &self.game_data,
            &self.planet_intel_snapshots,
            self.player.record_index_1_based as u8,
        )
        .worlds
        .into_iter()
        .filter(|world| {
            world.known_owner_empire_id.is_none()
                && !self.friendly_colonize_target_claimed_elsewhere(world.coords, selected_records)
        })
        .min_by_key(|world| sector_distance_sq(anchor, world.coords))
        .map(|world| world.coords)
    }

    fn fleet_row_supports_mission(&self, row: &FleetRow, order_code: u8) -> bool {
        let Some(fleet) = self
            .game_data
            .fleets
            .records
            .get(row.fleet_record_index_1_based - 1)
        else {
            return false;
        };
        let has_combat = fleet.battleship_count() > 0
            || fleet.cruiser_count() > 0
            || fleet.destroyer_count() > 0;
        let has_loaded_troops = fleet.army_count() > 0;
        let has_scout = fleet.scout_count() > 0;
        let has_etac = fleet.etac_count() > 0;

        match order_code {
            0 | 1 | 2 | 3 | 9 | 13 | 14 | 15 => true,
            4 | 5 | 6 => has_combat,
            7 => has_combat && has_loaded_troops,
            8 => has_loaded_troops,
            10 | 11 => has_scout,
            12 => has_etac,
            _ => false,
        }
    }

    pub(crate) fn fleet_mission_picker_enabled_flags(&self) -> Vec<bool> {
        match self.fleet.mission_picker_caller {
            Some(FleetMissionPickerCaller::SingleOrder) => {
                let selected = self
                    .fleet_order_selected_row()
                    .map(|row| vec![row])
                    .unwrap_or_default();
                FLEET_MISSION_OPTIONS
                    .iter()
                    .map(|option| {
                        !selected.is_empty()
                            && selected
                                .iter()
                                .all(|row| self.fleet_row_supports_mission(row, option.code))
                    })
                    .collect()
            }
            Some(FleetMissionPickerCaller::GroupOrder) => {
                let selected = self.fleet_group_selected_rows();
                FLEET_MISSION_OPTIONS
                    .iter()
                    .map(|option| {
                        !selected.is_empty()
                            && selected
                                .iter()
                                .all(|row| self.fleet_row_supports_mission(row, option.code))
                    })
                    .collect()
            }
            None => vec![true; FLEET_MISSION_OPTIONS.len()],
        }
    }

    fn first_enabled_fleet_mission_index(&self) -> Option<usize> {
        self.fleet_mission_picker_enabled_flags()
            .iter()
            .position(|flag| *flag)
    }

    fn apply_fleet_orders_to_rows(
        &mut self,
        selected_rows: &[FleetRow],
        mission_code: u8,
        target: [u8; 2],
        aux0: u8,
        aux1: u8,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for row in selected_rows {
            let speed = self
                .game_data
                .fleets
                .records
                .get(row.fleet_record_index_1_based - 1)
                .map(|fleet| fleet.current_speed())
                .unwrap_or(row.current_speed);
            self.game_data.set_fleet_order(
                row.fleet_record_index_1_based,
                speed,
                mission_code,
                if fleet_group_order_requires_target(mission_code) {
                    target
                } else {
                    [0, 0]
                },
                Some(aux0),
                Some(aux1),
            )?;
        }
        self.save_game_data()?;
        Ok(())
    }

    fn apply_fleet_group_order(
        &mut self,
        mission_code: u8,
        target: [u8; 2],
        aux0: u8,
        aux1: u8,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let selected_rows = self.fleet_group_selected_rows();
        if selected_rows.is_empty() {
            self.fleet.group_mode = FleetGroupOrderMode::SelectingFleets;
            self.fleet.group_status = Some("Select at least one fleet.".to_string());
            return Ok(());
        }
        self.apply_fleet_orders_to_rows(&selected_rows, mission_code, target, aux0, aux1)?;
        let selected_count = selected_rows.len();
        self.fleet.group_mode = FleetGroupOrderMode::SelectingFleets;
        self.fleet.group_mission_code = None;
        self.fleet.group_input.clear();
        self.fleet.group_selected_fleets.clear();
        self.current_screen = ScreenId::FleetGroupOrder;
        self.fleet.group_status = Some(if fleet_group_order_requires_target(mission_code) {
            format!(
                "Applied {} order to {} fleets for sector [{},{}].",
                fleet_group_order_label(mission_code),
                selected_count,
                target[0],
                target[1]
            )
        } else {
            format!(
                "Applied {} order to {} fleets.",
                fleet_group_order_label(mission_code),
                selected_count
            )
        });
        Ok(())
    }

    fn apply_fleet_group_join_order(
        &mut self,
        host: FleetRow,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let selected_rows = self.fleet_group_selected_rows();
        if selected_rows.is_empty() {
            self.fleet.group_mode = FleetGroupOrderMode::SelectingFleets;
            self.fleet.group_status = Some("Select at least one fleet.".to_string());
            return Ok(());
        }
        for row in &selected_rows {
            self.game_data.set_join_fleet_order(
                self.player.record_index_1_based,
                row.fleet_record_index_1_based,
                host.fleet_record_index_1_based,
            )?;
        }
        self.save_game_data()?;
        let selected_count = selected_rows.len();
        self.fleet.group_mode = FleetGroupOrderMode::SelectingFleets;
        self.fleet.group_mission_code = None;
        self.fleet.group_input.clear();
        self.fleet.group_selected_fleets.clear();
        self.current_screen = ScreenId::FleetGroupOrder;
        self.fleet.group_status = Some(format!(
            "Applied join-fleet order to {} fleets with host Fleet #{}.",
            selected_count, host.fleet_number
        ));
        Ok(())
    }

    fn apply_fleet_single_order(
        &mut self,
        mission_code: u8,
        target: [u8; 2],
        aux0: u8,
        aux1: u8,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(selected_row) = self.fleet_order_selected_row() else {
            self.fleet.order_mode = FleetSingleOrderMode::SelectingFleet;
            self.fleet.order_status = Some("Select a fleet.".to_string());
            return Ok(());
        };
        self.apply_fleet_orders_to_rows(&[selected_row.clone()], mission_code, target, aux0, aux1)?;
        self.fleet.order_mode = FleetSingleOrderMode::SelectingFleet;
        self.fleet.order_mission_code = None;
        self.fleet.order_input.clear();
        self.fleet.order_fleet_record_index_1_based = Some(selected_row.fleet_record_index_1_based);
        self.current_screen = ScreenId::FleetOrder;
        self.fleet.order_status = Some(if fleet_group_order_requires_target(mission_code) {
            format!(
                "Applied {} to Fleet #{} for sector [{},{}].",
                fleet_group_order_label(mission_code),
                selected_row.fleet_number,
                target[0],
                target[1]
            )
        } else {
            format!(
                "Applied {} to Fleet #{}.",
                fleet_group_order_label(mission_code),
                selected_row.fleet_number
            )
        });
        Ok(())
    }

    fn apply_fleet_single_join_order(
        &mut self,
        host: FleetRow,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(selected_row) = self.fleet_order_selected_row() else {
            self.fleet.order_mode = FleetSingleOrderMode::SelectingFleet;
            self.fleet.order_status = Some("Select a fleet.".to_string());
            return Ok(());
        };
        self.game_data.set_join_fleet_order(
            self.player.record_index_1_based,
            selected_row.fleet_record_index_1_based,
            host.fleet_record_index_1_based,
        )?;
        self.save_game_data()?;
        self.fleet.order_mode = FleetSingleOrderMode::SelectingFleet;
        self.fleet.order_mission_code = None;
        self.fleet.order_input.clear();
        self.fleet.order_fleet_record_index_1_based = Some(selected_row.fleet_record_index_1_based);
        self.current_screen = ScreenId::FleetOrder;
        self.fleet.order_status = Some(format!(
            "Applied join-fleet order to Fleet #{} with host Fleet #{}.",
            selected_row.fleet_number, host.fleet_number
        ));
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FleetTargetInputKind {
    None,
    Coordinates,
    StarbaseId,
    FleetId,
}

pub(super) fn fleet_target_input_kind(order_code: Option<u8>) -> FleetTargetInputKind {
    match order_code {
        Some(4) => FleetTargetInputKind::StarbaseId,
        Some(13) => FleetTargetInputKind::FleetId,
        Some(code) if fleet_group_order_requires_target(code) => FleetTargetInputKind::Coordinates,
        _ => FleetTargetInputKind::None,
    }
}

pub(super) fn fleet_target_status_line(order_code: Option<u8>) -> String {
    match order_code {
        Some(4) => "Enter the starbase number for Guard a Starbase.".to_string(),
        Some(13) => "Enter the host fleet number for Join another fleet.".to_string(),
        Some(0) => "Enter the target coordinates for None (hold position).".to_string(),
        Some(1) => "Enter the target coordinates for Move Fleet (only).".to_string(),
        Some(2) => "Enter the target coordinates for Seek Home.".to_string(),
        Some(3) => "Enter the target coordinates for Patrol a Sector.".to_string(),
        Some(5) => "Enter the target coordinates for Guard/Blockade a World.".to_string(),
        Some(6) => "Enter the target coordinates for Bombard a World.".to_string(),
        Some(7) => "Enter the target coordinates for Invade a World.".to_string(),
        Some(8) => "Enter the target coordinates for Blitz a World.".to_string(),
        Some(9) => "Enter the target coordinates for View a World.".to_string(),
        Some(10) => "Enter the target coordinates for Scout a Sector.".to_string(),
        Some(11) => "Enter the target coordinates for Scout a Solar System.".to_string(),
        Some(12) => "Enter the target coordinates for Colonize a World.".to_string(),
        Some(14) => "Enter the target coordinates for Rendezvous at Sector.".to_string(),
        Some(15) => "Enter the target coordinates for Salvage.".to_string(),
        _ => "Enter the target for the selected fleet mission.".to_string(),
    }
}

fn fleet_group_order_requires_target(order_code: u8) -> bool {
    !matches!(order_code, 0 | 2)
}

fn fleet_mission_requires_preselected_target(order_code: u8) -> bool {
    matches!(order_code, 4 | 12)
}

fn fleet_order_target_requires_planet_system(order_code: u8) -> bool {
    matches!(order_code, 5 | 6 | 7 | 8 | 9 | 11 | 12 | 15)
}

fn fleet_order_target_rejects_owned_planet(order_code: u8) -> bool {
    matches!(order_code, 6 | 7 | 8)
}

fn fleet_order_target_requires_owned_planet(order_code: u8) -> bool {
    matches!(order_code, 15)
}

fn fleet_group_order_label(order_code: u8) -> &'static str {
    match ec_data::Order::from_raw(order_code) {
        ec_data::Order::HoldPosition => "hold",
        ec_data::Order::MoveOnly => "move",
        ec_data::Order::SeekHome => "seek-home",
        ec_data::Order::PatrolSector => "patrol",
        ec_data::Order::GuardBlockadeWorld => "guard/blockade",
        ec_data::Order::BombardWorld => "bombard",
        ec_data::Order::InvadeWorld => "invade",
        ec_data::Order::BlitzWorld => "blitz",
        ec_data::Order::ViewWorld => "view",
        ec_data::Order::ScoutSector => "scout-sector",
        ec_data::Order::ScoutSolarSystem => "scout-system",
        ec_data::Order::ColonizeWorld => "colonize",
        ec_data::Order::RendezvousSector => "rendezvous",
        ec_data::Order::Salvage => "salvage",
        ec_data::Order::GuardStarbase => "guard-starbase",
        ec_data::Order::JoinAnotherFleet => "join-fleet",
        ec_data::Order::Unknown(_) => "unknown",
    }
}

fn resolve_default_u8_input(input: &str, default: u8) -> Option<u8> {
    let raw = input.trim();
    if raw.is_empty() {
        return Some(default);
    }
    raw.parse::<u8>().ok()
}

fn resolve_default_u16_input(input: &str, default: u16) -> Option<u16> {
    let raw = input.trim();
    if raw.is_empty() {
        return Some(default);
    }
    raw.parse::<u16>().ok()
}

fn sector_distance_sq(a: [u8; 2], b: [u8; 2]) -> u32 {
    let dx = i32::from(a[0]) - i32::from(b[0]);
    let dy = i32::from(a[1]) - i32::from(b[1]);
    (dx * dx + dy * dy) as u32
}

pub(super) fn resolve_yes_no_input(input: &str, default: bool) -> bool {
    match input.trim().to_ascii_uppercase().as_str() {
        "" => default,
        "Y" | "YES" => true,
        "N" | "NO" => false,
        _ => default,
    }
}
