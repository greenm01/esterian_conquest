//! App struct, main loop, and action dispatch.

pub mod input;
pub mod render;
pub mod state;

use crossterm::event::{Event, KeyCode, KeyEvent};
use input::{key_to_action, Action};
use nc_ui::table_selection;
use nc_ui::{ScreenGeometry, Terminal};
use state::{
    ActiveOverlay, ActivePopup, DashApp, FleetOverlayFilter, FleetOverlayPromptMode,
    FleetOverlaySort, HelpContext, IntelOverlayFilter, IntelOverlayPromptMode, IntelOverlaySort,
    MapViewMode, PlanetOverlayFilter, PlanetOverlayPromptMode, PlanetOverlaySort,
};

use crate::inbox::{project_inbox_items, DashInboxItemSource};
use crate::layout::dashboard;
use crate::overlays::{fleet_list, inbox, intel_database, planet_list};
use crate::panels::starmap;
use crate::planet_view;

impl DashApp {
    /// Run the main event loop.
    pub fn run(&mut self, terminal: &mut dyn Terminal) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let playfield = render::render(self)?;
            terminal.render(&playfield)?;
            if self.should_quit {
                break;
            }
            match terminal.read_event()? {
                Event::Key(key) => {
                    if !self.is_terminal_too_small {
                        self.handle_key(key);
                    } else if key.code == KeyCode::Char('q') || key.code == KeyCode::Char('Q') {
                        self.should_quit = true;
                    }
                }
                Event::Resize(cols, rows) => {
                    self.geometry = ScreenGeometry::new(cols as usize, rows as usize);
                    let required = dashboard::required_dashboard_frame(self);
                    if self.geometry.width() < required.width() || self.geometry.height() < required.height() {
                        self.is_terminal_too_small = true;
                    } else {
                        self.is_terminal_too_small = false;
                        self.frame = required;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        if self.popup != ActivePopup::None && self.handle_popup_key(key) {
            return;
        }
        if self.overlay != ActiveOverlay::None && self.handle_overlay_key(key) {
            return;
        }
        if self.handle_map_coord_key(key) {
            return;
        }
        let action = key_to_action(key, self.focus, self.overlay);
        if action != Action::None {
            self.map_coord_input.clear();
        }
        self.apply_action(action);
    }

    fn apply_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::FocusNext => self.focus = self.focus.next(),
            Action::FocusPrev => self.focus = self.focus.prev(),
            Action::ToggleAutopilot => self.autopilot_on = !self.autopilot_on,
            Action::OpenOverlay(overlay) => {
                if overlay == ActiveOverlay::Help {
                    self.help_context = HelpContext::Global;
                    self.help_return_overlay = ActiveOverlay::None;
                }
                self.overlay = overlay;
            }
            Action::CloseOverlay => self.close_active_overlay(),
            Action::ClosePopup => self.popup = ActivePopup::None,
            Action::MoveCrosshairUp => {
                // Up arrow → higher Y (row 18 at top of screen).
                let map_size =
                    nc_data::map_size_for_player_count(self.game_data.conquest.player_count());
                if self.crosshair_y < map_size {
                    self.crosshair_y += 1;
                }
            }
            Action::MoveCrosshairDown => {
                // Down arrow → lower Y (row 1 at bottom of screen).
                if self.crosshair_y > 1 {
                    self.crosshair_y -= 1;
                }
            }
            Action::MoveCrosshairLeft => {
                if self.crosshair_x > 1 {
                    self.crosshair_x -= 1;
                }
            }
            Action::MoveCrosshairRight => {
                let map_size =
                    nc_data::map_size_for_player_count(self.game_data.conquest.player_count());
                if self.crosshair_x < map_size {
                    self.crosshair_x += 1;
                }
            }
            Action::JumpPlanetBackward => {
                self.jump_crosshair_to_planet(starmap::PlanetJumpDirection::Backward);
            }
            Action::JumpPlanetForward => {
                self.jump_crosshair_to_planet(starmap::PlanetJumpDirection::Forward);
            }
            Action::ToggleMapViewMode => {
                self.map_view_mode = match self.map_view_mode {
                    MapViewMode::Readable => MapViewMode::Fill,
                    MapViewMode::Fill => MapViewMode::Readable,
                };
            }
            Action::ZoomMapIn => {
                self.map_zoom_level = self.map_zoom_level.saturating_add(1).min(5);
            }
            Action::ZoomMapOut => {
                self.map_zoom_level = self.map_zoom_level.saturating_sub(1);
            }
            Action::ResetMapZoom => self.map_zoom_level = 0,
            Action::OpenPlanetDetailPopup => self.open_planet_detail_popup_at_cursor(),
            Action::ScrollUp => self.scroll_up(),
            Action::ScrollDown => self.scroll_down(),
            Action::PageUp => {
                for _ in 0..10 {
                    self.scroll_up();
                }
            }
            Action::PageDown => {
                for _ in 0..10 {
                    self.scroll_down();
                }
            }
            Action::Home => self.scroll_home(),
            Action::End => self.scroll_end(),
            // SetTaxRate requires a multi-key input prompt.
            Action::SetTaxRate | Action::None => {}
        }
    }

    fn handle_map_coord_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(ch)
                if self.map_coord_input.len() < 16
                    && !matches!(ch, '[' | ']')
                    && table_selection::is_coordinate_input_char(ch) =>
            {
                self.map_coord_input.push(ch);
                if self.sync_map_cursor_to_input() {
                    self.map_coord_input.clear();
                }
                true
            }
            KeyCode::Backspace => {
                self.map_coord_input.pop();
                true
            }
            _ => false,
        }
    }

    fn handle_popup_key(&mut self, key: KeyEvent) -> bool {
        let _ = key;
        self.apply_action(Action::ClosePopup);
        true
    }

    fn jump_crosshair_to_planet(&mut self, direction: starmap::PlanetJumpDirection) {
        if let Some(target) = starmap::jump_planet_target_for_app(
            self,
            [self.crosshair_x, self.crosshair_y],
            direction,
        ) {
            self.crosshair_x = target[0];
            self.crosshair_y = target[1];
        }
    }

    fn open_planet_detail_popup_at_cursor(&mut self) {
        let Some(detail) = planet_view::selected_planet_detail(self) else {
            return;
        };
        self.popup = ActivePopup::PlanetDetail {
            planet_record_index_1_based: detail.planet_record_index_1_based,
        };
    }

    fn sync_map_cursor_to_input(&mut self) -> bool {
        let rows = map_coord_rows(self);
        let Some(matched) = table_selection::find_typed_jump(&rows, 0, &self.map_coord_input)
        else {
            return false;
        };
        let Some(coords) = rows
            .get(matched.index)
            .and_then(|row| row.first())
            .and_then(|coords| parse_table_coord(coords))
        else {
            return false;
        };
        self.crosshair_x = coords[0];
        self.crosshair_y = coords[1];
        matched.is_terminal_exact_match
    }

    fn scroll_up(&mut self) {
        use state::PanelFocus::*;
        match self.focus {
            Diplomacy => self.diplomacy_scroll = self.diplomacy_scroll.saturating_sub(1),
            _ => {}
        }
    }

    fn scroll_down(&mut self) {
        use state::PanelFocus::*;
        match self.focus {
            Diplomacy => self.diplomacy_scroll += 1,
            _ => {}
        }
    }

    fn scroll_home(&mut self) {
        use state::PanelFocus::*;
        match self.focus {
            Diplomacy => self.diplomacy_scroll = 0,
            _ => {}
        }
    }

    fn scroll_end(&mut self) {
        // End scrolling: set to a large number; render will clamp.
        use state::PanelFocus::*;
        match self.focus {
            Diplomacy => self.diplomacy_scroll = usize::MAX,
            _ => {}
        }
    }

    fn handle_overlay_key(&mut self, key: KeyEvent) -> bool {
        match self.overlay {
            ActiveOverlay::None => false,
            ActiveOverlay::PlanetList => {
                self.handle_planet_overlay_key(key);
                true
            }
            ActiveOverlay::FleetList => {
                self.handle_fleet_overlay_key(key);
                true
            }
            ActiveOverlay::IntelDatabase => {
                self.handle_intel_overlay_key(key);
                true
            }
            ActiveOverlay::Diplomacy => {
                if self.handle_overlay_close_or_help(key, HelpContext::Diplomacy) {
                    return true;
                }
                let total_rows = self.game_data.player.records.len();
                handle_list_overlay_key(
                    key,
                    &mut self.diplomacy_overlay.selected,
                    &mut self.diplomacy_overlay.scroll,
                    total_rows,
                );
                true
            }
            ActiveOverlay::Inbox => {
                if self.handle_overlay_close_or_help(key, HelpContext::Inbox) {
                    return true;
                }
                self.handle_inbox_overlay_key(key);
                true
            }
            ActiveOverlay::Settings | ActiveOverlay::Help => {
                let _ = key;
                self.close_active_overlay();
                true
            }
        }
    }

    fn handle_overlay_close_or_help(&mut self, key: KeyEvent, help_context: HelpContext) -> bool {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.close_active_overlay();
                true
            }
            KeyCode::Char('?') if self.overlay == ActiveOverlay::Help => {
                self.close_active_overlay();
                true
            }
            KeyCode::Char('?') => {
                self.help_return_overlay = self.overlay;
                self.help_context = help_context;
                self.overlay = ActiveOverlay::Help;
                true
            }
            _ => false,
        }
    }

    fn close_active_overlay(&mut self) {
        if self.overlay == ActiveOverlay::Help {
            self.overlay = self.help_return_overlay;
            self.help_return_overlay = ActiveOverlay::None;
            self.help_context = HelpContext::Global;
        } else {
            self.overlay = ActiveOverlay::None;
        }
    }

    fn handle_planet_overlay_key(&mut self, key: KeyEvent) {
        match self.planet_overlay.prompt_mode {
            PlanetOverlayPromptMode::SortMenu => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PlanetListSort),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    self.reset_planet_overlay_prompt();
                }
                KeyCode::Enter | KeyCode::Char('c') | KeyCode::Char('C') => {
                    self.apply_planet_overlay_sort(PlanetOverlaySort::CurrentProduction);
                }
                KeyCode::Char('l') | KeyCode::Char('L') => {
                    self.apply_planet_overlay_sort(PlanetOverlaySort::Location);
                }
                KeyCode::Char('m') | KeyCode::Char('M') => {
                    self.apply_planet_overlay_sort(PlanetOverlaySort::MaxProduction);
                }
                _ => {}
            },
            PlanetOverlayPromptMode::FilterMenu => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PlanetListFilter),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    self.reset_planet_overlay_prompt();
                }
                KeyCode::Enter | KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.apply_planet_overlay_filter(PlanetOverlayFilter::All);
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    self.planet_overlay.prompt_mode = PlanetOverlayPromptMode::FilterRangeCoords;
                    self.planet_overlay.prompt_input.clear();
                    self.planet_overlay.prompt_default = planet_list::table_rows(self)
                        .get(self.planet_overlay.selected)
                        .map(|row| nc_ui::coords::format_sector_coords_default(row.coords))
                        .unwrap_or_else(|| "00,00".to_string());
                    self.planet_overlay.pending_range_anchor = None;
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.apply_planet_overlay_filter(PlanetOverlayFilter::Starbase);
                }
                KeyCode::Char('t') | KeyCode::Char('T') => {
                    self.apply_planet_overlay_filter(PlanetOverlayFilter::Stardock);
                }
                _ => {}
            },
            PlanetOverlayPromptMode::FilterRangeCoords => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PromptInput),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    self.reset_planet_overlay_prompt();
                }
                KeyCode::Enter => {
                    let default = planet_list::parse_coords_input(
                        &self.planet_overlay.prompt_default,
                        [0, 0],
                    )
                    .unwrap_or([0, 0]);
                    if let Some(anchor) =
                        planet_list::parse_coords_input(&self.planet_overlay.prompt_input, default)
                    {
                        self.planet_overlay.pending_range_anchor = Some(anchor);
                        self.planet_overlay.prompt_mode =
                            PlanetOverlayPromptMode::FilterRangeDistance;
                        self.planet_overlay.prompt_input.clear();
                        self.planet_overlay.prompt_default = "5".to_string();
                    }
                }
                KeyCode::Backspace => {
                    self.planet_overlay.prompt_input.pop();
                }
                KeyCode::Char(ch) if table_selection::is_coordinate_input_char(ch) => {
                    self.planet_overlay.prompt_input.push(ch);
                }
                _ => {}
            },
            PlanetOverlayPromptMode::FilterRangeDistance => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PromptInput),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    self.reset_planet_overlay_prompt();
                }
                KeyCode::Enter => {
                    let default = self
                        .planet_overlay
                        .prompt_default
                        .trim()
                        .parse::<u8>()
                        .unwrap_or(5);
                    let radius = if self.planet_overlay.prompt_input.trim().is_empty() {
                        default
                    } else {
                        match self.planet_overlay.prompt_input.trim().parse::<u8>() {
                            Ok(value) => value,
                            Err(_) => return,
                        }
                    };
                    let Some(anchor) = self.planet_overlay.pending_range_anchor else {
                        return;
                    };
                    self.apply_planet_overlay_filter(PlanetOverlayFilter::Range { anchor, radius });
                }
                KeyCode::Backspace => {
                    self.planet_overlay.prompt_input.pop();
                }
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    self.planet_overlay.prompt_input.push(ch);
                }
                _ => {}
            },
            PlanetOverlayPromptMode::None => {}
        }
        if self.planet_overlay.prompt_mode != PlanetOverlayPromptMode::None {
            return;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => self.close_active_overlay(),
            KeyCode::Char('?') => self.open_overlay_help(HelpContext::PlanetList),
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.planet_overlay.prompt_mode = PlanetOverlayPromptMode::FilterMenu;
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.planet_overlay.prompt_mode = PlanetOverlayPromptMode::SortMenu;
            }
            KeyCode::Char(ch)
                if self.planet_overlay.jump_input.len() < 16
                    && table_selection::is_coordinate_input_char(ch) =>
            {
                self.planet_overlay.jump_input.push(ch);
                if planet_list::sync_cursor_to_jump_input(self) {
                    self.planet_overlay.jump_input.clear();
                }
            }
            KeyCode::Backspace => {
                self.planet_overlay.jump_input.pop();
            }
            _ => {
                let total_rows = planet_list::selection_rows(self).len();
                handle_list_overlay_key(
                    key,
                    &mut self.planet_overlay.selected,
                    &mut self.planet_overlay.scroll,
                    total_rows,
                );
            }
        }
    }

    fn handle_fleet_overlay_key(&mut self, key: KeyEvent) {
        match self.fleet_overlay.prompt_mode {
            FleetOverlayPromptMode::SortMenu => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::FleetListSort),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    self.fleet_overlay.prompt_mode = FleetOverlayPromptMode::None;
                }
                KeyCode::Enter | KeyCode::Char('i') | KeyCode::Char('I') => {
                    self.apply_fleet_overlay_sort(FleetOverlaySort::Id);
                }
                KeyCode::Char('l') | KeyCode::Char('L') => {
                    self.apply_fleet_overlay_sort(FleetOverlaySort::Location);
                }
                KeyCode::Char('o') | KeyCode::Char('O') => {
                    self.apply_fleet_overlay_sort(FleetOverlaySort::Order);
                }
                KeyCode::Char('e') | KeyCode::Char('E') => {
                    self.apply_fleet_overlay_sort(FleetOverlaySort::Eta);
                }
                KeyCode::Char('t') | KeyCode::Char('T') => {
                    self.apply_fleet_overlay_sort(FleetOverlaySort::Strength);
                }
                _ => {}
            },
            FleetOverlayPromptMode::FilterMenu => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::FleetListFilter),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    self.fleet_overlay.prompt_mode = FleetOverlayPromptMode::None;
                }
                KeyCode::Enter | KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.apply_fleet_overlay_filter(FleetOverlayFilter::All);
                }
                KeyCode::Char('h') | KeyCode::Char('H') => {
                    self.apply_fleet_overlay_filter(FleetOverlayFilter::Holding);
                }
                KeyCode::Char('m') | KeyCode::Char('M') => {
                    self.apply_fleet_overlay_filter(FleetOverlayFilter::Moving);
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    self.apply_fleet_overlay_filter(FleetOverlayFilter::Combat);
                }
                _ => {}
            },
            FleetOverlayPromptMode::None => {}
        }
        if self.fleet_overlay.prompt_mode != FleetOverlayPromptMode::None {
            return;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => self.close_active_overlay(),
            KeyCode::Char('?') => self.open_overlay_help(HelpContext::FleetList),
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.fleet_overlay.prompt_mode = FleetOverlayPromptMode::FilterMenu;
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.fleet_overlay.prompt_mode = FleetOverlayPromptMode::SortMenu;
            }
            KeyCode::Char(ch)
                if self.fleet_overlay.jump_input.len() < 8 && ch.is_ascii_alphanumeric() =>
            {
                self.fleet_overlay.jump_input.push(ch);
                if fleet_list::sync_cursor_to_jump_input(self) {
                    self.fleet_overlay.jump_input.clear();
                }
            }
            KeyCode::Backspace => {
                self.fleet_overlay.jump_input.pop();
            }
            _ => {
                let total_rows = fleet_list::selection_rows(self).len();
                handle_list_overlay_key(
                    key,
                    &mut self.fleet_overlay.selected,
                    &mut self.fleet_overlay.scroll,
                    total_rows,
                );
            }
        }
    }

    fn handle_intel_overlay_key(&mut self, key: KeyEvent) {
        match self.intel_overlay.prompt_mode {
            IntelOverlayPromptMode::SortMenu => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::IntelDatabaseSort),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    self.reset_intel_overlay_prompt();
                }
                KeyCode::Enter | KeyCode::Char('l') | KeyCode::Char('L') => {
                    self.apply_intel_overlay_sort(IntelOverlaySort::Location);
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    self.intel_overlay.prompt_mode = IntelOverlayPromptMode::SortRangeInput;
                    self.intel_overlay.prompt_input.clear();
                    self.intel_overlay.prompt_default = intel_database::table_rows(self)
                        .get(self.intel_overlay.selected)
                        .map(|row| nc_ui::coords::format_sector_coords_default(row.coords))
                        .unwrap_or_else(|| "00,00".to_string());
                }
                KeyCode::Char('e') | KeyCode::Char('E') => {
                    self.apply_intel_overlay_sort(IntelOverlaySort::Empire);
                }
                KeyCode::Char('m') | KeyCode::Char('M') => {
                    self.apply_intel_overlay_sort(IntelOverlaySort::MaxProduction);
                }
                _ => {}
            },
            IntelOverlayPromptMode::SortRangeInput => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PromptInput),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    self.reset_intel_overlay_prompt();
                }
                KeyCode::Enter => {
                    let default = intel_database::parse_coords_input(
                        &self.intel_overlay.prompt_default,
                        [0, 0],
                    )
                    .unwrap_or([0, 0]);
                    if let Some(anchor) =
                        intel_database::parse_coords_input(&self.intel_overlay.prompt_input, default)
                    {
                        self.apply_intel_overlay_sort(IntelOverlaySort::Range(anchor));
                    }
                }
                KeyCode::Backspace => {
                    self.intel_overlay.prompt_input.pop();
                }
                KeyCode::Char(ch) if table_selection::is_coordinate_input_char(ch) => {
                    self.intel_overlay.prompt_input.push(ch);
                }
                _ => {}
            },
            IntelOverlayPromptMode::FilterMenu => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::IntelDatabaseFilter),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    self.reset_intel_overlay_prompt();
                }
                KeyCode::Enter | KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.apply_intel_overlay_filter(IntelOverlayFilter::All);
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    self.intel_overlay.prompt_mode = IntelOverlayPromptMode::FilterRangeCoords;
                    self.intel_overlay.prompt_input.clear();
                    self.intel_overlay.prompt_default = intel_database::table_rows(self)
                        .get(self.intel_overlay.selected)
                        .map(|row| nc_ui::coords::format_sector_coords_default(row.coords))
                        .unwrap_or_else(|| "00,00".to_string());
                    self.intel_overlay.pending_range_anchor = None;
                }
                KeyCode::Char('e') | KeyCode::Char('E') => {
                    self.intel_overlay.prompt_mode = IntelOverlayPromptMode::FilterEmpireInput;
                    self.intel_overlay.prompt_input.clear();
                    self.intel_overlay.prompt_default = intel_database::table_rows(self)
                        .get(self.intel_overlay.selected)
                        .and_then(|row| row.known_owner_empire_id)
                        .unwrap_or(self.player_record_index_1_based as u8)
                        .to_string();
                }
                KeyCode::Char('m') | KeyCode::Char('M') => {
                    self.intel_overlay.prompt_mode =
                        IntelOverlayPromptMode::FilterMaxProductionInput;
                    self.intel_overlay.prompt_input.clear();
                    self.intel_overlay.prompt_default = intel_database::table_rows(self)
                        .get(self.intel_overlay.selected)
                        .and_then(|row| row.known_max_production)
                        .unwrap_or(100)
                        .to_string();
                }
                _ => {}
            },
            IntelOverlayPromptMode::FilterRangeCoords => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PromptInput),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    self.reset_intel_overlay_prompt();
                }
                KeyCode::Enter => {
                    let default = intel_database::parse_coords_input(
                        &self.intel_overlay.prompt_default,
                        [0, 0],
                    )
                    .unwrap_or([0, 0]);
                    if let Some(anchor) =
                        intel_database::parse_coords_input(&self.intel_overlay.prompt_input, default)
                    {
                        self.intel_overlay.pending_range_anchor = Some(anchor);
                        self.intel_overlay.prompt_mode =
                            IntelOverlayPromptMode::FilterRangeDistance;
                        self.intel_overlay.prompt_input.clear();
                        self.intel_overlay.prompt_default = "5".to_string();
                    }
                }
                KeyCode::Backspace => {
                    self.intel_overlay.prompt_input.pop();
                }
                KeyCode::Char(ch) if table_selection::is_coordinate_input_char(ch) => {
                    self.intel_overlay.prompt_input.push(ch);
                }
                _ => {}
            },
            IntelOverlayPromptMode::FilterRangeDistance => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PromptInput),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    self.reset_intel_overlay_prompt();
                }
                KeyCode::Enter => {
                    let default = self.intel_overlay.prompt_default.parse::<u8>().unwrap_or(5);
                    let radius = if self.intel_overlay.prompt_input.trim().is_empty() {
                        default
                    } else {
                        match self.intel_overlay.prompt_input.trim().parse::<u8>() {
                            Ok(value) => value,
                            Err(_) => return,
                        }
                    };
                    let Some(anchor) = self.intel_overlay.pending_range_anchor else {
                        return;
                    };
                    self.apply_intel_overlay_filter(IntelOverlayFilter::Range { anchor, radius });
                }
                KeyCode::Backspace => {
                    self.intel_overlay.prompt_input.pop();
                }
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    self.intel_overlay.prompt_input.push(ch);
                }
                _ => {}
            },
            IntelOverlayPromptMode::FilterEmpireInput => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PromptInput),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    self.reset_intel_overlay_prompt();
                }
                KeyCode::Enter => {
                    let default = self
                        .intel_overlay
                        .prompt_default
                        .parse::<u8>()
                        .unwrap_or(self.player_record_index_1_based as u8);
                    let empire = if self.intel_overlay.prompt_input.trim().is_empty() {
                        default
                    } else {
                        match self.intel_overlay.prompt_input.trim().parse::<u8>() {
                            Ok(value) => value,
                            Err(_) => return,
                        }
                    };
                    self.apply_intel_overlay_filter(IntelOverlayFilter::Empire(empire));
                }
                KeyCode::Backspace => {
                    self.intel_overlay.prompt_input.pop();
                }
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    self.intel_overlay.prompt_input.push(ch);
                }
                _ => {}
            },
            IntelOverlayPromptMode::FilterMaxProductionInput => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PromptInput),
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    self.reset_intel_overlay_prompt();
                }
                KeyCode::Enter => {
                    let default = self.intel_overlay.prompt_default.parse::<u16>().unwrap_or(100);
                    let min_prod = if self.intel_overlay.prompt_input.trim().is_empty() {
                        default
                    } else {
                        match self.intel_overlay.prompt_input.trim().parse::<u16>() {
                            Ok(value) => value,
                            Err(_) => return,
                        }
                    };
                    self.apply_intel_overlay_filter(IntelOverlayFilter::MaxProduction(min_prod));
                }
                KeyCode::Backspace => {
                    self.intel_overlay.prompt_input.pop();
                }
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    self.intel_overlay.prompt_input.push(ch);
                }
                _ => {}
            },
            IntelOverlayPromptMode::None => {}
        }
        if self.intel_overlay.prompt_mode != IntelOverlayPromptMode::None {
            return;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => self.close_active_overlay(),
            KeyCode::Char('?') => self.open_overlay_help(HelpContext::IntelDatabase),
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.intel_overlay.prompt_mode = IntelOverlayPromptMode::FilterMenu;
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.intel_overlay.prompt_mode = IntelOverlayPromptMode::SortMenu;
            }
            KeyCode::Char(ch)
                if self.intel_overlay.jump_input.len() < 16
                    && table_selection::is_coordinate_input_char(ch) =>
            {
                self.intel_overlay.jump_input.push(ch);
                if intel_database::sync_cursor_to_jump_input(self) {
                    self.intel_overlay.jump_input.clear();
                }
            }
            KeyCode::Backspace => {
                self.intel_overlay.jump_input.pop();
            }
            _ => {
                let total_rows = intel_database::selection_rows(self).len();
                handle_list_overlay_key(
                    key,
                    &mut self.intel_overlay.selected,
                    &mut self.intel_overlay.scroll,
                    total_rows,
                );
            }
        }
    }

    fn handle_inbox_overlay_key(&mut self, key: KeyEvent) {
        if self.inbox_overlay.delete_confirm {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    delete_selected_inbox_item(self);
                    self.inbox_overlay.delete_confirm = false;
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.inbox_overlay.delete_confirm = false;
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Tab => {
                self.inbox_overlay.focus = match self.inbox_overlay.focus {
                    state::InboxFocus::List => state::InboxFocus::Preview,
                    state::InboxFocus::Preview => state::InboxFocus::List,
                };
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.inbox_overlay.filter = state::InboxFilter::All;
                self.inbox_overlay.selected = 0;
                self.inbox_overlay.scroll = 0;
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.inbox_overlay.filter = state::InboxFilter::Reports;
                self.inbox_overlay.selected = 0;
                self.inbox_overlay.scroll = 0;
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                self.inbox_overlay.filter = state::InboxFilter::Messages;
                self.inbox_overlay.selected = 0;
                self.inbox_overlay.scroll = 0;
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.inbox_overlay.current_year_only = !self.inbox_overlay.current_year_only;
                self.inbox_overlay.selected = 0;
                self.inbox_overlay.scroll = 0;
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.inbox_overlay.delete_confirm = true;
            }
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                self.inbox_overlay.jump_input.push(ch);
                if self.sync_inbox_overlay_cursor_to_input() {
                    self.inbox_overlay.jump_input.clear();
                }
            }
            KeyCode::Backspace => {
                self.inbox_overlay.jump_input.pop();
            }
            KeyCode::Up | KeyCode::Char('k') => match self.inbox_overlay.focus {
                state::InboxFocus::List => {
                    let total_rows = inbox::selection_rows(self).len();
                    self.inbox_overlay.selected =
                        wrap_prev_index(self.inbox_overlay.selected, total_rows);
                    if self.inbox_overlay.selected < self.inbox_overlay.scroll {
                        self.inbox_overlay.scroll = self.inbox_overlay.selected;
                    }
                }
                state::InboxFocus::Preview => {
                    self.inbox_overlay.preview_scroll =
                        self.inbox_overlay.preview_scroll.saturating_sub(1);
                }
            },
            KeyCode::Down | KeyCode::Char('j') => match self.inbox_overlay.focus {
                state::InboxFocus::List => {
                    let total_rows = inbox::selection_rows(self).len();
                    self.inbox_overlay.selected =
                        wrap_next_index(self.inbox_overlay.selected, total_rows);
                }
                state::InboxFocus::Preview => {
                    self.inbox_overlay.preview_scroll += 1;
                }
            },
            KeyCode::PageUp => match self.inbox_overlay.focus {
                state::InboxFocus::List => {
                    let total_rows = inbox::selection_rows(self).len();
                    let last = total_rows.saturating_sub(1);
                    self.inbox_overlay.selected = self.inbox_overlay.selected.saturating_sub(10);
                    self.inbox_overlay.selected = self.inbox_overlay.selected.min(last);
                    self.inbox_overlay.scroll = self.inbox_overlay.scroll.saturating_sub(10);
                }
                state::InboxFocus::Preview => {
                    self.inbox_overlay.preview_scroll =
                        self.inbox_overlay.preview_scroll.saturating_sub(10);
                }
            },
            KeyCode::PageDown => match self.inbox_overlay.focus {
                state::InboxFocus::List => {
                    let total_rows = inbox::selection_rows(self).len();
                    let last = total_rows.saturating_sub(1);
                    self.inbox_overlay.selected =
                        self.inbox_overlay.selected.saturating_add(10).min(last);
                }
                state::InboxFocus::Preview => {
                    self.inbox_overlay.preview_scroll += 10;
                }
            },
            KeyCode::Home => match self.inbox_overlay.focus {
                state::InboxFocus::List => {
                    self.inbox_overlay.selected = 0;
                    self.inbox_overlay.scroll = 0;
                }
                state::InboxFocus::Preview => {
                    self.inbox_overlay.preview_scroll = 0;
                }
            },
            KeyCode::End => {
                if matches!(self.inbox_overlay.focus, state::InboxFocus::List) {
                    let last = inbox::selection_rows(self).len().saturating_sub(1);
                    self.inbox_overlay.selected = last;
                    self.inbox_overlay.scroll = self.inbox_overlay.selected.saturating_sub(5);
                } else {
                    self.inbox_overlay.preview_scroll = usize::MAX / 4;
                }
            }
            _ => {}
        }
    }

    fn sync_inbox_overlay_cursor_to_input(&mut self) -> bool {
        let rows = inbox::selection_rows(self);
        let Some(matched) =
            table_selection::find_typed_jump(&rows, 0, &self.inbox_overlay.jump_input)
        else {
            return false;
        };
        self.inbox_overlay.selected = matched.index;
        sync_scroll_to_cursor(&mut self.inbox_overlay.scroll, matched.index, 10);
        self.inbox_overlay.preview_scroll = 0;
        matched.is_terminal_exact_match
    }
}

impl DashApp {
    fn open_overlay_help(&mut self, help_context: HelpContext) {
        self.help_return_overlay = self.overlay;
        self.help_context = help_context;
        self.overlay = ActiveOverlay::Help;
    }

    fn reset_planet_overlay_prompt(&mut self) {
        self.planet_overlay.prompt_mode = PlanetOverlayPromptMode::None;
        self.planet_overlay.prompt_input.clear();
        self.planet_overlay.prompt_default.clear();
        self.planet_overlay.pending_range_anchor = None;
    }

    fn apply_planet_overlay_sort(&mut self, sort: PlanetOverlaySort) {
        let selected_record = planet_list::table_rows(self)
            .get(self.planet_overlay.selected)
            .map(|row| row.planet_record_index_1_based);
        self.planet_overlay.sort = sort;
        self.reset_planet_overlay_prompt();
        let rows = planet_list::table_rows(self);
        self.planet_overlay.selected = selected_record
            .and_then(|record| {
                rows.iter()
                    .position(|row| row.planet_record_index_1_based == record)
            })
            .unwrap_or(0);
        sync_scroll_to_cursor(&mut self.planet_overlay.scroll, self.planet_overlay.selected, 1_000);
    }

    fn apply_planet_overlay_filter(&mut self, filter: PlanetOverlayFilter) {
        let selected_record = planet_list::table_rows(self)
            .get(self.planet_overlay.selected)
            .map(|row| row.planet_record_index_1_based);
        self.planet_overlay.filter = filter;
        self.reset_planet_overlay_prompt();
        let rows = planet_list::table_rows(self);
        if rows.is_empty() {
            self.planet_overlay.filter = PlanetOverlayFilter::All;
        }
        let rows = planet_list::table_rows(self);
        self.planet_overlay.selected = selected_record
            .and_then(|record| {
                rows.iter()
                    .position(|row| row.planet_record_index_1_based == record)
            })
            .unwrap_or(0);
        sync_scroll_to_cursor(&mut self.planet_overlay.scroll, self.planet_overlay.selected, 1_000);
    }

    fn apply_fleet_overlay_sort(&mut self, sort: FleetOverlaySort) {
        let selected_key = fleet_list::table_rows(self)
            .get(self.fleet_overlay.selected)
            .map(|row| row.key);
        self.fleet_overlay.sort = sort;
        self.fleet_overlay.prompt_mode = FleetOverlayPromptMode::None;
        let rows = fleet_list::table_rows(self);
        self.fleet_overlay.selected = selected_key
            .and_then(|key| rows.iter().position(|row| row.key == key))
            .unwrap_or(0);
        sync_scroll_to_cursor(&mut self.fleet_overlay.scroll, self.fleet_overlay.selected, 1_000);
    }

    fn apply_fleet_overlay_filter(&mut self, filter: FleetOverlayFilter) {
        let selected_key = fleet_list::table_rows(self)
            .get(self.fleet_overlay.selected)
            .map(|row| row.key);
        self.fleet_overlay.filter = filter;
        self.fleet_overlay.prompt_mode = FleetOverlayPromptMode::None;
        let rows = fleet_list::table_rows(self);
        if rows.is_empty() {
            self.fleet_overlay.filter = FleetOverlayFilter::All;
        }
        let rows = fleet_list::table_rows(self);
        self.fleet_overlay.selected = selected_key
            .and_then(|key| rows.iter().position(|row| row.key == key))
            .unwrap_or(0);
        sync_scroll_to_cursor(&mut self.fleet_overlay.scroll, self.fleet_overlay.selected, 1_000);
    }

    fn reset_intel_overlay_prompt(&mut self) {
        self.intel_overlay.prompt_mode = IntelOverlayPromptMode::None;
        self.intel_overlay.prompt_input.clear();
        self.intel_overlay.prompt_default.clear();
        self.intel_overlay.pending_range_anchor = None;
    }

    fn apply_intel_overlay_sort(&mut self, sort: IntelOverlaySort) {
        let selected_record = intel_database::table_rows(self)
            .get(self.intel_overlay.selected)
            .map(|row| row.planet_record_index_1_based);
        self.intel_overlay.sort = sort;
        self.reset_intel_overlay_prompt();
        let rows = intel_database::table_rows(self);
        self.intel_overlay.selected = selected_record
            .and_then(|record| {
                rows.iter()
                    .position(|row| row.planet_record_index_1_based == record)
            })
            .unwrap_or(0);
        sync_scroll_to_cursor(&mut self.intel_overlay.scroll, self.intel_overlay.selected, 10_000);
    }

    fn apply_intel_overlay_filter(&mut self, filter: IntelOverlayFilter) {
        let selected_record = intel_database::table_rows(self)
            .get(self.intel_overlay.selected)
            .map(|row| row.planet_record_index_1_based);
        self.intel_overlay.filter = filter;
        self.reset_intel_overlay_prompt();
        let rows = intel_database::table_rows(self);
        if rows.is_empty() {
            self.intel_overlay.filter = IntelOverlayFilter::All;
        }
        let rows = intel_database::table_rows(self);
        self.intel_overlay.selected = selected_record
            .and_then(|record| {
                rows.iter()
                    .position(|row| row.planet_record_index_1_based == record)
            })
            .unwrap_or(0);
        sync_scroll_to_cursor(&mut self.intel_overlay.scroll, self.intel_overlay.selected, 10_000);
    }
}

fn handle_list_overlay_key(
    key: KeyEvent,
    selected: &mut usize,
    scroll: &mut usize,
    total_rows: usize,
) {
    let last = total_rows.saturating_sub(1);
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            *selected = wrap_prev_index(*selected, total_rows);
            if *selected < *scroll {
                *scroll = *selected;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            *selected = wrap_next_index(*selected, total_rows);
        }
        KeyCode::PageUp => {
            *selected = (*selected).saturating_sub(10);
            *selected = (*selected).min(last);
            *scroll = (*scroll).saturating_sub(10);
        }
        KeyCode::PageDown => {
            *selected = (*selected).saturating_add(10).min(last);
        }
        KeyCode::Home => {
            *selected = 0;
            *scroll = 0;
        }
        KeyCode::End => {
            *selected = last;
            *scroll = last.saturating_sub(10);
        }
        _ => {}
    }
}

fn map_coord_rows(app: &DashApp) -> Vec<Vec<String>> {
    let map_size = nc_data::map_size_for_player_count(app.game_data.conquest.player_count());
    let mut rows = Vec::with_capacity(usize::from(map_size) * usize::from(map_size));
    for x in 1..=map_size {
        for y in 1..=map_size {
            rows.push(vec![format!("({x:02},{y:02})")]);
        }
    }
    rows
}

fn parse_table_coord(cell: &str) -> Option<[u8; 2]> {
    let digits = cell
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let [x, y] = digits.as_slice() else {
        return None;
    };
    Some([x.parse().ok()?, y.parse().ok()?])
}

fn wrap_prev_index(selected: usize, total_rows: usize) -> usize {
    if total_rows == 0 {
        0
    } else if selected == 0 {
        total_rows - 1
    } else {
        selected - 1
    }
}

fn wrap_next_index(selected: usize, total_rows: usize) -> usize {
    if total_rows == 0 {
        0
    } else if selected + 1 >= total_rows {
        0
    } else {
        selected + 1
    }
}

fn sync_scroll_to_cursor(scroll_offset: &mut usize, cursor: usize, visible: usize) {
    if cursor < *scroll_offset {
        *scroll_offset = cursor;
    } else if cursor >= scroll_offset.saturating_add(visible) {
        *scroll_offset = cursor + 1 - visible;
    }
}

fn delete_selected_inbox_item(app: &mut DashApp) {
    let viewer = app.player_record_index_1_based as u8;
    let state = &app.inbox_overlay;
    let current_year = app.game_data.conquest.game_year();
    let items = project_inbox_items(
        &app.game_data,
        viewer,
        &app.report_block_rows,
        &app.queued_mail,
    )
    .into_iter()
    .filter(|item| item.matches_filter(state.filter, state.current_year_only, current_year))
    .collect::<Vec<_>>();

    let selected = state.selected.min(items.len().saturating_sub(1));
    let Some(item) = items.get(selected) else {
        return;
    };

    match item.source {
        DashInboxItemSource::ReportBlock(idx) => {
            if let Some(block) = app.report_block_rows.get_mut(idx) {
                block.recipient_deleted = true;
            }
        }
        DashInboxItemSource::QueuedMail(idx) => {
            if let Some(mail) = app.queued_mail.get_mut(idx) {
                mail.mark_deleted_by_recipient();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{map_coord_rows, parse_table_coord, wrap_next_index, wrap_prev_index};
    use crate::app::state::{
        DashApp, FleetOverlayFilter, IntelOverlayFilter, MapViewMode, PlanetOverlayFilter,
    };
    use crate::overlays::{fleet_list, intel_database, planet_list};
    use crossterm::event::{KeyCode, KeyEvent};
    use nc_data::GameStateBuilder;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    #[test]
    fn wrap_prev_goes_from_first_to_last() {
        assert_eq!(wrap_prev_index(0, 4), 3);
    }

    #[test]
    fn wrap_next_goes_from_last_to_first() {
        assert_eq!(wrap_next_index(3, 4), 0);
    }

    #[test]
    fn parse_table_coord_reads_table_style_coords() {
        assert_eq!(parse_table_coord("(02,03)"), Some([2, 3]));
        assert_eq!(parse_table_coord("[02,03]"), Some([2, 3]));
        assert_eq!(parse_table_coord("bogus"), None);
    }

    #[test]
    fn map_coord_rows_cover_entire_map_in_numeric_coordinate_order() {
        let app = dash_app();
        let rows = map_coord_rows(&app);
        assert_eq!(
            rows.first().and_then(|row| row.first()),
            Some(&"(01,01)".to_string())
        );
        assert_eq!(
            rows.get(1).and_then(|row| row.first()),
            Some(&"(01,02)".to_string())
        );
        assert_eq!(
            rows.get(18).and_then(|row| row.first()),
            Some(&"(02,01)".to_string())
        );
    }

    #[test]
    fn typed_map_coords_move_crosshair_and_clear_on_exact_match() {
        let mut app = dash_app();
        app.handle_key(KeyEvent::new(
            KeyCode::Char('0'),
            crossterm::event::KeyModifiers::NONE,
        ));
        app.handle_key(KeyEvent::new(
            KeyCode::Char('2'),
            crossterm::event::KeyModifiers::NONE,
        ));
        app.handle_key(KeyEvent::new(
            KeyCode::Char(','),
            crossterm::event::KeyModifiers::NONE,
        ));
        app.handle_key(KeyEvent::new(
            KeyCode::Char('0'),
            crossterm::event::KeyModifiers::NONE,
        ));
        app.handle_key(KeyEvent::new(
            KeyCode::Char('3'),
            crossterm::event::KeyModifiers::NONE,
        ));

        assert_eq!([app.crosshair_x, app.crosshair_y], [2, 3]);
        assert!(app.map_coord_input.is_empty());
    }

    #[test]
    fn typed_map_coords_keep_partial_input_visible() {
        let mut app = dash_app();
        app.handle_key(KeyEvent::new(
            KeyCode::Char('0'),
            crossterm::event::KeyModifiers::NONE,
        ));
        app.handle_key(KeyEvent::new(
            KeyCode::Char('2'),
            crossterm::event::KeyModifiers::NONE,
        ));

        assert_eq!([app.crosshair_x, app.crosshair_y], [2, 1]);
        assert_eq!(app.map_coord_input, "02");
    }

    #[test]
    fn typed_map_coords_do_not_enter_readable_void_rows() {
        let mut app = dash_app();
        app.handle_key(KeyEvent::new(
            KeyCode::Char('0'),
            crossterm::event::KeyModifiers::NONE,
        ));
        app.handle_key(KeyEvent::new(
            KeyCode::Char('1'),
            crossterm::event::KeyModifiers::NONE,
        ));
        app.handle_key(KeyEvent::new(
            KeyCode::Char(','),
            crossterm::event::KeyModifiers::NONE,
        ));
        app.handle_key(KeyEvent::new(
            KeyCode::Char('2'),
            crossterm::event::KeyModifiers::NONE,
        ));
        app.handle_key(KeyEvent::new(
            KeyCode::Char('3'),
            crossterm::event::KeyModifiers::NONE,
        ));

        assert!(app.crosshair_x <= 18);
        assert!(app.crosshair_y <= 18);
    }

    #[test]
    fn dashboard_actions_clear_partial_map_coord_input() {
        let mut app = dash_app();
        app.handle_key(KeyEvent::new(
            KeyCode::Char('0'),
            crossterm::event::KeyModifiers::NONE,
        ));
        app.handle_key(KeyEvent::new(
            KeyCode::Char('2'),
            crossterm::event::KeyModifiers::NONE,
        ));
        app.handle_key(KeyEvent::new(
            KeyCode::Char(']'),
            crossterm::event::KeyModifiers::NONE,
        ));

        assert!(app.map_coord_input.is_empty());
    }

    #[test]
    fn zoom_keys_adjust_map_zoom_level() {
        let mut app = dash_app();

        app.handle_key(KeyEvent::new(
            KeyCode::Char('='),
            crossterm::event::KeyModifiers::NONE,
        ));
        app.handle_key(KeyEvent::new(
            KeyCode::Char('='),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(app.map_zoom_level, 2);

        app.handle_key(KeyEvent::new(
            KeyCode::Char('-'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(app.map_zoom_level, 1);

        app.handle_key(KeyEvent::new(
            KeyCode::Char('z'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(app.map_zoom_level, 0);

        assert_eq!(app.map_view_mode, MapViewMode::Readable);
        app.handle_key(KeyEvent::new(
            KeyCode::Char('v'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(app.map_view_mode, MapViewMode::Fill);
        app.handle_key(KeyEvent::new(
            KeyCode::Char('v'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(app.map_view_mode, MapViewMode::Readable);
    }

    #[test]
    fn empty_planet_filter_reverts_to_all_rows() {
        let mut app = dash_app();
        let all_rows = planet_list::table_rows(&app).len();

        app.apply_planet_overlay_filter(PlanetOverlayFilter::Range {
            anchor: [18, 18],
            radius: 0,
        });

        assert_eq!(app.planet_overlay.filter, PlanetOverlayFilter::All);
        assert_eq!(planet_list::table_rows(&app).len(), all_rows);
    }

    #[test]
    fn empty_fleet_filter_reverts_to_all_rows() {
        let mut app = dash_app();
        app.game_data.fleets.records.clear();
        app.game_data.bases.records.clear();
        let all_rows = fleet_list::table_rows(&app).len();

        app.apply_fleet_overlay_filter(FleetOverlayFilter::Combat);

        assert_eq!(app.fleet_overlay.filter, FleetOverlayFilter::All);
        assert_eq!(fleet_list::table_rows(&app).len(), all_rows);
    }

    #[test]
    fn empty_intel_filter_reverts_to_all_rows() {
        let mut app = dash_app();
        let all_rows = intel_database::table_rows(&app).len();

        app.apply_intel_overlay_filter(IntelOverlayFilter::Empire(99));

        assert_eq!(app.intel_overlay.filter, IntelOverlayFilter::All);
        assert_eq!(intel_database::table_rows(&app).len(), all_rows);
    }

    fn dash_app() -> DashApp {
        DashApp::new(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            nc_ui::ScreenGeometry::new(160, 40),
            nc_ui::ScreenGeometry::new(108, 26),
            1,
        )
    }
}
