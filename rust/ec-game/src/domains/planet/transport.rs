use crate::app::helpers::{
    center_scroll_to_cursor, resolve_default_coords_input, sync_scroll_to_cursor,
};
use crate::app::state::App;
use crate::domains::fleet::state::FleetMenuPromptMode;
use crate::domains::planet::PlanetAction;
use crate::domains::planet::state::PlanetMenuTransportPromptMode;
use crate::screen::{
    CommandMenu, PlanetTransportFleetRow, PlanetTransportMode, PlanetTransportPlanetRow, ScreenId,
    format_sector_coords, format_sector_coords_default,
};
use ec_data::GameStateMutationError;
use std::cmp::Reverse;

impl App {
    fn owned_planet_row_at_coords(
        &self,
        coords: [u8; 2],
    ) -> Option<ec_data::EmpirePlanetEconomyRow> {
        self.build_planet_rows().into_iter().find(|row| {
            row.coords == coords
                && self
                    .game_data
                    .planets
                    .records
                    .get(row.planet_record_index_1_based - 1)
                    .map(|planet| {
                        planet.owner_empire_slot_raw() as usize == self.player.record_index_1_based
                    })
                    .unwrap_or(false)
        })
    }

    fn owned_planet_row_by_record(
        &self,
        planet_record_index_1_based: usize,
    ) -> Option<ec_data::EmpirePlanetEconomyRow> {
        self.build_planet_rows().into_iter().find(|row| {
            row.planet_record_index_1_based == planet_record_index_1_based
                && self
                    .game_data
                    .planets
                    .records
                    .get(row.planet_record_index_1_based - 1)
                    .map(|planet| {
                        planet.owner_empire_slot_raw() as usize == self.player.record_index_1_based
                    })
                    .unwrap_or(false)
        })
    }

    pub(crate) fn clear_planet_transport_prompt(&mut self) {
        self.planet.transport_prompt_mode = None;
        self.planet.transport_prompt_input.clear();
        self.planet.transport_prompt_default_value.clear();
        self.planet.transport_status = None;
    }

    fn open_planet_transport_menu_prompt(
        &mut self,
        mode: PlanetMenuTransportPromptMode,
        default_value: impl Into<String>,
    ) {
        self.clear_command_menu_notice();
        self.current_screen = ScreenId::PlanetMenu;
        self.planet.transport_prompt_mode = Some(mode);
        self.planet.transport_prompt_input.clear();
        self.planet.transport_prompt_default_value = default_value.into();
        self.planet.transport_status = None;
    }

    pub(crate) fn planet_transport_prompt_label(&self) -> Option<String> {
        Some(match self.planet.transport_prompt_mode? {
            PlanetMenuTransportPromptMode::Planet(PlanetTransportMode::Load) => {
                "Load Planet XX ".to_string()
            }
            PlanetMenuTransportPromptMode::Planet(PlanetTransportMode::Unload) => {
                "Unload Planet XX ".to_string()
            }
            PlanetMenuTransportPromptMode::Fleet(PlanetTransportMode::Load) => {
                "Load Fleet # ".to_string()
            }
            PlanetMenuTransportPromptMode::Fleet(PlanetTransportMode::Unload) => {
                "Unload Fleet # ".to_string()
            }
            PlanetMenuTransportPromptMode::Quantity(PlanetTransportMode::Load) => {
                "How many armies to load? ".to_string()
            }
            PlanetMenuTransportPromptMode::Quantity(PlanetTransportMode::Unload) => {
                "How many armies to unload? ".to_string()
            }
        })
    }

    fn default_planet_transport_planet_coords(&self, mode: PlanetTransportMode) -> Option<[u8; 2]> {
        self.planet_transport_planet_rows(mode)
            .into_iter()
            .max_by_key(|row| {
                (
                    row.transport_capacity,
                    Reverse((row.coords[0], row.coords[1])),
                )
            })
            .map(|row| row.coords)
    }

    fn default_planet_transport_fleet_number_for_planet(
        &self,
        mode: PlanetTransportMode,
        planet: &ec_data::EmpirePlanetEconomyRow,
    ) -> Option<u16> {
        self.game_data
            .fleets
            .records
            .iter()
            .filter(|fleet| {
                fleet.owner_empire_raw() as usize == self.player.record_index_1_based
                    && fleet.current_location_coords_raw() == planet.coords
                    && fleet.troop_transport_count() > 0
                    && self.transport_available_qty(mode, fleet, planet) > 0
            })
            .max_by_key(|fleet| {
                let ranking_qty = match mode {
                    PlanetTransportMode::Load => fleet
                        .troop_transport_count()
                        .saturating_sub(fleet.army_count()),
                    PlanetTransportMode::Unload => fleet.army_count(),
                };
                (ranking_qty, Reverse(fleet.local_slot_word_raw()))
            })
            .map(|fleet| fleet.local_slot_word_raw())
    }

    pub fn open_planet_transport_prompt(&mut self, mode: PlanetTransportMode) {
        self.command_return_menu = CommandMenu::Planet;
        self.close_planet_tax_prompt();
        self.close_planet_info_prompt();
        self.close_planet_auto_commission_prompt();
        self.planet.transport_mode = Some(mode);
        self.planet.transport_selected_planet_record = None;
        self.planet.transport_selected_fleet_record = None;
        self.planet.transport_fleet_first = false;
        self.planet.transport_qty_input.clear();
        self.planet.transport_status = None;
        let Some(default_coords) = self.default_planet_transport_planet_coords(mode) else {
            self.show_command_menu_notice(
                CommandMenu::Planet,
                match mode {
                    PlanetTransportMode::Load => {
                        "No planets have armies and troop transports ready to load."
                    }
                    PlanetTransportMode::Unload => {
                        "No fleets have loaded armies ready to unload onto planets with free capacity."
                    }
                },
            );
            return;
        };
        self.open_planet_transport_menu_prompt(
            PlanetMenuTransportPromptMode::Planet(mode),
            format_sector_coords_default(default_coords),
        );
    }

    pub fn open_fleet_transport_prompt(&mut self, mode: PlanetTransportMode) {
        self.command_return_menu = CommandMenu::Fleet;
        self.planet.transport_mode = Some(mode);
        self.planet.transport_selected_planet_record = None;
        self.planet.transport_selected_fleet_record = None;
        self.planet.transport_fleet_first = false;
        self.planet.transport_qty_input.clear();
        self.planet.transport_status = None;
        if self.planet_transport_planet_rows(mode).is_empty() {
            self.show_command_menu_notice(
                CommandMenu::Fleet,
                match mode {
                    PlanetTransportMode::Load => {
                        "No planets have armies and troop transports ready to load."
                    }
                    PlanetTransportMode::Unload => {
                        "No fleets have loaded armies ready to unload onto planets with free capacity."
                    }
                },
            );
            return;
        }
        self.open_fleet_menu_prompt(
            FleetMenuPromptMode::TransportFleet(mode),
            self.default_fleet_transport_fleet_number(mode)
                .map(|value| value.to_string())
                .unwrap_or_default(),
        );
    }

    pub fn open_planet_transport_planet_select(&mut self, mode: PlanetTransportMode) {
        self.command_return_menu = CommandMenu::Planet;
        self.open_transport_planet_select(mode);
    }

    pub fn open_fleet_transport_planet_select(&mut self, mode: PlanetTransportMode) {
        self.command_return_menu = CommandMenu::Fleet;
        self.open_transport_planet_select(mode);
    }

    fn open_transport_planet_select(&mut self, mode: PlanetTransportMode) {
        self.planet.transport_mode = Some(mode);
        self.planet.transport_planet_cursor = 0;
        self.planet.transport_planet_scroll_offset = 0;
        self.planet.transport_selected_planet_record = None;
        self.planet.transport_selected_fleet_record = None;
        self.planet.transport_fleet_first = false;
        self.planet.transport_planet_input.clear();
        self.planet.transport_fleet_cursor = 0;
        self.planet.transport_fleet_scroll_offset = 0;
        self.planet.transport_qty_input.clear();
        self.planet.transport_status = None;
        if self.planet_transport_planet_rows(mode).is_empty() {
            self.show_command_menu_notice(
                self.command_return_menu,
                match mode {
                    PlanetTransportMode::Load => {
                        "No planets have armies and troop transports ready to load."
                    }
                    PlanetTransportMode::Unload => {
                        "No fleets have loaded armies ready to unload onto planets with free capacity."
                    }
                },
            );
        } else {
            self.clear_command_menu_notice();
            self.current_screen = ScreenId::PlanetTransportPlanetSelect(mode);
        }
    }

    fn owned_planet_row_for_fleet(
        &self,
        fleet: &ec_data::FleetRecord,
    ) -> Option<ec_data::EmpirePlanetEconomyRow> {
        self.owned_planet_row_at_coords(fleet.current_location_coords_raw())
    }

    fn default_fleet_transport_fleet_number(&self, mode: PlanetTransportMode) -> Option<u16> {
        self.game_data
            .fleets
            .records
            .iter()
            .filter(|fleet| fleet.owner_empire_raw() as usize == self.player.record_index_1_based)
            .filter_map(|fleet| {
                let planet = self.owned_planet_row_for_fleet(fleet)?;
                let ranking_qty = match mode {
                    PlanetTransportMode::Load => fleet
                        .troop_transport_count()
                        .saturating_sub(fleet.army_count()),
                    PlanetTransportMode::Unload => fleet.army_count(),
                };
                if ranking_qty == 0 {
                    return None;
                }
                if self.transport_available_qty(mode, fleet, &planet) == 0 {
                    return None;
                }
                Some((ranking_qty, Reverse(fleet.local_slot_word_raw())))
            })
            .max()
            .map(|(_, fleet_number)| fleet_number.0)
    }

    fn transport_available_qty(
        &self,
        mode: PlanetTransportMode,
        fleet: &ec_data::FleetRecord,
        planet: &ec_data::EmpirePlanetEconomyRow,
    ) -> u16 {
        match mode {
            PlanetTransportMode::Load => fleet
                .troop_transport_count()
                .saturating_sub(fleet.army_count())
                .min(u16::from(planet.armies)),
            PlanetTransportMode::Unload => fleet
                .army_count()
                .min(u16::from(u8::MAX.saturating_sub(planet.armies))),
        }
    }

    fn resolve_planet_transport_planet_selection(
        &self,
        mode: PlanetTransportMode,
        coords: [u8; 2],
    ) -> Result<ec_data::EmpirePlanetEconomyRow, String> {
        let Some(planet) = self.owned_planet_row_at_coords(coords) else {
            return Err(format!(
                "Planet [{},{}] is not one of your worlds.",
                coords[0], coords[1]
            ));
        };
        let fleets = self.planet_transport_fleet_rows_for_planet(mode, &planet);
        if fleets.is_empty() {
            return Err("No troop transports are present at that world.".to_string());
        }
        match mode {
            PlanetTransportMode::Load => {
                if planet.armies == 0 {
                    return Err("That world has no armies available to load.".to_string());
                }
                if fleets.iter().all(|fleet| fleet.available_qty == 0) {
                    return Err("All troop transports at that world are already full.".to_string());
                }
            }
            PlanetTransportMode::Unload => {
                if planet.armies == u8::MAX {
                    return Err("That world has no room to receive unloaded armies.".to_string());
                }
                if fleets.iter().all(|fleet| fleet.available_qty == 0) {
                    return Err("All troop transports at that world are already empty.".to_string());
                }
            }
        }
        Ok(planet)
    }

    fn resolve_planet_transport_fleet_selection(
        &self,
        mode: PlanetTransportMode,
        fleet_number: u16,
    ) -> Result<PlanetTransportFleetRow, String> {
        let planet = self
            .planet
            .transport_selected_planet_record
            .and_then(|record| self.owned_planet_row_by_record(record))
            .ok_or_else(|| "Select a planet first.".to_string())?;
        let fleet = self
            .game_data
            .fleets
            .records
            .iter()
            .enumerate()
            .find(|(_, fleet)| {
                fleet.owner_empire_raw() as usize == self.player.record_index_1_based
                    && fleet.local_slot_word_raw() == fleet_number
            })
            .ok_or_else(|| "Enter one of your fleet numbers.".to_string())?;
        let fleet_record_index_1_based = fleet.0 + 1;
        let fleet = fleet.1;
        if fleet.current_location_coords_raw() != planet.coords {
            return Err(format!(
                "Fleet #{} is not at {}.",
                fleet_number,
                format_sector_coords(planet.coords)
            ));
        }
        if fleet.troop_transport_count() == 0 {
            return Err("That fleet has no troop transports.".to_string());
        }
        match mode {
            PlanetTransportMode::Load => {
                if fleet
                    .troop_transport_count()
                    .saturating_sub(fleet.army_count())
                    == 0
                {
                    return Err("That fleet's troop transports are already full.".to_string());
                }
                if planet.armies == 0 {
                    return Err("That world has no armies available to load.".to_string());
                }
            }
            PlanetTransportMode::Unload => {
                if fleet.army_count() == 0 {
                    return Err("That fleet's troop transports are already empty.".to_string());
                }
                if planet.armies == u8::MAX {
                    return Err("That world has no room to receive unloaded armies.".to_string());
                }
            }
        }
        let available_qty = self.transport_available_qty(mode, fleet, &planet);
        if available_qty == 0 {
            return Err(match mode {
                PlanetTransportMode::Load => {
                    "That fleet cannot load any armies from that world right now.".to_string()
                }
                PlanetTransportMode::Unload => {
                    "That fleet cannot unload any armies to that world right now.".to_string()
                }
            });
        }
        Ok(PlanetTransportFleetRow {
            fleet_record_index_1_based,
            fleet_number: fleet.local_slot_word_raw(),
            troop_transports: fleet.troop_transport_count(),
            loaded_armies: fleet.army_count(),
            available_qty,
        })
    }

    pub(crate) fn open_fleet_transport_quantity_prompt(
        &mut self,
        mode: PlanetTransportMode,
        fleet_record_index_1_based: usize,
    ) -> Result<(), String> {
        let fleet = self
            .game_data
            .fleets
            .records
            .get(fleet_record_index_1_based - 1)
            .ok_or_else(|| "Selected fleet is no longer available.".to_string())?;
        if fleet.owner_empire_raw() as usize != self.player.record_index_1_based {
            return Err("Enter one of your fleet numbers.".to_string());
        }
        let planet = self
            .owned_planet_row_for_fleet(fleet)
            .ok_or_else(|| "That fleet is not at one of your worlds.".to_string())?;
        if fleet.troop_transport_count() == 0 {
            return Err("That fleet has no troop transports.".to_string());
        }
        match mode {
            PlanetTransportMode::Load => {
                if fleet
                    .troop_transport_count()
                    .saturating_sub(fleet.army_count())
                    == 0
                {
                    return Err("That fleet's troop transports are already full.".to_string());
                }
                if planet.armies == 0 {
                    return Err("That world has no armies available to load.".to_string());
                }
            }
            PlanetTransportMode::Unload => {
                if fleet.army_count() == 0 {
                    return Err("That fleet's troop transports are already empty.".to_string());
                }
                if planet.armies == u8::MAX {
                    return Err("That world has no room to receive unloaded armies.".to_string());
                }
            }
        }
        let max_qty = self.transport_available_qty(mode, fleet, &planet);
        if max_qty == 0 {
            return Err(match mode {
                PlanetTransportMode::Load => {
                    "That fleet cannot load any armies from that world right now.".to_string()
                }
                PlanetTransportMode::Unload => {
                    "That fleet cannot unload any armies to that world right now.".to_string()
                }
            });
        }
        self.command_return_menu = CommandMenu::Fleet;
        self.planet.transport_mode = Some(mode);
        self.planet.transport_selected_fleet_record = Some(fleet_record_index_1_based);
        self.planet.transport_selected_planet_record = Some(planet.planet_record_index_1_based);
        self.planet.transport_fleet_first = true;
        self.planet.transport_qty_input.clear();
        self.planet.transport_status = None;
        self.fleet.menu_prompt_context_fleet_record_index_1_based =
            Some(fleet_record_index_1_based);
        self.open_fleet_menu_prompt(
            FleetMenuPromptMode::TransportQuantity(mode),
            max_qty.to_string(),
        );
        Ok(())
    }

    pub(crate) fn handle_planet_transport_prompt_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        let mode = self.planet.transport_prompt_mode;
        match key.code {
            KeyCode::Enter => crate::app::Action::Planet(PlanetAction::SubmitTransportPrompt),
            KeyCode::Backspace => {
                crate::app::Action::Planet(PlanetAction::BackspaceTransportPromptInput)
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                crate::app::Action::Planet(PlanetAction::CancelTransportPrompt)
            }
            KeyCode::Char(ch)
                if match mode {
                    Some(PlanetMenuTransportPromptMode::Planet(_)) => {
                        ch.is_ascii_digit() || matches!(ch, ',' | ' ' | '(' | ')' | '[' | ']')
                    }
                    Some(PlanetMenuTransportPromptMode::Fleet(_))
                    | Some(PlanetMenuTransportPromptMode::Quantity(_)) => ch.is_ascii_digit(),
                    None => false,
                } =>
            {
                crate::app::Action::Planet(PlanetAction::AppendTransportPromptChar(ch))
            }
            _ => crate::app::Action::Noop,
        }
    }

    pub fn append_planet_transport_prompt_char(&mut self, ch: char) {
        if !self.inline_planet_transport_prompt_active_on_current_screen() {
            return;
        }
        if self.planet.transport_prompt_input.len() < 16 {
            self.planet.transport_prompt_input.push(ch);
            self.planet.transport_status = None;
        }
    }

    pub fn backspace_planet_transport_prompt_input(&mut self) {
        if !self.inline_planet_transport_prompt_active_on_current_screen() {
            return;
        }
        self.planet.transport_prompt_input.pop();
        self.planet.transport_status = None;
    }

    pub fn cancel_planet_transport_prompt(&mut self) {
        let Some(prompt_mode) = self.planet.transport_prompt_mode else {
            return;
        };
        match prompt_mode {
            PlanetMenuTransportPromptMode::Planet(_) => {
                self.clear_planet_transport_prompt();
                self.planet.transport_mode = None;
                self.planet.transport_selected_planet_record = None;
                self.planet.transport_selected_fleet_record = None;
                self.planet.transport_qty_input.clear();
                self.current_screen = ScreenId::PlanetMenu;
            }
            PlanetMenuTransportPromptMode::Fleet(mode) => {
                let Some(planet) =
                    self.planet
                        .transport_selected_planet_record
                        .and_then(|record| {
                            self.build_planet_rows()
                                .into_iter()
                                .find(|row| row.planet_record_index_1_based == record)
                        })
                else {
                    self.clear_planet_transport_prompt();
                    self.current_screen = ScreenId::PlanetMenu;
                    return;
                };
                self.planet.transport_selected_fleet_record = None;
                self.open_planet_transport_menu_prompt(
                    PlanetMenuTransportPromptMode::Planet(mode),
                    format_sector_coords_default(planet.coords),
                );
            }
            PlanetMenuTransportPromptMode::Quantity(mode) => {
                let Some(planet_record) = self.planet.transport_selected_planet_record else {
                    self.clear_planet_transport_prompt();
                    self.current_screen = ScreenId::PlanetMenu;
                    return;
                };
                let Some(base_planet) = self
                    .build_planet_rows()
                    .into_iter()
                    .find(|row| row.planet_record_index_1_based == planet_record)
                else {
                    self.clear_planet_transport_prompt();
                    self.current_screen = ScreenId::PlanetMenu;
                    return;
                };
                let default_fleet = self
                    .planet
                    .transport_selected_fleet_record
                    .and_then(|record| {
                        self.game_data
                            .fleets
                            .records
                            .get(record - 1)
                            .map(|fleet| fleet.local_slot_word_raw())
                    })
                    .or_else(|| {
                        self.default_planet_transport_fleet_number_for_planet(mode, &base_planet)
                    })
                    .map(|value| value.to_string())
                    .unwrap_or_default();
                self.open_planet_transport_menu_prompt(
                    PlanetMenuTransportPromptMode::Fleet(mode),
                    default_fleet,
                );
            }
        }
    }

    pub fn submit_planet_transport_prompt(&mut self) {
        let Some(prompt_mode) = self.planet.transport_prompt_mode else {
            return;
        };
        match prompt_mode {
            PlanetMenuTransportPromptMode::Planet(mode) => {
                let raw = if self.planet.transport_prompt_input.trim().is_empty() {
                    self.planet
                        .transport_prompt_default_value
                        .trim()
                        .to_string()
                } else {
                    self.planet.transport_prompt_input.trim().to_string()
                };
                let default_coords = self
                    .default_planet_transport_planet_coords(mode)
                    .unwrap_or_else(|| self.default_planet_prompt_coords());
                let Some(coords) = resolve_default_coords_input(&raw, default_coords) else {
                    self.planet.transport_status = Some("Enter coordinates like 5,2".to_string());
                    return;
                };
                match self.resolve_planet_transport_planet_selection(mode, coords) {
                    Ok(planet) => {
                        self.planet.transport_selected_planet_record =
                            Some(planet.planet_record_index_1_based);
                        self.planet.transport_selected_fleet_record = None;
                        let default_fleet = self
                            .default_planet_transport_fleet_number_for_planet(mode, &planet)
                            .map(|value| value.to_string())
                            .unwrap_or_default();
                        self.open_planet_transport_menu_prompt(
                            PlanetMenuTransportPromptMode::Fleet(mode),
                            default_fleet,
                        );
                    }
                    Err(err) => self.planet.transport_status = Some(err),
                }
            }
            PlanetMenuTransportPromptMode::Fleet(mode) => {
                let raw = if self.planet.transport_prompt_input.trim().is_empty() {
                    self.planet
                        .transport_prompt_default_value
                        .trim()
                        .to_string()
                } else {
                    self.planet.transport_prompt_input.trim().to_string()
                };
                let fleet_number = match raw.parse::<u16>() {
                    Ok(value) => value,
                    Err(_) => {
                        self.planet.transport_status =
                            Some("Enter one of your fleet numbers.".to_string());
                        return;
                    }
                };
                match self.resolve_planet_transport_fleet_selection(mode, fleet_number) {
                    Ok(fleet) => {
                        self.planet.transport_selected_fleet_record =
                            Some(fleet.fleet_record_index_1_based);
                        self.planet.transport_qty_input.clear();
                        self.open_planet_transport_menu_prompt(
                            PlanetMenuTransportPromptMode::Quantity(mode),
                            fleet.available_qty.to_string(),
                        );
                    }
                    Err(err) => self.planet.transport_status = Some(err),
                }
            }
            PlanetMenuTransportPromptMode::Quantity(_) => {
                self.planet.transport_qty_input = self.planet.transport_prompt_input.clone();
                self.planet.transport_status = None;
                if let Err(err) = self.submit_planet_transport_qty() {
                    self.planet.transport_status = Some(err.to_string());
                }
            }
        }
    }

    pub fn move_planet_transport_planet(&mut self, delta: i8) {
        let ScreenId::PlanetTransportPlanetSelect(mode) = self.current_screen else {
            return;
        };
        let total = self.planet_transport_planet_rows(mode).len();
        if total == 0 {
            self.planet.transport_planet_cursor = 0;
            return;
        }
        let next = self.planet.transport_planet_cursor as isize + delta as isize;
        self.planet.transport_planet_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.planet.transport_planet_scroll_offset,
            self.planet.transport_planet_cursor,
            crate::screen::PLANET_TRANSPORT_VISIBLE_ROWS,
        );
        self.planet.transport_planet_input.clear();
        self.planet.transport_status = None;
    }

    pub fn confirm_planet_transport_planet(&mut self) {
        let ScreenId::PlanetTransportPlanetSelect(mode) = self.current_screen else {
            return;
        };
        let Some(selected_planet) = self
            .planet_transport_planet_rows(mode)
            .get(self.planet.transport_planet_cursor)
            .cloned()
        else {
            return;
        };
        self.planet.transport_selected_planet_record =
            Some(selected_planet.planet_record_index_1_based);
        self.planet.transport_fleet_cursor = 0;
        self.planet.transport_fleet_scroll_offset = 0;
        self.planet.transport_qty_input.clear();
        self.planet.transport_status = None;
        if self
            .current_planet_transport_fleet_rows(mode)
            .unwrap_or_default()
            .is_empty()
        {
            self.planet.transport_status = Some(match mode {
                PlanetTransportMode::Load => "No fleets here can take more armies.".to_string(),
                PlanetTransportMode::Unload => "No fleets here have loaded armies.".to_string(),
            });
            self.current_screen = ScreenId::PlanetTransportPlanetSelect(mode);
        } else {
            self.current_screen = ScreenId::PlanetTransportFleetSelect(mode);
        }
    }

    pub fn append_planet_transport_planet_char(&mut self, ch: char) {
        if matches!(
            self.current_screen,
            ScreenId::PlanetTransportPlanetSelect(PlanetTransportMode::Load)
                | ScreenId::PlanetTransportPlanetSelect(PlanetTransportMode::Unload)
        ) && self.planet.transport_planet_input.len() < 16
        {
            self.planet.transport_planet_input.push(ch);
            if let ScreenId::PlanetTransportPlanetSelect(mode) = self.current_screen {
                self.sync_planet_transport_planet_cursor_to_input(mode);
            }
            self.planet.transport_status = None;
        }
    }

    pub fn backspace_planet_transport_planet_input(&mut self) {
        if matches!(
            self.current_screen,
            ScreenId::PlanetTransportPlanetSelect(PlanetTransportMode::Load)
                | ScreenId::PlanetTransportPlanetSelect(PlanetTransportMode::Unload)
        ) {
            self.planet.transport_planet_input.pop();
            if let ScreenId::PlanetTransportPlanetSelect(mode) = self.current_screen {
                self.sync_planet_transport_planet_cursor_to_input(mode);
            }
            self.planet.transport_status = None;
        }
    }

    pub fn submit_planet_transport_planet(&mut self) {
        let ScreenId::PlanetTransportPlanetSelect(mode) = self.current_screen else {
            return;
        };
        if self.planet.transport_planet_input.trim().is_empty() {
            self.confirm_planet_transport_planet();
            return;
        }
        let Some(coords) = resolve_default_coords_input(
            &self.planet.transport_planet_input,
            self.planet_transport_planet_default_coords(mode),
        ) else {
            self.planet.transport_status = Some("Enter coordinates like 5,2".to_string());
            return;
        };
        let rows = self.planet_transport_planet_rows(mode);
        let Some(index) = rows.iter().position(|row| row.coords == coords) else {
            self.planet.transport_status = Some(format!(
                "No eligible planet found at [{},{}].",
                coords[0], coords[1]
            ));
            return;
        };
        self.planet.transport_planet_cursor = index;
        center_scroll_to_cursor(
            &mut self.planet.transport_planet_scroll_offset,
            self.planet.transport_planet_cursor,
            crate::screen::PLANET_TRANSPORT_VISIBLE_ROWS,
            rows.len(),
        );
        self.planet.transport_planet_input.clear();
        self.planet.transport_status = None;
        self.confirm_planet_transport_planet();
    }

    pub fn move_planet_transport_fleet(&mut self, delta: i8) {
        let ScreenId::PlanetTransportFleetSelect(mode) = self.current_screen else {
            return;
        };
        let total = self
            .current_planet_transport_fleet_rows(mode)
            .map(|rows| rows.len())
            .unwrap_or(0);
        if total == 0 {
            self.planet.transport_fleet_cursor = 0;
            return;
        }
        let next = self.planet.transport_fleet_cursor as isize + delta as isize;
        self.planet.transport_fleet_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.planet.transport_fleet_scroll_offset,
            self.planet.transport_fleet_cursor,
            crate::screen::PLANET_TRANSPORT_VISIBLE_ROWS,
        );
        self.planet.transport_qty_input.clear();
        self.planet.transport_status = None;
    }

    pub fn confirm_planet_transport_fleet(&mut self) {
        let ScreenId::PlanetTransportFleetSelect(mode) = self.current_screen else {
            return;
        };
        self.planet.transport_qty_input.clear();
        self.planet.transport_status = None;
        self.current_screen = ScreenId::PlanetTransportFleetSelect(mode);
    }

    pub fn append_planet_transport_qty_char(&mut self, ch: char) {
        if matches!(
            self.current_screen,
            ScreenId::PlanetTransportFleetSelect(_) | ScreenId::PlanetTransportQuantityPrompt(_)
        ) && self.planet.transport_qty_input.len() < 3
        {
            self.planet.transport_qty_input.push(ch);
            self.planet.transport_status = None;
        }
    }

    pub fn backspace_planet_transport_qty(&mut self) {
        if matches!(
            self.current_screen,
            ScreenId::PlanetTransportFleetSelect(_) | ScreenId::PlanetTransportQuantityPrompt(_)
        ) {
            self.planet.transport_qty_input.pop();
            self.planet.transport_status = None;
        }
    }

    pub fn submit_planet_transport_qty(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let inline_fleet_menu_mode = if self.current_screen == ScreenId::FleetMenu {
            match self.fleet.menu_prompt_mode {
                Some(FleetMenuPromptMode::TransportQuantity(mode)) => Some(mode),
                _ => None,
            }
        } else {
            None
        };
        let inline_planet_menu_mode = if self.current_screen == ScreenId::PlanetMenu {
            match self.planet.transport_prompt_mode {
                Some(PlanetMenuTransportPromptMode::Quantity(mode)) => Some(mode),
                _ => None,
            }
        } else {
            None
        };
        let mode = match self.current_screen {
            ScreenId::PlanetTransportFleetSelect(mode)
            | ScreenId::PlanetTransportQuantityPrompt(mode) => mode,
            ScreenId::FleetMenu => inline_fleet_menu_mode.ok_or("transport mode missing")?,
            ScreenId::PlanetMenu => inline_planet_menu_mode.ok_or("transport mode missing")?,
            _ => return Ok(()),
        };
        let inline_fleet_menu = inline_fleet_menu_mode == Some(mode);
        let inline_planet_menu = inline_planet_menu_mode == Some(mode);
        let inline_menu = inline_fleet_menu || inline_planet_menu;
        let fleet = self.current_planet_transport_fleet_row(mode)?;
        let max_qty = fleet.available_qty;
        if max_qty == 0 {
            self.planet.transport_status = Some(match mode {
                PlanetTransportMode::Load => {
                    format!("Fleet {} cannot take any more armies.", fleet.fleet_number)
                }
                PlanetTransportMode::Unload => {
                    format!(
                        "Fleet {} has no loaded armies to unload.",
                        fleet.fleet_number
                    )
                }
            });
            if !inline_menu {
                self.current_screen = ScreenId::PlanetTransportFleetSelect(mode);
            }
            return Ok(());
        }
        let qty = if self.planet.transport_qty_input.trim().is_empty() {
            max_qty
        } else {
            match self.planet.transport_qty_input.trim().parse::<u16>() {
                Ok(value) if value > 0 => value,
                _ => {
                    self.planet.transport_status = Some("Enter a positive army count.".to_string());
                    return Ok(());
                }
            }
        };
        if qty > max_qty {
            self.planet.transport_status = Some(format!("Enter a value from 1 to {max_qty}."));
            return Ok(());
        }
        let planet = self.current_planet_transport_planet_row(mode)?;
        let result = match mode {
            PlanetTransportMode::Load => self.game_data.load_planet_armies_onto_fleet(
                self.player.record_index_1_based,
                planet.planet_record_index_1_based,
                fleet.fleet_record_index_1_based,
                qty,
            ),
            PlanetTransportMode::Unload => self.game_data.unload_fleet_armies_to_planet(
                self.player.record_index_1_based,
                planet.planet_record_index_1_based,
                fleet.fleet_record_index_1_based,
                qty,
            ),
        };
        match result {
            Ok(()) => {}
            Err(GameStateMutationError::PlanetArmyCapacityExceeded { available, .. }) => {
                self.planet.transport_status = Some(if available == 0 {
                    "This planet is already at the maximum 255 armies.".to_string()
                } else {
                    format!("Planet can receive only {available} more armies.")
                });
                if !inline_menu {
                    self.current_screen = ScreenId::PlanetTransportFleetSelect(mode);
                }
                return Ok(());
            }
            Err(err) => return Err(err.into()),
        }
        self.save_game_data()?;
        if self.planet.transport_fleet_first {
            let fleet_number = fleet.fleet_number;
            let planet_name = planet.planet_name.clone();
            let coords = planet.coords;
            self.planet.transport_qty_input.clear();
            self.planet.transport_status = None;
            self.planet.transport_selected_planet_record = None;
            self.planet.transport_selected_fleet_record = None;
            self.planet.transport_fleet_first = false;
            self.clear_fleet_menu_prompt();
            self.show_command_menu_notice(
                CommandMenu::Fleet,
                match mode {
                    PlanetTransportMode::Load => format!(
                        "Loaded {qty} armies from {} [{:02},{:02}] onto Fleet #{}.",
                        planet_name, coords[0], coords[1], fleet_number
                    ),
                    PlanetTransportMode::Unload => format!(
                        "Unloaded {qty} armies from Fleet #{} to {} [{:02},{:02}].",
                        fleet_number, planet_name, coords[0], coords[1]
                    ),
                },
            );
            self.return_to_command_menu();
            return Ok(());
        }
        if inline_planet_menu {
            let fleet_number = fleet.fleet_number;
            let planet_name = planet.planet_name.clone();
            let coords = planet.coords;
            self.planet.transport_qty_input.clear();
            self.planet.transport_status = None;
            self.planet.transport_selected_planet_record = None;
            self.planet.transport_selected_fleet_record = None;
            self.planet.transport_mode = None;
            self.clear_planet_transport_prompt();
            self.show_command_menu_notice(
                CommandMenu::Planet,
                match mode {
                    PlanetTransportMode::Load => format!(
                        "Loaded {qty} armies from {} [{:02},{:02}] onto Fleet #{}.",
                        planet_name, coords[0], coords[1], fleet_number
                    ),
                    PlanetTransportMode::Unload => format!(
                        "Unloaded {qty} armies from Fleet #{} to {} [{:02},{:02}].",
                        fleet_number, planet_name, coords[0], coords[1]
                    ),
                },
            );
            self.current_screen = ScreenId::PlanetMenu;
            return Ok(());
        }
        self.planet.transport_status = None;
        self.planet.transport_qty_input.clear();
        let base_row = self
            .build_planet_rows()
            .into_iter()
            .find(|row| row.planet_record_index_1_based == planet.planet_record_index_1_based)
            .ok_or("transport planet row missing after submit")?;
        let eligible_fleets = self.planet_transport_eligible_fleet_rows_for_planet(mode, &base_row);
        if !eligible_fleets.is_empty() {
            self.planet.transport_fleet_cursor = self
                .planet
                .transport_fleet_cursor
                .min(eligible_fleets.len() - 1);
            self.current_screen = ScreenId::PlanetTransportFleetSelect(mode);
        } else {
            let planet_rows = self.planet_transport_planet_rows(mode);
            self.planet.transport_selected_planet_record = None;
            if !planet_rows.is_empty() {
                self.planet.transport_planet_cursor = self
                    .planet
                    .transport_planet_cursor
                    .min(planet_rows.len() - 1);
                self.current_screen = ScreenId::PlanetTransportPlanetSelect(mode);
            } else {
                self.planet.transport_status = None;
                self.return_to_command_menu();
            }
        }
        Ok(())
    }

    pub(crate) fn planet_transport_planet_default_coords(
        &self,
        mode: PlanetTransportMode,
    ) -> [u8; 2] {
        self.planet_transport_planet_rows(mode)
            .get(self.planet.transport_planet_cursor)
            .map(|row| row.coords)
            .unwrap_or_else(|| self.default_planet_prompt_coords())
    }

    fn sync_planet_transport_planet_cursor_to_input(&mut self, mode: PlanetTransportMode) {
        let raw = self.planet.transport_planet_input.trim();
        if raw.is_empty() {
            return;
        }
        let rows = self.planet_transport_planet_rows(mode);
        let default_coords = self.planet_transport_planet_default_coords(mode);
        let Some(coords) = resolve_default_coords_input(raw, default_coords) else {
            return;
        };
        if let Some(index) = rows.iter().position(|row| row.coords == coords) {
            self.planet.transport_planet_cursor = index;
            sync_scroll_to_cursor(
                &mut self.planet.transport_planet_scroll_offset,
                self.planet.transport_planet_cursor,
                crate::screen::PLANET_TRANSPORT_VISIBLE_ROWS,
            );
        }
    }

    pub(crate) fn planet_transport_planet_rows(
        &self,
        mode: PlanetTransportMode,
    ) -> Vec<PlanetTransportPlanetRow> {
        self.build_planet_rows()
            .into_iter()
            .filter_map(|row| {
                self.owned_planet_row_by_record(row.planet_record_index_1_based)?;
                if mode == PlanetTransportMode::Load && row.armies == 0 {
                    return None;
                }
                let fleets = self.planet_transport_eligible_fleet_rows_for_planet(mode, &row);
                if fleets.is_empty() {
                    return None;
                }
                Some(PlanetTransportPlanetRow {
                    planet_record_index_1_based: row.planet_record_index_1_based,
                    planet_name: row.planet_name,
                    coords: row.coords,
                    planet_armies: row.armies,
                    transport_capacity: fleets.iter().map(|fleet| fleet.available_qty).sum(),
                })
            })
            .collect()
    }

    pub(crate) fn current_planet_transport_planet_row(
        &self,
        mode: PlanetTransportMode,
    ) -> Result<PlanetTransportPlanetRow, Box<dyn std::error::Error>> {
        if let Some(selected_record) = self.planet.transport_selected_planet_record {
            let base_row = self
                .owned_planet_row_by_record(selected_record)
                .ok_or_else(|| "current transport planet missing".to_string())?;
            let transport_capacity = self
                .planet_transport_fleet_rows_for_planet(mode, &base_row)
                .iter()
                .map(|fleet| fleet.available_qty)
                .sum();
            return Ok(PlanetTransportPlanetRow {
                planet_record_index_1_based: base_row.planet_record_index_1_based,
                planet_name: base_row.planet_name,
                coords: base_row.coords,
                planet_armies: base_row.armies,
                transport_capacity,
            });
        }

        self.planet_transport_planet_rows(mode)
            .get(self.planet.transport_planet_cursor)
            .cloned()
            .ok_or_else(|| "current transport planet missing".into())
    }

    pub(crate) fn current_planet_transport_fleet_rows(
        &self,
        mode: PlanetTransportMode,
    ) -> Result<Vec<PlanetTransportFleetRow>, Box<dyn std::error::Error>> {
        let planet = self.current_planet_transport_planet_row(mode)?;
        let base_row = self
            .owned_planet_row_by_record(planet.planet_record_index_1_based)
            .ok_or("transport planet row missing")?;
        Ok(self.planet_transport_fleet_rows_for_planet(mode, &base_row))
    }

    pub(crate) fn current_planet_transport_fleet_row(
        &self,
        mode: PlanetTransportMode,
    ) -> Result<PlanetTransportFleetRow, Box<dyn std::error::Error>> {
        if let Some(selected_record) = self.planet.transport_selected_fleet_record {
            let planet = self.current_planet_transport_planet_row(mode)?;
            let base_row = self
                .owned_planet_row_by_record(planet.planet_record_index_1_based)
                .ok_or("transport planet row missing")?;
            return self
                .planet_transport_fleet_rows_for_planet(mode, &base_row)
                .into_iter()
                .find(|row| row.fleet_record_index_1_based == selected_record)
                .ok_or_else(|| "current transport fleet missing".into());
        }
        self.current_planet_transport_fleet_rows(mode)?
            .get(self.planet.transport_fleet_cursor)
            .cloned()
            .ok_or_else(|| "current transport fleet missing".into())
    }

    fn planet_transport_fleet_rows_for_planet(
        &self,
        mode: PlanetTransportMode,
        row: &ec_data::EmpirePlanetEconomyRow,
    ) -> Vec<PlanetTransportFleetRow> {
        self.game_data
            .fleets
            .records
            .iter()
            .enumerate()
            .filter(|(_, fleet)| {
                fleet.owner_empire_raw() as usize == self.player.record_index_1_based
                    && fleet.current_location_coords_raw() == row.coords
                    && fleet.troop_transport_count() > 0
            })
            .map(|(idx, fleet)| {
                let available_qty = self.transport_available_qty(mode, fleet, row);
                PlanetTransportFleetRow {
                    fleet_record_index_1_based: idx + 1,
                    fleet_number: fleet.local_slot_word_raw(),
                    troop_transports: fleet.troop_transport_count(),
                    loaded_armies: fleet.army_count(),
                    available_qty,
                }
            })
            .collect()
    }

    fn planet_transport_eligible_fleet_rows_for_planet(
        &self,
        mode: PlanetTransportMode,
        row: &ec_data::EmpirePlanetEconomyRow,
    ) -> Vec<PlanetTransportFleetRow> {
        self.planet_transport_fleet_rows_for_planet(mode, row)
            .into_iter()
            .filter(|fleet| fleet.available_qty > 0)
            .collect()
    }

    pub(super) fn current_commission_planet_row(
        &self,
    ) -> Result<ec_data::EmpirePlanetEconomyRow, Box<dyn std::error::Error>> {
        self.commission_planet_rows()
            .get(self.planet.commission_index)
            .cloned()
            .ok_or_else(|| "current commission planet missing".into())
    }
}
