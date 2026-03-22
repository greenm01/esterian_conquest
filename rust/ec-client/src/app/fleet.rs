use super::fleet_manip::fleet_eta_label;
use super::fleet_order::{
    FleetTargetInputKind, fleet_target_input_kind, fleet_target_status_line, resolve_yes_no_input,
};
use super::helpers::{
    center_scroll_to_cursor, resolve_default_coords_input, sync_scroll_to_cursor,
};
use crate::app::state::App;
use crate::domains::fleet::FleetAction;
use crate::screen::{CommandMenu, FleetEtaMode, FleetListMode, FleetRow, ScreenId};

impl App {
    pub fn show_fleet_expert_mode_notice(&mut self) {
        self.show_command_menu_notice(
            CommandMenu::Fleet,
            "Expert mode not implemented yet. Plan for Helix style commands.",
        );
    }

    pub fn open_fleet_menu(&mut self) {
        self.clear_command_menu_notice();
        self.current_screen = ScreenId::FleetMenu;
    }

    pub fn open_fleet_help(&mut self) {
        self.clear_command_menu_notice();
        self.current_screen = ScreenId::FleetHelp;
    }

    pub fn open_fleet_list(&mut self, mode: FleetListMode) {
        if self.fleet_rows().is_empty() {
            self.show_command_menu_notice(CommandMenu::Fleet, "You have no active fleets.");
            return;
        }
        self.clear_command_menu_notice();
        self.fleet.list_mode = mode;
        self.fleet.scroll_offset = 0;
        self.fleet.cursor = 0;
        self.current_screen = ScreenId::FleetList(mode);
    }

    pub fn open_fleet_review(&mut self) {
        let total = self.fleet_rows().len();
        if total == 0 {
            self.show_command_menu_notice(CommandMenu::Fleet, "You have no active fleets.");
            return;
        }
        self.clear_command_menu_notice();
        self.fleet.review_index = self.fleet.cursor.min(total - 1);
        self.current_screen = ScreenId::FleetReview;
    }

    pub fn submit_fleet_review_select(&mut self) {
        if self.current_screen != ScreenId::FleetReviewSelect {
            return;
        }
        let rows = self.fleet_rows();
        let Some(_) = rows.get(self.fleet.cursor) else {
            self.current_screen = ScreenId::FleetMenu;
            return;
        };
        if !self.fleet.review_select_input.trim().is_empty() {
            let target_fleet_id = match self.fleet.review_select_input.trim().parse::<u16>() {
                Ok(value) => value,
                Err(_) => {
                    self.fleet.review_status =
                        Some("Enter a fleet number from the table.".to_string());
                    return;
                }
            };
            let Some(index) = rows
                .iter()
                .position(|row| row.fleet_number == target_fleet_id)
            else {
                self.fleet.review_status = Some(format!(
                    "Fleet #{target_fleet_id} is not in your fleet list."
                ));
                return;
            };
            self.fleet.cursor = index;
            sync_scroll_to_cursor(
                &mut self.fleet.scroll_offset,
                self.fleet.cursor,
                crate::screen::FLEET_VISIBLE_ROWS,
            );
        }
        self.fleet.review_select_input.clear();
        self.fleet.review_status = None;
        self.open_fleet_review();
    }

    pub fn open_fleet_review_select(&mut self) {
        let total = self.fleet_rows().len();
        if total == 0 {
            self.show_command_menu_notice(CommandMenu::Fleet, "You have no active fleets.");
            return;
        }
        self.clear_command_menu_notice();
        self.fleet.cursor = self.fleet.cursor.min(total - 1);
        self.fleet.review_select_input.clear();
        self.fleet.review_status = None;
        center_scroll_to_cursor(
            &mut self.fleet.scroll_offset,
            self.fleet.cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
            total,
        );
        self.current_screen = ScreenId::FleetReviewSelect;
    }

    pub fn open_fleet_roe_select(&mut self) {
        if self.current_screen == ScreenId::FleetRoeSelect {
            self.fleet.roe_editing = false;
            self.fleet.roe_select_input.clear();
            self.fleet.roe_input.clear();
            self.fleet.roe_status = None;
            return;
        }
        if self.fleet_rows().is_empty() {
            self.show_command_menu_notice(CommandMenu::Fleet, "You have no active fleets.");
            return;
        }
        self.clear_command_menu_notice();
        self.fleet.roe_status = None;
        self.fleet.roe_select_input.clear();
        self.fleet.roe_input.clear();
        let total = self.fleet_rows().len();
        self.fleet.roe_cursor = self.fleet.roe_cursor.min(total - 1);
        center_scroll_to_cursor(
            &mut self.fleet.roe_scroll_offset,
            self.fleet.roe_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
            total,
        );
        self.fleet.roe_editing = false;
        self.current_screen = ScreenId::FleetRoeSelect;
    }

    pub fn open_fleet_eta(&mut self) {
        if self.fleet_rows().is_empty() {
            self.show_command_menu_notice(CommandMenu::Fleet, "You have no active fleets.");
            return;
        }
        self.clear_command_menu_notice();
        self.fleet.eta_status = None;
        self.fleet.eta_select_input.clear();
        self.fleet.eta_destination_input.clear();
        self.fleet.eta_include_system_input.clear();
        let total = self.fleet_rows().len();
        self.fleet.eta_cursor = self.fleet.eta_cursor.min(total - 1);
        center_scroll_to_cursor(
            &mut self.fleet.eta_scroll_offset,
            self.fleet.eta_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
            total,
        );
        self.fleet.eta_mode = FleetEtaMode::SelectingFleet;
        self.current_screen = ScreenId::FleetEta;
    }

    pub fn move_fleet_list(&mut self, delta: i8) {
        let ScreenId::FleetList(_) = self.current_screen else {
            return;
        };
        let total = self.fleet_rows().len();
        if total == 0 {
            self.fleet.cursor = 0;
            return;
        }
        let next = self.fleet.cursor as isize + delta as isize;
        self.fleet.cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.fleet.scroll_offset,
            self.fleet.cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
    }

    pub fn move_fleet_review_select(&mut self, delta: i8) {
        if self.current_screen != ScreenId::FleetReviewSelect {
            return;
        }
        let total = self.fleet_rows().len();
        if total == 0 {
            self.fleet.cursor = 0;
            return;
        }
        let next = self.fleet.cursor as isize + delta as isize;
        self.fleet.cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.fleet.scroll_offset,
            self.fleet.cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
        self.fleet.review_select_input.clear();
        self.fleet.review_status = None;
    }

    pub fn move_fleet_review(&mut self, delta: i8) {
        if self.current_screen != ScreenId::FleetReview {
            return;
        }
        let total = self.fleet_rows().len();
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
        sync_scroll_to_cursor(
            &mut self.fleet.scroll_offset,
            self.fleet.cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
    }

    pub fn move_fleet_roe_select(&mut self, delta: i8) {
        if self.current_screen != ScreenId::FleetRoeSelect || self.fleet.roe_editing {
            return;
        }
        let total = self.fleet_rows().len();
        if total == 0 {
            self.fleet.roe_cursor = 0;
            return;
        }
        let next = self.fleet.roe_cursor as isize + delta as isize;
        self.fleet.roe_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.fleet.roe_scroll_offset,
            self.fleet.roe_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
        self.fleet.roe_select_input.clear();
        self.fleet.roe_status = None;
    }

    pub fn move_fleet_eta_select(&mut self, delta: i8) {
        if self.current_screen != ScreenId::FleetEta
            || self.fleet.eta_mode != FleetEtaMode::SelectingFleet
        {
            return;
        }
        let total = self.fleet_rows().len();
        if total == 0 {
            self.fleet.eta_cursor = 0;
            return;
        }
        let next = self.fleet.eta_cursor as isize + delta as isize;
        self.fleet.eta_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.fleet.eta_scroll_offset,
            self.fleet.eta_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
        self.fleet.eta_select_input.clear();
        self.fleet.eta_status = None;
    }

    pub fn append_fleet_roe_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::FleetRoeSelect
            && if self.fleet.roe_editing {
                self.fleet.roe_input.len() < 2
            } else {
                self.fleet.roe_select_input.len() < 2
            }
        {
            if self.fleet.roe_editing {
                self.fleet.roe_input.push(ch);
            } else {
                self.fleet.roe_select_input.push(ch);
                self.sync_fleet_roe_cursor_to_input();
            }
            self.fleet.roe_status = None;
        }
    }

    pub fn append_fleet_review_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::FleetReviewSelect || !ch.is_ascii_digit() {
            return;
        }
        if self.fleet.review_select_input.len() >= 4 {
            return;
        }
        self.fleet.review_select_input.push(ch);
        self.sync_fleet_review_cursor_to_input();
        self.fleet.review_status = None;
    }

    pub fn append_fleet_eta_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::FleetEta {
            return;
        }
        match self.fleet.eta_mode {
            FleetEtaMode::SelectingFleet => {
                if ch.is_ascii_digit() && self.fleet.eta_select_input.len() < 4 {
                    self.fleet.eta_select_input.push(ch);
                    self.sync_fleet_eta_cursor_to_input();
                    self.fleet.eta_status = None;
                }
            }
            FleetEtaMode::EnteringDestination => {
                if self.fleet.eta_destination_input.len() < 16
                    && (ch.is_ascii_digit() || matches!(ch, ',' | ' ' | '(' | ')' | '[' | ']'))
                {
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
                }
            }
            FleetEtaMode::ShowingResult => {}
        }
    }

    pub fn backspace_fleet_roe_input(&mut self) {
        if self.current_screen == ScreenId::FleetRoeSelect {
            if self.fleet.roe_editing {
                self.fleet.roe_input.pop();
            } else {
                self.fleet.roe_select_input.pop();
                self.sync_fleet_roe_cursor_to_input();
            }
            self.fleet.roe_status = None;
        }
    }

    pub fn backspace_fleet_review_input(&mut self) {
        if self.current_screen != ScreenId::FleetReviewSelect {
            return;
        }
        self.fleet.review_select_input.pop();
        self.sync_fleet_review_cursor_to_input();
        self.fleet.review_status = None;
    }

    pub fn backspace_fleet_eta_input(&mut self) {
        if self.current_screen != ScreenId::FleetEta {
            return;
        }
        match self.fleet.eta_mode {
            FleetEtaMode::SelectingFleet => {
                self.fleet.eta_select_input.pop();
                self.sync_fleet_eta_cursor_to_input();
            }
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

    pub fn submit_fleet_roe(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::FleetRoeSelect {
            return Ok(());
        }
        if !self.fleet.roe_editing {
            let rows = self.fleet_rows();
            if rows.is_empty() {
                self.current_screen = ScreenId::FleetMenu;
                return Ok(());
            }
            if self.fleet.roe_select_input.trim().is_empty() {
                self.fleet.roe_cursor = self.fleet.roe_cursor.min(rows.len() - 1);
            } else {
                let target_fleet_id = match self.fleet.roe_select_input.trim().parse::<u16>() {
                    Ok(value) => value,
                    Err(_) => {
                        self.fleet.roe_status =
                            Some("Enter a fleet number from the table.".to_string());
                        return Ok(());
                    }
                };
                let Some(index) = rows
                    .iter()
                    .position(|row| row.fleet_number == target_fleet_id)
                else {
                    self.fleet.roe_status = Some(format!(
                        "Fleet #{target_fleet_id} is not in your fleet list."
                    ));
                    return Ok(());
                };
                self.fleet.roe_cursor = index;
                sync_scroll_to_cursor(
                    &mut self.fleet.roe_scroll_offset,
                    self.fleet.roe_cursor,
                    crate::screen::FLEET_VISIBLE_ROWS,
                );
            }
            self.fleet.roe_select_input.clear();
            self.fleet.roe_input.clear();
            self.fleet.roe_status = None;
            self.fleet.roe_editing = true;
            return Ok(());
        }
        let rows = self.fleet_rows();
        let Some(selected_row) = rows.get(self.fleet.roe_cursor) else {
            self.current_screen = ScreenId::FleetMenu;
            return Ok(());
        };
        let parsed = if self.fleet.roe_input.trim().is_empty() {
            selected_row.rules_of_engagement
        } else {
            match self.fleet.roe_input.trim().parse::<u8>() {
                Ok(value) => value,
                Err(_) => {
                    self.fleet.roe_status = Some("Enter an ROE from 0 to 10.".to_string());
                    return Ok(());
                }
            }
        };
        if parsed > 10 {
            self.fleet.roe_status = Some("Enter an ROE from 0 to 10.".to_string());
            return Ok(());
        }
        if let Err(err) = self.game_data.set_fleet_rules_of_engagement(
            self.player.record_index_1_based,
            selected_row.fleet_record_index_1_based,
            parsed,
        ) {
            self.fleet.roe_status = Some(match err {
                ec_data::GameStateMutationError::InvalidFleetPlayerInput {
                    reason:
                        ec_data::FleetPlayerInputValidationError::NonCombatFleetMustUseZeroRoe {
                            ..
                        },
                    ..
                } => "Non-combat fleets must use ROE 0.".to_string(),
                ec_data::GameStateMutationError::InvalidFleetPlayerInput {
                    reason:
                        ec_data::FleetPlayerInputValidationError::RulesOfEngagementOutOfRange { .. },
                    ..
                } => "Enter an ROE from 0 to 10.".to_string(),
                _ => err.to_string(),
            });
            return Ok(());
        }
        self.save_game_data()?;
        self.fleet.roe_input.clear();
        self.fleet.roe_status = None;
        self.fleet.roe_editing = false;
        Ok(())
    }

    pub fn submit_fleet_eta(&mut self) {
        if self.current_screen != ScreenId::FleetEta {
            return;
        }
        let rows = self.fleet_rows();
        let Some(selected_row) = rows.get(self.fleet.eta_cursor) else {
            self.fleet.eta_status = Some("You have no active fleets.".to_string());
            self.fleet.eta_mode = FleetEtaMode::SelectingFleet;
            return;
        };
        match self.fleet.eta_mode {
            FleetEtaMode::SelectingFleet => {
                if !self.fleet.eta_select_input.trim().is_empty() {
                    let target_fleet_id = match self.fleet.eta_select_input.trim().parse::<u16>() {
                        Ok(value) => value,
                        Err(_) => {
                            self.fleet.eta_status =
                                Some("Enter a fleet number from the table.".to_string());
                            return;
                        }
                    };
                    let Some(index) = rows
                        .iter()
                        .position(|row| row.fleet_number == target_fleet_id)
                    else {
                        self.fleet.eta_status =
                            Some("Enter a fleet number from the table.".to_string());
                        return;
                    };
                    self.fleet.eta_cursor = index;
                    sync_scroll_to_cursor(
                        &mut self.fleet.eta_scroll_offset,
                        self.fleet.eta_cursor,
                        crate::screen::FLEET_VISIBLE_ROWS,
                    );
                }
                self.fleet.eta_select_input.clear();
                self.fleet.eta_destination_input.clear();
                self.fleet.eta_include_system_input.clear();
                self.fleet.eta_status = None;
                self.fleet.eta_mode = FleetEtaMode::EnteringDestination;
            }
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
                    ec_engine::map_size_for_player_count(self.game_data.conquest.player_count());
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
                    self.calculate_fleet_eta_message(selected_row, destination, include_system);
                self.fleet.eta_status = Some(result);
                self.fleet.eta_include_system_input.clear();
                self.fleet.eta_mode = FleetEtaMode::ShowingResult;
            }
            FleetEtaMode::ShowingResult => {
                self.fleet.eta_status = None;
                self.fleet.eta_destination_input.clear();
                self.fleet.eta_include_system_input.clear();
                self.fleet.eta_mode = FleetEtaMode::SelectingFleet;
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

    pub fn selected_fleet_roe_id(&self) -> Option<u16> {
        let rows = self.fleet_rows();
        rows.get(self.fleet.roe_cursor).map(|row| row.fleet_number)
    }

    pub fn selected_fleet_eta_id(&self) -> Option<u16> {
        let rows = self.fleet_rows();
        rows.get(self.fleet.eta_cursor).map(|row| row.fleet_number)
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
                rules_of_engagement: fleet.rules_of_engagement(),
                order_label: fleet.standing_order_summary(),
                composition_label: fleet.ship_composition_summary(),
            })
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| {
            left.order_code
                .cmp(&right.order_code)
                .then_with(|| right.fleet_number.cmp(&left.fleet_number))
        });
        rows
    }

    pub(super) fn handle_fleet_roe_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        if self.fleet.roe_editing {
            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::Fleet(FleetAction::OpenRoeSelect)
                }
                KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitRoe),
                KeyCode::Backspace => crate::app::Action::Fleet(FleetAction::BackspaceRoeInput),
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    crate::app::Action::Fleet(FleetAction::AppendRoeChar(ch))
                }
                _ => crate::app::Action::Noop,
            }
        } else {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    crate::app::Action::Fleet(FleetAction::MoveRoeSelect(-1))
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    crate::app::Action::Fleet(FleetAction::MoveRoeSelect(1))
                }
                KeyCode::PageUp => crate::app::Action::Fleet(FleetAction::MoveRoeSelect(-8)),
                KeyCode::PageDown => crate::app::Action::Fleet(FleetAction::MoveRoeSelect(8)),
                KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitRoe),
                KeyCode::Backspace => crate::app::Action::Fleet(FleetAction::BackspaceRoeInput),
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    crate::app::Action::Fleet(FleetAction::AppendRoeChar(ch))
                }
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::Fleet(FleetAction::OpenMenu)
                }
                _ => crate::app::Action::Noop,
            }
        }
    }

    pub(super) fn handle_fleet_review_select_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                crate::app::Action::Fleet(FleetAction::MoveReviewSelect(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                crate::app::Action::Fleet(FleetAction::MoveReviewSelect(1))
            }
            KeyCode::PageUp => crate::app::Action::Fleet(FleetAction::MoveReviewSelect(-8)),
            KeyCode::PageDown => crate::app::Action::Fleet(FleetAction::MoveReviewSelect(8)),
            KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitReviewSelect),
            KeyCode::Backspace => crate::app::Action::Fleet(FleetAction::BackspaceReviewInput),
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                crate::app::Action::Fleet(FleetAction::AppendReviewChar(ch))
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                crate::app::Action::Fleet(FleetAction::OpenMenu)
            }
            _ => crate::app::Action::Noop,
        }
    }

    fn sync_fleet_roe_cursor_to_input(&mut self) {
        if self.current_screen != ScreenId::FleetRoeSelect || self.fleet.roe_editing {
            return;
        }
        let Ok(target_fleet_id) = self.fleet.roe_select_input.trim().parse::<u16>() else {
            return;
        };
        let rows = self.fleet_rows();
        let Some(index) = rows
            .iter()
            .position(|row| row.fleet_number == target_fleet_id)
        else {
            return;
        };
        self.fleet.roe_cursor = index;
        sync_scroll_to_cursor(
            &mut self.fleet.roe_scroll_offset,
            self.fleet.roe_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
    }

    fn sync_fleet_review_cursor_to_input(&mut self) {
        if self.current_screen != ScreenId::FleetReviewSelect {
            return;
        }
        let Ok(target_fleet_id) = self.fleet.review_select_input.trim().parse::<u16>() else {
            return;
        };
        let rows = self.fleet_rows();
        let Some(index) = rows
            .iter()
            .position(|row| row.fleet_number == target_fleet_id)
        else {
            return;
        };
        self.fleet.cursor = index;
        sync_scroll_to_cursor(
            &mut self.fleet.scroll_offset,
            self.fleet.cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
    }

    pub(super) fn handle_fleet_eta_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match self.fleet.eta_mode {
            FleetEtaMode::SelectingFleet => match key.code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    crate::app::Action::Fleet(FleetAction::MoveEtaSelect(-1))
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    crate::app::Action::Fleet(FleetAction::MoveEtaSelect(1))
                }
                KeyCode::PageUp => crate::app::Action::Fleet(FleetAction::MoveEtaSelect(-8)),
                KeyCode::PageDown => crate::app::Action::Fleet(FleetAction::MoveEtaSelect(8)),
                KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitEta),
                KeyCode::Backspace => crate::app::Action::Fleet(FleetAction::BackspaceEtaInput),
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    crate::app::Action::Fleet(FleetAction::AppendEtaChar(ch))
                }
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::Fleet(FleetAction::OpenMenu)
                }
                _ => crate::app::Action::Noop,
            },
            FleetEtaMode::EnteringDestination => match key.code {
                KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitEta),
                KeyCode::Backspace => crate::app::Action::Fleet(FleetAction::BackspaceEtaInput),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::Fleet(FleetAction::OpenEta)
                }
                KeyCode::Char(ch)
                    if ch.is_ascii_digit() || matches!(ch, ',' | ' ' | '(' | ')' | '[' | ']') =>
                {
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
            FleetEtaMode::ShowingResult => match key.code {
                KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    crate::app::Action::Fleet(FleetAction::SubmitEta)
                }
                _ => crate::app::Action::Noop,
            },
        }
    }

    fn sync_fleet_eta_cursor_to_input(&mut self) {
        if self.current_screen != ScreenId::FleetEta
            || self.fleet.eta_mode != FleetEtaMode::SelectingFleet
        {
            return;
        }
        let Ok(target_fleet_id) = self.fleet.eta_select_input.trim().parse::<u16>() else {
            return;
        };
        let rows = self.fleet_rows();
        let Some(index) = rows
            .iter()
            .position(|row| row.fleet_number == target_fleet_id)
        else {
            return;
        };
        self.fleet.eta_cursor = index;
        sync_scroll_to_cursor(
            &mut self.fleet.eta_scroll_offset,
            self.fleet.eta_cursor,
            crate::screen::FLEET_VISIBLE_ROWS,
        );
    }

    pub(crate) fn fleet_eta_default_destination(&self) -> [u8; 2] {
        let rows = self.fleet_rows();
        let Some(row) = rows.get(self.fleet.eta_cursor) else {
            return [8, 2];
        };
        if row.target_coords[0] > 0 && row.target_coords[1] > 0 {
            row.target_coords
        } else {
            row.coords
        }
    }

    pub(crate) fn fleet_group_target_status_line(&self) -> String {
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
            FleetTargetInputKind::Coordinates | FleetTargetInputKind::None => {
                let target = self.fleet_group_default_target();
                format!("{},{}", target[0], target[1])
            }
        }
    }
}
