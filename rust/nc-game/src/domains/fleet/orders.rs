use crate::app::helpers::{center_scroll_to_cursor, sync_scroll_to_cursor};
use crate::app::state::App;
use crate::domains::fleet::FleetAction;
use crate::domains::fleet::missions::fleet_record_supports_mission_code;
use crate::domains::fleet::state::{FleetMenuPromptMode, FleetMissionPickerCaller};
use crate::screen::layout::PromptFeedback;
use crate::screen::{
    CommandMenu, FLEET_MISSION_OPTIONS, FleetGroupOrderMode, FleetRow, FleetSingleOrderMode,
    ScreenId, StarbaseRow,
};
use nc_data::PlayerStarmapWorld;
use nc_data::build_player_starmap_projection_from_snapshots;
use nc_data::map_size_for_player_count;
use std::collections::BTreeSet;

impl App {
    fn clear_fleet_order_target_inputs(&mut self) {
        self.fleet.order_input.clear();
        self.fleet.order_target_x_input.clear();
        self.fleet.order_target_y_input.clear();
        self.fleet.order_confirm_input.clear();
    }

    fn clear_fleet_group_target_inputs(&mut self) {
        self.fleet.group_input.clear();
        self.fleet.group_target_x_input.clear();
        self.fleet.group_target_y_input.clear();
        self.fleet.group_confirm_input.clear();
    }

    pub fn open_fleet_order(&mut self) {
        self.fleet.order_status = None;
        self.fleet.order_mission_code = None;
        self.fleet.order_return_to_menu = false;
        self.fleet.order_mode = FleetSingleOrderMode::EnteringTarget;
        self.clear_fleet_order_target_inputs();
        self.fleet.mission_picker_caller = None;
        self.open_fleet_menu_prompt(
            FleetMenuPromptMode::Order,
            self.strongest_owned_fleet_number()
                .map(|value| value.to_string())
                .unwrap_or_default(),
        );
    }

    pub(crate) fn open_fleet_order_with_selected_record(
        &mut self,
        fleet_record_index_1_based: usize,
    ) {
        self.clear_command_menu_notice();
        self.clear_fleet_menu_prompt();
        self.fleet.order_status = None;
        self.fleet.order_mission_code = None;
        self.fleet.order_return_to_menu = true;
        self.fleet.order_mode = FleetSingleOrderMode::EnteringTarget;
        self.clear_fleet_order_target_inputs();
        self.fleet.order_fleet_record_index_1_based = Some(fleet_record_index_1_based);
        self.fleet.mission_picker_caller = None;
        self.current_screen = ScreenId::FleetOrder;
        self.open_fleet_mission_picker();
    }

    pub fn open_fleet_group_order(&mut self) {
        if self.current_screen == ScreenId::FleetGroupOrder
            && self.fleet.group_mode != FleetGroupOrderMode::SelectingFleets
        {
            self.fleet.group_mode = FleetGroupOrderMode::SelectingFleets;
            self.fleet.group_mission_code = None;
            self.clear_fleet_group_target_inputs();
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
        self.clear_fleet_group_target_inputs();
        self.fleet.group_selected_fleets.clear();
        self.fleet.group_cursor = self.fleet.group_cursor.min(total - 1);
        center_scroll_to_cursor(
            &mut self.fleet.group_scroll_offset,
            self.fleet.group_cursor,
            crate::domains::fleet::screens::fleet::fleet_visible_rows(self.screen_geometry),
            total,
        );
        self.current_screen = ScreenId::FleetGroupOrder;
    }

    pub fn open_fleet_mission_picker(&mut self) {
        match self.current_screen {
            ScreenId::FleetMenu
                if self.fleet.menu_prompt_mode == Some(FleetMenuPromptMode::Order) =>
            {
                if self.fleet.order_fleet_record_index_1_based.is_none() {
                    self.fleet.menu_prompt_status =
                        Some(PromptFeedback::error("Enter one of your fleet numbers."));
                    return;
                }
                self.fleet.mission_picker_caller =
                    Some(FleetMissionPickerCaller::SingleOrderReturnToMenu);
            }
            ScreenId::FleetOrder => {
                let Some(row) = self.fleet_order_selected_row() else {
                    self.fleet.order_status =
                        Some("Selected fleet is no longer available.".to_string());
                    return;
                };
                self.fleet.order_fleet_record_index_1_based = Some(row.fleet_record_index_1_based);
                self.fleet.mission_picker_caller = Some(if self.fleet.order_return_to_menu {
                    FleetMissionPickerCaller::SingleOrderReturnToMenu
                } else {
                    FleetMissionPickerCaller::SingleOrderReturnToOrder
                });
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
                Some(FleetMissionPickerCaller::SingleOrderReturnToOrder) => {
                    self.fleet.mission_picker_input.clear();
                    self.fleet.mission_picker_status = None;
                    self.fleet.mission_picker_caller = None;
                    self.current_screen = ScreenId::FleetOrder;
                    return;
                }
                Some(FleetMissionPickerCaller::SingleOrderReturnToMenu) => {
                    self.fleet.mission_picker_input.clear();
                    self.fleet.mission_picker_status = None;
                    self.fleet.mission_picker_caller = None;
                    self.fleet.order_return_to_menu = false;
                    self.current_screen = ScreenId::FleetMenu;
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
        let visible_rows =
            crate::domains::fleet::screens::fleet::fleet_visible_rows(self.screen_geometry);
        sync_scroll_to_cursor(
            &mut self.fleet.group_scroll_offset,
            self.fleet.group_cursor,
            visible_rows,
        );
        if self.fleet.group_mode == FleetGroupOrderMode::SelectingFleets {
            self.fleet.group_input.clear();
        }
        self.fleet.group_status = None;
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
            FleetSingleOrderMode::EnteringTargetX => {
                if self.fleet.order_target_x_input.len() < 2 && ch.is_ascii_digit() {
                    self.fleet.order_target_x_input.push(ch);
                    self.fleet.order_status = None;
                }
            }
            FleetSingleOrderMode::EnteringTargetY => {
                if self.fleet.order_target_y_input.len() < 2 && ch.is_ascii_digit() {
                    self.fleet.order_target_y_input.push(ch);
                    self.fleet.order_status = None;
                }
            }
            FleetSingleOrderMode::ConfirmingTarget => {
                if self.fleet.order_confirm_input.is_empty() && matches!(ch, 'y' | 'Y' | 'n' | 'N')
                {
                    self.fleet.order_confirm_input.push(ch.to_ascii_uppercase());
                    self.fleet.order_status = None;
                }
            }
        }
    }

    pub fn backspace_fleet_order_input(&mut self) {
        if self.current_screen != ScreenId::FleetOrder {
            return;
        }
        match self.fleet.order_mode {
            FleetSingleOrderMode::EnteringTarget => {
                self.fleet.order_input.pop();
            }
            FleetSingleOrderMode::EnteringTargetX => {
                self.fleet.order_target_x_input.pop();
            }
            FleetSingleOrderMode::EnteringTargetY => {
                self.fleet.order_target_y_input.pop();
            }
            FleetSingleOrderMode::ConfirmingTarget => {
                self.fleet.order_confirm_input.pop();
            }
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
            FleetGroupOrderMode::EnteringTargetX => {
                if self.fleet.group_target_x_input.len() < 2 && ch.is_ascii_digit() {
                    self.fleet.group_target_x_input.push(ch);
                    self.fleet.group_status = None;
                }
            }
            FleetGroupOrderMode::EnteringTargetY => {
                if self.fleet.group_target_y_input.len() < 2 && ch.is_ascii_digit() {
                    self.fleet.group_target_y_input.push(ch);
                    self.fleet.group_status = None;
                }
            }
            FleetGroupOrderMode::ConfirmingTarget => {
                if self.fleet.group_confirm_input.is_empty() && matches!(ch, 'y' | 'Y' | 'n' | 'N')
                {
                    self.fleet.group_confirm_input.push(ch.to_ascii_uppercase());
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
        match self.fleet.group_mode {
            FleetGroupOrderMode::SelectingFleets => {}
            FleetGroupOrderMode::EnteringTarget => {
                self.fleet.group_input.pop();
                self.fleet.group_status = None;
            }
            FleetGroupOrderMode::EnteringTargetX => {
                self.fleet.group_target_x_input.pop();
                self.fleet.group_status = None;
            }
            FleetGroupOrderMode::EnteringTargetY => {
                self.fleet.group_target_y_input.pop();
                self.fleet.group_status = None;
            }
            FleetGroupOrderMode::ConfirmingTarget => {
                self.fleet.group_confirm_input.pop();
                self.fleet.group_status = None;
            }
        }
    }

    pub fn cancel_fleet_order(&mut self) {
        if self.current_screen != ScreenId::FleetOrder {
            return;
        }
        self.fleet.order_status = None;
        match self.fleet.order_mode {
            FleetSingleOrderMode::EnteringTargetY => {
                self.fleet.order_mode = FleetSingleOrderMode::EnteringTargetX
            }
            FleetSingleOrderMode::ConfirmingTarget => {
                self.fleet.order_confirm_input.clear();
                self.fleet.order_mode = FleetSingleOrderMode::EnteringTargetY;
            }
            FleetSingleOrderMode::EnteringTarget | FleetSingleOrderMode::EnteringTargetX => {
                self.open_fleet_mission_picker();
            }
        }
    }

    pub fn cancel_fleet_group_order(&mut self) {
        if self.current_screen != ScreenId::FleetGroupOrder {
            return;
        }
        self.fleet.group_status = None;
        match self.fleet.group_mode {
            FleetGroupOrderMode::SelectingFleets => {}
            FleetGroupOrderMode::EnteringTargetY => {
                self.fleet.group_mode = FleetGroupOrderMode::EnteringTargetX;
            }
            FleetGroupOrderMode::ConfirmingTarget => {
                self.fleet.group_confirm_input.clear();
                self.fleet.group_mode = FleetGroupOrderMode::EnteringTargetY;
            }
            FleetGroupOrderMode::EnteringTarget | FleetGroupOrderMode::EnteringTargetX => {
                self.open_fleet_mission_picker();
            }
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

    fn resolve_target_axis_input(
        &self,
        input: &str,
        default: Option<u8>,
        axis_label: &str,
    ) -> Result<u8, String> {
        let max = map_size_for_player_count(self.game_data.conquest.player_count());
        let raw = input.trim();
        let value = if raw.is_empty() {
            default.ok_or_else(|| format!("Enter {axis_label} from 1 to {max}."))?
        } else {
            raw.parse::<u8>()
                .map_err(|_| format!("Enter {axis_label} from 1 to {max}."))?
        };
        if value == 0 || value > max {
            return Err(format!("Enter {axis_label} from 1 to {max}."));
        }
        Ok(value)
    }

    fn resolve_fleet_order_split_target(&self) -> Result<[u8; 2], String> {
        let default_x = self
            .fleet_order_default_target_coords()
            .map(|coords| coords[0]);
        let default_y = self.fleet_order_default_target_y_value();
        Ok([
            self.resolve_target_axis_input(&self.fleet.order_target_x_input, default_x, "XX")?,
            self.resolve_target_axis_input(&self.fleet.order_target_y_input, default_y, "YY")?,
        ])
    }

    pub(crate) fn resolve_fleet_group_split_target(&self) -> Result<[u8; 2], String> {
        let default_x = self
            .fleet_group_default_target_coords()
            .map(|coords| coords[0]);
        let default_y = self.fleet_group_default_target_y_value();
        Ok([
            self.resolve_target_axis_input(&self.fleet.group_target_x_input, default_x, "XX")?,
            self.resolve_target_axis_input(&self.fleet.group_target_y_input, default_y, "YY")?,
        ])
    }

    fn validate_fleet_target_for_mission(
        &self,
        mission_code: u8,
        destination: [u8; 2],
        single: bool,
    ) -> Result<(), String> {
        let target_planet = self
            .game_data
            .planets
            .records
            .iter()
            .find(|planet| planet.coords_raw() == destination);
        if fleet_order_target_requires_planet_system(mission_code) && target_planet.is_none() {
            return Err(if single {
                "That mission needs a system with a planet at the target.".to_string()
            } else {
                "That mission requires a system with a planet at the target coordinates."
                    .to_string()
            });
        }
        if fleet_order_target_rejects_owned_planet(mission_code)
            && target_planet
                .map(|planet| {
                    planet.owner_empire_slot_raw() as usize == self.player.record_index_1_based
                })
                .unwrap_or(false)
        {
            return Err(if single {
                "You cannot send that mission to your own world.".to_string()
            } else {
                "You cannot order that combat mission against your own planet.".to_string()
            });
        }
        if fleet_order_target_rejects_owned_scout_target(mission_code)
            && target_planet
                .map(|planet| {
                    planet.owner_empire_slot_raw() as usize == self.player.record_index_1_based
                })
                .unwrap_or(false)
        {
            return Err(if single {
                "You cannot scout your own planet or system.".to_string()
            } else {
                "You cannot order scouts to target your own planet or system.".to_string()
            });
        }
        if fleet_order_target_requires_owned_planet(mission_code)
            && target_planet
                .map(|planet| {
                    planet.owner_empire_slot_raw() as usize != self.player.record_index_1_based
                })
                .unwrap_or(true)
        {
            return Err("That mission requires one of your owned planets.".to_string());
        }
        Ok(())
    }

    fn fleet_target_eta_estimate(
        &self,
        row: &FleetRow,
        mission_code: u8,
        destination: [u8; 2],
    ) -> nc_engine::FleetEtaEstimate {
        nc_engine::estimate_fleet_eta_to_destination(
            &self.game_data,
            row.fleet_record_index_1_based - 1,
            destination,
            fleet_order_target_requires_planet_system(mission_code),
            true,
        )
    }

    fn fleet_target_eta_sort_key(estimate: nc_engine::FleetEtaEstimate) -> (u8, u16) {
        match estimate {
            nc_engine::FleetEtaEstimate::Arrived => (0, 0),
            nc_engine::FleetEtaEstimate::Years(years) => (1, years),
            nc_engine::FleetEtaEstimate::Stopped => (2, 0),
            nc_engine::FleetEtaEstimate::Unreachable => (3, 0),
        }
    }

    fn format_target_eta_message(
        &self,
        subject: &str,
        destination: [u8; 2],
        estimate: nc_engine::FleetEtaEstimate,
    ) -> String {
        let target = format!("({:02},{:02})", destination[0], destination[1]);
        match estimate {
            nc_engine::FleetEtaEstimate::Arrived => format!(
                "{subject} reaches {target} in 0 year(s), arriving in {}.",
                self.game_data.conquest.game_year()
            ),
            nc_engine::FleetEtaEstimate::Years(years) => format!(
                "{subject} reaches {target} in {years} year(s), arriving in {}.",
                self.game_data.conquest.game_year() + years
            ),
            nc_engine::FleetEtaEstimate::Stopped => {
                format!("{subject} is stopped and cannot reach {target}.")
            }
            nc_engine::FleetEtaEstimate::Unreachable => {
                format!("No route found for {subject} to {target}.")
            }
        }
    }

    fn fleet_order_confirmation_eta_message(
        &self,
        mission_code: u8,
        destination: [u8; 2],
    ) -> Option<String> {
        let row = self.fleet_order_selected_row()?;
        let estimate = self.fleet_target_eta_estimate(&row, mission_code, destination);
        Some(self.format_target_eta_message(
            &format!("Fleet {}", row.fleet_number),
            destination,
            estimate,
        ))
    }

    pub(crate) fn fleet_group_confirmation_eta_message(
        &self,
        mission_code: u8,
        destination: [u8; 2],
    ) -> Option<String> {
        let selected = self.fleet_group_selected_rows();
        let (row, estimate) = selected
            .iter()
            .map(|row| {
                (
                    row,
                    self.fleet_target_eta_estimate(row, mission_code, destination),
                )
            })
            .max_by_key(|(_, estimate)| Self::fleet_target_eta_sort_key(*estimate))?;
        let subject = if selected.len() == 1 {
            format!("Fleet {}", row.fleet_number)
        } else {
            format!("Slowest selected fleet (Fleet {})", row.fleet_number)
        };
        Some(self.format_target_eta_message(&subject, destination, estimate))
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
                        self.fleet.group_mode = FleetGroupOrderMode::EnteringTargetX;
                        return;
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
                if let Err(err) =
                    self.validate_fleet_target_for_mission(mission_code, destination, false)
                {
                    self.fleet.group_status = Some(err);
                    return;
                }
                if let Err(err) =
                    self.apply_fleet_group_order(mission_code, destination, aux0, aux1)
                {
                    self.fleet.group_status = Some(err.to_string());
                }
            }
            FleetGroupOrderMode::EnteringTargetX => {
                let default = self
                    .fleet_group_default_target_coords()
                    .map(|coords| coords[0]);
                match self.resolve_target_axis_input(
                    &self.fleet.group_target_x_input,
                    default,
                    "XX",
                ) {
                    Ok(value) => {
                        if self.fleet.group_target_x_input.trim().is_empty() {
                            self.fleet.group_target_x_input = format!("{value:02}");
                        }
                        self.fleet.group_mode = FleetGroupOrderMode::EnteringTargetY;
                        self.fleet.group_status = None;
                    }
                    Err(err) => self.fleet.group_status = Some(err),
                }
            }
            FleetGroupOrderMode::EnteringTargetY => {
                let Some(mission_code) = self.fleet.group_mission_code else {
                    self.fleet.group_mode = FleetGroupOrderMode::SelectingFleets;
                    self.fleet.group_status = Some("Choose a group mission first.".to_string());
                    return;
                };
                let destination = match self.resolve_fleet_group_split_target() {
                    Ok(coords) => coords,
                    Err(err) => {
                        self.fleet.group_mode = FleetGroupOrderMode::EnteringTargetX;
                        self.fleet.group_status = Some(err);
                        return;
                    }
                };
                if self.fleet.group_target_y_input.trim().is_empty() {
                    if let Some(default_y) = self.fleet_group_default_target_y_value() {
                        self.fleet.group_target_y_input = format!("{default_y:02}");
                    }
                }
                if let Err(err) =
                    self.validate_fleet_target_for_mission(mission_code, destination, false)
                {
                    self.fleet.group_mode = FleetGroupOrderMode::EnteringTargetX;
                    self.fleet.group_status = Some(err);
                    return;
                }
                self.fleet.group_confirm_input.clear();
                self.fleet.group_mode = FleetGroupOrderMode::ConfirmingTarget;
                self.fleet.group_status = None;
            }
            FleetGroupOrderMode::ConfirmingTarget => {
                if !resolve_yes_no_input(&self.fleet.group_confirm_input, true) {
                    self.fleet.group_mode = FleetGroupOrderMode::SelectingFleets;
                    self.fleet.group_mission_code = None;
                    self.clear_fleet_group_target_inputs();
                    self.fleet.group_selected_fleets.clear();
                    self.open_fleet_menu();
                    return;
                }
                let Some(mission_code) = self.fleet.group_mission_code else {
                    self.fleet.group_mode = FleetGroupOrderMode::SelectingFleets;
                    self.fleet.group_status = Some("Choose a group mission first.".to_string());
                    return;
                };
                let destination = match self.resolve_fleet_group_split_target() {
                    Ok(coords) => coords,
                    Err(err) => {
                        self.fleet.group_mode = FleetGroupOrderMode::EnteringTargetX;
                        self.fleet.group_status = Some(err);
                        return;
                    }
                };
                if let Err(err) =
                    self.validate_fleet_target_for_mission(mission_code, destination, false)
                {
                    self.fleet.group_mode = FleetGroupOrderMode::EnteringTargetX;
                    self.fleet.group_status = Some(err);
                    return;
                }
                if let Err(err) = self.apply_fleet_group_order(mission_code, destination, 0, 0) {
                    self.fleet.group_status = Some(err.to_string());
                }
            }
        }
    }

    pub fn submit_fleet_order(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::FleetOrder {
            return Ok(());
        }
        let Some(mission_code) = self.fleet.order_mission_code else {
            self.fleet.order_status = Some("Choose a fleet mission first.".to_string());
            return Ok(());
        };
        match self.fleet.order_mode {
            FleetSingleOrderMode::EnteringTarget => {
                let (destination, aux0, aux1) = match fleet_target_input_kind(Some(mission_code)) {
                    FleetTargetInputKind::Coordinates | FleetTargetInputKind::None => {
                        self.fleet.order_mode = FleetSingleOrderMode::EnteringTargetX;
                        return Ok(());
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
                if let Err(err) =
                    self.validate_fleet_target_for_mission(mission_code, destination, true)
                {
                    self.fleet.order_status = Some(err);
                    return Ok(());
                }
                if let Err(err) =
                    self.apply_fleet_single_order(mission_code, destination, aux0, aux1)
                {
                    self.fleet.order_status = Some(err.to_string());
                }
            }
            FleetSingleOrderMode::EnteringTargetX => {
                let default = self
                    .fleet_order_default_target_coords()
                    .map(|coords| coords[0]);
                match self.resolve_target_axis_input(
                    &self.fleet.order_target_x_input,
                    default,
                    "XX",
                ) {
                    Ok(value) => {
                        if self.fleet.order_target_x_input.trim().is_empty() {
                            self.fleet.order_target_x_input = format!("{value:02}");
                        }
                        self.fleet.order_mode = FleetSingleOrderMode::EnteringTargetY;
                        self.fleet.order_status = None;
                    }
                    Err(err) => self.fleet.order_status = Some(err),
                }
            }
            FleetSingleOrderMode::EnteringTargetY => {
                let destination = match self.resolve_fleet_order_split_target() {
                    Ok(coords) => coords,
                    Err(err) => {
                        self.fleet.order_mode = FleetSingleOrderMode::EnteringTargetX;
                        self.fleet.order_status = Some(err);
                        return Ok(());
                    }
                };
                if self.fleet.order_target_y_input.trim().is_empty() {
                    if let Some(default_y) = self.fleet_order_default_target_y_value() {
                        self.fleet.order_target_y_input = format!("{default_y:02}");
                    }
                }
                if let Err(err) =
                    self.validate_fleet_target_for_mission(mission_code, destination, true)
                {
                    self.fleet.order_mode = FleetSingleOrderMode::EnteringTargetX;
                    self.fleet.order_status = Some(err);
                    return Ok(());
                }
                self.fleet.order_confirm_input.clear();
                self.fleet.order_mode = FleetSingleOrderMode::ConfirmingTarget;
                self.fleet.order_status = None;
            }
            FleetSingleOrderMode::ConfirmingTarget => {
                if !resolve_yes_no_input(&self.fleet.order_confirm_input, true) {
                    self.fleet.order_mode = FleetSingleOrderMode::EnteringTarget;
                    self.fleet.order_mission_code = None;
                    self.clear_fleet_order_target_inputs();
                    self.open_fleet_menu();
                    return Ok(());
                }
                let destination = match self.resolve_fleet_order_split_target() {
                    Ok(coords) => coords,
                    Err(err) => {
                        self.fleet.order_mode = FleetSingleOrderMode::EnteringTargetX;
                        self.fleet.order_status = Some(err);
                        return Ok(());
                    }
                };
                if let Err(err) =
                    self.validate_fleet_target_for_mission(mission_code, destination, true)
                {
                    self.fleet.order_mode = FleetSingleOrderMode::EnteringTargetX;
                    self.fleet.order_status = Some(err);
                    return Ok(());
                }
                if let Err(err) = self.apply_fleet_single_order(mission_code, destination, 0, 0) {
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
            Some(FleetMissionPickerCaller::SingleOrderReturnToOrder)
            | Some(FleetMissionPickerCaller::SingleOrderReturnToMenu) => {
                self.fleet.order_mission_code = Some(mission_code);
                self.fleet.mission_picker_status = None;
                if fleet_group_order_requires_target(mission_code) {
                    if !self.fleet_order_has_target_available(mission_code) {
                        self.fleet.mission_picker_status = Some(match mission_code {
                            4 => "You have no starbases available to guard.".to_string(),
                            12 => "No colonize target available.".to_string(),
                            13 => "You need another fleet available to join.".to_string(),
                            _ => "No valid target available for that mission.".to_string(),
                        });
                        return;
                    }
                    self.fleet.mission_picker_caller = None;
                    self.fleet.order_mode = match fleet_target_input_kind(Some(mission_code)) {
                        FleetTargetInputKind::Coordinates => FleetSingleOrderMode::EnteringTargetX,
                        FleetTargetInputKind::StarbaseId
                        | FleetTargetInputKind::FleetId
                        | FleetTargetInputKind::None => FleetSingleOrderMode::EnteringTarget,
                    };
                    self.clear_fleet_order_target_inputs();
                    self.fleet.order_status = None;
                    self.current_screen = ScreenId::FleetOrder;
                } else if let Err(err) = self.apply_fleet_single_order(mission_code, [0, 0], 0, 0) {
                    self.current_screen = ScreenId::FleetMissionPicker;
                    self.fleet.mission_picker_caller = Some(if self.fleet.order_return_to_menu {
                        FleetMissionPickerCaller::SingleOrderReturnToMenu
                    } else {
                        FleetMissionPickerCaller::SingleOrderReturnToOrder
                    });
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
                    self.clear_fleet_group_target_inputs();
                    self.fleet.group_mode = match fleet_target_input_kind(Some(mission_code)) {
                        FleetTargetInputKind::Coordinates => FleetGroupOrderMode::EnteringTargetX,
                        FleetTargetInputKind::StarbaseId
                        | FleetTargetInputKind::FleetId
                        | FleetTargetInputKind::None => FleetGroupOrderMode::EnteringTarget,
                    };
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
            FleetSingleOrderMode::EnteringTarget => match key.code {
                KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitOrder),
                KeyCode::Backspace => crate::app::Action::Fleet(FleetAction::BackspaceOrderInput),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::Fleet(FleetAction::CancelOrder)
                }
                KeyCode::Char(ch)
                    if ch.is_ascii_digit() || matches!(ch, ',' | ' ' | '(' | ')' | '[' | ']') =>
                {
                    crate::app::Action::Fleet(FleetAction::AppendOrderChar(ch))
                }
                _ => crate::app::Action::Noop,
            },
            FleetSingleOrderMode::EnteringTargetX | FleetSingleOrderMode::EnteringTargetY => {
                match key.code {
                    KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitOrder),
                    KeyCode::Backspace => {
                        crate::app::Action::Fleet(FleetAction::BackspaceOrderInput)
                    }
                    KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                        crate::app::Action::Fleet(FleetAction::CancelOrder)
                    }
                    KeyCode::Char(ch) if ch.is_ascii_digit() => {
                        crate::app::Action::Fleet(FleetAction::AppendOrderChar(ch))
                    }
                    _ => crate::app::Action::Noop,
                }
            }
            FleetSingleOrderMode::ConfirmingTarget => match key.code {
                KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitOrder),
                KeyCode::Backspace => crate::app::Action::Fleet(FleetAction::BackspaceOrderInput),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::Fleet(FleetAction::CancelOrder)
                }
                KeyCode::Char(ch) if matches!(ch, 'y' | 'Y' | 'n' | 'N') => {
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
                    crate::app::Action::Fleet(FleetAction::CancelGroupOrder)
                }
                _ => crate::app::Action::Noop,
            },
            FleetGroupOrderMode::EnteringTargetX | FleetGroupOrderMode::EnteringTargetY => {
                match key.code {
                    KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitGroupOrder),
                    KeyCode::Backspace => {
                        crate::app::Action::Fleet(FleetAction::BackspaceGroupOrderInput)
                    }
                    KeyCode::Char(ch) if ch.is_ascii_digit() => {
                        crate::app::Action::Fleet(FleetAction::AppendGroupOrderChar(ch))
                    }
                    KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                        crate::app::Action::Fleet(FleetAction::CancelGroupOrder)
                    }
                    _ => crate::app::Action::Noop,
                }
            }
            FleetGroupOrderMode::ConfirmingTarget => match key.code {
                KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitGroupOrder),
                KeyCode::Backspace => {
                    crate::app::Action::Fleet(FleetAction::BackspaceGroupOrderInput)
                }
                KeyCode::Char(ch) if matches!(ch, 'y' | 'Y' | 'n' | 'N') => {
                    crate::app::Action::Fleet(FleetAction::AppendGroupOrderChar(ch))
                }
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::Fleet(FleetAction::CancelGroupOrder)
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

    fn sync_fleet_mission_picker_cursor_to_input(&mut self) {
        if self.current_screen != ScreenId::FleetMissionPicker {
            return;
        }
        let rows = FLEET_MISSION_OPTIONS
            .iter()
            .map(|option| vec![format!("{:02}", option.code)])
            .collect::<Vec<_>>();
        let Some(index) = crate::screen::table_selection::find_typed_jump_index(
            &rows,
            0,
            &self.fleet.mission_picker_input,
        ) else {
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

    pub(crate) fn fleet_order_selected_row(&self) -> Option<FleetRow> {
        let rows = self.fleet_rows();
        let record_index = self.fleet.order_fleet_record_index_1_based?;
        rows.into_iter()
            .find(|row| row.fleet_record_index_1_based == record_index)
    }

    pub(crate) fn fleet_order_current_order_label(&self) -> String {
        self.fleet_order_selected_row()
            .map(|row| crate::domains::fleet::screens::fleet::fleet_order_label(row.order_code))
            .unwrap_or("Unknown")
            .to_string()
    }

    pub(crate) fn fleet_order_new_order_label(&self) -> String {
        self.fleet
            .order_mission_code
            .map(crate::domains::fleet::screens::fleet::fleet_order_label)
            .unwrap_or("Unknown")
            .to_string()
    }

    pub(crate) fn fleet_order_target_status_line(&self) -> String {
        if self.fleet.order_mode == FleetSingleOrderMode::ConfirmingTarget
            && let (Some(mission_code), Ok(destination)) = (
                self.fleet.order_mission_code,
                self.resolve_fleet_order_split_target(),
            )
            && let Some(message) =
                self.fleet_order_confirmation_eta_message(mission_code, destination)
        {
            return message;
        }
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
            FleetTargetInputKind::Coordinates | FleetTargetInputKind::None => self
                .fleet_order_default_target_coords()
                .map(|target| format!("{},{}", target[0], target[1]))
                .unwrap_or_default(),
        }
    }

    pub(crate) fn fleet_order_target_x_default(&self) -> String {
        self.fleet_order_default_target_coords()
            .map(|coords| format!("{:02}", coords[0]))
            .unwrap_or_default()
    }

    pub(crate) fn fleet_order_target_y_default(&self) -> String {
        self.fleet_order_default_target_y_value()
            .map(|value| format!("{value:02}"))
            .unwrap_or_default()
    }

    pub(crate) fn fleet_order_target_x_display_input(&self) -> String {
        self.fleet.order_target_x_input.clone()
    }

    pub(crate) fn fleet_order_target_y_display_input(&self) -> String {
        self.fleet.order_target_y_input.clone()
    }

    pub(crate) fn fleet_group_target_x_default_value(&self) -> String {
        self.fleet_group_default_target_coords()
            .map(|coords| format!("{:02}", coords[0]))
            .unwrap_or_default()
    }

    pub(crate) fn fleet_group_target_y_default_value(&self) -> String {
        self.fleet_group_default_target_y_value()
            .map(|value| format!("{value:02}"))
            .unwrap_or_default()
    }

    pub(crate) fn fleet_group_target_x_display_input(&self) -> String {
        self.fleet.group_target_x_input.clone()
    }

    pub(crate) fn fleet_group_target_y_display_input(&self) -> String {
        self.fleet.group_target_y_input.clone()
    }

    fn fleet_order_default_target_for_mission(&self, mission_code: u8) -> Option<[u8; 2]> {
        let selected = self
            .fleet_order_selected_row()
            .map(|row| vec![row])
            .unwrap_or_default();
        self.recommended_fleet_target(mission_code, &selected, BTreeSet::new())
    }

    pub(crate) fn fleet_order_default_target_coords(&self) -> Option<[u8; 2]> {
        let mission_code = self.fleet.order_mission_code?;
        self.fleet_order_default_target_for_mission(mission_code)
    }

    pub(crate) fn fleet_group_default_target_coords(&self) -> Option<[u8; 2]> {
        let mission_code = self.fleet.group_mission_code?;
        self.fleet_group_default_target_for_mission(mission_code)
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
                    && fleet.standing_order_kind() == nc_data::Order::ColonizeWorld
                    && fleet.standing_order_target_coords_raw() == coords
            })
    }

    fn friendly_scout_target_claimed_elsewhere(
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
                    && fleet.scout_count() > 0
                    && matches!(
                        fleet.standing_order_kind(),
                        nc_data::Order::ScoutSector | nc_data::Order::ScoutSolarSystem
                    )
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
        self.recommended_fleet_target_candidates(mission_code, selected_rows, selected_records)
            .into_iter()
            .next()
    }

    fn recommended_fleet_target_candidates(
        &self,
        mission_code: u8,
        selected_rows: &[FleetRow],
        selected_records: BTreeSet<usize>,
    ) -> Vec<[u8; 2]> {
        let anchor = selected_rows
            .first()
            .map(|row| row.coords)
            .unwrap_or(self.default_planet_prompt_coords());
        match mission_code {
            0 | 1 | 3 => vec![anchor],
            4 => self
                .closest_owned_starbase_target_from(anchor)
                .into_iter()
                .collect(),
            2 | 5 | 15 => self.database_owned_planet_targets_from(anchor),
            6 | 7 | 8 => self.database_known_enemy_planet_targets_from(anchor),
            9 => self.under_scouted_world_targets_from(anchor),
            10 => self.scout_sector_target_candidates_from(anchor, &selected_records),
            11 => self.scout_system_target_candidates_from(anchor, &selected_records),
            12 => self.colonize_target_candidates_from(anchor, &selected_records),
            14 => self.rendezvous_target_candidates_from(anchor, &selected_records),
            _ => Vec::new(),
        }
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

    fn database_owned_planet_targets_from(&self, anchor: [u8; 2]) -> Vec<[u8; 2]> {
        self.filtered_planet_database_targets_from(anchor, |world| {
            world.known_owner_empire_id == Some(self.player.record_index_1_based as u8)
        })
    }

    fn planet_database_world_targets_from(&self, anchor: [u8; 2]) -> Vec<[u8; 2]> {
        self.filtered_planet_database_targets_from(anchor, |_| true)
    }

    /// Hostile targets only from the player's fog-of-war planet database.
    /// No raw runtime ownership fallback is used here.
    fn database_known_enemy_planet_targets_from(&self, anchor: [u8; 2]) -> Vec<[u8; 2]> {
        self.filtered_planet_database_targets_from(anchor, |world| {
            matches!(
                world.known_owner_empire_id,
                Some(owner)
                    if owner > 0 && owner != self.player.record_index_1_based as u8
            )
        })
    }

    /// Under-scouted worlds: Partial or Unknown intel tier, falling back to all known.
    /// View is typically used to scan worlds the player knows little about.
    fn under_scouted_world_targets_from(&self, anchor: [u8; 2]) -> Vec<[u8; 2]> {
        let under_scouted = self.filtered_planet_database_targets_from(anchor, |world| {
            matches!(
                world.intel_tier,
                nc_data::IntelTier::Partial | nc_data::IntelTier::Unknown
            )
        });
        if under_scouted.is_empty() {
            self.planet_database_world_targets_from(anchor)
        } else {
            under_scouted
        }
    }

    fn colonize_target_candidates_from(
        &self,
        anchor: [u8; 2],
        selected_records: &BTreeSet<usize>,
    ) -> Vec<[u8; 2]> {
        self.filtered_planet_database_targets_from(anchor, |world| {
            matches!(world.known_owner_empire_id, None | Some(0))
                && !self.friendly_colonize_target_claimed_elsewhere(world.coords, selected_records)
        })
    }

    fn scout_sector_target_candidates_from(
        &self,
        anchor: [u8; 2],
        selected_records: &BTreeSet<usize>,
    ) -> Vec<[u8; 2]> {
        let mut coords = self.filtered_planet_database_targets_from(anchor, |world| {
            world.known_owner_empire_id != Some(self.player.record_index_1_based as u8)
                && !self.friendly_scout_target_claimed_elsewhere(world.coords, selected_records)
        });
        if coords.is_empty() && !self.coords_are_owned_system(anchor) {
            coords.push(anchor);
        }
        coords
    }

    fn scout_system_target_candidates_from(
        &self,
        anchor: [u8; 2],
        selected_records: &BTreeSet<usize>,
    ) -> Vec<[u8; 2]> {
        self.filtered_planet_database_targets_from(anchor, |world| {
            world.known_owner_empire_id != Some(self.player.record_index_1_based as u8)
                && !self.friendly_scout_target_claimed_elsewhere(world.coords, selected_records)
        })
    }

    fn rendezvous_target_candidates_from(
        &self,
        anchor: [u8; 2],
        selected_records: &BTreeSet<usize>,
    ) -> Vec<[u8; 2]> {
        let mut coords = self
            .game_data
            .fleets
            .records
            .iter()
            .enumerate()
            .filter(|(idx, fleet)| {
                fleet.owner_empire_raw() as usize == self.player.record_index_1_based
                    && !selected_records.contains(&(idx + 1))
                    && fleet.standing_order_kind() == nc_data::Order::RendezvousSector
            })
            .map(|(_, fleet)| fleet.standing_order_target_coords_raw())
            .collect::<Vec<_>>();
        self.sort_unique_coords_by_distance(anchor, &mut coords);
        if coords.is_empty() {
            coords.push(anchor);
        }
        coords
    }

    fn filtered_planet_database_targets_from<F>(
        &self,
        anchor: [u8; 2],
        predicate: F,
    ) -> Vec<[u8; 2]>
    where
        F: Fn(&PlayerStarmapWorld) -> bool,
    {
        let mut coords = self
            .player_planet_database_worlds()
            .into_iter()
            .filter(predicate)
            .map(|world| world.coords)
            .collect::<Vec<_>>();
        self.sort_unique_coords_by_distance(anchor, &mut coords);
        coords
    }

    fn player_planet_database_worlds(&self) -> Vec<PlayerStarmapWorld> {
        build_player_starmap_projection_from_snapshots(
            &self.game_data,
            &self.planet_intel_snapshots,
            self.player.record_index_1_based as u8,
        )
        .worlds
    }

    fn coords_are_owned_system(&self, coords: [u8; 2]) -> bool {
        self.game_data.planets.records.iter().any(|planet| {
            planet.coords_raw() == coords
                && planet.owner_empire_slot_raw() as usize == self.player.record_index_1_based
        })
    }

    fn sort_unique_coords_by_distance(&self, anchor: [u8; 2], coords: &mut Vec<[u8; 2]>) {
        coords.sort_by_key(|coords| sector_distance_sq(anchor, *coords));
        coords.dedup();
    }

    fn fleet_order_default_target_y_value(&self) -> Option<u8> {
        let mission_code = self.fleet.order_mission_code?;
        let selected = self
            .fleet_order_selected_row()
            .map(|row| vec![row])
            .unwrap_or_default();
        self.recommended_fleet_target_for_entered_x(
            mission_code,
            &selected,
            BTreeSet::new(),
            self.fleet.order_target_x_input.trim(),
        )
        .map(|coords| coords[1])
    }

    fn fleet_group_default_target_y_value(&self) -> Option<u8> {
        let mission_code = self.fleet.group_mission_code?;
        let selected = self.fleet_group_selected_rows();
        self.recommended_fleet_target_for_entered_x(
            mission_code,
            &selected,
            self.fleet.group_selected_fleets.clone(),
            self.fleet.group_target_x_input.trim(),
        )
        .map(|coords| coords[1])
    }

    fn recommended_fleet_target_for_entered_x(
        &self,
        mission_code: u8,
        selected_rows: &[FleetRow],
        selected_records: BTreeSet<usize>,
        entered_x: &str,
    ) -> Option<[u8; 2]> {
        let candidates =
            self.recommended_fleet_target_candidates(mission_code, selected_rows, selected_records);
        if !fleet_order_target_y_depends_on_entered_x(mission_code) {
            return candidates.into_iter().next();
        }
        let entered_x = entered_x.trim();
        if entered_x.is_empty() {
            return candidates.into_iter().next();
        }
        let target_x = entered_x.parse::<u8>().ok()?;
        candidates.into_iter().find(|coords| coords[0] == target_x)
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
        fleet_record_supports_mission_code(fleet, order_code)
    }

    pub(crate) fn fleet_mission_picker_enabled_flags(&self) -> Vec<bool> {
        match self.fleet.mission_picker_caller {
            Some(FleetMissionPickerCaller::SingleOrderReturnToOrder)
            | Some(FleetMissionPickerCaller::SingleOrderReturnToMenu) => {
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
        self.fleet.group_mode = FleetGroupOrderMode::SelectingFleets;
        self.fleet.group_mission_code = None;
        self.clear_fleet_group_target_inputs();
        self.fleet.group_selected_fleets.clear();
        self.current_screen = ScreenId::FleetGroupOrder;
        self.fleet.group_status = None;
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
        self.fleet.group_mode = FleetGroupOrderMode::SelectingFleets;
        self.fleet.group_mission_code = None;
        self.clear_fleet_group_target_inputs();
        self.fleet.group_selected_fleets.clear();
        self.current_screen = ScreenId::FleetGroupOrder;
        self.fleet.group_status = None;
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
            self.open_fleet_menu_prompt(
                FleetMenuPromptMode::Order,
                self.strongest_owned_fleet_number()
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            );
            self.fleet.menu_prompt_status = Some(PromptFeedback::error(
                "Selected fleet is no longer available.",
            ));
            return Ok(());
        };
        self.apply_fleet_orders_to_rows(&[selected_row.clone()], mission_code, target, aux0, aux1)?;
        self.fleet.order_mode = FleetSingleOrderMode::EnteringTarget;
        self.fleet.order_mission_code = None;
        self.clear_fleet_order_target_inputs();
        self.fleet.order_fleet_record_index_1_based = Some(selected_row.fleet_record_index_1_based);
        self.fleet.order_return_to_menu = false;
        self.show_command_menu_notice(
            CommandMenu::Fleet,
            if fleet_group_order_requires_target(mission_code) {
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
            },
        );
        Ok(())
    }

    fn apply_fleet_single_join_order(
        &mut self,
        host: FleetRow,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(selected_row) = self.fleet_order_selected_row() else {
            self.open_fleet_menu_prompt(
                FleetMenuPromptMode::Order,
                self.strongest_owned_fleet_number()
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            );
            self.fleet.menu_prompt_status = Some(PromptFeedback::error(
                "Selected fleet is no longer available.",
            ));
            return Ok(());
        };
        self.game_data.set_join_fleet_order(
            self.player.record_index_1_based,
            selected_row.fleet_record_index_1_based,
            host.fleet_record_index_1_based,
        )?;
        self.save_game_data()?;
        self.fleet.order_mode = FleetSingleOrderMode::EnteringTarget;
        self.fleet.order_mission_code = None;
        self.clear_fleet_order_target_inputs();
        self.fleet.order_fleet_record_index_1_based = Some(selected_row.fleet_record_index_1_based);
        self.fleet.order_return_to_menu = false;
        self.show_command_menu_notice(
            CommandMenu::Fleet,
            format!(
                "Applied join-fleet order to Fleet #{} with host Fleet #{}.",
                selected_row.fleet_number, host.fleet_number
            ),
        );
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

fn fleet_group_order_requires_target(_order_code: u8) -> bool {
    true
}

fn fleet_mission_requires_preselected_target(order_code: u8) -> bool {
    matches!(order_code, 4)
}

fn fleet_order_target_requires_planet_system(order_code: u8) -> bool {
    matches!(order_code, 2 | 5 | 6 | 7 | 8 | 9 | 11 | 12 | 15)
}

fn fleet_order_target_rejects_owned_planet(order_code: u8) -> bool {
    matches!(order_code, 6 | 7 | 8)
}

fn fleet_order_target_rejects_owned_scout_target(order_code: u8) -> bool {
    matches!(order_code, 10 | 11)
}

fn fleet_order_target_requires_owned_planet(order_code: u8) -> bool {
    matches!(order_code, 2 | 15)
}

fn fleet_order_target_y_depends_on_entered_x(order_code: u8) -> bool {
    matches!(
        fleet_target_input_kind(Some(order_code)),
        FleetTargetInputKind::Coordinates
    )
}

fn fleet_group_order_label(order_code: u8) -> &'static str {
    match nc_data::Order::from_raw(order_code) {
        nc_data::Order::HoldPosition => "hold",
        nc_data::Order::MoveOnly => "move",
        nc_data::Order::SeekHome => "seek-home",
        nc_data::Order::PatrolSector => "patrol",
        nc_data::Order::GuardBlockadeWorld => "guard/blockade",
        nc_data::Order::BombardWorld => "bombard",
        nc_data::Order::InvadeWorld => "invade",
        nc_data::Order::BlitzWorld => "blitz",
        nc_data::Order::ViewWorld => "view",
        nc_data::Order::ScoutSector => "scout-sector",
        nc_data::Order::ScoutSolarSystem => "scout-system",
        nc_data::Order::ColonizeWorld => "colonize",
        nc_data::Order::RendezvousSector => "rendezvous",
        nc_data::Order::Salvage => "salvage",
        nc_data::Order::GuardStarbase => "guard-starbase",
        nc_data::Order::JoinAnotherFleet => "join-fleet",
        nc_data::Order::Unknown(_) => "unknown",
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
