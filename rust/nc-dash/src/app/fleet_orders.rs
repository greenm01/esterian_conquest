use std::collections::{BTreeMap, BTreeSet};

use nc_data::{Order, PlanetIntelSnapshot, map_size_for_player_count};
use nc_engine::{
    FLEET_MISSION_OPTIONS, default_host_fleet_target, default_starbase_target,
    fleet_mission_option, fleet_order_target_rejects_owned_planet,
    fleet_order_target_rejects_owned_scout_target, fleet_order_target_requires_owned_planet,
    fleet_order_target_requires_planet_system, fleet_record_supports_mission_code,
    fleet_target_input_kind, fleet_target_status_line, format_guard_fleet_clause,
    guard_fleet_numbers_for_starbase, owned_fleet_targets, owned_starbase_targets,
    recommended_coordinate_target, recommended_coordinate_target_y_for_entered_x,
    target_available_for_mission,
};

use crate::overlays::fleet_list;

use super::state::{DashApp, FleetOverlayPromptMode, FleetOverlayRowKey, HelpContext};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OrderFleetRow {
    pub fleet_record_index_1_based: usize,
    pub fleet_number: u16,
    pub coords: [u8; 2],
    pub target_coords: [u8; 2],
    pub order_code: u8,
    pub current_speed: u8,
    pub max_speed: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OrderStarbaseRow {
    pub base_record_index_1_based: usize,
    pub base_id: u8,
    pub coords: [u8; 2],
    pub destination_coords: [u8; 2],
}

impl DashApp {
    fn order_intel_snapshots(&self) -> BTreeMap<usize, PlanetIntelSnapshot> {
        self.planet_intel_snapshots
            .iter()
            .cloned()
            .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
            .collect()
    }

    pub(crate) fn open_selected_fleet_order_flow(&mut self) {
        let rows = fleet_list::table_rows(self);
        let selected = self
            .fleet_overlay
            .selected
            .min(rows.len().saturating_sub(1));
        let Some(row) = rows.get(selected) else {
            return;
        };
        self.fleet_overlay.active_row_key = Some(match row.key {
            FleetOverlayRowKey::Fleet(idx) => FleetOverlayRowKey::Fleet(idx),
            FleetOverlayRowKey::Starbase(idx) => FleetOverlayRowKey::Starbase(idx),
        });
        self.fleet_overlay.order_status = None;
        self.fleet_overlay.order_input.clear();
        self.fleet_overlay.order_target_x_input.clear();
        self.fleet_overlay.order_target_y_input.clear();
        self.fleet_overlay.order_confirm_input.clear();
        self.fleet_overlay.mission_picker_input.clear();
        self.fleet_overlay.mission_picker_status = None;
        self.fleet_overlay.starbase_move_input.clear();
        self.fleet_overlay.starbase_move_status = None;
        match row.key {
            FleetOverlayRowKey::Fleet(_) => {
                self.fleet_overlay.order_mission_code = None;
                self.fleet_overlay.prompt_mode = FleetOverlayPromptMode::MissionPicker;
                self.fleet_overlay.mission_picker_cursor =
                    self.first_enabled_fleet_mission_index().unwrap_or(0);
                self.help_context = HelpContext::FleetMissionPicker;
            }
            FleetOverlayRowKey::Starbase(_) => {
                self.fleet_overlay.prompt_mode = FleetOverlayPromptMode::StarbaseMoveDecision;
                self.help_context = HelpContext::StarbaseMove;
            }
        }
    }

    pub(crate) fn append_fleet_mission_picker_char(&mut self, ch: char) {
        if self.fleet_overlay.mission_picker_input.len() >= 2 {
            return;
        }
        self.fleet_overlay.mission_picker_input.push(ch);
        self.sync_fleet_mission_picker_cursor_to_input();
        self.fleet_overlay.mission_picker_status = None;
    }

    pub(crate) fn backspace_fleet_mission_picker_input(&mut self) {
        self.fleet_overlay.mission_picker_input.pop();
        self.sync_fleet_mission_picker_cursor_to_input();
        self.fleet_overlay.mission_picker_status = None;
    }

    pub(crate) fn move_fleet_mission_picker(&mut self, delta: i8) {
        let enabled = self.fleet_mission_picker_enabled_flags();
        if !enabled.iter().any(|flag| *flag) {
            self.fleet_overlay.mission_picker_status =
                Some("No missions are available for the selected fleet.".to_string());
            return;
        }
        let total = FLEET_MISSION_OPTIONS.len();
        let mut next = self.fleet_overlay.mission_picker_cursor as isize;
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
        self.fleet_overlay.mission_picker_cursor = next as usize;
        self.fleet_overlay.mission_picker_input.clear();
        self.fleet_overlay.mission_picker_status = None;
    }

    pub(crate) fn submit_fleet_mission_picker(&mut self) {
        let mission_code = match self.fleet_overlay.mission_picker_input.trim() {
            "" => FLEET_MISSION_OPTIONS
                .get(self.fleet_overlay.mission_picker_cursor)
                .map(|option| option.code)
                .unwrap_or(1),
            raw => match raw.parse::<u8>() {
                Ok(value) => value,
                Err(_) => {
                    self.fleet_overlay.mission_picker_status =
                        Some("Enter a mission number from 0 to 15.".to_string());
                    return;
                }
            },
        };
        let Some(index) = FLEET_MISSION_OPTIONS
            .iter()
            .position(|option| option.code == mission_code)
        else {
            self.fleet_overlay.mission_picker_status =
                Some("Enter a mission number from 0 to 15.".to_string());
            return;
        };
        if !self
            .fleet_mission_picker_enabled_flags()
            .get(index)
            .copied()
            .unwrap_or(false)
        {
            self.fleet_overlay.mission_picker_status =
                Some("That mission does not apply to the selected fleet.".to_string());
            return;
        }
        let snapshots = self.order_intel_snapshots();
        let anchor = self
            .selected_fleet_order_row()
            .map(|row| row.coords)
            .unwrap_or([self.crosshair_x, self.crosshair_y]);
        let selected_records = self
            .selected_fleet_order_row()
            .map(|row| BTreeSet::from([row.fleet_record_index_1_based]))
            .unwrap_or_default();
        if !target_available_for_mission(
            &self.game_data,
            &snapshots,
            self.player_record_index_1_based as u8,
            mission_code,
            anchor,
            &selected_records,
        ) {
            self.fleet_overlay.mission_picker_status =
                Some("That mission does not have a valid target.".to_string());
            return;
        }
        self.fleet_overlay.mission_picker_cursor = index;
        self.fleet_overlay.mission_picker_input.clear();
        self.fleet_overlay.mission_picker_status = None;
        self.fleet_overlay.order_mission_code = Some(mission_code);
        self.fleet_overlay.order_input.clear();
        self.fleet_overlay.order_target_x_input.clear();
        self.fleet_overlay.order_target_y_input.clear();
        self.fleet_overlay.order_confirm_input.clear();
        self.fleet_overlay.prompt_mode = match fleet_target_input_kind(Some(mission_code)) {
            nc_engine::FleetTargetInputKind::Coordinates => FleetOverlayPromptMode::OrderTargetX,
            nc_engine::FleetTargetInputKind::StarbaseId
            | nc_engine::FleetTargetInputKind::FleetId
            | nc_engine::FleetTargetInputKind::None => FleetOverlayPromptMode::OrderTarget,
        };
        self.help_context = HelpContext::FleetOrderInput;
    }

    pub(crate) fn cancel_fleet_order_input(&mut self) {
        self.fleet_overlay.order_status = None;
        match self.fleet_overlay.prompt_mode {
            FleetOverlayPromptMode::MissionPicker => self.close_fleet_order_overlay(),
            FleetOverlayPromptMode::OrderTarget | FleetOverlayPromptMode::OrderTargetX => {
                self.fleet_overlay.prompt_mode = FleetOverlayPromptMode::MissionPicker;
                self.help_context = HelpContext::FleetMissionPicker;
            }
            FleetOverlayPromptMode::OrderTargetY => {
                self.fleet_overlay.prompt_mode = FleetOverlayPromptMode::OrderTargetX;
            }
            FleetOverlayPromptMode::OrderConfirm => {
                self.fleet_overlay.order_confirm_input.clear();
                self.fleet_overlay.prompt_mode = FleetOverlayPromptMode::OrderTargetY;
            }
            FleetOverlayPromptMode::StarbaseMoveDecision => self.close_fleet_order_overlay(),
            FleetOverlayPromptMode::StarbaseMoveDestination
            | FleetOverlayPromptMode::StarbaseHaltConfirm => {
                self.fleet_overlay.prompt_mode = FleetOverlayPromptMode::StarbaseMoveDecision;
                self.help_context = HelpContext::StarbaseMove;
            }
            FleetOverlayPromptMode::FilterMenu
            | FleetOverlayPromptMode::SortMenu
            | FleetOverlayPromptMode::None => {}
        }
    }

    pub(crate) fn append_fleet_order_char(&mut self, ch: char) {
        match self.fleet_overlay.prompt_mode {
            FleetOverlayPromptMode::OrderTarget => {
                if self.fleet_overlay.order_input.len() < 16 {
                    self.fleet_overlay.order_input.push(ch);
                    self.fleet_overlay.order_status = None;
                }
            }
            FleetOverlayPromptMode::OrderTargetX => {
                if self.fleet_overlay.order_target_x_input.len() < 2 {
                    self.fleet_overlay.order_target_x_input.push(ch);
                    self.fleet_overlay.order_status = None;
                }
            }
            FleetOverlayPromptMode::OrderTargetY => {
                if self.fleet_overlay.order_target_y_input.len() < 2 {
                    self.fleet_overlay.order_target_y_input.push(ch);
                    self.fleet_overlay.order_status = None;
                }
            }
            FleetOverlayPromptMode::OrderConfirm => {
                if self.fleet_overlay.order_confirm_input.is_empty() {
                    self.fleet_overlay
                        .order_confirm_input
                        .push(ch.to_ascii_uppercase());
                    self.fleet_overlay.order_status = None;
                }
            }
            FleetOverlayPromptMode::StarbaseMoveDecision
            | FleetOverlayPromptMode::StarbaseMoveDestination
            | FleetOverlayPromptMode::StarbaseHaltConfirm
            | FleetOverlayPromptMode::MissionPicker
            | FleetOverlayPromptMode::FilterMenu
            | FleetOverlayPromptMode::SortMenu
            | FleetOverlayPromptMode::None => {}
        }
    }

    pub(crate) fn backspace_fleet_order_input(&mut self) {
        match self.fleet_overlay.prompt_mode {
            FleetOverlayPromptMode::OrderTarget => {
                self.fleet_overlay.order_input.pop();
            }
            FleetOverlayPromptMode::OrderTargetX => {
                self.fleet_overlay.order_target_x_input.pop();
            }
            FleetOverlayPromptMode::OrderTargetY => {
                self.fleet_overlay.order_target_y_input.pop();
            }
            FleetOverlayPromptMode::OrderConfirm => {
                self.fleet_overlay.order_confirm_input.pop();
            }
            _ => {}
        }
        self.fleet_overlay.order_status = None;
    }

    pub(crate) fn submit_fleet_order(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(mission_code) = self.fleet_overlay.order_mission_code else {
            self.fleet_overlay.order_status = Some("Choose a fleet mission first.".to_string());
            return Ok(());
        };
        match self.fleet_overlay.prompt_mode {
            FleetOverlayPromptMode::OrderTarget => {
                let (destination, aux0, aux1) = match fleet_target_input_kind(Some(mission_code)) {
                    nc_engine::FleetTargetInputKind::Coordinates => {
                        self.fleet_overlay.prompt_mode = FleetOverlayPromptMode::OrderTargetX;
                        return Ok(());
                    }
                    nc_engine::FleetTargetInputKind::StarbaseId => {
                        let Some(base) = self.resolve_fleet_order_starbase_target_for_current_mission() else {
                            self.fleet_overlay.order_status =
                                Some("Enter a starbase number from your starbase list.".to_string());
                            return Ok(());
                        };
                        (base.coords, base.base_id, 1)
                    }
                    nc_engine::FleetTargetInputKind::FleetId => {
                        let Some(host) = self.resolve_fleet_order_host_fleet_for_current_mission() else {
                            self.fleet_overlay.order_status =
                                Some("Enter another fleet number from your fleet list.".to_string());
                            return Ok(());
                        };
                        return self.apply_fleet_single_join_order(host);
                    }
                    nc_engine::FleetTargetInputKind::None => ([0, 0], 0, 0),
                };
                if let Err(err) =
                    self.validate_fleet_target_for_mission(mission_code, destination)
                {
                    self.fleet_overlay.order_status = Some(err);
                    return Ok(());
                }
                self.apply_fleet_single_order(mission_code, destination, aux0, aux1)
            }
            FleetOverlayPromptMode::OrderTargetX => {
                let default = self.fleet_order_target_x_default_value().parse::<u8>().ok();
                match self.resolve_target_axis_input(&self.fleet_overlay.order_target_x_input, default, "XX") {
                    Ok(value) => {
                        if self.fleet_overlay.order_target_x_input.trim().is_empty() {
                            self.fleet_overlay.order_target_x_input = format!("{value:02}");
                        }
                        self.fleet_overlay.prompt_mode = FleetOverlayPromptMode::OrderTargetY;
                        self.fleet_overlay.order_status = None;
                    }
                    Err(err) => self.fleet_overlay.order_status = Some(err),
                }
                Ok(())
            }
            FleetOverlayPromptMode::OrderTargetY => {
                let destination = match self.resolve_fleet_order_split_target() {
                    Ok(coords) => coords,
                    Err(err) => {
                        self.fleet_overlay.prompt_mode = FleetOverlayPromptMode::OrderTargetX;
                        self.fleet_overlay.order_status = Some(err);
                        return Ok(());
                    }
                };
                if self.fleet_overlay.order_target_y_input.trim().is_empty()
                    && let Some(default_y) = self.fleet_order_default_target_y_value()
                {
                    self.fleet_overlay.order_target_y_input = format!("{default_y:02}");
                }
                if let Err(err) = self.validate_fleet_target_for_mission(mission_code, destination) {
                    self.fleet_overlay.prompt_mode = FleetOverlayPromptMode::OrderTargetX;
                    self.fleet_overlay.order_status = Some(err);
                    return Ok(());
                }
                self.fleet_overlay.order_confirm_input.clear();
                self.fleet_overlay.prompt_mode = FleetOverlayPromptMode::OrderConfirm;
                self.fleet_overlay.order_status = None;
                Ok(())
            }
            FleetOverlayPromptMode::OrderConfirm => {
                if !resolve_yes_no_input(&self.fleet_overlay.order_confirm_input, true) {
                    self.fleet_overlay.prompt_mode = FleetOverlayPromptMode::MissionPicker;
                    self.fleet_overlay.order_mission_code = None;
                    self.fleet_overlay.order_input.clear();
                    self.fleet_overlay.order_target_x_input.clear();
                    self.fleet_overlay.order_target_y_input.clear();
                    self.fleet_overlay.order_confirm_input.clear();
                    self.help_context = HelpContext::FleetMissionPicker;
                    return Ok(());
                }
                let destination = match self.resolve_fleet_order_split_target() {
                    Ok(coords) => coords,
                    Err(err) => {
                        self.fleet_overlay.prompt_mode = FleetOverlayPromptMode::OrderTargetX;
                        self.fleet_overlay.order_status = Some(err);
                        return Ok(());
                    }
                };
                if let Err(err) = self.validate_fleet_target_for_mission(mission_code, destination) {
                    self.fleet_overlay.prompt_mode = FleetOverlayPromptMode::OrderTargetX;
                    self.fleet_overlay.order_status = Some(err);
                    return Ok(());
                }
                self.apply_fleet_single_order(mission_code, destination, 0, 0)
            }
            _ => Ok(()),
        }
    }

    pub(crate) fn append_starbase_move_char(&mut self, ch: char) {
        let max_len = match self.fleet_overlay.prompt_mode {
            FleetOverlayPromptMode::StarbaseMoveDecision => 1,
            FleetOverlayPromptMode::StarbaseMoveDestination => 16,
            FleetOverlayPromptMode::StarbaseHaltConfirm => 1,
            _ => 0,
        };
        if max_len > 0 && self.fleet_overlay.starbase_move_input.len() < max_len {
            self.fleet_overlay.starbase_move_input.push(ch);
            self.fleet_overlay.starbase_move_status = None;
        }
    }

    pub(crate) fn backspace_starbase_move_input(&mut self) {
        self.fleet_overlay.starbase_move_input.pop();
        self.fleet_overlay.starbase_move_status = None;
    }

    pub(crate) fn submit_starbase_move(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(row) = self.selected_starbase_move_row() else {
            self.close_fleet_order_overlay();
            return Ok(());
        };
        match self.fleet_overlay.prompt_mode {
            FleetOverlayPromptMode::StarbaseMoveDecision => {
                let raw = self.fleet_overlay.starbase_move_input.trim();
                let choice = raw
                    .chars()
                    .next()
                    .map(|ch| ch.to_ascii_uppercase())
                    .unwrap_or('M');
                match choice {
                    'H' => {
                        self.fleet_overlay.starbase_move_input.clear();
                        self.fleet_overlay.starbase_move_status = None;
                        self.fleet_overlay.prompt_mode = FleetOverlayPromptMode::StarbaseHaltConfirm;
                    }
                    'M' => {
                        self.fleet_overlay.starbase_move_input.clear();
                        self.fleet_overlay.starbase_move_status = None;
                        self.fleet_overlay.prompt_mode =
                            FleetOverlayPromptMode::StarbaseMoveDestination;
                    }
                    _ => {
                        self.fleet_overlay.starbase_move_status = Some("Choose H or M.".to_string());
                    }
                }
                Ok(())
            }
            FleetOverlayPromptMode::StarbaseMoveDestination => {
                let destination = resolve_default_coords_input(
                    &self.fleet_overlay.starbase_move_input,
                    row.destination_coords,
                )
                .ok_or_else(|| "Enter coordinates like 10,13".to_string())?;
                let map_size = map_size_for_player_count(self.game_data.conquest.player_count());
                if destination[0] == 0
                    || destination[1] == 0
                    || destination[0] > map_size
                    || destination[1] > map_size
                {
                    self.fleet_overlay.starbase_move_status =
                        Some(format!("Enter coordinates within 1..{map_size}"));
                    return Ok(());
                }
                self.finalize_starbase_destination(row, destination)
            }
            FleetOverlayPromptMode::StarbaseHaltConfirm => {
                if !resolve_yes_no_input(&self.fleet_overlay.starbase_move_input, true) {
                    self.fleet_overlay.starbase_move_input.clear();
                    self.fleet_overlay.prompt_mode = FleetOverlayPromptMode::StarbaseMoveDecision;
                    return Ok(());
                }
                self.finalize_starbase_destination(row, row.coords)
            }
            _ => Ok(()),
        }
    }

    pub(crate) fn close_fleet_order_overlay(&mut self) {
        self.fleet_overlay.prompt_mode = FleetOverlayPromptMode::None;
        self.fleet_overlay.active_row_key = None;
        self.fleet_overlay.mission_picker_input.clear();
        self.fleet_overlay.mission_picker_status = None;
        self.fleet_overlay.order_mission_code = None;
        self.fleet_overlay.order_status = None;
        self.fleet_overlay.order_input.clear();
        self.fleet_overlay.order_target_x_input.clear();
        self.fleet_overlay.order_target_y_input.clear();
        self.fleet_overlay.order_confirm_input.clear();
        self.fleet_overlay.starbase_move_input.clear();
        self.fleet_overlay.starbase_move_status = None;
        self.help_context = HelpContext::FleetList;
    }

    pub(crate) fn fleet_mission_picker_enabled_flags(&self) -> Vec<bool> {
        let Some(row) = self.selected_fleet_order_row() else {
            return vec![false; FLEET_MISSION_OPTIONS.len()];
        };
        let Some(fleet) = self
            .game_data
            .fleets
            .records
            .get(row.fleet_record_index_1_based.saturating_sub(1))
        else {
            return vec![false; FLEET_MISSION_OPTIONS.len()];
        };
        FLEET_MISSION_OPTIONS
            .iter()
            .map(|option| fleet_record_supports_mission_code(fleet, option.code))
            .collect()
    }

    pub(crate) fn fleet_order_target_status_line(&self) -> String {
        if self.fleet_overlay.prompt_mode == FleetOverlayPromptMode::OrderConfirm
            && let (Some(_mission_code), Ok(destination)) = (
                self.fleet_overlay.order_mission_code,
                self.resolve_fleet_order_split_target(),
            )
        {
            return format!("Confirm [{:02},{:02}] for {}.", destination[0], destination[1], self.fleet_order_new_order_label());
        }
        fleet_target_status_line(self.fleet_overlay.order_mission_code)
    }

    pub(crate) fn fleet_order_target_prompt(&self) -> String {
        match fleet_target_input_kind(self.fleet_overlay.order_mission_code) {
            nc_engine::FleetTargetInputKind::StarbaseId => "Starbase # ".to_string(),
            nc_engine::FleetTargetInputKind::FleetId => "Fleet # ".to_string(),
            nc_engine::FleetTargetInputKind::Coordinates | nc_engine::FleetTargetInputKind::None => {
                "Target ".to_string()
            }
        }
    }

    pub(crate) fn fleet_order_target_default_value(&self) -> String {
        match fleet_target_input_kind(self.fleet_overlay.order_mission_code) {
            nc_engine::FleetTargetInputKind::StarbaseId => self
                .fleet_order_default_starbase()
                .map(|row| row.base_id.to_string())
                .unwrap_or_else(|| "1".to_string()),
            nc_engine::FleetTargetInputKind::FleetId => self
                .fleet_order_default_host_fleet()
                .map(|row| row.fleet_number.to_string())
                .unwrap_or_else(|| "1".to_string()),
            nc_engine::FleetTargetInputKind::Coordinates | nc_engine::FleetTargetInputKind::None => self
                .fleet_order_default_target_coords()
                .map(|target| format!("{},{}", target[0], target[1]))
                .unwrap_or_default(),
        }
    }

    pub(crate) fn fleet_order_target_x_default_value(&self) -> String {
        self.fleet_order_default_target_coords()
            .map(|coords| format!("{:02}", coords[0]))
            .unwrap_or_default()
    }

    pub(crate) fn fleet_order_target_y_default_value(&self) -> String {
        self.fleet_order_default_target_y_value()
            .map(|value| format!("{value:02}"))
            .unwrap_or_default()
    }

    pub(crate) fn fleet_order_current_order_label(&self) -> String {
        self.selected_fleet_order_row()
            .and_then(|row| fleet_mission_option(row.order_code))
            .map(|option| option.mission.to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    pub(crate) fn fleet_order_new_order_label(&self) -> String {
        self.fleet_overlay
            .order_mission_code
            .and_then(fleet_mission_option)
            .map(|option| option.mission.to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    pub(crate) fn selected_fleet_order_row(&self) -> Option<OrderFleetRow> {
        match self.fleet_overlay.active_row_key {
            Some(FleetOverlayRowKey::Fleet(record_index)) => self
                .owned_fleet_rows_for_orders()
                .into_iter()
                .find(|row| row.fleet_record_index_1_based == record_index),
            _ => None,
        }
    }

    pub(crate) fn selected_starbase_move_row(&self) -> Option<OrderStarbaseRow> {
        match self.fleet_overlay.active_row_key {
            Some(FleetOverlayRowKey::Starbase(record_index)) => self
                .owned_starbase_rows_for_orders()
                .into_iter()
                .find(|row| row.base_record_index_1_based == record_index),
            _ => None,
        }
    }

    fn first_enabled_fleet_mission_index(&self) -> Option<usize> {
        self.fleet_mission_picker_enabled_flags()
            .iter()
            .position(|flag| *flag)
    }

    fn sync_fleet_mission_picker_cursor_to_input(&mut self) {
        let rows = FLEET_MISSION_OPTIONS
            .iter()
            .map(|option| vec![format!("{:02}", option.code)])
            .collect::<Vec<_>>();
        let Some(index) = nc_ui::table_selection::find_typed_jump_index(
            &rows,
            0,
            &self.fleet_overlay.mission_picker_input,
        ) else {
            return;
        };
        if self
            .fleet_mission_picker_enabled_flags()
            .get(index)
            .copied()
            .unwrap_or(false)
        {
            self.fleet_overlay.mission_picker_cursor = index;
        }
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
        let default_x = self.fleet_order_default_target_coords().map(|coords| coords[0]);
        let default_y = self.fleet_order_default_target_y_value();
        Ok([
            self.resolve_target_axis_input(&self.fleet_overlay.order_target_x_input, default_x, "XX")?,
            self.resolve_target_axis_input(&self.fleet_overlay.order_target_y_input, default_y, "YY")?,
        ])
    }

    fn validate_fleet_target_for_mission(
        &self,
        mission_code: u8,
        destination: [u8; 2],
    ) -> Result<(), String> {
        let target_planet = self
            .game_data
            .planets
            .records
            .iter()
            .find(|planet| planet.coords_raw() == destination);
        if fleet_order_target_requires_planet_system(mission_code) && target_planet.is_none() {
            return Err("That mission needs a system with a planet at the target.".to_string());
        }
        if fleet_order_target_rejects_owned_planet(mission_code)
            && target_planet
                .map(|planet| planet.owner_empire_slot_raw() as usize == self.player_record_index_1_based)
                .unwrap_or(false)
        {
            return Err("You cannot send that mission to your own world.".to_string());
        }
        if fleet_order_target_rejects_owned_scout_target(mission_code)
            && target_planet
                .map(|planet| planet.owner_empire_slot_raw() as usize == self.player_record_index_1_based)
                .unwrap_or(false)
        {
            return Err("You cannot scout your own planet or system.".to_string());
        }
        if fleet_order_target_requires_owned_planet(mission_code)
            && target_planet
                .map(|planet| planet.owner_empire_slot_raw() as usize != self.player_record_index_1_based)
                .unwrap_or(true)
        {
            return Err("That mission requires one of your owned planets.".to_string());
        }
        if mission_code == Order::ColonizeWorld.to_raw() {
            let Some(row) = self.selected_fleet_order_row() else {
                return Err("Selected fleet is no longer available.".to_string());
            };
            use std::collections::BTreeSet;
            self.game_data
                .validate_friendly_colonize_target_available(
                    self.player_record_index_1_based as u8,
                    destination,
                    &BTreeSet::from([row.fleet_record_index_1_based]),
                )
                .map_err(|err| match err {
                    nc_data::FleetOrderValidationError::DuplicateFriendlyColonizeTarget { .. } => {
                        "Another one of your ETAC fleets is already ordered to colonize that world."
                            .to_string()
                    }
                    other => other.to_string(),
                })?;
        }
        Ok(())
    }

    fn apply_fleet_single_order(
        &mut self,
        mission_code: u8,
        target: [u8; 2],
        aux0: u8,
        aux1: u8,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(selected_row) = self.selected_fleet_order_row() else {
            self.fleet_overlay.order_status = Some("Selected fleet is no longer available.".to_string());
            return Ok(());
        };
        let speed = self
            .game_data
            .fleets
            .records
            .get(selected_row.fleet_record_index_1_based.saturating_sub(1))
            .map(|fleet| {
                let speed = fleet.current_speed();
                if speed == 0 { fleet.max_speed() } else { speed }
            })
            .unwrap_or(selected_row.current_speed);
        self.game_data.set_fleet_order(
            selected_row.fleet_record_index_1_based,
            speed,
            mission_code,
            target,
            Some(aux0),
            Some(aux1),
        )?;
        self.save_and_refresh_runtime()?;
        self.reselect_fleet_overlay_row(FleetOverlayRowKey::Fleet(
            selected_row.fleet_record_index_1_based,
        ));
        self.close_fleet_order_overlay();
        Ok(())
    }

    fn apply_fleet_single_join_order(
        &mut self,
        host: OrderFleetRow,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(selected_row) = self.selected_fleet_order_row() else {
            self.fleet_overlay.order_status = Some("Selected fleet is no longer available.".to_string());
            return Ok(());
        };
        self.game_data.set_join_fleet_order(
            self.player_record_index_1_based,
            selected_row.fleet_record_index_1_based,
            host.fleet_record_index_1_based,
        )?;
        self.save_and_refresh_runtime()?;
        self.reselect_fleet_overlay_row(FleetOverlayRowKey::Fleet(
            selected_row.fleet_record_index_1_based,
        ));
        self.close_fleet_order_overlay();
        Ok(())
    }

    fn resolve_fleet_order_starbase_target_for_current_mission(&self) -> Option<OrderStarbaseRow> {
        let default_base_id = self.fleet_order_default_starbase()?.base_id;
        let base_id = resolve_default_u8_input(&self.fleet_overlay.order_input, default_base_id)?;
        self.owned_starbase_rows_for_orders()
            .into_iter()
            .find(|row| row.base_id == base_id)
    }

    fn resolve_fleet_order_host_fleet_for_current_mission(&self) -> Option<OrderFleetRow> {
        let default_fleet_number = self.fleet_order_default_host_fleet()?.fleet_number;
        let fleet_number =
            resolve_default_u16_input(&self.fleet_overlay.order_input, default_fleet_number)?;
        let selected_record = self.selected_fleet_order_row()?.fleet_record_index_1_based;
        self.owned_fleet_rows_for_orders().into_iter().find(|row| {
            row.fleet_number == fleet_number && row.fleet_record_index_1_based != selected_record
        })
    }

    fn fleet_order_default_starbase(&self) -> Option<OrderStarbaseRow> {
        let anchor = self.selected_fleet_order_row()?.coords;
        let target = default_starbase_target(
            &self.game_data,
            self.player_record_index_1_based as u8,
            anchor,
        )?;
        self.owned_starbase_rows_for_orders()
            .into_iter()
            .find(|row| row.base_record_index_1_based == target.base_record_index_1_based)
    }

    fn fleet_order_default_host_fleet(&self) -> Option<OrderFleetRow> {
        let selected = self.selected_fleet_order_row()?;
        let excluded = BTreeSet::from([selected.fleet_record_index_1_based]);
        let target = default_host_fleet_target(
            &self.game_data,
            self.player_record_index_1_based as u8,
            selected.coords,
            &excluded,
        )?;
        self.owned_fleet_rows_for_orders()
            .into_iter()
            .find(|row| row.fleet_record_index_1_based == target.fleet_record_index_1_based)
    }

    fn fleet_order_default_target_coords(&self) -> Option<[u8; 2]> {
        let mission_code = self.fleet_overlay.order_mission_code?;
        let selected = self.selected_fleet_order_row()?;
        if selected.target_coords != [0, 0] {
            return Some(selected.target_coords);
        }
        let snapshots = self.order_intel_snapshots();
        let selected_records = BTreeSet::from([selected.fleet_record_index_1_based]);
        recommended_coordinate_target(
            &self.game_data,
            &snapshots,
            self.player_record_index_1_based as u8,
            mission_code,
            selected.coords,
            &selected_records,
        )
    }

    fn fleet_order_default_target_y_value(&self) -> Option<u8> {
        let mission_code = self.fleet_overlay.order_mission_code?;
        let Some(selected) = self.selected_fleet_order_row() else {
            return None;
        };
        let snapshots = self.order_intel_snapshots();
        let selected_records = BTreeSet::from([selected.fleet_record_index_1_based]);
        recommended_coordinate_target_y_for_entered_x(
            &self.game_data,
            &snapshots,
            self.player_record_index_1_based as u8,
            mission_code,
            selected.coords,
            &selected_records,
            self.fleet_overlay.order_target_x_input.trim(),
        )
    }

    fn finalize_starbase_destination(
        &mut self,
        row: OrderStarbaseRow,
        destination: [u8; 2],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if destination == row.coords {
            self.game_data
                .halt_starbase(self.player_record_index_1_based, row.base_record_index_1_based)?;
        } else {
            self.game_data.set_starbase_destination(
                self.player_record_index_1_based,
                row.base_record_index_1_based,
                destination,
            )?;
        }
        self.append_report_block(self.starbase_move_report_text(row, destination));
        self.save_and_refresh_runtime()?;
        self.reselect_fleet_overlay_row(FleetOverlayRowKey::Starbase(
            row.base_record_index_1_based,
        ));
        self.close_fleet_order_overlay();
        Ok(())
    }

    fn starbase_move_report_text(&self, row: OrderStarbaseRow, destination: [u8; 2]) -> String {
        if destination == row.coords {
            return format!(
                "Starbase {} halted at [{:02},{:02}].",
                row.base_id, destination[0], destination[1]
            );
        }
        let mut text = format!(
            "Starbase {} is moving to [{:02},{:02}].",
            row.base_id, destination[0], destination[1]
        );
        let guard_fleets = guard_fleet_numbers_for_starbase(
            &self.game_data,
            self.player_record_index_1_based,
            row.base_id,
        );
        if let Some(clause) = format_guard_fleet_clause(&guard_fleets) {
            text.push(' ');
            text.push_str(&clause);
        }
        text
    }

    fn reselect_fleet_overlay_row(&mut self, key: FleetOverlayRowKey) {
        if let Some(index) = fleet_list::table_rows(self).iter().position(|row| row.key == key) {
            self.fleet_overlay.selected = index;
        }
    }

    fn owned_fleet_rows_for_orders(&self) -> Vec<OrderFleetRow> {
        let mut rows = owned_fleet_targets(&self.game_data, self.player_record_index_1_based as u8)
            .into_iter()
            .map(|row| OrderFleetRow {
                fleet_record_index_1_based: row.fleet_record_index_1_based,
                fleet_number: row.fleet_number,
                coords: row.coords,
                target_coords: row.target_coords,
                order_code: row.order_code,
                current_speed: row.current_speed,
                max_speed: row.max_speed,
            })
            .collect::<Vec<_>>();
        rows.sort_by_key(|row| row.fleet_number);
        rows
    }

    fn owned_starbase_rows_for_orders(&self) -> Vec<OrderStarbaseRow> {
        let mut rows =
            owned_starbase_targets(&self.game_data, self.player_record_index_1_based as u8)
                .into_iter()
                .map(|row| OrderStarbaseRow {
                    base_record_index_1_based: row.base_record_index_1_based,
                    base_id: row.base_id,
                    coords: row.coords,
                    destination_coords: row.destination_coords,
                })
                .collect::<Vec<_>>();
        rows.sort_by_key(|row| row.base_id);
        rows
    }
}

fn resolve_default_u8_input(input: &str, default: u8) -> Option<u8> {
    let raw = input.trim();
    if raw.is_empty() {
        Some(default)
    } else {
        raw.parse::<u8>().ok()
    }
}

fn resolve_default_u16_input(input: &str, default: u16) -> Option<u16> {
    let raw = input.trim();
    if raw.is_empty() {
        Some(default)
    } else {
        raw.parse::<u16>().ok()
    }
}

fn resolve_default_coords_input(input: &str, default: [u8; 2]) -> Option<[u8; 2]> {
    let raw = input.trim();
    if raw.is_empty() {
        return Some(default);
    }
    let Some((left, right)) = raw.split_once(',') else {
        return None;
    };
    let x = left.trim().parse::<u8>().ok()?;
    let y = right.trim().parse::<u8>().ok()?;
    Some([x, y])
}

fn resolve_yes_no_input(input: &str, default: bool) -> bool {
    match input.trim().to_ascii_uppercase().as_str() {
        "" => default,
        "Y" | "YES" => true,
        "N" | "NO" => false,
        _ => default,
    }
}
