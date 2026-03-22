use crate::app::helpers::{resolve_default_coords_input, sync_scroll_to_cursor};
use crate::app::state::App;
use crate::domains::planet::PlanetAction;
use crate::screen::{CommandMenu, PlanetDatabaseRow, PlanetListMode, PlanetListSort, ScreenId};
use ec_data::{
    PlanetIntelSnapshot, PlayerStarmapWorld, build_player_starmap_projection_from_snapshots,
};

impl App {
    pub fn open_planet_menu(&mut self) {
        self.clear_command_menu_notice();
        self.command_return_menu = CommandMenu::Planet;
        self.current_screen = ScreenId::PlanetMenu;
    }

    pub fn open_planet_help(&mut self) {
        self.clear_command_menu_notice();
        self.current_screen = ScreenId::PlanetHelp;
    }

    pub fn open_planet_tax_prompt(&mut self) {
        self.clear_command_menu_notice();
        self.planet.tax_input = String::new();
        self.planet.tax_status = None;
        self.current_screen = ScreenId::PlanetTaxPrompt;
    }

    pub fn open_planet_database(&mut self) {
        if !matches!(
            self.current_screen,
            ScreenId::PlanetDatabaseList | ScreenId::PlanetDatabaseDetail
        ) {
            self.command_return_menu = self.origin_command_menu();
            let default_coords = self.default_planet_prompt_coords();
            let rows = self.planet_database_rows();
            let default_index = rows
                .iter()
                .position(|row| row.coords == default_coords)
                .unwrap_or(0);
            self.planet.database_cursor = default_index;
            self.planet.database_detail_index = default_index;
            self.planet.database_scroll_offset =
                default_index.saturating_sub(crate::screen::PLANET_DATABASE_VISIBLE_ROWS / 2);
            self.planet.database_input.clear();
            self.planet.database_status = None;
        }
        self.current_screen = ScreenId::PlanetDatabaseList;
    }

    pub fn open_planet_database_detail(&mut self) {
        let total = self.planet_database_rows().len();
        if total == 0 {
            self.current_screen = ScreenId::PlanetDatabaseList;
            return;
        }
        self.planet.database_detail_index = self.planet.database_cursor.min(total - 1);
        self.current_screen = ScreenId::PlanetDatabaseDetail;
    }

    pub fn open_planet_list_sort_prompt(&mut self, mode: PlanetListMode) {
        if self.sorted_planet_rows(PlanetListSort::Location).is_empty() {
            self.show_command_menu_notice(
                CommandMenu::Planet,
                "You do not currently control any planets.",
            );
            return;
        }
        self.clear_command_menu_notice();
        self.planet.list_sort_status = None;
        self.current_screen = ScreenId::PlanetListSortPrompt(mode);
    }

    pub fn submit_planet_list_sort(&mut self, mode: PlanetListMode, sort: PlanetListSort) {
        let total = self.sorted_planet_rows(sort).len();
        if total == 0 {
            self.show_command_menu_notice(
                CommandMenu::Planet,
                "You do not currently control any planets.",
            );
            return;
        }
        self.clear_command_menu_notice();
        self.planet.list_sort_status = None;
        self.planet.brief_scroll_offset = 0;
        self.planet.brief_cursor = 0;
        self.planet.detail_index = 0;
        self.current_screen = match mode {
            PlanetListMode::Brief => ScreenId::PlanetBriefList(sort),
            PlanetListMode::Detail => ScreenId::PlanetDetailList(sort),
            PlanetListMode::Stub(_) => ScreenId::PlanetMenu,
        };
    }

    pub fn scroll_planet_brief(&mut self, delta: i8) {
        let ScreenId::PlanetBriefList(sort) = self.current_screen else {
            return;
        };
        let total = self.sorted_planet_rows(sort).len();
        let max_offset = total.saturating_sub(crate::screen::PLANET_BRIEF_VISIBLE_ROWS);
        self.planet.brief_scroll_offset = self
            .planet
            .brief_scroll_offset
            .saturating_add_signed(delta as isize)
            .min(max_offset);
    }

    pub fn move_planet_brief_cursor(&mut self, delta: i8) {
        let ScreenId::PlanetBriefList(sort) = self.current_screen else {
            return;
        };
        let total = self.sorted_planet_rows(sort).len();
        if total == 0 {
            self.planet.brief_cursor = 0;
            return;
        }
        let next = self.planet.brief_cursor as isize + delta as isize;
        self.planet.brief_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.planet.brief_scroll_offset,
            self.planet.brief_cursor,
            crate::screen::PLANET_BRIEF_VISIBLE_ROWS,
        );
    }

    pub fn move_planet_detail(&mut self, delta: i8) {
        let ScreenId::PlanetDetailList(sort) = self.current_screen else {
            return;
        };
        let total = self.sorted_planet_rows(sort).len();
        if total == 0 {
            self.planet.detail_index = 0;
            return;
        }
        self.planet.detail_index = match delta {
            i8::MIN => 0,
            i8::MAX => total - 1,
            _ => self
                .planet
                .detail_index
                .saturating_add_signed(delta as isize)
                .min(total - 1),
        };
    }

    pub fn move_planet_database_list(&mut self, delta: i8) {
        if self.current_screen != ScreenId::PlanetDatabaseList {
            return;
        }
        let total = self.planet_database_rows().len();
        if total == 0 {
            self.planet.database_cursor = 0;
            return;
        }
        let next = self.planet.database_cursor as isize + delta as isize;
        self.planet.database_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.planet.database_scroll_offset,
            self.planet.database_cursor,
            crate::screen::PLANET_DATABASE_VISIBLE_ROWS,
        );
    }

    pub fn move_planet_database_detail(&mut self, delta: i8) {
        if self.current_screen != ScreenId::PlanetDatabaseDetail {
            return;
        }
        let total = self.planet_database_rows().len();
        if total == 0 {
            self.planet.database_detail_index = 0;
            return;
        }
        self.planet.database_detail_index = match delta {
            i8::MIN => 0,
            i8::MAX => total - 1,
            _ => self
                .planet
                .database_detail_index
                .saturating_add_signed(delta as isize)
                .min(total - 1),
        };
        self.planet.database_cursor = self.planet.database_detail_index;
        sync_scroll_to_cursor(
            &mut self.planet.database_scroll_offset,
            self.planet.database_cursor,
            crate::screen::PLANET_DATABASE_VISIBLE_ROWS,
        );
    }

    pub fn append_planet_database_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::PlanetDatabaseList {
            return;
        }
        if self.planet.database_input.len() < 16 && (ch.is_ascii_digit() || ch == ',' || ch == ' ')
        {
            self.planet.database_input.push(ch);
            self.planet.database_status = None;
        }
    }

    pub fn backspace_planet_database_input(&mut self) {
        if self.current_screen != ScreenId::PlanetDatabaseList {
            return;
        }
        self.planet.database_input.pop();
        self.planet.database_status = None;
    }

    pub fn submit_planet_database_lookup(&mut self) {
        if self.current_screen != ScreenId::PlanetDatabaseList {
            return;
        }
        let rows = self.planet_database_rows();
        if self.planet.database_input.trim().is_empty() {
            self.open_planet_database_detail();
            return;
        }
        let Some(coords) = resolve_default_coords_input(
            &self.planet.database_input,
            self.default_planet_prompt_coords(),
        ) else {
            self.planet.database_status = Some("Enter coordinates like 5,2".to_string());
            return;
        };
        let Some(index) = rows.iter().position(|row| row.coords == coords) else {
            self.planet.database_status =
                Some(format!("No world found at [{},{}]", coords[0], coords[1]));
            return;
        };
        self.planet.database_cursor = index;
        sync_scroll_to_cursor(
            &mut self.planet.database_scroll_offset,
            self.planet.database_cursor,
            crate::screen::PLANET_DATABASE_VISIBLE_ROWS,
        );
        self.planet.database_status = None;
        self.planet.database_input.clear();
        self.open_planet_database_detail();
    }

    pub fn append_planet_tax_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::PlanetTaxPrompt && self.planet.tax_input.len() < 3 {
            self.planet.tax_input.push(ch);
            self.planet.tax_status = None;
        }
    }

    pub fn backspace_planet_tax_input(&mut self) {
        if self.current_screen == ScreenId::PlanetTaxPrompt {
            self.planet.tax_input.pop();
            self.planet.tax_status = None;
        }
    }

    pub fn submit_planet_tax(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let raw = self.planet.tax_input.trim();
        let parsed = if raw.is_empty() {
            self.game_data.player.records[self.player.record_index_1_based - 1].tax_rate()
        } else {
            match raw.parse::<u8>() {
                Ok(value) => value,
                Err(_) => {
                    self.planet.tax_status =
                        Some("Enter an integer tax rate from 0 to 100.".to_string());
                    return Ok(());
                }
            }
        };
        if parsed > 100 {
            self.planet.tax_status = Some("Enter an integer tax rate from 0 to 100.".to_string());
            return Ok(());
        }
        self.game_data
            .set_player_tax_rate(self.player.record_index_1_based, parsed)?;
        self.save_game_data()?;
        self.planet.tax_input = parsed.to_string();
        self.planet.tax_status = Some(format!("Empire tax rate set to {parsed}%."));
        self.current_screen = ScreenId::PlanetTaxDone;
        Ok(())
    }

    pub fn open_planet_info_prompt(&mut self, menu: CommandMenu) {
        self.command_return_menu = menu;
        self.planet.info_input.clear();
        self.planet.info_error = None;
        self.planet.info_selected = None;
        self.current_screen = ScreenId::PlanetInfoPrompt;
    }

    pub fn append_planet_info_char(&mut self, ch: char) {
        if self.planet.info_input.len() < 16 {
            self.planet.info_input.push(ch);
            self.planet.info_error = None;
        }
    }

    pub fn backspace_planet_info_input(&mut self) {
        self.planet.info_input.pop();
        self.planet.info_error = None;
    }

    pub fn submit_planet_info_prompt(&mut self) {
        let Some(coords) = resolve_default_coords_input(
            &self.planet.info_input,
            self.default_planet_prompt_coords(),
        ) else {
            self.planet.info_error = Some("Enter coordinates like 5,2".to_string());
            return;
        };

        let Some(planet_idx) = self.game_data.planet_record_index_at_coords(coords) else {
            self.planet.info_error =
                Some(format!("No world found at [{},{}]", coords[0], coords[1]));
            return;
        };

        self.planet.info_selected = Some(planet_idx);
        self.planet.info_error = None;
        self.current_screen = ScreenId::PlanetInfoDetail;
    }

    pub fn planet_info_input(&self) -> &str {
        &self.planet.info_input
    }

    pub fn selected_planet_info(&self) -> Option<usize> {
        self.planet.info_selected
    }

    pub(crate) fn sorted_planet_rows(
        &self,
        sort: PlanetListSort,
    ) -> Vec<ec_data::EmpirePlanetEconomyRow> {
        let mut rows = self
            .game_data
            .empire_planet_economy_rows(self.player.record_index_1_based);
        rows.sort_by(|left, right| match sort {
            PlanetListSort::CurrentProduction => right
                .present_production
                .cmp(&left.present_production)
                .then_with(|| left.coords.cmp(&right.coords)),
            PlanetListSort::Location => left.coords.cmp(&right.coords),
            PlanetListSort::PotentialProduction => right
                .potential_production
                .cmp(&left.potential_production)
                .then_with(|| left.coords.cmp(&right.coords)),
        });
        rows
    }

    pub(crate) fn planet_database_rows(&self) -> Vec<PlanetDatabaseRow> {
        let mut rows = build_player_starmap_projection_from_snapshots(
            &self.game_data,
            &self.planet_intel_snapshots,
            self.player.record_index_1_based as u8,
        )
        .worlds
        .into_iter()
        .map(|world| {
            let intel_snapshot = self
                .planet_intel_snapshots
                .get(&world.planet_record_index_1_based);
            let intel_label = planet_database_intel_label(intel_snapshot, &world);
            let owner_label = world
                .known_owner_empire_id
                .map(|id| format!("#{}", id))
                .unwrap_or_else(|| "?".to_string());
            let year_label = intel_snapshot
                .and_then(|snapshot| snapshot.last_intel_year)
                .map(|year| year.to_string())
                .unwrap_or_else(|| "?".to_string());
            PlanetDatabaseRow {
                planet_record_index_1_based: world.planet_record_index_1_based,
                coords: world.coords,
                name_label: world.known_name.unwrap_or_else(|| "?".to_string()),
                owner_label,
                max_prod_label: world
                    .known_potential_production
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "?".to_string()),
                year_seen_label: year_label.clone(),
                armies_label: world
                    .known_armies
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "?".to_string()),
                batteries_label: world
                    .known_ground_batteries
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "?".to_string()),
                current_prod_label: world
                    .known_current_production
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "?".to_string()),
                stored_points_label: world
                    .known_stored_points
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "?".to_string()),
                year_scout_label: year_label,
                intel_label,
            }
        })
        .collect::<Vec<_>>();
        rows.sort_by_key(|row| row.coords);
        rows
    }

    pub(crate) fn handle_planet_info_prompt_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                crate::app::Action::ReturnToCommandMenu
            }
            KeyCode::Enter => crate::app::Action::Planet(PlanetAction::SubmitInfoPrompt),
            KeyCode::Backspace => crate::app::Action::Planet(PlanetAction::BackspaceInfoInput),
            KeyCode::Char(ch)
                if ch.is_ascii_digit()
                    || matches!(ch, ',' | ' ' | ':' | '/' | '-' | '(' | ')' | '[' | ']') =>
            {
                crate::app::Action::Planet(PlanetAction::AppendInfoChar(ch))
            }
            _ => crate::app::Action::Noop,
        }
    }

    pub(crate) fn default_planet_prompt_coords(&self) -> [u8; 2] {
        let homeworld_index = self
            .game_data
            .player
            .records
            .get(self.player.record_index_1_based - 1)
            .map(|player| player.homeworld_planet_index_1_based_raw() as usize)
            .unwrap_or(0);
        if homeworld_index != 0 {
            if let Some(planet) = self.game_data.planets.records.get(homeworld_index - 1) {
                return planet.coords_raw();
            }
        }
        self.game_data
            .planets
            .records
            .iter()
            .find(|planet| {
                planet.owner_empire_slot_raw() as usize == self.player.record_index_1_based
                    && planet.is_homeworld_seed_ignoring_name()
            })
            .map(|planet| planet.coords_raw())
            .unwrap_or([8, 2])
    }
}

fn planet_database_intel_label(
    snapshot: Option<&PlanetIntelSnapshot>,
    world: &PlayerStarmapWorld,
) -> String {
    if let Some(snapshot) = snapshot {
        return match snapshot.intel_tier {
            ec_data::IntelTier::Owned => "owned".to_string(),
            ec_data::IntelTier::Full => "full".to_string(),
            ec_data::IntelTier::Partial => "partial".to_string(),
            ec_data::IntelTier::Unknown => "unknown".to_string(),
        };
    }
    if world.known_owner_empire_id == Some(0) {
        return "unknown".to_string();
    }
    if world.known_armies.is_some() || world.known_ground_batteries.is_some() {
        "full".to_string()
    } else if world.known_name.is_some()
        || world.known_owner_empire_id.is_some()
        || world.known_owner_empire_name.is_some()
        || world.known_potential_production.is_some()
    {
        "partial".to_string()
    } else {
        "unknown".to_string()
    }
}
