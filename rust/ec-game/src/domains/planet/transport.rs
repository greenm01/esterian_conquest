use crate::app::helpers::{
    center_scroll_to_cursor, resolve_default_coords_input, sync_scroll_to_cursor,
};
use crate::app::state::App;
use crate::screen::{
    CommandMenu, PlanetTransportFleetRow, PlanetTransportMode, PlanetTransportPlanetRow, ScreenId,
};
use ec_data::GameStateMutationError;

impl App {
    pub fn open_fleet_transport_prompt(&mut self, mode: PlanetTransportMode) {
        self.command_return_menu = CommandMenu::Fleet;
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
            crate::domains::fleet::state::FleetMenuPromptMode::TransportFleet(mode),
            self.strongest_owned_fleet_number()
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

    pub(crate) fn open_fleet_transport_planet_prompt(
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
        if fleet.troop_transport_count() == 0 {
            return Err("That fleet has no troop transports.".to_string());
        }
        let eligible_planets = self.fleet_transport_planet_rows_for_fleet(mode, fleet_record_index_1_based);
        if eligible_planets.is_empty() {
            return Err(match mode {
                PlanetTransportMode::Load => {
                    "No eligible planets at that fleet's location can load armies.".to_string()
                }
                PlanetTransportMode::Unload => {
                    "No eligible planets at that fleet's location can receive unloaded armies."
                        .to_string()
                }
            });
        }
        self.command_return_menu = CommandMenu::Fleet;
        self.planet.transport_mode = Some(mode);
        self.planet.transport_selected_fleet_record = Some(fleet_record_index_1_based);
        self.planet.transport_selected_planet_record = None;
        self.planet.transport_fleet_first = true;
        self.planet.transport_qty_input.clear();
        self.planet.transport_status = None;
        self.fleet.menu_prompt_context_fleet_record_index_1_based = Some(fleet_record_index_1_based);
        self.open_fleet_menu_prompt(
            crate::domains::fleet::state::FleetMenuPromptMode::TransportPlanet(mode),
            format!("{:02},{:02}", fleet.current_location_coords_raw()[0], fleet.current_location_coords_raw()[1]),
        );
        Ok(())
    }

    pub(crate) fn open_fleet_transport_quantity_prompt_from_menu(
        &mut self,
        mode: PlanetTransportMode,
    ) -> Result<(), String> {
        let fleet_record_index_1_based = self
            .planet
            .transport_selected_fleet_record
            .or(self.fleet.menu_prompt_context_fleet_record_index_1_based)
            .ok_or_else(|| "Select a fleet first.".to_string())?;
        let fleet = self
            .game_data
            .fleets
            .records
            .get(fleet_record_index_1_based - 1)
            .ok_or_else(|| "Selected fleet is no longer available.".to_string())?;
        let default_coords = fleet.current_location_coords_raw();
        let coords = resolve_default_coords_input(&self.fleet.menu_prompt_input, default_coords)
            .ok_or_else(|| "Enter coordinates like 05,02".to_string())?;
        let planet_row = self
            .fleet_transport_planet_rows_for_fleet(mode, fleet_record_index_1_based)
            .into_iter()
            .find(|row| row.coords == coords)
            .ok_or_else(|| {
                format!(
                    "Fleet #{:02} has no eligible planet at [{:02},{:02}].",
                    fleet.local_slot_word_raw(),
                    coords[0],
                    coords[1]
                )
            })?;
        self.clear_fleet_menu_prompt();
        self.planet.transport_mode = Some(mode);
        self.planet.transport_selected_fleet_record = Some(fleet_record_index_1_based);
        self.planet.transport_selected_planet_record = Some(planet_row.planet_record_index_1_based);
        self.planet.transport_fleet_first = true;
        self.planet.transport_qty_input.clear();
        self.planet.transport_status = None;
        self.current_screen = ScreenId::PlanetTransportQuantityPrompt(mode);
        Ok(())
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
        let mode = match self.current_screen {
            ScreenId::PlanetTransportFleetSelect(mode)
            | ScreenId::PlanetTransportQuantityPrompt(mode) => mode,
            _ => return Ok(()),
        };
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
            self.current_screen = ScreenId::PlanetTransportFleetSelect(mode);
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
                self.current_screen = ScreenId::PlanetTransportFleetSelect(mode);
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
        if matches!(
            self.current_screen,
            ScreenId::PlanetTransportFleetSelect(_) | ScreenId::PlanetTransportQuantityPrompt(_)
        ) {
            let selected_record = self
                .planet
                .transport_selected_planet_record
                .ok_or_else(|| "current transport planet missing".to_string())?;
            let base_row = self
                .build_planet_rows()
                .into_iter()
                .find(|row| row.planet_record_index_1_based == selected_record)
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
            .build_planet_rows()
            .into_iter()
            .find(|row| row.planet_record_index_1_based == planet.planet_record_index_1_based)
            .ok_or("transport planet row missing")?;
        Ok(self.planet_transport_fleet_rows_for_planet(mode, &base_row))
    }

    pub(crate) fn current_planet_transport_fleet_row(
        &self,
        mode: PlanetTransportMode,
    ) -> Result<PlanetTransportFleetRow, Box<dyn std::error::Error>> {
        if matches!(self.current_screen, ScreenId::PlanetTransportQuantityPrompt(_))
            && self.planet.transport_fleet_first
        {
            let selected_record = self
                .planet
                .transport_selected_fleet_record
                .ok_or_else(|| "current transport fleet missing".to_string())?;
            let planet = self.current_planet_transport_planet_row(mode)?;
            let base_row = self
                .build_planet_rows()
                .into_iter()
                .find(|row| row.planet_record_index_1_based == planet.planet_record_index_1_based)
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

    fn fleet_transport_planet_rows_for_fleet(
        &self,
        mode: PlanetTransportMode,
        fleet_record_index_1_based: usize,
    ) -> Vec<PlanetTransportPlanetRow> {
        let Some(fleet) = self.game_data.fleets.records.get(fleet_record_index_1_based - 1) else {
            return Vec::new();
        };
        let available_qty = match mode {
            PlanetTransportMode::Load => fleet
                .troop_transport_count()
                .saturating_sub(fleet.army_count()),
            PlanetTransportMode::Unload => fleet.army_count(),
        };
        if available_qty == 0 {
            return Vec::new();
        }
        self.build_planet_rows()
            .into_iter()
            .filter_map(|row| {
                if row.coords != fleet.current_location_coords_raw() {
                    return None;
                }
                let fleet_available_qty = match mode {
                    PlanetTransportMode::Load => available_qty,
                    PlanetTransportMode::Unload => {
                        available_qty.min(u16::from(u8::MAX.saturating_sub(row.armies)))
                    }
                };
                if fleet_available_qty == 0 || (mode == PlanetTransportMode::Load && row.armies == 0)
                {
                    return None;
                }
                Some(PlanetTransportPlanetRow {
                    planet_record_index_1_based: row.planet_record_index_1_based,
                    planet_name: row.planet_name,
                    coords: row.coords,
                    planet_armies: row.armies,
                    transport_capacity: fleet_available_qty,
                })
            })
            .collect()
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
                let available_qty = match mode {
                    PlanetTransportMode::Load => fleet
                        .troop_transport_count()
                        .saturating_sub(fleet.army_count()),
                    PlanetTransportMode::Unload => fleet
                        .army_count()
                        .min(u16::from(u8::MAX.saturating_sub(row.armies))),
                };
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
