use crate::app::helpers::{resolve_default_coords_input, sync_scroll_to_cursor};
use crate::app::state::App;
use crate::domains::planet::PlanetAction;
use crate::screen::{
    CommandMenu, PlanetDatabaseFilter, PlanetDatabaseFilterMode, PlanetDatabasePromptMode,
    PlanetDatabaseRow, PlanetDatabaseSort, PlanetDatabaseSortMode, PlanetListMode, PlanetListSort,
    ScreenId,
};
use nc_data::build_player_starmap_projection_from_snapshots;

impl App {
    fn planet_database_visible_rows(&self) -> usize {
        crate::screen::layout::stacked_table_visible_rows_for(self.screen_geometry, 1)
    }

    fn planet_brief_visible_rows(&self) -> usize {
        crate::screen::layout::stacked_table_visible_rows_for(self.screen_geometry, 1)
    }

    fn command_menu_for_planet_list_mode(mode: PlanetListMode) -> CommandMenu {
        match mode {
            PlanetListMode::Brief | PlanetListMode::Stub(_) => CommandMenu::Planet,
            PlanetListMode::BuildSelect => CommandMenu::PlanetBuild,
        }
    }

    pub fn open_planet_menu(&mut self) {
        self.clear_command_menu_notice();
        self.close_planet_auto_commission_prompt();
        self.clear_planet_auto_commission_report();
        self.close_planet_tax_prompt();
        self.clear_planet_scorch_prompt();
        self.clear_planet_transport_prompt();
        self.command_return_menu = CommandMenu::Planet;
        self.current_screen = ScreenId::PlanetMenu;
    }

    pub fn open_planet_tax_prompt(&mut self) {
        self.clear_command_menu_notice();
        self.close_planet_auto_commission_prompt();
        self.close_planet_info_prompt();
        self.planet.tax_prompt_active = true;
        self.planet.tax_input = String::new();
        self.planet.tax_error = None;
        self.planet.tax_notice = None;
        self.current_screen = ScreenId::PlanetMenu;
    }

    pub fn close_planet_tax_prompt(&mut self) {
        self.planet.tax_prompt_active = false;
        self.planet.tax_input.clear();
        self.planet.tax_error = None;
        self.planet.tax_notice = None;
    }

    pub fn open_planet_auto_commission_prompt(&mut self) {
        if self.commission_planet_rows().is_empty() {
            self.show_command_menu_notice(
                CommandMenu::Planet,
                "No ships or starbases are waiting in stardock.",
            );
            return;
        }
        self.clear_command_menu_notice();
        self.close_planet_tax_prompt();
        self.close_planet_info_prompt();
        self.clear_planet_auto_commission_report();
        self.planet.auto_commission_prompt_active = true;
        self.current_screen = ScreenId::PlanetMenu;
    }

    pub fn close_planet_auto_commission_prompt(&mut self) {
        self.planet.auto_commission_prompt_active = false;
    }

    pub fn clear_planet_auto_commission_report(&mut self) {
        self.planet.auto_commission_report_rows.clear();
        self.planet.auto_commission_report_revealed_rows = 0;
    }

    pub fn open_planet_database(&mut self) {
        if !matches!(
            self.current_screen,
            ScreenId::PlanetDatabaseList | ScreenId::PlanetDatabaseFilterPrompt
        ) {
            self.command_return_menu = self.origin_command_menu();
            let default_index = 0usize;
            self.planet.database_cursor = default_index;
            self.planet.database_scroll_offset =
                default_index.saturating_sub(self.planet_database_visible_rows() / 2);
            self.planet.database_input.clear();
            self.planet.database_prompt_default_value.clear();
            self.planet.database_pending_range_anchor = None;
            self.planet.database_status = None;
            self.planet.database_filter = PlanetDatabaseFilter::All;
            self.planet.database_sort = PlanetDatabaseSort::Location;
        }
        self.planet.database_prompt_mode = PlanetDatabasePromptMode::FilterMenu;
        self.planet.database_input.clear();
        self.planet.database_prompt_default_value.clear();
        self.planet.database_pending_range_anchor = None;
        self.planet.database_status = None;
        self.current_screen = ScreenId::PlanetDatabaseList;
    }

    pub fn open_planet_database_filter_prompt(&mut self) {
        if self.current_screen != ScreenId::PlanetDatabaseList {
            return;
        }
        self.planet.database_prompt_mode = PlanetDatabasePromptMode::FilterMenu;
        self.planet.database_input.clear();
        self.planet.database_prompt_default_value.clear();
        self.planet.database_pending_range_anchor = None;
        self.planet.database_status = None;
        self.current_screen = ScreenId::PlanetDatabaseFilterPrompt;
    }

    pub fn open_planet_database_sort_prompt(&mut self) {
        if self.current_screen != ScreenId::PlanetDatabaseList {
            return;
        }
        self.planet.database_prompt_mode = PlanetDatabasePromptMode::SortMenu;
        self.planet.database_input.clear();
        self.planet.database_prompt_default_value.clear();
        self.planet.database_pending_range_anchor = None;
        self.planet.database_status = None;
        self.current_screen = ScreenId::PlanetDatabaseFilterPrompt;
    }

    pub fn open_planet_database_detail(&mut self) {
        let rows = self.planet_database_rows();
        let total = rows.len();
        if total == 0 {
            self.current_screen = ScreenId::PlanetDatabaseList;
            return;
        }
        let coords = rows[self.planet.database_cursor.min(total - 1)].coords;
        let _ = self.open_planet_info_detail_at_coords(coords, Some(ScreenId::PlanetDatabaseList));
    }

    pub fn open_planet_list_sort_prompt(&mut self, mode: PlanetListMode) {
        if self.sorted_planet_rows(PlanetListSort::Location).is_empty() {
            self.show_command_menu_notice(
                Self::command_menu_for_planet_list_mode(mode),
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
                Self::command_menu_for_planet_list_mode(mode),
                "You do not currently control any planets.",
            );
            return;
        }
        self.clear_command_menu_notice();
        self.planet.list_sort = sort;
        self.planet.list_sort_status = None;
        self.planet.brief_scroll_offset = 0;
        self.planet.brief_cursor = 0;
        self.planet.brief_input.clear();
        self.current_screen = match mode {
            PlanetListMode::Brief | PlanetListMode::BuildSelect => {
                self.select_planet_brief_origin_row(mode, sort);
                ScreenId::PlanetBriefList(mode, sort)
            }
            PlanetListMode::Stub(_) => ScreenId::PlanetMenu,
        };
    }

    pub fn close_planet_list_sort_prompt(&mut self, mode: PlanetListMode) {
        self.planet.list_sort_status = None;
        self.current_screen = match mode {
            PlanetListMode::Brief | PlanetListMode::BuildSelect => {
                ScreenId::PlanetBriefList(mode, self.planet.list_sort)
            }
            PlanetListMode::Stub(_) => ScreenId::PlanetMenu,
        };
    }

    pub fn scroll_planet_brief(&mut self, delta: i8) {
        let ScreenId::PlanetBriefList(_, sort) = self.current_screen else {
            return;
        };
        let total = self.sorted_planet_rows(sort).len();
        let max_offset = total.saturating_sub(self.planet_brief_visible_rows());
        self.planet.brief_scroll_offset = self
            .planet
            .brief_scroll_offset
            .saturating_add_signed(delta as isize)
            .min(max_offset);
    }

    pub fn move_planet_brief_cursor(&mut self, delta: i8) {
        let ScreenId::PlanetBriefList(_, sort) = self.current_screen else {
            return;
        };
        let total = self.sorted_planet_rows(sort).len();
        if total == 0 {
            self.planet.brief_cursor = 0;
            return;
        }
        let visible_rows = self.planet_brief_visible_rows();
        let next = self.planet.brief_cursor as isize + delta as isize;
        self.planet.brief_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.planet.brief_scroll_offset,
            self.planet.brief_cursor,
            visible_rows,
        );
    }

    pub fn append_planet_brief_char(&mut self, ch: char) {
        let ScreenId::PlanetBriefList(_, sort) = self.current_screen else {
            return;
        };
        if self.planet.brief_input.len() < 16 && (ch.is_ascii_digit() || ch == ',' || ch == ' ') {
            self.planet.brief_input.push(ch);
            self.sync_planet_brief_cursor_to_input(sort);
            self.planet.list_sort_status = None;
        }
    }

    pub fn backspace_planet_brief_input(&mut self) {
        let ScreenId::PlanetBriefList(_, sort) = self.current_screen else {
            return;
        };
        self.planet.brief_input.pop();
        self.sync_planet_brief_cursor_to_input(sort);
        self.planet.list_sort_status = None;
    }

    pub fn submit_planet_brief_input(&mut self) {
        let ScreenId::PlanetBriefList(mode, sort) = self.current_screen else {
            return;
        };
        let rows = self.sorted_planet_rows(sort);
        if rows.is_empty() {
            return;
        }

        let default_coords = rows
            .get(self.planet.brief_cursor)
            .map(|row| row.coords)
            .unwrap_or([0, 0]);

        if self.planet.brief_input.trim().is_empty() {
            let coords = rows[self.planet.brief_cursor.min(rows.len() - 1)].coords;
            match mode {
                PlanetListMode::Brief => {
                    let _ = self.open_planet_info_detail_at_coords(
                        coords,
                        Some(ScreenId::PlanetBriefList(mode, sort)),
                    );
                }
                PlanetListMode::BuildSelect => {
                    let _ = self.open_planet_build_menu_at_coords(coords);
                }
                PlanetListMode::Stub(_) => {}
            }
            return;
        }

        let Some(coords) = resolve_default_coords_input(&self.planet.brief_input, default_coords)
        else {
            self.planet.list_sort_status = Some("Enter coordinates like 5,2".to_string());
            return;
        };

        let Some(index) = rows.iter().position(|row| row.coords == coords) else {
            self.planet.list_sort_status = Some(format!(
                "No world found at ({:02},{:02})",
                coords[0], coords[1]
            ));
            return;
        };

        self.planet.brief_cursor = index;
        let visible_rows = self.planet_brief_visible_rows();
        sync_scroll_to_cursor(
            &mut self.planet.brief_scroll_offset,
            self.planet.brief_cursor,
            visible_rows,
        );
        self.planet.brief_input.clear();
        self.planet.list_sort_status = None;
        match mode {
            PlanetListMode::Brief => {
                let _ = self.open_planet_info_detail_at_coords(
                    coords,
                    Some(ScreenId::PlanetBriefList(mode, sort)),
                );
            }
            PlanetListMode::BuildSelect => {
                let _ = self.open_planet_build_menu_at_coords(coords);
            }
            PlanetListMode::Stub(_) => {}
        }
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
        let visible_rows = self.planet_database_visible_rows();
        let next = self.planet.database_cursor as isize + delta as isize;
        self.planet.database_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.planet.database_scroll_offset,
            self.planet.database_cursor,
            visible_rows,
        );
    }

    pub fn append_planet_database_char(&mut self, ch: char) {
        let accepts_input = match self.current_screen {
            ScreenId::PlanetDatabaseList => true,
            ScreenId::PlanetDatabaseFilterPrompt => matches!(
                self.planet.database_prompt_mode,
                PlanetDatabasePromptMode::FilterRangeCoords
                    | PlanetDatabasePromptMode::FilterRangeDistance
                    | PlanetDatabasePromptMode::FilterEmpireInput
                    | PlanetDatabasePromptMode::FilterMaxProductionInput
                    | PlanetDatabasePromptMode::SortRangeInput
            ),
            _ => false,
        };
        if accepts_input
            && self.planet.database_input.len() < 16
            && (ch.is_ascii_digit()
                || matches!(
                    self.planet.database_prompt_mode,
                    PlanetDatabasePromptMode::FilterRangeCoords
                        | PlanetDatabasePromptMode::SortRangeInput
                ) && (ch == ',' || ch == ' '))
        {
            self.planet.database_input.push(ch);
            if self.current_screen == ScreenId::PlanetDatabaseList {
                self.sync_planet_database_cursor_to_input();
            }
            self.planet.database_status = None;
        }
    }

    pub fn backspace_planet_database_input(&mut self) {
        let accepts_input = match self.current_screen {
            ScreenId::PlanetDatabaseList => true,
            ScreenId::PlanetDatabaseFilterPrompt => matches!(
                self.planet.database_prompt_mode,
                PlanetDatabasePromptMode::FilterRangeCoords
                    | PlanetDatabasePromptMode::FilterRangeDistance
                    | PlanetDatabasePromptMode::FilterEmpireInput
                    | PlanetDatabasePromptMode::FilterMaxProductionInput
                    | PlanetDatabasePromptMode::SortRangeInput
            ),
            _ => false,
        };
        if !accepts_input {
            return;
        }
        self.planet.database_input.pop();
        if self.current_screen == ScreenId::PlanetDatabaseList {
            self.sync_planet_database_cursor_to_input();
        }
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
            self.default_planet_database_coords(),
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
        let visible_rows = self.planet_database_visible_rows();
        sync_scroll_to_cursor(
            &mut self.planet.database_scroll_offset,
            self.planet.database_cursor,
            visible_rows,
        );
        self.planet.database_status = None;
        self.planet.database_input.clear();
        self.open_planet_database_detail();
    }

    pub fn submit_planet_database_filter(&mut self, mode: PlanetDatabaseFilterMode) {
        if self.current_screen != ScreenId::PlanetDatabaseFilterPrompt {
            return;
        }
        match self.planet.database_prompt_mode {
            PlanetDatabasePromptMode::FilterMenu => match mode {
                PlanetDatabaseFilterMode::All => {
                    self.apply_planet_database_filter(PlanetDatabaseFilter::All);
                }
                PlanetDatabaseFilterMode::Range => {
                    self.planet.database_prompt_mode = PlanetDatabasePromptMode::FilterRangeCoords;
                    self.planet.database_input.clear();
                    self.planet.database_prompt_default_value = format!(
                        "{:02},{:02}",
                        self.default_planet_database_coords()[0],
                        self.default_planet_database_coords()[1]
                    );
                    self.planet.database_pending_range_anchor = None;
                    self.planet.database_status = None;
                }
                PlanetDatabaseFilterMode::Empire => {
                    self.planet.database_prompt_mode = PlanetDatabasePromptMode::FilterEmpireInput;
                    self.planet.database_input.clear();
                    self.planet.database_prompt_default_value = self
                        .planet_database_rows()
                        .get(self.planet.database_cursor)
                        .and_then(|row| row.known_owner_empire_id)
                        .unwrap_or(self.player.record_index_1_based as u8)
                        .to_string();
                    self.planet.database_status = None;
                }
                PlanetDatabaseFilterMode::MaxProduction => {
                    self.planet.database_prompt_mode =
                        PlanetDatabasePromptMode::FilterMaxProductionInput;
                    self.planet.database_input.clear();
                    self.planet.database_prompt_default_value = self
                        .planet_database_rows()
                        .get(self.planet.database_cursor)
                        .and_then(|row| row.known_max_production)
                        .unwrap_or(100)
                        .to_string();
                    self.planet.database_status = None;
                }
            },
            PlanetDatabasePromptMode::FilterRangeCoords => {
                let default_coords = self.default_planet_database_coords();
                let Some(coords) =
                    resolve_default_coords_input(self.planet.database_input.trim(), default_coords)
                else {
                    self.planet.database_status = Some("Enter coordinates like 5,2".to_string());
                    return;
                };
                self.planet.database_pending_range_anchor = Some(coords);
                self.planet.database_prompt_mode = PlanetDatabasePromptMode::FilterRangeDistance;
                self.planet.database_input.clear();
                self.planet.database_prompt_default_value = "5".to_string();
                self.planet.database_status = None;
            }
            PlanetDatabasePromptMode::FilterRangeDistance => {
                let default_radius = self
                    .planet
                    .database_prompt_default_value
                    .trim()
                    .parse::<u8>()
                    .unwrap_or(5);
                let radius =
                    resolve_default_u8_input(self.planet.database_input.trim(), default_radius)
                        .unwrap_or(default_radius);
                let anchor = self
                    .planet
                    .database_pending_range_anchor
                    .unwrap_or_else(|| self.default_planet_database_coords());
                self.apply_planet_database_filter(PlanetDatabaseFilter::Range { anchor, radius });
            }
            PlanetDatabasePromptMode::FilterEmpireInput => {
                let default_empire = self
                    .planet
                    .database_prompt_default_value
                    .trim()
                    .parse::<u8>()
                    .unwrap_or(self.player.record_index_1_based as u8);
                let Some(empire_id) =
                    resolve_default_u8_input(self.planet.database_input.trim(), default_empire)
                else {
                    return;
                };
                self.apply_planet_database_filter(PlanetDatabaseFilter::Empire(empire_id));
            }
            PlanetDatabasePromptMode::FilterMaxProductionInput => {
                let default_prod = self
                    .planet
                    .database_prompt_default_value
                    .trim()
                    .parse::<u16>()
                    .unwrap_or(100);
                let min_prod =
                    resolve_default_u16_input(self.planet.database_input.trim(), default_prod)
                        .unwrap_or(default_prod);
                self.apply_planet_database_filter(PlanetDatabaseFilter::MaxProduction(min_prod));
            }
            PlanetDatabasePromptMode::SortMenu | PlanetDatabasePromptMode::SortRangeInput => {}
        }
    }

    pub fn submit_planet_database_sort(&mut self, mode: PlanetDatabaseSortMode) {
        if self.current_screen != ScreenId::PlanetDatabaseFilterPrompt {
            return;
        }
        match self.planet.database_prompt_mode {
            PlanetDatabasePromptMode::SortMenu => match mode {
                PlanetDatabaseSortMode::Location => {
                    self.apply_planet_database_sort(PlanetDatabaseSort::Location);
                }
                PlanetDatabaseSortMode::Range => {
                    self.planet.database_prompt_mode = PlanetDatabasePromptMode::SortRangeInput;
                    self.planet.database_input.clear();
                    let default = self.default_planet_database_coords();
                    self.planet.database_prompt_default_value =
                        format!("{:02},{:02}", default[0], default[1]);
                    self.planet.database_status = None;
                }
                PlanetDatabaseSortMode::Empire => {
                    self.apply_planet_database_sort(PlanetDatabaseSort::Empire);
                }
                PlanetDatabaseSortMode::MaxProduction => {
                    self.apply_planet_database_sort(PlanetDatabaseSort::MaxProduction);
                }
            },
            PlanetDatabasePromptMode::SortRangeInput => {
                let default_coords = self.default_planet_database_coords();
                let Some(coords) =
                    resolve_default_coords_input(self.planet.database_input.trim(), default_coords)
                else {
                    self.planet.database_status = Some("Enter coordinates like 5,2".to_string());
                    return;
                };
                self.apply_planet_database_sort(PlanetDatabaseSort::Range(coords));
            }
            _ => {}
        }
    }

    pub fn append_planet_tax_char(&mut self, ch: char) {
        if self.inline_planet_tax_active_on_current_screen() && self.planet.tax_input.len() < 3 {
            self.planet.tax_input.push(ch);
            self.planet.tax_error = None;
            self.planet.tax_notice = None;
        }
    }

    pub fn backspace_planet_tax_input(&mut self) {
        if self.inline_planet_tax_active_on_current_screen() {
            self.planet.tax_input.pop();
            self.planet.tax_error = None;
            self.planet.tax_notice = None;
        }
    }

    pub fn submit_planet_tax(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.inline_planet_tax_active_on_current_screen() {
            return Ok(());
        }
        let raw = self.planet.tax_input.trim();
        let parsed = if raw.is_empty() {
            self.game_data.player.records[self.player.record_index_1_based - 1].tax_rate()
        } else {
            match raw.parse::<u8>() {
                Ok(value) => value,
                Err(_) => {
                    self.planet.tax_error =
                        Some("Enter an integer tax rate from 0 to 100.".to_string());
                    return Ok(());
                }
            }
        };
        if parsed > 100 {
            self.planet.tax_error = Some("Enter an integer tax rate from 0 to 100.".to_string());
            return Ok(());
        }
        self.game_data
            .set_player_tax_rate(self.player.record_index_1_based, parsed)?;
        self.save_game_data()?;
        self.close_planet_tax_prompt();
        self.show_command_menu_notice(
            CommandMenu::Planet,
            format!("Empire tax rate set to {parsed}%."),
        );
        Ok(())
    }

    pub fn open_planet_info_prompt(&mut self, menu: CommandMenu) {
        self.close_planet_tax_prompt();
        self.close_planet_auto_commission_prompt();
        self.close_planet_build_abort_prompt();
        self.messaging.delete_reviewables_prompt_active = false;
        if menu == CommandMenu::PlanetBuild {
            self.planet.build_status = None;
        }
        self.command_return_menu = menu;
        self.return_screen = None;
        self.clear_command_menu_notice();
        self.planet.info_prompt_active = true;
        self.planet.info_input.clear();
        self.planet.info_error = None;
        self.planet.info_selected = None;
        self.current_screen = match menu {
            CommandMenu::Main => ScreenId::MainMenu,
            CommandMenu::General => ScreenId::GeneralMenu,
            CommandMenu::Fleet => ScreenId::FleetMenu,
            CommandMenu::Starbase => ScreenId::StarbaseMenu,
            CommandMenu::Planet => ScreenId::PlanetMenu,
            CommandMenu::PlanetBuild => ScreenId::PlanetBuildMenu,
        };
    }

    pub fn close_planet_info_prompt(&mut self) {
        self.planet.info_prompt_active = false;
        self.planet.info_input.clear();
        self.planet.info_error = None;
    }

    pub fn append_planet_info_char(&mut self, ch: char) {
        if self.planet.info_input.len() < 16 {
            self.planet.info_input.push(ch);
            self.planet.info_error = None;
        }
    }

    fn sync_planet_brief_cursor_to_input(&mut self, sort: PlanetListSort) {
        let rows = self.sorted_planet_rows(sort);
        let match_rows = rows
            .iter()
            .map(|row| vec![crate::screen::format_sector_coords_table(row.coords)])
            .collect::<Vec<_>>();
        let Some(index) = crate::screen::table_selection::find_typed_jump_index(
            &match_rows,
            0,
            &self.planet.brief_input,
        ) else {
            return;
        };
        self.planet.brief_cursor = index;
        let visible_rows = self.planet_brief_visible_rows();
        sync_scroll_to_cursor(
            &mut self.planet.brief_scroll_offset,
            self.planet.brief_cursor,
            visible_rows,
        );
    }

    fn sync_planet_database_cursor_to_input(&mut self) {
        let rows = self.planet_database_rows();
        let match_rows = rows
            .iter()
            .map(|row| vec![crate::screen::format_sector_coords_table(row.coords)])
            .collect::<Vec<_>>();
        let Some(index) = crate::screen::table_selection::find_typed_jump_index(
            &match_rows,
            0,
            &self.planet.database_input,
        ) else {
            return;
        };
        self.planet.database_cursor = index;
        let visible_rows = self.planet_database_visible_rows();
        sync_scroll_to_cursor(
            &mut self.planet.database_scroll_offset,
            self.planet.database_cursor,
            visible_rows,
        );
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

        if let Err(message) = self.open_planet_info_detail_at_coords(coords, None) {
            self.planet.info_error = Some(message);
        }
    }

    pub fn open_planet_info_detail_at_coords(
        &mut self,
        coords: [u8; 2],
        return_screen: Option<ScreenId>,
    ) -> Result<(), String> {
        let Some(planet_idx) = self.game_data.planet_record_index_at_coords(coords) else {
            return Err(format!(
                "No world found at [{:02},{:02}]",
                coords[0], coords[1]
            ));
        };

        self.return_screen = return_screen;
        self.planet.info_prompt_active = false;
        self.planet.info_selected = Some(planet_idx);
        self.planet.info_error = None;
        self.current_screen = ScreenId::PlanetInfoDetail;
        Ok(())
    }

    pub(crate) fn inline_planet_tax_active_on_current_screen(&self) -> bool {
        self.planet.tax_prompt_active && self.current_screen == ScreenId::PlanetMenu
    }

    pub(crate) fn inline_planet_transport_prompt_active_on_current_screen(&self) -> bool {
        self.current_screen == ScreenId::PlanetMenu && self.planet.transport_prompt_mode.is_some()
    }

    pub(crate) fn inline_planet_auto_commission_active_on_current_screen(&self) -> bool {
        self.planet.auto_commission_prompt_active && self.current_screen == ScreenId::PlanetMenu
    }

    pub(crate) fn inline_planet_build_abort_active_on_current_screen(&self) -> bool {
        self.planet.build_abort_prompt_active && self.current_screen == ScreenId::PlanetBuildMenu
    }

    pub(crate) fn inline_planet_info_active_on_current_screen(&self) -> bool {
        self.planet.info_prompt_active
            && matches!(
                self.current_screen,
                ScreenId::MainMenu
                    | ScreenId::GeneralMenu
                    | ScreenId::FleetMenu
                    | ScreenId::StarbaseMenu
                    | ScreenId::PlanetMenu
                    | ScreenId::PlanetBuildMenu
            )
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
    ) -> Vec<nc_data::EmpirePlanetEconomyRow> {
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
                known_owner_empire_id: world.known_owner_empire_id,
                known_max_production: world.known_potential_production,
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
                starbase_count_label: world
                    .known_starbase_count
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
            }
        })
        .collect::<Vec<_>>();
        rows.retain(|row| match self.planet.database_filter {
            PlanetDatabaseFilter::All => true,
            PlanetDatabaseFilter::Range { anchor, radius } => {
                planet_database_distance_sq(anchor, row.coords)
                    <= u32::from(radius) * u32::from(radius)
            }
            PlanetDatabaseFilter::Empire(empire_id) => row.known_owner_empire_id == Some(empire_id),
            PlanetDatabaseFilter::MaxProduction(min_prod) => row
                .known_max_production
                .is_some_and(|value| value >= min_prod),
        });

        match self.planet.database_sort {
            PlanetDatabaseSort::Location => rows.sort_by_key(|row| row.coords),
            PlanetDatabaseSort::Range(anchor) => rows
                .sort_by_key(|row| (planet_database_distance_sq(anchor, row.coords), row.coords)),
            PlanetDatabaseSort::Empire => rows.sort_by_key(|row| {
                (
                    row.known_owner_empire_id.is_none(),
                    row.known_owner_empire_id.unwrap_or(0),
                    row.coords,
                )
            }),
            PlanetDatabaseSort::MaxProduction => rows.sort_by(|left, right| {
                right
                    .known_max_production
                    .cmp(&left.known_max_production)
                    .then_with(|| left.coords.cmp(&right.coords))
            }),
        }
        rows
    }

    pub(crate) fn default_planet_database_coords(&self) -> [u8; 2] {
        self.planet_database_rows()
            .get(self.planet.database_cursor)
            .map(|row| row.coords)
            .unwrap_or([0, 0])
    }

    fn apply_planet_database_filter(&mut self, filter: PlanetDatabaseFilter) {
        let selected_record = self
            .planet_database_rows()
            .get(self.planet.database_cursor)
            .map(|row| row.planet_record_index_1_based);
        self.planet.database_filter = filter;
        self.planet.database_prompt_mode = PlanetDatabasePromptMode::FilterMenu;
        self.planet.database_status = None;
        self.planet.database_input.clear();
        self.planet.database_prompt_default_value.clear();
        self.planet.database_pending_range_anchor = None;
        self.current_screen = ScreenId::PlanetDatabaseList;

        let rows = self.planet_database_rows();
        if rows.is_empty() {
            self.planet.database_cursor = 0;
            self.planet.database_scroll_offset = 0;
            return;
        }

        self.planet.database_cursor = selected_record
            .and_then(|record| {
                rows.iter()
                    .position(|row| row.planet_record_index_1_based == record)
            })
            .unwrap_or(0);
        let visible_rows = self.planet_database_visible_rows();
        sync_scroll_to_cursor(
            &mut self.planet.database_scroll_offset,
            self.planet.database_cursor,
            visible_rows,
        );
    }

    fn apply_planet_database_sort(&mut self, sort: PlanetDatabaseSort) {
        let selected_record = self
            .planet_database_rows()
            .get(self.planet.database_cursor)
            .map(|row| row.planet_record_index_1_based);
        self.planet.database_sort = sort;
        self.planet.database_prompt_mode = PlanetDatabasePromptMode::FilterMenu;
        self.planet.database_status = None;
        self.planet.database_input.clear();
        self.planet.database_prompt_default_value.clear();
        self.planet.database_pending_range_anchor = None;
        self.current_screen = ScreenId::PlanetDatabaseList;

        let rows = self.planet_database_rows();
        if rows.is_empty() {
            self.planet.database_cursor = 0;
            self.planet.database_scroll_offset = 0;
            return;
        }

        self.planet.database_cursor = selected_record
            .and_then(|record| {
                rows.iter()
                    .position(|row| row.planet_record_index_1_based == record)
            })
            .unwrap_or(0);
        let visible_rows = self.planet_database_visible_rows();
        sync_scroll_to_cursor(
            &mut self.planet.database_scroll_offset,
            self.planet.database_cursor,
            visible_rows,
        );
    }

    pub(crate) fn handle_planet_info_prompt_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                crate::app::Action::Planet(PlanetAction::CloseInfoPrompt)
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

    pub(crate) fn handle_planet_auto_commission_prompt_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                crate::app::Action::Planet(PlanetAction::ConfirmAutoCommission)
            }
            KeyCode::Char('q')
            | KeyCode::Char('Q')
            | KeyCode::Char('n')
            | KeyCode::Char('N')
            | KeyCode::Enter
            | KeyCode::Esc => crate::app::Action::Planet(PlanetAction::CloseAutoCommissionPrompt),
            _ => crate::app::Action::Noop,
        }
    }

    pub(crate) fn handle_planet_build_abort_prompt_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                crate::app::Action::Planet(PlanetAction::ConfirmBuildAbort)
            }
            KeyCode::Char('q')
            | KeyCode::Char('Q')
            | KeyCode::Char('n')
            | KeyCode::Char('N')
            | KeyCode::Enter
            | KeyCode::Esc => crate::app::Action::Planet(PlanetAction::CloseBuildAbortPrompt),
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

    fn select_planet_brief_origin_row(&mut self, mode: PlanetListMode, sort: PlanetListSort) {
        if mode != PlanetListMode::BuildSelect {
            return;
        }
        let Some(selected_record) = self
            .build_planet_rows()
            .get(self.planet.build_index)
            .map(|row| row.planet_record_index_1_based)
        else {
            return;
        };
        let rows = self.sorted_planet_rows(sort);
        let Some(index) = rows
            .iter()
            .position(|row| row.planet_record_index_1_based == selected_record)
        else {
            return;
        };
        self.planet.brief_cursor = index;
        let visible_rows = self.planet_brief_visible_rows();
        sync_scroll_to_cursor(
            &mut self.planet.brief_scroll_offset,
            self.planet.brief_cursor,
            visible_rows,
        );
    }

    fn open_planet_build_menu_at_coords(
        &mut self,
        coords: [u8; 2],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let rows = self.build_planet_rows();
        let Some(index) = rows.iter().position(|row| row.coords == coords) else {
            self.planet.list_sort_status = Some(format!(
                "No build target found at ({:02},{:02})",
                coords[0], coords[1]
            ));
            return Ok(());
        };
        self.planet.build_index = index;
        self.planet.list_sort_status = None;
        self.open_planet_build_menu();
        Ok(())
    }
}

fn planet_database_distance_sq(a: [u8; 2], b: [u8; 2]) -> u32 {
    let dx = i32::from(a[0]) - i32::from(b[0]);
    let dy = i32::from(a[1]) - i32::from(b[1]);
    (dx * dx + dy * dy) as u32
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
