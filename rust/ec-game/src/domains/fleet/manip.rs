use crate::app::state::App;
use crate::domains::fleet::FleetAction;
use crate::screen::{CommandMenu, FleetDetachMode, FleetRow, FleetTransferMode, ScreenId};
use ec_data::{CoreGameData, FleetDetachSelection};
use ec_engine::{FleetEtaEstimate, estimate_fleet_eta, estimate_fleet_eta_to_destination};

impl App {
    pub(crate) fn submit_inline_fleet_merge(
        &mut self,
        host_fleet_record_index_1_based: usize,
    ) -> Result<(), String> {
        let source_record_index_1_based = self
            .fleet
            .merge_source_record_index_1_based
            .ok_or_else(|| "Select the fleet that will merge first.".to_string())?;
        if source_record_index_1_based == host_fleet_record_index_1_based {
            return Err("Choose a different host fleet.".to_string());
        }
        let host_fleet_number = self
            .fleet_number_for_record_index(host_fleet_record_index_1_based)
            .ok_or_else(|| "Host fleet is no longer available.".to_string())?;
        let source_fleet_number = self
            .fleet_number_for_record_index(source_record_index_1_based)
            .ok_or_else(|| "Selected fleet is no longer available.".to_string())?;
        self.game_data
            .set_join_fleet_order(
                self.player.record_index_1_based,
                source_record_index_1_based,
                host_fleet_record_index_1_based,
            )
            .map_err(|err| err.to_string())?;
        self.save_game_data().map_err(|err| err.to_string())?;
        self.fleet.merge_source_record_index_1_based = None;
        self.show_command_menu_notice(
            CommandMenu::Fleet,
            format!(
                "Fleet #{} ordered to join Fleet #{}.",
                source_fleet_number, host_fleet_number
            ),
        );
        self.clear_fleet_menu_prompt();
        self.current_screen = ScreenId::FleetMenu;
        Ok(())
    }

    pub fn open_fleet_merge(&mut self) {
        let total = self.fleet_rows().len();
        if total < 2 {
            self.show_command_menu_notice(
                CommandMenu::Fleet,
                "You need at least two fleets to merge.",
            );
            return;
        }
        self.fleet.merge_source_record_index_1_based = None;
        self.open_fleet_menu_prompt(
            crate::domains::fleet::state::FleetMenuPromptMode::MergeSource,
            self.strongest_owned_fleet_number()
                .map(|value| value.to_string())
                .unwrap_or_default(),
        );
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
        self.fleet.transfer_status = None;
        self.fleet.transfer_input.clear();
        self.fleet.transfer_donor_record_index_1_based = None;
        self.fleet.transfer_host_record_index_1_based = None;
        self.fleet.transfer_selection = FleetDetachSelection::default();
        self.open_fleet_menu_prompt(
            crate::domains::fleet::state::FleetMenuPromptMode::TransferDonor,
            self.strongest_owned_fleet_number()
                .map(|value| value.to_string())
                .unwrap_or_default(),
        );
    }

    pub fn open_fleet_detach(&mut self) {
        if self.fleet_rows().is_empty() {
            self.show_command_menu_notice(CommandMenu::Fleet, "You have no active fleets.");
            return;
        }
        self.fleet.detach_status = None;
        self.fleet.detach_input.clear();
        self.fleet.detach_selection = FleetDetachSelection::default();
        self.fleet.detach_donor_speed = None;
        self.open_fleet_menu_prompt(
            crate::domains::fleet::state::FleetMenuPromptMode::DetachFleet,
            self.strongest_owned_fleet_number()
                .map(|value| value.to_string())
                .unwrap_or_default(),
        );
    }

    pub(crate) fn open_fleet_transfer_with_selected_records(
        &mut self,
        host_fleet_record_index_1_based: usize,
    ) {
        let Some(donor_record_index_1_based) = self.fleet.transfer_donor_record_index_1_based else {
            self.fleet.menu_prompt_status = Some("Select a donor fleet first.".to_string());
            return;
        };
        self.clear_command_menu_notice();
        self.clear_fleet_menu_prompt();
        self.fleet.transfer_status = None;
        self.fleet.transfer_host_record_index_1_based = Some(host_fleet_record_index_1_based);
        self.fleet.transfer_mode = FleetTransferMode::EnteringBattleships;
        self.fleet.transfer_input.clear();
        self.fleet.transfer_selection = FleetDetachSelection::default();
        if donor_record_index_1_based == host_fleet_record_index_1_based {
            self.fleet.transfer_status = Some("Choose a different host fleet.".to_string());
            self.open_fleet_menu();
            return;
        }
        self.current_screen = ScreenId::FleetTransfer;
    }

    pub(crate) fn open_fleet_detach_with_selected_record(
        &mut self,
        fleet_record_index_1_based: usize,
    ) -> Result<(), String> {
        if self
            .fleet_row_by_record_index(fleet_record_index_1_based)
            .is_none()
        {
            return Err("Selected fleet is no longer available.".to_string());
        }
        self.clear_command_menu_notice();
        self.clear_fleet_menu_prompt();
        self.fleet.detach_status = None;
        self.fleet.detach_input.clear();
        self.fleet.detach_donor_record_index_1_based = Some(fleet_record_index_1_based);
        if self.current_fleet_detach_ship_total(fleet_record_index_1_based) <= 1 {
            return Err("A fleet must contain at least two ships to detach.".to_string());
        }
        self.fleet.detach_selection = FleetDetachSelection::default();
        self.fleet.detach_donor_speed = None;
        self.fleet.detach_mode = self
            .next_fleet_detach_prompt_mode(fleet_record_index_1_based, None)
            .unwrap_or(FleetDetachMode::SettingNewFleetRoe);
        self.current_screen = ScreenId::FleetDetach;
        Ok(())
    }

    pub fn append_fleet_transfer_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::FleetTransfer || !ch.is_ascii_digit() {
            return;
        }
        if self.fleet.transfer_input.len() >= 3 {
            return;
        }
        self.fleet.transfer_input.push(ch);
        self.fleet.transfer_status = None;
    }

    pub fn append_fleet_detach_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::FleetDetach || !ch.is_ascii_digit() {
            return;
        }
        if self.fleet.detach_input.len() >= 3 {
            return;
        }
        self.fleet.detach_input.push(ch);
        self.fleet.detach_status = None;
    }

    pub fn backspace_fleet_transfer_input(&mut self) {
        if self.current_screen != ScreenId::FleetTransfer {
            return;
        }
        self.fleet.transfer_input.pop();
        self.fleet.transfer_status = None;
    }

    pub fn backspace_fleet_detach_input(&mut self) {
        if self.current_screen != ScreenId::FleetDetach {
            return;
        }
        self.fleet.detach_input.pop();
        self.fleet.detach_status = None;
    }

    pub fn submit_fleet_transfer(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::FleetTransfer {
            return Ok(());
        }
        if self.fleet.transfer_donor_record_index_1_based.is_none()
            || self.fleet.transfer_host_record_index_1_based.is_none()
        {
            self.fleet.transfer_status = Some("Select transfer fleets from the command menu.".to_string());
            self.open_fleet_transfer();
            return Ok(());
        }
        let value = if self.fleet.transfer_input.trim().is_empty() {
            0
        } else {
            match self.fleet.transfer_input.trim().parse::<u16>() {
                Ok(value) => value,
                Err(_) => {
                    self.fleet.transfer_status = Some("Enter a number from 0 up.".to_string());
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
        }
        self.fleet.transfer_input.clear();
        Ok(())
    }

    pub fn submit_fleet_detach(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::FleetDetach {
            return Ok(());
        }
        let Some(donor_record_index_1_based) = self.fleet.detach_donor_record_index_1_based else {
            self.current_screen = ScreenId::FleetMenu;
            return Ok(());
        };
        let Some(selected_row) = self.fleet_row_by_record_index(donor_record_index_1_based) else {
            self.current_screen = ScreenId::FleetMenu;
            return Ok(());
        };

        let Some(record) = self
            .game_data
            .fleets
            .records
            .get(donor_record_index_1_based - 1)
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
                if let Some(next_mode) =
                    self.next_fleet_detach_prompt_mode(donor_record_index_1_based, Some(self.fleet.detach_mode))
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
                    donor_record_index_1_based,
                    self.fleet.detach_selection,
                    donor_speed,
                    new_roe,
                )?;
                self.save_game_data()?;
                self.fleet.detach_input.clear();
                self.fleet.detach_status = None;
                self.fleet.detach_selection = FleetDetachSelection::default();
                self.fleet.detach_donor_speed = None;
                self.fleet.detach_donor_record_index_1_based = None;
                self.show_command_menu_notice(
                    CommandMenu::Fleet,
                    format!(
                        "Detached ships from Fleet #{} into a new fleet.",
                        selected_row.fleet_number
                    ),
                );
                self.current_screen = ScreenId::FleetMenu;
            }
        }
        Ok(())
    }

    pub(crate) fn handle_fleet_detach_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitDetach),
            KeyCode::Backspace => crate::app::Action::Fleet(FleetAction::BackspaceDetachInput),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                crate::app::Action::Fleet(FleetAction::OpenDetach)
            }
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                crate::app::Action::Fleet(FleetAction::AppendDetachChar(ch))
            }
            _ => crate::app::Action::Noop,
        }
    }

    pub(crate) fn handle_fleet_transfer_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitTransfer),
            KeyCode::Backspace => crate::app::Action::Fleet(FleetAction::BackspaceTransferInput),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                crate::app::Action::Fleet(FleetAction::OpenTransfer)
            }
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                crate::app::Action::Fleet(FleetAction::AppendTransferChar(ch))
            }
            _ => crate::app::Action::Noop,
        }
    }

    pub(crate) fn fleet_detach_prompt_and_default(&self) -> (String, String) {
        let fleet_number = self
            .fleet
            .detach_donor_record_index_1_based
            .and_then(|idx| self.fleet_number_for_record_index(idx))
            .unwrap_or(1);
        match self.fleet.detach_mode {
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
                format!("Fleet #{} new speed ", fleet_number),
                self.fleet_detach_donor_default_speed().to_string(),
            ),
            FleetDetachMode::SettingNewFleetRoe => ("New fleet ROE ".to_string(), "6".to_string()),
        }
    }

    fn current_fleet_detach_ship_total(&self, fleet_record_index_1_based: usize) -> u32 {
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

    fn next_fleet_detach_prompt_mode(
        &self,
        fleet_record_index_1_based: usize,
        current: Option<FleetDetachMode>,
    ) -> Option<FleetDetachMode> {
        let fleet = self
            .game_data
            .fleets
            .records
            .get(fleet_record_index_1_based - 1)?;
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
            None => 0,
            Some(FleetDetachMode::EnteringBattleships) => 1,
            Some(FleetDetachMode::EnteringCruisers) => 2,
            Some(FleetDetachMode::EnteringDestroyers) => 3,
            Some(FleetDetachMode::EnteringFullTransports) => 4,
            Some(FleetDetachMode::EnteringEmptyTransports) => 5,
            Some(FleetDetachMode::EnteringScouts) => 6,
            Some(FleetDetachMode::EnteringEtacs)
            | Some(FleetDetachMode::AdjustingDonorSpeed)
            | Some(FleetDetachMode::SettingNewFleetRoe) => modes.len(),
        };
        modes
            .iter()
            .skip(start_idx)
            .find_map(|(mode, include)| (*include).then_some(*mode))
    }

    fn fleet_detach_requires_speed_prompt(&self) -> bool {
        let Some(fleet_record_index_1_based) = self.fleet.detach_donor_record_index_1_based else {
            return false;
        };
        let Some(fleet) = self
            .game_data
            .fleets
            .records
            .get(fleet_record_index_1_based - 1)
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
        let Some(fleet_record_index_1_based) = self.fleet.detach_donor_record_index_1_based else {
            return 1;
        };
        let Some(fleet) = self
            .game_data
            .fleets
            .records
            .get(fleet_record_index_1_based - 1)
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

    pub(crate) fn fleet_number_for_record_index(&self, record_index_1_based: usize) -> Option<u16> {
        self.game_data
            .fleets
            .records
            .get(record_index_1_based - 1)
            .map(|fleet| fleet.local_slot_word_raw())
    }

    pub(crate) fn fleet_transfer_donor_row(&self) -> Option<crate::screen::FleetRow> {
        self.fleet
            .transfer_donor_record_index_1_based
            .and_then(|idx| self.fleet_row_by_record_index(idx))
    }

    pub(crate) fn fleet_transfer_host_row(&self) -> Option<crate::screen::FleetRow> {
        self.fleet
            .transfer_host_record_index_1_based
            .and_then(|idx| self.fleet_row_by_record_index(idx))
    }

    pub(crate) fn fleet_detach_donor_row(&self) -> Option<crate::screen::FleetRow> {
        self.fleet
            .detach_donor_record_index_1_based
            .and_then(|idx| self.fleet_row_by_record_index(idx))
    }

    pub(crate) fn fleet_transfer_prompt_and_default(&self) -> (String, String) {
        match self.fleet.transfer_mode {
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
        self.fleet.transfer_mode = FleetTransferMode::EnteringBattleships;
        self.fleet.transfer_donor_record_index_1_based = None;
        self.fleet.transfer_host_record_index_1_based = None;
        self.fleet.transfer_input.clear();
        self.fleet.transfer_selection = FleetDetachSelection::default();
        self.show_command_menu_notice(
            CommandMenu::Fleet,
            format!(
                "Transferred ships from Fleet #{} to Fleet #{}.",
                donor_fleet_number, host_fleet_number
            ),
        );
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
