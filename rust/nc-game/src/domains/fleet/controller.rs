use super::manip::{fleet_eta_label, fleet_list_eta_label};
use super::orders::{
    FleetTargetInputKind, fleet_target_input_kind, fleet_target_status_line, resolve_yes_no_input,
};
use crate::app::helpers::{
    is_coordinate_input_char, resolve_default_coords_input, sync_scroll_to_cursor,
};
use crate::app::state::App;
use crate::domains::fleet::FleetAction;
use crate::domains::fleet::state::{FleetChangeField, FleetCommandContext, FleetMenuPromptMode};
use crate::screen::layout::PromptFeedback;
use crate::screen::{CommandMenu, FleetEtaMode, FleetRow, PlanetTransportMode, ScreenId};
use nc_data::{FleetRecord, Order};
use std::cmp::Reverse;

fn fleet_strength_key(fleet: &FleetRecord) -> (u16, u16, u16, u16, u8, u16, Reverse<u16>) {
    (
        fleet.battleship_count(),
        fleet.cruiser_count(),
        fleet.destroyer_count(),
        fleet.troop_transport_count(),
        fleet.scout_count(),
        fleet.etac_count(),
        Reverse(fleet.local_slot_word_raw()),
    )
}

fn fleet_combat_only_strength_key(fleet: &FleetRecord) -> (u16, u16, u16, Reverse<u16>) {
    (
        fleet.battleship_count(),
        fleet.cruiser_count(),
        fleet.destroyer_count(),
        Reverse(fleet.local_slot_word_raw()),
    )
}

fn fleet_is_idle_hold(fleet: &FleetRecord) -> bool {
    fleet.standing_order_kind() == Order::HoldPosition
}

fn fleet_is_combat_only_fallback_candidate(fleet: &FleetRecord) -> bool {
    (fleet.destroyer_count() > 0 || fleet.cruiser_count() > 0 || fleet.battleship_count() > 0)
        && fleet.scout_count() == 0
        && fleet.etac_count() == 0
}

impl App {
    fn fleet_list_visible_rows(&self) -> usize {
        crate::domains::fleet::screens::fleet::fleet_list_visible_rows(self.screen_geometry)
    }

    fn fleet_group_visible_rows(&self) -> usize {
        crate::domains::fleet::screens::fleet::fleet_visible_rows(self.screen_geometry)
    }

    pub fn open_fleet_menu(&mut self) {
        self.clear_command_menu_notice();
        self.clear_fleet_menu_prompt();
        self.fleet.command_context = FleetCommandContext::Menu;
        self.fleet.order_return_to_menu = false;
        self.fleet.list_dismiss_message = None;
        self.fleet.dismiss_message = None;
        self.current_screen = ScreenId::FleetMenu;
    }

    pub(crate) fn clear_fleet_menu_prompt(&mut self) {
        self.fleet.menu_prompt_mode = None;
        self.fleet.menu_prompt_input.clear();
        self.fleet.menu_prompt_status = None;
        self.fleet.menu_prompt_default_value.clear();
        self.fleet.menu_prompt_context_fleet_record_index_1_based = None;
        self.fleet.menu_prompt_change_field = None;
    }

    fn clear_fleet_list_input(&mut self) {
        self.fleet.list_input.clear();
        self.fleet.list_status = None;
    }

    pub(crate) fn clear_fleet_list_dismiss_message(&mut self) {
        self.fleet.list_dismiss_message = None;
    }

    pub(crate) fn fleet_context_screen(&self) -> ScreenId {
        match self.fleet.command_context {
            FleetCommandContext::Menu => ScreenId::FleetMenu,
            FleetCommandContext::List => ScreenId::FleetList,
        }
    }

    pub(crate) fn fleet_selected_list_row(&self) -> Result<FleetRow, String> {
        let ScreenId::FleetList = self.current_screen else {
            return Err("Fleet list is not active.".to_string());
        };
        let rows = self.fleet_list_rows();
        if rows.is_empty() {
            return Err("You have no active fleets.".to_string());
        }
        let input = self.fleet.list_input.trim();
        if !input.is_empty() {
            let fleet_number = input
                .parse::<u16>()
                .map_err(|_| "Enter a fleet number from the table.".to_string())?;
            return rows
                .into_iter()
                .find(|row| row.fleet_number == fleet_number)
                .ok_or_else(|| format!("Fleet #{fleet_number} is not in your fleet list."));
        }
        rows.get(self.fleet.cursor)
            .cloned()
            .ok_or_else(|| "You have no active fleets.".to_string())
    }

    pub(crate) fn dismiss_fleet_message(&mut self) {
        if self.fleet.list_dismiss_message.is_some() {
            self.fleet.list_dismiss_message = None;
            self.current_screen = ScreenId::FleetList;
            return;
        }
        self.fleet.dismiss_message = None;
        self.current_screen = self.fleet_context_screen();
    }

    pub(crate) fn show_fleet_list_dismiss_message(&mut self, message: impl Into<String>) {
        self.clear_command_menu_notice();
        self.current_screen = ScreenId::FleetList;
        self.fleet.list_status = None;
        self.fleet.dismiss_message = None;
        self.fleet.list_dismiss_message = Some(message.into());
    }

    pub(crate) fn show_fleet_context_notice(&mut self, message: impl Into<String>) {
        let message = message.into();
        match self.fleet.command_context {
            FleetCommandContext::Menu => {
                self.clear_fleet_menu_prompt();
                self.show_command_menu_notice(CommandMenu::Fleet, message);
            }
            FleetCommandContext::List => {
                self.clear_fleet_menu_prompt();
                self.show_fleet_list_dismiss_message(message);
            }
        }
    }

    pub(crate) fn show_fleet_context_success(
        &mut self,
        message: impl Into<String>,
        visible_in_fleet_list: bool,
    ) {
        if visible_in_fleet_list && self.fleet.command_context == FleetCommandContext::List {
            self.clear_fleet_menu_prompt();
            self.clear_command_menu_notice();
            self.current_screen = ScreenId::FleetList;
            self.fleet.list_status = None;
            self.fleet.list_dismiss_message = None;
            self.fleet.dismiss_message = None;
            return;
        }
        self.show_fleet_context_notice(message);
    }

    pub(crate) fn show_fleet_prompt_feedback(
        &mut self,
        feedback: PromptFeedback,
    ) -> Option<PromptFeedback> {
        if self.current_screen == ScreenId::FleetList
            && self.inline_fleet_menu_prompt_active_on_current_screen()
        {
            self.show_fleet_list_dismiss_message(match &feedback {
                PromptFeedback::Notice(value) => value.clone(),
                PromptFeedback::Error(value) => format!("Error: {value}"),
                PromptFeedback::Warning(value) => format!("Warning: {value}"),
            });
            return None;
        }
        Some(feedback)
    }

    pub(crate) fn strongest_owned_fleet_number(&self) -> Option<u16> {
        self.game_data
            .fleets
            .records
            .iter()
            .filter(|fleet| fleet.owner_empire_raw() as usize == self.player.record_index_1_based)
            .max_by_key(|fleet| fleet_strength_key(fleet))
            .map(|fleet| fleet.local_slot_word_raw())
    }

    pub(crate) fn largest_owned_fleet_number_by_ship_total(&self) -> Option<u16> {
        self.game_data
            .fleets
            .records
            .iter()
            .filter(|fleet| fleet.owner_empire_raw() as usize == self.player.record_index_1_based)
            .max_by_key(|fleet| {
                (
                    u32::from(fleet.battleship_count())
                        + u32::from(fleet.cruiser_count())
                        + u32::from(fleet.destroyer_count())
                        + u32::from(fleet.troop_transport_count())
                        + u32::from(fleet.scout_count())
                        + u32::from(fleet.etac_count()),
                    Reverse(fleet.local_slot_word_raw()),
                )
            })
            .map(|fleet| fleet.local_slot_word_raw())
    }

    pub(crate) fn remember_newly_commissioned_fleet_record(
        &mut self,
        fleet_record_index_1_based: usize,
    ) {
        self.fleet
            .recently_commissioned_fleet_records_mru
            .retain(|idx| *idx != fleet_record_index_1_based);
        self.fleet
            .recently_commissioned_fleet_records_mru
            .insert(0, fleet_record_index_1_based);
    }

    fn prune_recently_commissioned_fleet_records(&mut self) {
        self.fleet
            .recently_commissioned_fleet_records_mru
            .retain(|record_index_1_based| {
                self.game_data
                    .fleets
                    .records
                    .get(*record_index_1_based - 1)
                    .is_some_and(|fleet| {
                        fleet.owner_empire_raw() as usize == self.player.record_index_1_based
                            && fleet_is_idle_hold(fleet)
                    })
            });
    }

    pub(crate) fn order_prompt_default_fleet_number(&mut self) -> Option<u16> {
        self.prune_recently_commissioned_fleet_records();
        let owned = self
            .game_data
            .fleets
            .records
            .iter()
            .filter(|fleet| fleet.owner_empire_raw() as usize == self.player.record_index_1_based)
            .collect::<Vec<_>>();

        if let Some(fleet_number) = self
            .fleet
            .recently_commissioned_fleet_records_mru
            .iter()
            .find_map(|record_index_1_based| {
                self.game_data
                    .fleets
                    .records
                    .get(*record_index_1_based - 1)
                    .map(FleetRecord::local_slot_word_raw)
            })
        {
            return Some(fleet_number);
        }

        let idle_etacs = owned
            .iter()
            .copied()
            .filter(|fleet| fleet_is_idle_hold(fleet) && fleet.etac_count() > 0)
            .collect::<Vec<_>>();
        if let Some(fleet) = idle_etacs
            .into_iter()
            .max_by_key(|fleet| fleet_strength_key(fleet))
        {
            return Some(fleet.local_slot_word_raw());
        }

        let idle_hold = owned
            .iter()
            .copied()
            .filter(|fleet| fleet_is_idle_hold(fleet))
            .collect::<Vec<_>>();
        if let Some(fleet) = idle_hold
            .into_iter()
            .max_by_key(|fleet| fleet_strength_key(fleet))
        {
            return Some(fleet.local_slot_word_raw());
        }

        let fallback_combat = owned
            .iter()
            .copied()
            .filter(|fleet| fleet_is_combat_only_fallback_candidate(fleet))
            .collect::<Vec<_>>();
        if let Some(fleet) = fallback_combat
            .into_iter()
            .max_by_key(|fleet| fleet_combat_only_strength_key(fleet))
        {
            return Some(fleet.local_slot_word_raw());
        }

        owned
            .into_iter()
            .max_by_key(|fleet| fleet_strength_key(fleet))
            .map(|fleet| fleet.local_slot_word_raw())
    }

    pub(crate) fn fleet_menu_prompt_label(&self) -> Option<String> {
        let mode = self.fleet.menu_prompt_mode?;
        Some(match mode {
            FleetMenuPromptMode::Review => "Review Fleet # ".to_string(),
            FleetMenuPromptMode::Order => "Order Fleet # ".to_string(),
            FleetMenuPromptMode::ChangeFleet => "Change Fleet # ".to_string(),
            FleetMenuPromptMode::ChangeField => "Change <R>OE, <I>D, or <S>peed ".to_string(),
            FleetMenuPromptMode::ChangeValue => match self.fleet.menu_prompt_change_field {
                Some(FleetChangeField::Roe) => "New ROE ".to_string(),
                Some(FleetChangeField::Id) => "New Fleet ID ".to_string(),
                Some(FleetChangeField::Speed) => "New Speed ".to_string(),
                None => "New Value ".to_string(),
            },
            FleetMenuPromptMode::EtaFleet => "ETA Fleet # ".to_string(),
            FleetMenuPromptMode::DetachFleet => "Detach Fleet # ".to_string(),
            FleetMenuPromptMode::MergeSource => "Merge Fleet # ".to_string(),
            FleetMenuPromptMode::MergeHost => "Into Fleet # ".to_string(),
            FleetMenuPromptMode::TransferDonor => "Transfer From Fleet # ".to_string(),
            FleetMenuPromptMode::TransferHost => "Transfer To Fleet # ".to_string(),
            FleetMenuPromptMode::TransportFleet(PlanetTransportMode::Load) => {
                "Load Fleet # ".to_string()
            }
            FleetMenuPromptMode::TransportFleet(PlanetTransportMode::Unload) => {
                "Unload Fleet # ".to_string()
            }
            FleetMenuPromptMode::TransportQuantity(PlanetTransportMode::Load) => {
                "How many armies to load? ".to_string()
            }
            FleetMenuPromptMode::TransportQuantity(PlanetTransportMode::Unload) => {
                "How many armies to unload? ".to_string()
            }
        })
    }

    pub(crate) fn open_fleet_menu_prompt(
        &mut self,
        mode: FleetMenuPromptMode,
        default_value: impl Into<String>,
    ) {
        if matches!(
            mode,
            FleetMenuPromptMode::Review
                | FleetMenuPromptMode::Order
                | FleetMenuPromptMode::ChangeFleet
                | FleetMenuPromptMode::EtaFleet
                | FleetMenuPromptMode::DetachFleet
                | FleetMenuPromptMode::MergeSource
                | FleetMenuPromptMode::TransferDonor
                | FleetMenuPromptMode::TransportFleet(_)
        ) && self.fleet_rows().is_empty()
        {
            self.show_command_menu_notice(CommandMenu::Fleet, "You have no active fleets.");
            return;
        }
        self.clear_command_menu_notice();
        self.current_screen = self.fleet_context_screen();
        self.fleet.list_dismiss_message = None;
        self.fleet.menu_prompt_mode = Some(mode);
        self.fleet.menu_prompt_input.clear();
        self.fleet.menu_prompt_status = None;
        self.fleet.menu_prompt_default_value = default_value.into();
    }

    pub fn open_fleet_review_prompt(&mut self) {
        self.open_fleet_menu_prompt(
            FleetMenuPromptMode::Review,
            self.strongest_owned_fleet_number()
                .map(|value| value.to_string())
                .unwrap_or_default(),
        );
    }

    pub fn open_fleet_change_prompt(&mut self) {
        if self.current_screen == ScreenId::FleetList {
            match self.fleet_selected_list_row() {
                Ok(row) => {
                    self.fleet.command_context = FleetCommandContext::List;
                    self.clear_command_menu_notice();
                    self.clear_fleet_list_input();
                    self.clear_fleet_list_dismiss_message();
                    self.fleet.menu_prompt_context_fleet_record_index_1_based =
                        Some(row.fleet_record_index_1_based);
                    self.fleet.menu_prompt_change_field = None;
                    self.fleet.menu_prompt_mode = Some(FleetMenuPromptMode::ChangeField);
                    self.fleet.menu_prompt_input.clear();
                    self.fleet.menu_prompt_status = None;
                    self.fleet.menu_prompt_default_value = "R".to_string();
                    return;
                }
                Err(err) => {
                    let message = if err == "You have no active fleets." {
                        err
                    } else {
                        "Fleet unavailable".to_string()
                    };
                    self.show_fleet_list_dismiss_message(message);
                    return;
                }
            }
        }
        self.fleet.command_context = FleetCommandContext::Menu;
        self.open_fleet_menu_prompt(
            FleetMenuPromptMode::ChangeFleet,
            self.strongest_owned_fleet_number()
                .map(|value| value.to_string())
                .unwrap_or_default(),
        );
    }

    pub(crate) fn inline_fleet_menu_prompt_active_on_current_screen(&self) -> bool {
        matches!(
            self.current_screen,
            ScreenId::FleetMenu | ScreenId::FleetList
        ) && self.fleet.menu_prompt_mode.is_some()
    }

    pub fn open_fleet_list(&mut self) {
        if self.fleet_list_rows().is_empty() {
            self.show_command_menu_notice(CommandMenu::Fleet, "You have no active fleets.");
            return;
        }
        self.clear_command_menu_notice();
        self.clear_fleet_menu_prompt();
        self.fleet.command_context = FleetCommandContext::List;
        self.fleet.dismiss_message = None;
        self.fleet.list_dismiss_message = None;
        self.clear_fleet_list_input();
        self.fleet.scroll_offset = 0;
        self.fleet.cursor = 0;
        self.current_screen = ScreenId::FleetList;
    }

    pub fn open_fleet_review(&mut self) {
        let review_return_to_list = self.current_screen == ScreenId::FleetList;
        let rows = if review_return_to_list {
            self.fleet_list_rows()
        } else {
            self.fleet_rows()
        };
        let total = rows.len();
        if total == 0 {
            self.show_command_menu_notice(CommandMenu::Fleet, "You have no active fleets.");
            return;
        }
        self.clear_command_menu_notice();
        self.clear_fleet_menu_prompt();
        if review_return_to_list && !self.fleet.list_input.trim().is_empty() {
            let fleet_number = match self.fleet.list_input.trim().parse::<u16>() {
                Ok(value) => value,
                Err(_) => {
                    self.fleet.list_status =
                        Some("Enter a fleet number from the table.".to_string());
                    return;
                }
            };
            let Some(index) = rows.iter().position(|row| row.fleet_number == fleet_number) else {
                self.fleet.list_status =
                    Some(format!("Fleet #{fleet_number} is not in your fleet list."));
                return;
            };
            self.fleet.cursor = index;
        }
        self.clear_fleet_list_input();
        self.fleet.review_return_to_list = review_return_to_list;
        self.fleet.review_index = self.fleet.cursor.min(total - 1);
        self.current_screen = ScreenId::FleetReview;
    }

    pub fn close_fleet_review(&mut self) {
        if self.current_screen != ScreenId::FleetReview {
            return;
        }
        let total = self.fleet_review_rows().len();
        if total == 0 {
            self.open_fleet_menu();
            return;
        }
        self.fleet.cursor = self.fleet.review_index.min(total - 1);
        if self.fleet.review_return_to_list {
            let visible_rows = self.fleet_list_visible_rows();
            sync_scroll_to_cursor(
                &mut self.fleet.scroll_offset,
                self.fleet.cursor,
                visible_rows,
            );
            self.current_screen = ScreenId::FleetList;
        } else {
            self.current_screen = ScreenId::FleetMenu;
        }
    }

    pub fn open_fleet_eta(&mut self) {
        if self.current_screen == ScreenId::FleetList {
            match self.fleet_selected_list_row() {
                Ok(row) => {
                    self.fleet.command_context = FleetCommandContext::List;
                    self.open_fleet_eta_with_selected_record(row.fleet_record_index_1_based);
                    return;
                }
                Err(err) => {
                    let message = if err == "You have no active fleets." {
                        err
                    } else {
                        "Fleet unavailable".to_string()
                    };
                    self.show_fleet_list_dismiss_message(message);
                    return;
                }
            }
        }
        self.fleet.command_context = FleetCommandContext::Menu;
        self.fleet.eta_fleet_record_index_1_based = None;
        self.fleet.eta_status = None;
        self.fleet.eta_destination_input.clear();
        self.fleet.eta_include_system_input.clear();
        self.open_fleet_menu_prompt(
            FleetMenuPromptMode::EtaFleet,
            self.strongest_owned_fleet_number()
                .map(|value| value.to_string())
                .unwrap_or_default(),
        );
    }

    pub(crate) fn open_fleet_eta_with_selected_record(
        &mut self,
        fleet_record_index_1_based: usize,
    ) {
        if self
            .fleet_rows()
            .iter()
            .all(|row| row.fleet_record_index_1_based != fleet_record_index_1_based)
        {
            let message = if self.current_screen == ScreenId::FleetList {
                "Fleet unavailable".to_string()
            } else {
                "Selected fleet is no longer available.".to_string()
            };
            if self.current_screen == ScreenId::FleetList {
                self.show_fleet_list_dismiss_message(message);
            } else {
                self.fleet.menu_prompt_status =
                    self.show_fleet_prompt_feedback(PromptFeedback::error(message));
            }
            return;
        }
        self.clear_command_menu_notice();
        self.clear_fleet_menu_prompt();
        self.fleet.eta_fleet_record_index_1_based = Some(fleet_record_index_1_based);
        self.fleet.eta_status = None;
        self.fleet.eta_destination_input.clear();
        self.fleet.eta_include_system_input.clear();
        self.fleet.eta_mode = FleetEtaMode::EnteringDestination;
        self.current_screen = ScreenId::FleetEta;
    }

    fn submit_inline_fleet_change(&mut self) -> Result<(), String> {
        let row = self.prompt_context_fleet_row()?;
        let raw = if self.fleet.menu_prompt_input.trim().is_empty() {
            self.fleet.menu_prompt_default_value.trim().to_string()
        } else {
            self.fleet.menu_prompt_input.trim().to_string()
        };
        let field = self
            .fleet
            .menu_prompt_change_field
            .ok_or_else(|| "Choose ROE, ID, or Speed first.".to_string())?;
        match field {
            FleetChangeField::Roe => {
                let roe = raw
                    .parse::<u8>()
                    .map_err(|_| "Enter an ROE from 0 to 10.".to_string())?;
                self.game_data
                    .set_fleet_rules_of_engagement(
                        self.player.record_index_1_based,
                        row.fleet_record_index_1_based,
                        roe,
                    )
                    .map_err(|err| match err {
                        nc_data::GameStateMutationError::InvalidFleetPlayerInput {
                            reason:
                                nc_data::FleetPlayerInputValidationError::NonCombatFleetMustUseZeroRoe {
                                    ..
                                },
                            ..
                        } => "Non-combat fleets must use ROE 0.".to_string(),
                        nc_data::GameStateMutationError::InvalidFleetPlayerInput {
                            reason:
                                nc_data::FleetPlayerInputValidationError::RulesOfEngagementOutOfRange { .. },
                            ..
                        } => "Enter an ROE from 0 to 10.".to_string(),
                        other => other.to_string(),
                    })?;
                self.save_game_data().map_err(|err| err.to_string())?;
                self.show_fleet_context_success(
                    format!("Fleet #{} ROE set to {}.", row.fleet_number, roe),
                    true,
                );
            }
            FleetChangeField::Id => {
                let local_slot = raw
                    .parse::<u16>()
                    .map_err(|_| "Enter a fleet ID from 1 up.".to_string())?;
                self.game_data
                    .set_fleet_local_slot(
                        self.player.record_index_1_based,
                        row.fleet_record_index_1_based,
                        local_slot,
                    )
                    .map_err(|err| match err {
                        nc_data::GameStateMutationError::InvalidFleetLocalSlot { .. } => {
                            "Fleet ID is already in use.".to_string()
                        }
                        other => other.to_string(),
                    })?;
                self.save_game_data().map_err(|err| err.to_string())?;
                self.show_fleet_context_success(
                    format!(
                        "Fleet #{} renumbered to Fleet #{}.",
                        row.fleet_number, local_slot
                    ),
                    true,
                );
            }
            FleetChangeField::Speed => {
                let speed = raw
                    .parse::<u8>()
                    .map_err(|_| "Enter a speed from 0 up.".to_string())?;
                let fleet = self
                    .game_data
                    .fleets
                    .records
                    .get(row.fleet_record_index_1_based - 1)
                    .ok_or_else(|| "Selected fleet is no longer available.".to_string())?
                    .clone();
                let aux = fleet.mission_aux_bytes();
                self.game_data
                    .set_fleet_order(
                        row.fleet_record_index_1_based,
                        speed,
                        fleet.standing_order_code_raw(),
                        fleet.standing_order_target_coords_raw(),
                        Some(aux[0]),
                        Some(aux[1]),
                    )
                    .map_err(|err| match err {
                        nc_data::GameStateMutationError::InvalidFleetSpeed { max, .. } => {
                            format!("Enter a speed from 0 to {max}.")
                        }
                        other => other.to_string(),
                    })?;
                self.save_game_data().map_err(|err| err.to_string())?;
                self.show_fleet_context_success(
                    format!("Fleet #{} speed set to {}.", row.fleet_number, speed),
                    true,
                );
            }
        }
        Ok(())
    }

    pub fn move_fleet_list(&mut self, delta: i8) {
        let ScreenId::FleetList = self.current_screen else {
            return;
        };
        let total = self.fleet_list_rows().len();
        if total == 0 {
            self.fleet.cursor = 0;
            return;
        }
        let next = self.fleet.cursor as isize + delta as isize;
        self.fleet.cursor = next.rem_euclid(total as isize) as usize;
        let visible_rows = self.fleet_list_visible_rows();
        sync_scroll_to_cursor(
            &mut self.fleet.scroll_offset,
            self.fleet.cursor,
            visible_rows,
        );
        self.fleet.list_status = None;
    }

    pub fn move_fleet_review(&mut self, delta: i8) {
        if self.current_screen != ScreenId::FleetReview {
            return;
        }
        let total = self.fleet_review_rows().len();
        if total == 0 {
            self.fleet.review_index = 0;
            return;
        }
        self.fleet.review_index = match delta {
            i8::MIN => 0,
            i8::MAX => total - 1,
            _ => self
                .fleet
                .review_index
                .saturating_add_signed(delta as isize)
                .min(total - 1),
        };
        self.fleet.cursor = self.fleet.review_index;
        let visible_rows = if self.fleet.review_return_to_list {
            self.fleet_list_visible_rows()
        } else {
            self.fleet_group_visible_rows()
        };
        sync_scroll_to_cursor(
            &mut self.fleet.scroll_offset,
            self.fleet.cursor,
            visible_rows,
        );
    }

    pub fn append_fleet_list_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::FleetList || !ch.is_ascii_digit() {
            return;
        }
        if self.fleet.list_input.len() >= 4 {
            return;
        }
        self.fleet.list_input.push(ch);
        self.sync_fleet_list_cursor_to_input();
        self.fleet.list_status = None;
    }

    pub fn backspace_fleet_list_input(&mut self) {
        if self.current_screen != ScreenId::FleetList {
            return;
        }
        self.fleet.list_input.pop();
        self.sync_fleet_list_cursor_to_input();
        self.fleet.list_status = None;
    }

    pub fn append_fleet_menu_prompt_char(&mut self, ch: char) {
        if !self.inline_fleet_menu_prompt_active_on_current_screen() {
            return;
        }
        let Some(mode) = self.fleet.menu_prompt_mode else {
            return;
        };
        let (allowed, max_len) = match mode {
            FleetMenuPromptMode::ChangeField => (ch.is_ascii_alphabetic(), 1),
            FleetMenuPromptMode::ChangeValue => {
                let max_len = match self.fleet.menu_prompt_change_field {
                    Some(FleetChangeField::Roe) | Some(FleetChangeField::Speed) => 2,
                    Some(FleetChangeField::Id) | None => 4,
                };
                (ch.is_ascii_digit(), max_len)
            }
            _ => (ch.is_ascii_digit(), 4),
        };
        if !allowed || self.fleet.menu_prompt_input.len() >= max_len {
            return;
        }
        self.fleet.menu_prompt_input.push(match mode {
            FleetMenuPromptMode::ChangeField => ch.to_ascii_uppercase(),
            _ => ch,
        });
        self.fleet.menu_prompt_status = None;
        if mode == FleetMenuPromptMode::ChangeField {
            self.submit_fleet_menu_prompt();
        }
    }

    pub fn append_fleet_eta_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::FleetEta {
            return;
        }
        match self.fleet.eta_mode {
            FleetEtaMode::EnteringDestination => {
                if self.fleet.eta_destination_input.len() < 16 && is_coordinate_input_char(ch) {
                    self.fleet.eta_destination_input.push(ch);
                    self.fleet.eta_status = None;
                }
            }
            FleetEtaMode::ConfirmingSystemEntry => {
                if matches!(ch, 'y' | 'Y' | 'n' | 'N')
                    && self.fleet.eta_include_system_input.is_empty()
                {
                    self.fleet
                        .eta_include_system_input
                        .push(ch.to_ascii_uppercase());
                    self.fleet.eta_status = None;
                    self.submit_fleet_eta();
                }
            }
            FleetEtaMode::ShowingResult => {}
        }
    }

    pub fn backspace_fleet_menu_prompt_input(&mut self) {
        if !self.inline_fleet_menu_prompt_active_on_current_screen() {
            return;
        }
        self.fleet.menu_prompt_input.pop();
        self.fleet.menu_prompt_status = None;
    }

    pub fn backspace_fleet_eta_input(&mut self) {
        if self.current_screen != ScreenId::FleetEta {
            return;
        }
        match self.fleet.eta_mode {
            FleetEtaMode::EnteringDestination => {
                self.fleet.eta_destination_input.pop();
            }
            FleetEtaMode::ConfirmingSystemEntry => {
                self.fleet.eta_include_system_input.pop();
            }
            FleetEtaMode::ShowingResult => {}
        }
        self.fleet.eta_status = None;
    }

    pub fn submit_fleet_eta(&mut self) {
        if self.current_screen != ScreenId::FleetEta {
            return;
        }
        let Some(selected_row) = self.fleet_eta_selected_row() else {
            self.fleet.eta_status = Some("You have no active fleets.".to_string());
            self.open_fleet_eta();
            return;
        };
        match self.fleet.eta_mode {
            FleetEtaMode::EnteringDestination => {
                let default_destination = self.fleet_eta_default_destination();
                let Some(destination) = resolve_default_coords_input(
                    &self.fleet.eta_destination_input,
                    default_destination,
                ) else {
                    self.fleet.eta_status = Some("Enter coordinates like 10,13".to_string());
                    return;
                };
                let map_size =
                    nc_engine::map_size_for_player_count(self.game_data.conquest.player_count());
                if destination[0] == 0
                    || destination[1] == 0
                    || destination[0] > map_size
                    || destination[1] > map_size
                {
                    self.fleet.eta_status = Some(format!("Enter coordinates within 1..{map_size}"));
                    return;
                }
                self.fleet.eta_destination_input = format!("{},{}", destination[0], destination[1]);
                self.fleet.eta_include_system_input.clear();
                self.fleet.eta_status = None;
                self.fleet.eta_mode = FleetEtaMode::ConfirmingSystemEntry;
            }
            FleetEtaMode::ConfirmingSystemEntry => {
                let include_system =
                    resolve_yes_no_input(&self.fleet.eta_include_system_input, false);
                let destination = resolve_default_coords_input(
                    &self.fleet.eta_destination_input,
                    self.fleet_eta_default_destination(),
                )
                .unwrap_or(self.fleet_eta_default_destination());
                let result =
                    self.calculate_fleet_eta_message(&selected_row, destination, include_system);
                self.fleet.eta_status = Some(result);
                self.fleet.eta_include_system_input.clear();
                self.fleet.eta_mode = FleetEtaMode::ShowingResult;
            }
            FleetEtaMode::ShowingResult => {
                self.fleet.eta_status = None;
                self.fleet.eta_fleet_record_index_1_based = None;
                self.fleet.eta_destination_input.clear();
                self.fleet.eta_include_system_input.clear();
                self.current_screen = self.fleet_context_screen();
            }
        }
    }

    pub fn current_fleet_roe_by_id(&self, fleet_id: u16) -> Option<u8> {
        self.game_data
            .fleets
            .records
            .iter()
            .find(|fleet| {
                fleet.owner_empire_raw() as usize == self.player.record_index_1_based
                    && fleet.local_slot_word_raw() == fleet_id
            })
            .map(|fleet| fleet.rules_of_engagement())
    }

    pub(crate) fn fleet_list_rows(&self) -> Vec<FleetRow> {
        let mut rows = self.fleet_rows();
        rows.sort_by(|left, right| right.fleet_number.cmp(&left.fleet_number));
        rows
    }

    fn fleet_review_rows(&self) -> Vec<FleetRow> {
        if self.fleet.review_return_to_list {
            self.fleet_list_rows()
        } else {
            self.fleet_rows()
        }
    }

    pub(crate) fn fleet_rows(&self) -> Vec<FleetRow> {
        let mut rows = self
            .game_data
            .fleets
            .records
            .iter()
            .enumerate()
            .filter(|(_, fleet)| {
                fleet.owner_empire_raw() as usize == self.player.record_index_1_based
            })
            .map(|(idx, fleet)| FleetRow {
                fleet_record_index_1_based: idx + 1,
                fleet_number: fleet.local_slot_word_raw(),
                coords: fleet.current_location_coords_raw(),
                target_coords: fleet.standing_order_target_coords_raw(),
                order_code: fleet.standing_order_code_raw(),
                current_speed: fleet.current_speed(),
                max_speed: fleet.max_speed(),
                eta_label: fleet_eta_label(&self.game_data, idx),
                list_eta_label: fleet_list_eta_label(&self.game_data, idx),
                rules_of_engagement: fleet.rules_of_engagement(),
                loaded_armies: fleet.army_count(),
                order_label: fleet.standing_order_summary(),
                composition_label: fleet.ship_composition_summary(),
                table_ships_label: fleet.ship_composition_table_summary(),
            })
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| {
            left.order_code
                .cmp(&right.order_code)
                .then_with(|| right.fleet_number.cmp(&left.fleet_number))
        });
        rows
    }

    pub(crate) fn fleet_row_by_record_index(
        &self,
        fleet_record_index_1_based: usize,
    ) -> Option<FleetRow> {
        self.fleet_rows()
            .into_iter()
            .find(|row| row.fleet_record_index_1_based == fleet_record_index_1_based)
    }

    pub(crate) fn fleet_eta_selected_row(&self) -> Option<FleetRow> {
        self.fleet
            .eta_fleet_record_index_1_based
            .and_then(|idx| self.fleet_row_by_record_index(idx))
    }

    fn sync_fleet_list_cursor_to_input(&mut self) {
        let ScreenId::FleetList = self.current_screen else {
            return;
        };
        let rows = self.fleet_list_rows();
        let match_rows = rows
            .iter()
            .map(|row| vec![row.fleet_number.to_string()])
            .collect::<Vec<_>>();
        let Some(index) = crate::screen::table_selection::find_typed_jump_index(
            &match_rows,
            0,
            &self.fleet.list_input,
        ) else {
            return;
        };
        self.fleet.cursor = index;
        let visible_rows = self.fleet_list_visible_rows();
        sync_scroll_to_cursor(
            &mut self.fleet.scroll_offset,
            self.fleet.cursor,
            visible_rows,
        );
    }

    pub fn cancel_fleet_menu_prompt(&mut self) {
        if self.inline_fleet_menu_prompt_active_on_current_screen() {
            self.clear_fleet_menu_prompt();
            self.current_screen = self.fleet_context_screen();
        }
    }

    fn fleet_menu_default_fleet_number(&self) -> Option<u16> {
        if self.fleet.menu_prompt_default_value.trim().is_empty() {
            self.strongest_owned_fleet_number()
        } else {
            self.fleet
                .menu_prompt_default_value
                .trim()
                .parse::<u16>()
                .ok()
        }
    }

    fn resolve_fleet_prompt_row_from_rows(
        &self,
        rows: &[FleetRow],
        default_fleet_number: Option<u16>,
        invalid_message: &str,
    ) -> Result<(usize, FleetRow), String> {
        let default_fleet_number =
            default_fleet_number.ok_or_else(|| "You have no active fleets.".to_string())?;
        let fleet_number = if self.fleet.menu_prompt_input.trim().is_empty() {
            default_fleet_number
        } else {
            self.fleet
                .menu_prompt_input
                .trim()
                .parse::<u16>()
                .map_err(|_| invalid_message.to_string())?
        };
        let index = rows
            .iter()
            .position(|row| row.fleet_number == fleet_number)
            .ok_or_else(|| format!("Fleet #{fleet_number} is not in your fleet list."))?;
        Ok((index, rows[index].clone()))
    }

    fn resolve_fleet_menu_prompt_selection(&self) -> Result<(usize, FleetRow), String> {
        let rows = self.fleet_rows();
        self.resolve_fleet_prompt_row_from_rows(
            &rows,
            self.fleet_menu_default_fleet_number(),
            "Enter one of your fleet numbers.",
        )
    }

    fn prompt_context_fleet_row(&self) -> Result<FleetRow, String> {
        let record_index = self
            .fleet
            .menu_prompt_context_fleet_record_index_1_based
            .ok_or_else(|| "Select a fleet first.".to_string())?;
        self.fleet_rows()
            .into_iter()
            .find(|row| row.fleet_record_index_1_based == record_index)
            .ok_or_else(|| "Selected fleet is no longer available.".to_string())
    }

    fn fleet_change_value_default(&self) -> Result<String, String> {
        let row = self.prompt_context_fleet_row()?;
        Ok(match self.fleet.menu_prompt_change_field {
            Some(FleetChangeField::Roe) => row.rules_of_engagement.to_string(),
            Some(FleetChangeField::Id) => row.fleet_number.to_string(),
            Some(FleetChangeField::Speed) => row.current_speed.to_string(),
            None => String::new(),
        })
    }

    fn resolve_fleet_change_field(&self) -> Result<FleetChangeField, String> {
        let raw = if self.fleet.menu_prompt_input.trim().is_empty() {
            self.fleet.menu_prompt_default_value.trim().to_string()
        } else {
            self.fleet.menu_prompt_input.trim().to_string()
        };
        match raw.chars().next().map(|ch| ch.to_ascii_uppercase()) {
            Some('R') => Ok(FleetChangeField::Roe),
            Some('I') => Ok(FleetChangeField::Id),
            Some('S') => Ok(FleetChangeField::Speed),
            _ => Err("Enter R, I, or S.".to_string()),
        }
    }

    pub(crate) fn merge_host_default_value(&self) -> String {
        let Some(source_record_index) = self.fleet.merge_source_record_index_1_based else {
            return String::new();
        };
        self.eligible_merge_host_rows(source_record_index)
            .first()
            .map(|row| row.fleet_number.to_string())
            .unwrap_or_default()
    }

    fn eligible_transfer_host_rows(&self) -> Vec<FleetRow> {
        let Some(donor_record_index) = self.fleet.transfer_donor_record_index_1_based else {
            return Vec::new();
        };
        let rows = self.fleet_rows();
        let Some(donor_row) = rows
            .iter()
            .find(|row| row.fleet_record_index_1_based == donor_record_index)
        else {
            return Vec::new();
        };
        let donor_coords = donor_row.coords;
        let mut rows = rows
            .into_iter()
            .filter(|row| {
                row.fleet_record_index_1_based != donor_record_index && row.coords == donor_coords
            })
            .collect::<Vec<_>>();
        rows.sort_by_key(|row| {
            (
                self.fleet_ship_total(row.fleet_record_index_1_based),
                row.fleet_number,
            )
        });
        rows
    }

    pub(crate) fn transfer_host_default_value(&self) -> String {
        self.eligible_transfer_host_rows()
            .first()
            .map(|row| row.fleet_number.to_string())
            .unwrap_or_default()
    }

    fn fleet_ship_total(&self, fleet_record_index_1_based: usize) -> u32 {
        self.game_data
            .fleets
            .records
            .get(fleet_record_index_1_based - 1)
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

    fn co_located_merge_peer_rows(&self, source_record_index_1_based: usize) -> Vec<FleetRow> {
        let Some(source_row) = self.fleet_row_by_record_index(source_record_index_1_based) else {
            return Vec::new();
        };
        self.fleet_rows()
            .into_iter()
            .filter(|row| {
                row.fleet_record_index_1_based != source_record_index_1_based
                    && row.coords == source_row.coords
            })
            .collect()
    }

    fn eligible_merge_host_rows(&self, source_record_index_1_based: usize) -> Vec<FleetRow> {
        let Some(source_row) = self.fleet_row_by_record_index(source_record_index_1_based) else {
            return Vec::new();
        };
        let mut rows = self
            .co_located_merge_peer_rows(source_record_index_1_based)
            .into_iter()
            .filter(|row| row.fleet_number < source_row.fleet_number)
            .collect::<Vec<_>>();
        rows.sort_by_key(|row| {
            (
                self.fleet_ship_total(row.fleet_record_index_1_based),
                row.fleet_number,
            )
        });
        rows
    }

    pub(crate) fn eligible_merge_source_fleet_number(&self) -> Option<u16> {
        self.fleet_rows()
            .into_iter()
            .filter(|row| {
                !self
                    .eligible_merge_host_rows(row.fleet_record_index_1_based)
                    .is_empty()
            })
            .max_by_key(|row| {
                (
                    self.fleet_ship_total(row.fleet_record_index_1_based),
                    std::cmp::Reverse(row.fleet_number),
                )
            })
            .map(|row| row.fleet_number)
    }

    pub(crate) fn eligible_transfer_donor_fleet_number(&self) -> Option<u16> {
        self.fleet_rows()
            .into_iter()
            .filter(|row| self.fleet_ship_total(row.fleet_record_index_1_based) > 1)
            .filter(|row| {
                self.fleet_rows().iter().any(|other| {
                    other.fleet_record_index_1_based != row.fleet_record_index_1_based
                        && other.coords == row.coords
                })
            })
            .max_by_key(|row| {
                (
                    self.fleet_ship_total(row.fleet_record_index_1_based),
                    std::cmp::Reverse(row.fleet_number),
                )
            })
            .map(|row| row.fleet_number)
    }

    pub(crate) fn validate_merge_source_row(&self, row: &FleetRow) -> Result<(), String> {
        let peers = self.co_located_merge_peer_rows(row.fleet_record_index_1_based);
        if peers.is_empty() {
            return Err(format!(
                "Fleet #{} is not in a sector with another one of your fleets.",
                row.fleet_number
            ));
        }
        if peers
            .iter()
            .all(|peer| peer.fleet_number > row.fleet_number)
        {
            return Err("Fleets must be co-located in the same sector.".to_string());
        }
        Ok(())
    }

    pub(crate) fn validate_transfer_donor_row(&self, row: &FleetRow) -> Result<(), String> {
        if self.fleet_ship_total(row.fleet_record_index_1_based) <= 1 {
            return Err("Use merge instead".to_string());
        }
        let has_host = self.fleet_rows().iter().any(|other| {
            other.fleet_record_index_1_based != row.fleet_record_index_1_based
                && other.coords == row.coords
        });
        if !has_host {
            return Err(format!(
                "Fleet #{} is not in a sector with another one of your fleets.",
                row.fleet_number
            ));
        }
        Ok(())
    }

    fn resolve_merge_host_prompt_row(&self) -> Result<FleetRow, String> {
        let source_record_index = self
            .fleet
            .merge_source_record_index_1_based
            .ok_or_else(|| "Select the fleet that will merge first.".to_string())?;
        let source_row = self
            .fleet_row_by_record_index(source_record_index)
            .ok_or_else(|| "Selected fleet is no longer available.".to_string())?;
        let fleet_number = if self.fleet.menu_prompt_input.trim().is_empty() {
            self.fleet
                .menu_prompt_default_value
                .trim()
                .parse::<u16>()
                .map_err(|_| "Enter one of your fleet numbers.".to_string())?
        } else {
            self.fleet
                .menu_prompt_input
                .trim()
                .parse::<u16>()
                .map_err(|_| "Enter one of your fleet numbers.".to_string())?
        };
        let row = self
            .fleet_rows()
            .into_iter()
            .find(|row| row.fleet_number == fleet_number)
            .ok_or_else(|| format!("Fleet #{fleet_number} is not in your fleet list."))?;
        if row.fleet_record_index_1_based == source_record_index {
            return Err("Choose a different host fleet.".to_string());
        }
        if row.coords != source_row.coords {
            return Err(format!(
                "Fleet #{} is not in the same sector as Fleet #{}.",
                row.fleet_number, source_row.fleet_number
            ));
        }
        Ok(row)
    }

    fn resolve_transfer_host_prompt_row(&self) -> Result<FleetRow, String> {
        let donor_record_index = self
            .fleet
            .transfer_donor_record_index_1_based
            .ok_or_else(|| "Select a source fleet first.".to_string())?;
        let donor_row = self
            .fleet_row_by_record_index(donor_record_index)
            .ok_or_else(|| "Selected source fleet is no longer available.".to_string())?;
        let fleet_number = if self.fleet.menu_prompt_input.trim().is_empty() {
            self.fleet
                .menu_prompt_default_value
                .trim()
                .parse::<u16>()
                .map_err(|_| "Enter one of your fleet numbers.".to_string())?
        } else {
            self.fleet
                .menu_prompt_input
                .trim()
                .parse::<u16>()
                .map_err(|_| "Enter one of your fleet numbers.".to_string())?
        };
        let row = self
            .fleet_rows()
            .into_iter()
            .find(|row| row.fleet_number == fleet_number)
            .ok_or_else(|| format!("Fleet #{fleet_number} is not in your fleet list."))?;
        if row.fleet_record_index_1_based == donor_record_index {
            return Err("Choose a different destination fleet.".to_string());
        }
        if row.coords != donor_row.coords {
            return Err(format!(
                "Fleet #{} is not in the same sector as Fleet #{}.",
                row.fleet_number, donor_row.fleet_number
            ));
        }
        Ok(row)
    }

    pub fn submit_fleet_menu_prompt(&mut self) {
        if !self.inline_fleet_menu_prompt_active_on_current_screen() {
            return;
        }
        let Some(mode) = self.fleet.menu_prompt_mode else {
            return;
        };
        match mode {
            FleetMenuPromptMode::Review => match self.resolve_fleet_menu_prompt_selection() {
                Ok((index, row)) => {
                    self.fleet.cursor = index;
                    self.fleet.menu_prompt_input.clear();
                    self.fleet.menu_prompt_status = None;
                    self.fleet.menu_prompt_default_value = row.fleet_number.to_string();
                    self.open_fleet_review();
                }
                Err(err) => {
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(err))
                }
            },
            FleetMenuPromptMode::Order => match self.resolve_fleet_menu_prompt_selection() {
                Ok((index, row)) => {
                    self.fleet.cursor = index;
                    self.fleet.menu_prompt_input.clear();
                    self.fleet.menu_prompt_status = None;
                    self.fleet.menu_prompt_default_value = row.fleet_number.to_string();
                    self.open_fleet_order_with_selected_record(row.fleet_record_index_1_based);
                }
                Err(err) => {
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(err))
                }
            },
            FleetMenuPromptMode::ChangeFleet => match self.resolve_fleet_menu_prompt_selection() {
                Ok((_index, row)) => {
                    self.fleet.menu_prompt_context_fleet_record_index_1_based =
                        Some(row.fleet_record_index_1_based);
                    self.fleet.menu_prompt_change_field = None;
                    self.fleet.menu_prompt_mode = Some(FleetMenuPromptMode::ChangeField);
                    self.fleet.menu_prompt_input.clear();
                    self.fleet.menu_prompt_status = None;
                    self.fleet.menu_prompt_default_value = "R".to_string();
                }
                Err(err) => {
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(err))
                }
            },
            FleetMenuPromptMode::ChangeField => match self.resolve_fleet_change_field() {
                Ok(field) => {
                    self.fleet.menu_prompt_change_field = Some(field);
                    self.fleet.menu_prompt_mode = Some(FleetMenuPromptMode::ChangeValue);
                    self.fleet.menu_prompt_input.clear();
                    self.fleet.menu_prompt_status = None;
                    self.fleet.menu_prompt_default_value =
                        self.fleet_change_value_default().unwrap_or_default();
                }
                Err(err) => {
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(err))
                }
            },
            FleetMenuPromptMode::ChangeValue => {
                if let Err(err) = self.submit_inline_fleet_change() {
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(err));
                }
            }
            FleetMenuPromptMode::EtaFleet => match self.resolve_fleet_menu_prompt_selection() {
                Ok((index, row)) => {
                    self.fleet.cursor = index;
                    self.fleet.menu_prompt_input.clear();
                    self.fleet.menu_prompt_status = None;
                    self.fleet.menu_prompt_default_value = row.fleet_number.to_string();
                    self.open_fleet_eta_with_selected_record(row.fleet_record_index_1_based);
                }
                Err(err) => {
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(err))
                }
            },
            FleetMenuPromptMode::DetachFleet => match self.resolve_fleet_menu_prompt_selection() {
                Ok((index, row)) => {
                    self.fleet.cursor = index;
                    self.fleet.menu_prompt_input.clear();
                    self.fleet.menu_prompt_status = None;
                    if let Err(err) =
                        self.open_fleet_detach_with_selected_record(row.fleet_record_index_1_based)
                    {
                        self.fleet.menu_prompt_default_value = self
                            .largest_owned_fleet_number_by_ship_total()
                            .map(|value| value.to_string())
                            .unwrap_or_default();
                        self.fleet.menu_prompt_status =
                            self.show_fleet_prompt_feedback(PromptFeedback::error(err));
                    }
                }
                Err(err) => {
                    self.fleet.menu_prompt_default_value = self
                        .largest_owned_fleet_number_by_ship_total()
                        .map(|value| value.to_string())
                        .unwrap_or_default();
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(err));
                }
            },
            FleetMenuPromptMode::MergeSource => match self.resolve_fleet_menu_prompt_selection() {
                Ok((_index, row)) => {
                    if let Err(err) = self.validate_merge_source_row(&row) {
                        self.fleet.menu_prompt_default_value = self
                            .eligible_merge_source_fleet_number()
                            .map(|value| value.to_string())
                            .unwrap_or_default();
                        self.fleet.menu_prompt_status =
                            self.show_fleet_prompt_feedback(PromptFeedback::error(err));
                        return;
                    }
                    self.fleet.merge_source_record_index_1_based =
                        Some(row.fleet_record_index_1_based);
                    self.fleet.menu_prompt_mode = Some(FleetMenuPromptMode::MergeHost);
                    self.fleet.menu_prompt_input.clear();
                    self.fleet.menu_prompt_status = None;
                    self.fleet.menu_prompt_default_value = self.merge_host_default_value();
                }
                Err(err) => {
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(err))
                }
            },
            FleetMenuPromptMode::MergeHost => match self.resolve_merge_host_prompt_row() {
                Ok(row) => {
                    if let Err(err) = self.submit_inline_fleet_merge(row.fleet_record_index_1_based)
                    {
                        self.fleet.menu_prompt_status =
                            self.show_fleet_prompt_feedback(PromptFeedback::error(err));
                    }
                }
                Err(err) => {
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(err))
                }
            },
            FleetMenuPromptMode::TransferDonor => {
                match self.resolve_fleet_menu_prompt_selection() {
                    Ok((_index, row)) => {
                        if let Err(err) = self.validate_transfer_donor_row(&row) {
                            self.fleet.menu_prompt_status =
                                self.show_fleet_prompt_feedback(PromptFeedback::error(err));
                            return;
                        }
                        self.fleet.transfer_donor_record_index_1_based =
                            Some(row.fleet_record_index_1_based);
                        self.fleet.menu_prompt_mode = Some(FleetMenuPromptMode::TransferHost);
                        self.fleet.menu_prompt_input.clear();
                        self.fleet.menu_prompt_status = None;
                        self.fleet.menu_prompt_default_value = self.transfer_host_default_value();
                    }
                    Err(err) => {
                        self.fleet.menu_prompt_status =
                            self.show_fleet_prompt_feedback(PromptFeedback::error(err))
                    }
                }
            }
            FleetMenuPromptMode::TransferHost => match self.resolve_transfer_host_prompt_row() {
                Ok(row) => {
                    self.open_fleet_transfer_with_selected_records(row.fleet_record_index_1_based);
                }
                Err(err) => {
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(err))
                }
            },
            FleetMenuPromptMode::TransportFleet(mode) => match self
                .resolve_fleet_menu_prompt_selection()
            {
                Ok((_index, row)) => {
                    if let Err(err) = self
                        .open_fleet_transport_quantity_prompt(mode, row.fleet_record_index_1_based)
                    {
                        self.fleet.menu_prompt_status =
                            self.show_fleet_prompt_feedback(PromptFeedback::error(err));
                    }
                }
                Err(err) => {
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(err))
                }
            },
            FleetMenuPromptMode::TransportQuantity(_) => {
                self.planet.transport_qty_input = self.fleet.menu_prompt_input.clone();
                self.planet.transport_status = None;
                if let Err(err) = self.submit_planet_transport_qty() {
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(err.to_string()));
                    return;
                }
                if let Some(status) = self.planet.transport_status.take() {
                    self.fleet.menu_prompt_status =
                        self.show_fleet_prompt_feedback(PromptFeedback::error(status));
                }
            }
        }
    }

    pub(crate) fn handle_fleet_menu_prompt_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        let mode = self.fleet.menu_prompt_mode;
        match key.code {
            KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitMenuPrompt),
            KeyCode::Backspace => crate::app::Action::Fleet(FleetAction::BackspaceMenuPromptInput),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                crate::app::Action::Fleet(FleetAction::CancelMenuPrompt)
            }
            KeyCode::Char(ch)
                if match mode {
                    Some(FleetMenuPromptMode::ChangeField) => ch.is_ascii_alphabetic(),
                    _ => ch.is_ascii_digit(),
                } =>
            {
                crate::app::Action::Fleet(FleetAction::AppendMenuPromptChar(ch))
            }
            _ => crate::app::Action::Noop,
        }
    }

    pub(crate) fn handle_fleet_eta_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match self.fleet.eta_mode {
            FleetEtaMode::EnteringDestination => match key.code {
                KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitEta),
                KeyCode::Backspace => crate::app::Action::Fleet(FleetAction::BackspaceEtaInput),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::Fleet(FleetAction::OpenEta)
                }
                KeyCode::Char(ch) if is_coordinate_input_char(ch) => {
                    crate::app::Action::Fleet(FleetAction::AppendEtaChar(ch))
                }
                _ => crate::app::Action::Noop,
            },
            FleetEtaMode::ConfirmingSystemEntry => match key.code {
                KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitEta),
                KeyCode::Backspace => crate::app::Action::Fleet(FleetAction::BackspaceEtaInput),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::Fleet(FleetAction::OpenEta)
                }
                KeyCode::Char(ch) if matches!(ch, 'y' | 'Y' | 'n' | 'N') => {
                    crate::app::Action::Fleet(FleetAction::AppendEtaChar(ch))
                }
                _ => crate::app::Action::Noop,
            },
            FleetEtaMode::ShowingResult => crate::app::Action::Fleet(FleetAction::SubmitEta),
        }
    }

    pub(crate) fn fleet_eta_default_destination(&self) -> [u8; 2] {
        let Some(row) = self.fleet_eta_selected_row() else {
            return [8, 2];
        };
        if row.target_coords[0] > 0 && row.target_coords[1] > 0 {
            row.target_coords
        } else {
            row.coords
        }
    }

    pub(crate) fn fleet_group_target_status_line(&self) -> String {
        if self.fleet.group_mode == crate::screen::FleetGroupOrderMode::ConfirmingTarget
            && let (Some(mission_code), Ok(destination)) = (
                self.fleet.group_mission_code,
                self.resolve_fleet_group_split_target(),
            )
            && let Some(message) =
                self.fleet_group_confirmation_eta_message(mission_code, destination)
        {
            return message;
        }
        fleet_target_status_line(self.fleet.group_mission_code)
    }

    pub(crate) fn fleet_group_target_prompt(&self) -> String {
        match fleet_target_input_kind(self.fleet.group_mission_code) {
            FleetTargetInputKind::StarbaseId => "Starbase # ".to_string(),
            FleetTargetInputKind::FleetId => "Fleet # ".to_string(),
            FleetTargetInputKind::Coordinates => "Target ".to_string(),
            FleetTargetInputKind::None => "Target ".to_string(),
        }
    }

    pub(crate) fn fleet_group_target_default(&self) -> String {
        match fleet_target_input_kind(self.fleet.group_mission_code) {
            FleetTargetInputKind::StarbaseId => self
                .fleet_group_default_starbase()
                .map(|row| row.base_id.to_string())
                .unwrap_or_else(|| "1".to_string()),
            FleetTargetInputKind::FleetId => self
                .fleet_group_default_host_fleet()
                .map(|row| row.fleet_number.to_string())
                .unwrap_or_else(|| "1".to_string()),
            FleetTargetInputKind::Coordinates | FleetTargetInputKind::None => self
                .fleet_group_default_target_coords()
                .map(|target| format!("{},{}", target[0], target[1]))
                .unwrap_or_default(),
        }
    }
}
