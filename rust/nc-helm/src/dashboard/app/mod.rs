//! App struct, main loop, and action dispatch.

mod fleet_orders;
mod hosted_turns;
pub mod input;
mod owned_planet;
pub(crate) mod panel_cache;
mod persistence;
pub(crate) mod planet_build;
pub mod render;
mod settings;
pub mod state;

use crate::dashboard::buffer::PlayfieldBuffer;
use crate::dashboard::geometry::ScreenGeometry;
use crate::dashboard::input::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crate::dashboard::table_filter::{
    format_column_code_error, is_filter_column_char, parse_column_code, parse_filter_clause,
};
use crate::dashboard::table_selection;
use crate::dashboard::table_selection::{sync_scroll_to_cursor, wrap_next_index, wrap_prev_index};
use input::{Action, key_to_action};
use state::{
    ActiveMouseGesture, ActiveOverlay, ActivePopup, DashApp, DashboardExitRequest, FleetOrderScope,
    FleetOverlayFilter, FleetOverlayPromptMode, FleetOverlaySort, HelpContext, IntelOverlayFilter,
    IntelOverlayPromptMode, IntelOverlaySort, MapViewMode, OwnedPlanetPopupMode,
    PlanetOverlayFilter, PlanetOverlayPromptMode, PlanetOverlaySort,
    default_fleet_overlay_sort_direction, default_intel_overlay_sort_direction,
    default_planet_overlay_sort_direction,
};

use crate::dashboard::inbox::{DashInboxItemSource, matches_filter, project_inbox_items};
use crate::dashboard::layout::dashboard;
use crate::dashboard::modal::{Rect, modal_close_button_contains};
use crate::dashboard::native::NativeApp;
use crate::dashboard::overlays::{fleet_list, inbox, intel_database, planet_list};
use crate::dashboard::panels::starmap;
use crate::dashboard::planet_view;
use crate::dashboard::ui::UiScene;
use std::time::{Duration, Instant};

const COMMAND_LINE_TOAST_STEP: Duration = Duration::from_secs(1);

fn quit_confirm_message() -> &'static str {
    "Quit to Lobby? Y/[N]"
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MouseRenderState {
    overlay: ActiveOverlay,
    popup: ActivePopup,
    overlay_position: Option<crate::dashboard::overlays::frame::RelativePopupOrigin>,
    popup_position: Option<crate::dashboard::overlays::frame::RelativePopupOrigin>,
    mouse_gesture: ActiveMouseGesture,
    focus: state::PanelFocus,
    crosshair_x: u8,
    crosshair_y: u8,
}

const fn planet_sort_code(sort: PlanetOverlaySort) -> &'static str {
    match sort {
        PlanetOverlaySort::Location => "coo",
        PlanetOverlaySort::PlanetName => "pla",
        PlanetOverlaySort::MaxProduction => "max",
        PlanetOverlaySort::CurrentProduction => "cur",
        PlanetOverlaySort::Treasury => "trs",
        PlanetOverlaySort::Budget => "bdg",
        PlanetOverlaySort::Revenue => "rev",
        PlanetOverlaySort::Growth => "gro",
        PlanetOverlaySort::BuildQueue => "bui",
        PlanetOverlaySort::Stardock => "sta",
        PlanetOverlaySort::Starbase => "sbs",
        PlanetOverlaySort::Armies => "ars",
        PlanetOverlaySort::Batteries => "gbs",
    }
}

fn planet_sort_from_code(code: &str) -> Option<PlanetOverlaySort> {
    match code {
        "coo" => Some(PlanetOverlaySort::Location),
        "pla" => Some(PlanetOverlaySort::PlanetName),
        "max" => Some(PlanetOverlaySort::MaxProduction),
        "cur" => Some(PlanetOverlaySort::CurrentProduction),
        "trs" => Some(PlanetOverlaySort::Treasury),
        "bdg" => Some(PlanetOverlaySort::Budget),
        "rev" => Some(PlanetOverlaySort::Revenue),
        "gro" => Some(PlanetOverlaySort::Growth),
        "bui" => Some(PlanetOverlaySort::BuildQueue),
        "sta" => Some(PlanetOverlaySort::Stardock),
        "sbs" => Some(PlanetOverlaySort::Starbase),
        "ars" => Some(PlanetOverlaySort::Armies),
        "gbs" => Some(PlanetOverlaySort::Batteries),
        _ => None,
    }
}

const fn fleet_sort_code(sort: FleetOverlaySort) -> &'static str {
    match sort {
        FleetOverlaySort::Id => "id",
        FleetOverlaySort::Selected => "sel",
        FleetOverlaySort::Location => "loc",
        FleetOverlaySort::Order => "ord",
        FleetOverlaySort::Target => "tar",
        FleetOverlaySort::Speed => "spd",
        FleetOverlaySort::Eta => "eta",
        FleetOverlaySort::Roe => "roe",
        FleetOverlaySort::Armies => "ars",
        FleetOverlaySort::Strength => "shi",
    }
}

fn fleet_sort_from_code(code: &str) -> Option<FleetOverlaySort> {
    match code {
        "id" => Some(FleetOverlaySort::Id),
        "sel" => Some(FleetOverlaySort::Selected),
        "loc" => Some(FleetOverlaySort::Location),
        "ord" => Some(FleetOverlaySort::Order),
        "tar" => Some(FleetOverlaySort::Target),
        "spd" => Some(FleetOverlaySort::Speed),
        "eta" => Some(FleetOverlaySort::Eta),
        "roe" => Some(FleetOverlaySort::Roe),
        "ars" => Some(FleetOverlaySort::Armies),
        "shi" => Some(FleetOverlaySort::Strength),
        _ => None,
    }
}

const fn intel_sort_code(sort: IntelOverlaySort) -> &'static str {
    match sort {
        IntelOverlaySort::Location => "coo",
        IntelOverlaySort::Range(_) => "rng",
        IntelOverlaySort::PlanetName => "pla",
        IntelOverlaySort::Owner => "own",
        IntelOverlaySort::MaxProduction => "max",
        IntelOverlaySort::YearSeen => "see",
        IntelOverlaySort::Armies => "ars",
        IntelOverlaySort::Batteries => "gbs",
        IntelOverlaySort::Starbases => "sbs",
        IntelOverlaySort::CurrentProduction => "cur",
        IntelOverlaySort::Treasury => "trs",
        IntelOverlaySort::ScoutYear => "sco",
    }
}

fn intel_sort_from_code(code: &str) -> Option<IntelOverlaySort> {
    match code {
        "coo" => Some(IntelOverlaySort::Location),
        "pla" => Some(IntelOverlaySort::PlanetName),
        "own" => Some(IntelOverlaySort::Owner),
        "max" => Some(IntelOverlaySort::MaxProduction),
        "see" => Some(IntelOverlaySort::YearSeen),
        "ars" => Some(IntelOverlaySort::Armies),
        "gbs" => Some(IntelOverlaySort::Batteries),
        "sbs" => Some(IntelOverlaySort::Starbases),
        "cur" => Some(IntelOverlaySort::CurrentProduction),
        "trs" => Some(IntelOverlaySort::Treasury),
        "sco" => Some(IntelOverlaySort::ScoutYear),
        _ => None,
    }
}

impl DashApp {
    pub(crate) fn is_at_root_surface(&self) -> bool {
        self.overlay == ActiveOverlay::None && self.popup == ActivePopup::None
    }

    pub(crate) fn dispatch_key_event(&mut self, key: crate::dashboard::input::KeyEvent) {
        if !self.is_terminal_too_small {
            self.dismiss_active_command_line_toast();
            self.handle_key(key);
            let _ = self.update_command_line_toast_state(Instant::now());
        } else if key.modifiers.contains(KeyModifiers::ALT)
            && matches!(key.code, KeyCode::Char('q' | 'Q'))
        {
            self.request_exit(DashboardExitRequest::QuitClient);
        }
    }

    fn mouse_render_state(&self) -> MouseRenderState {
        MouseRenderState {
            overlay: self.overlay,
            popup: self.popup,
            overlay_position: self.overlay_position,
            popup_position: self.popup_position,
            mouse_gesture: self.mouse_gesture,
            focus: self.focus,
            crosshair_x: self.crosshair_x,
            crosshair_y: self.crosshair_y,
        }
    }

    pub(crate) fn dispatch_mouse_event(&mut self, mouse: MouseEvent) -> bool {
        if !self.is_terminal_too_small {
            let before = self.mouse_render_state();
            self.handle_mouse(mouse);
            let changed = before != self.mouse_render_state();
            let toast_changed = self.update_command_line_toast_state(Instant::now());
            changed || toast_changed
        } else {
            false
        }
    }

    pub(crate) fn resize_canvas(&mut self, cols: u16, rows: u16) {
        self.geometry = ScreenGeometry::new(cols as usize, rows as usize);
        self.refresh_terminal_fit_state();
    }

    pub(crate) fn render_playfield(&self) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        render::render(self)
    }

    pub(crate) fn take_exit_request(&mut self) -> Option<DashboardExitRequest> {
        let request = self.exit_request.take();
        if matches!(request, Some(DashboardExitRequest::QuitClient)) {
            self.should_quit = false;
        }
        request
    }

    #[doc(hidden)]
    pub fn dispatch_key_event_for_repro(&mut self, key: crate::dashboard::input::KeyEvent) {
        Self::dispatch_key_event(self, key);
    }

    #[doc(hidden)]
    pub fn dispatch_mouse_event_for_repro(&mut self, mouse: MouseEvent) -> bool {
        Self::dispatch_mouse_event(self, mouse)
    }

    #[doc(hidden)]
    pub fn screen_point_for_sector_for_repro(&self, target: [u8; 2]) -> Option<(u16, u16)> {
        let map_frame = crate::dashboard::layout::dashboard::dashboard_layout(self)
            .widgets
            .center_map;
        for row in map_frame.grid.row..map_frame.grid.row + map_frame.grid.height {
            for col in map_frame.grid.col..map_frame.grid.col + map_frame.grid.width {
                if crate::dashboard::panels::starmap::screen_sector_at_point(
                    self, map_frame, col, row,
                ) == Some(target)
                {
                    return Some((col as u16, row as u16));
                }
            }
        }
        None
    }

    #[doc(hidden)]
    pub fn first_owned_planet_coords_for_repro(&self) -> Option<[u8; 2]> {
        self.game_data
            .planets
            .records
            .iter()
            .find(|planet| planet.owner_empire_slot_raw() == 1 && planet.coords_raw() != [0, 0])
            .map(|planet| planet.coords_raw())
    }

    pub(crate) fn active_command_line_toast(&self) -> Option<&str> {
        match self.overlay {
            ActiveOverlay::None => match self.popup {
                ActivePopup::OwnedPlanet { .. } => self.owned_planet_popup.status.as_deref(),
                ActivePopup::QuitConfirm | ActivePopup::PlanetDetail { .. } | ActivePopup::None => {
                    None
                }
            },
            ActiveOverlay::PlanetList => match self.planet_overlay.prompt_mode {
                PlanetOverlayPromptMode::None => self.planet_overlay.footer_notice.as_deref(),
                PlanetOverlayPromptMode::SortMenu
                | PlanetOverlayPromptMode::FilterMenu
                | PlanetOverlayPromptMode::FilterValueInput => {
                    self.planet_overlay.prompt_status.as_deref()
                }
                PlanetOverlayPromptMode::BuildSpecify => {
                    self.planet_overlay.build_unit_status.as_deref()
                }
                PlanetOverlayPromptMode::BuildQuantity => {
                    self.planet_overlay.build_quantity_status.as_deref()
                }
            },
            ActiveOverlay::FleetList => match self.fleet_overlay.prompt_mode {
                FleetOverlayPromptMode::None
                | FleetOverlayPromptMode::SortMenu
                | FleetOverlayPromptMode::FilterMenu
                | FleetOverlayPromptMode::FilterValueInput => {
                    self.fleet_overlay.filter_prompt_status.as_deref()
                }
                FleetOverlayPromptMode::ChangeField
                | FleetOverlayPromptMode::ChangeValue
                | FleetOverlayPromptMode::MergeHost
                | FleetOverlayPromptMode::MergeConfirm
                | FleetOverlayPromptMode::TransferHost
                | FleetOverlayPromptMode::TransferStage => self.fleet_overlay.aux_status.as_deref(),
                FleetOverlayPromptMode::MissionPicker => {
                    self.fleet_overlay.mission_picker_status.as_deref()
                }
                FleetOverlayPromptMode::OrderTarget
                | FleetOverlayPromptMode::OrderTargetX
                | FleetOverlayPromptMode::OrderTargetY
                | FleetOverlayPromptMode::OrderConfirm => {
                    self.fleet_overlay.order_status.as_deref()
                }
                FleetOverlayPromptMode::StarbaseMoveDecision
                | FleetOverlayPromptMode::StarbaseMoveDestination
                | FleetOverlayPromptMode::StarbaseHaltConfirm => {
                    self.fleet_overlay.starbase_move_status.as_deref()
                }
            },
            ActiveOverlay::IntelDatabase => self.intel_overlay.prompt_status.as_deref(),
            ActiveOverlay::Settings => self.settings_overlay.status_message.as_deref(),
            ActiveOverlay::Inbox | ActiveOverlay::Diplomacy | ActiveOverlay::Help => None,
        }
    }

    fn any_command_line_toast_present(&self) -> bool {
        self.planet_overlay.footer_notice.is_some()
            || self.planet_overlay.prompt_status.is_some()
            || self
                .planet_overlay
                .prompt_stack
                .iter()
                .any(|frame| frame.prompt_status.is_some())
            || self.planet_overlay.build_unit_status.is_some()
            || self.planet_overlay.build_quantity_status.is_some()
            || self.fleet_overlay.filter_prompt_status.is_some()
            || self.fleet_overlay.aux_status.is_some()
            || self.fleet_overlay.mission_picker_status.is_some()
            || self.fleet_overlay.order_status.is_some()
            || self.fleet_overlay.starbase_move_status.is_some()
            || self.intel_overlay.prompt_status.is_some()
            || self
                .intel_overlay
                .prompt_stack
                .iter()
                .any(|frame| frame.prompt_status.is_some())
            || self.settings_overlay.status_message.is_some()
            || self.owned_planet_popup.status.is_some()
    }

    fn clear_all_command_line_toasts(&mut self) {
        self.command_line_toast_message = None;
        self.command_line_toast_deadline = None;
        self.planet_overlay.footer_notice = None;
        self.planet_overlay.prompt_status = None;
        for frame in &mut self.planet_overlay.prompt_stack {
            frame.prompt_status = None;
        }
        self.planet_overlay.build_unit_status = None;
        self.planet_overlay.build_quantity_status = None;
        self.fleet_overlay.filter_prompt_status = None;
        self.fleet_overlay.aux_status = None;
        self.fleet_overlay.mission_picker_status = None;
        self.fleet_overlay.order_status = None;
        self.fleet_overlay.starbase_move_status = None;
        self.intel_overlay.prompt_status = None;
        for frame in &mut self.intel_overlay.prompt_stack {
            frame.prompt_status = None;
        }
        self.settings_overlay.status_message = None;
        self.owned_planet_popup.status = None;
    }

    fn dismiss_active_command_line_toast(&mut self) {
        if self.active_command_line_toast().is_some() {
            self.clear_all_command_line_toasts();
        }
    }

    fn update_command_line_toast_state(&mut self, now: Instant) -> bool {
        let active = self.active_command_line_toast().map(str::to_string);
        if active != self.command_line_toast_message {
            self.command_line_toast_message = active.clone();
            self.command_line_toast_deadline =
                active.as_ref().map(|_| now + COMMAND_LINE_TOAST_STEP);
            return true;
        }
        if active.is_some() || self.any_command_line_toast_present() {
            if let Some(deadline) = self.command_line_toast_deadline {
                if deadline <= now {
                    self.clear_all_command_line_toasts();
                    return true;
                }
            } else {
                self.command_line_toast_deadline = Some(now + COMMAND_LINE_TOAST_STEP);
                return true;
            }
        } else if self.command_line_toast_deadline.is_some()
            || self.command_line_toast_message.is_some()
        {
            self.command_line_toast_deadline = None;
            self.command_line_toast_message = None;
            return true;
        }
        false
    }

    fn request_exit(&mut self, request: DashboardExitRequest) {
        self.exit_request = Some(request);
        self.should_quit = matches!(request, DashboardExitRequest::QuitClient);
    }

    fn open_quit_confirm(&mut self) {
        if self.popup == ActivePopup::QuitConfirm {
            return;
        }
        self.quit_confirm_return_popup = self.popup;
        self.quit_confirm_return_popup_position = self.popup_position.take();
        self.popup_position = None;
        self.mouse_gesture = ActiveMouseGesture::None;
        self.popup = ActivePopup::QuitConfirm;
    }

    fn close_active_popup(&mut self) {
        if self.popup == ActivePopup::QuitConfirm
            && self.quit_confirm_return_popup != ActivePopup::None
        {
            self.popup = self.quit_confirm_return_popup;
            self.popup_position = self.quit_confirm_return_popup_position.take();
            self.quit_confirm_return_popup = ActivePopup::None;
            self.mouse_gesture = ActiveMouseGesture::None;
            return;
        }
        if matches!(self.popup, ActivePopup::OwnedPlanet { .. }) {
            self.owned_planet_popup.reset();
        }
        self.popup = ActivePopup::None;
        self.popup_position = None;
        self.quit_confirm_return_popup = ActivePopup::None;
        self.quit_confirm_return_popup_position = None;
        self.mouse_gesture = ActiveMouseGesture::None;
    }

    fn handle_key(&mut self, key: crate::dashboard::input::KeyEvent) {
        if key.modifiers.contains(KeyModifiers::ALT) && matches!(key.code, KeyCode::Char('q' | 'Q'))
        {
            self.open_quit_confirm();
            return;
        }
        if self.overlay != ActiveOverlay::None && self.handle_overlay_key(key) {
            self.normalize_table_overlay_filters();
            return;
        }
        if self.popup != ActivePopup::None && self.handle_popup_key(key) {
            return;
        }
        if self.is_at_root_surface() && key.code == KeyCode::Esc {
            self.open_quit_confirm();
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
        self.normalize_table_overlay_filters();
    }

    fn apply_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.request_exit(DashboardExitRequest::QuitClient),
            Action::FocusNext => self.focus = self.focus.next(),
            Action::FocusPrev => self.focus = self.focus.prev(),
            Action::ToggleAutopilot => self.autopilot_on = !self.autopilot_on,
            Action::OpenOverlay(overlay) => {
                if overlay == ActiveOverlay::Help {
                    self.help_context = HelpContext::Global;
                    self.help_return_overlay = ActiveOverlay::None;
                    self.help_return_overlay_position = None;
                }
                if overlay == ActiveOverlay::FleetList {
                    self.fleet_overlay.clear_transient_location_filter();
                }
                if overlay == ActiveOverlay::Settings {
                    self.clear_settings_status();
                }
                self.overlay_position = None;
                self.mouse_gesture = ActiveMouseGesture::None;
                self.overlay = overlay;
            }
            Action::CloseOverlay => self.close_active_overlay(),
            Action::ClosePopup => self.close_active_popup(),
            Action::MoveCrosshairUp => {
                // Up arrow → higher Y (row 18 at top of screen).
                let map_size =
                    nc_data::map_size_for_player_count(self.game_data.conquest.player_count());
                if self.crosshair_y < map_size {
                    self.crosshair_y += 1;
                    starmap::advance_starmap_viewport(self);
                }
            }
            Action::MoveCrosshairDown => {
                // Down arrow → lower Y (row 1 at bottom of screen).
                if self.crosshair_y > 1 {
                    self.crosshair_y -= 1;
                    starmap::advance_starmap_viewport(self);
                }
            }
            Action::MoveCrosshairLeft => {
                if self.crosshair_x > 1 {
                    self.crosshair_x -= 1;
                    starmap::advance_starmap_viewport(self);
                }
            }
            Action::MoveCrosshairRight => {
                let map_size =
                    nc_data::map_size_for_player_count(self.game_data.conquest.player_count());
                if self.crosshair_x < map_size {
                    self.crosshair_x += 1;
                    starmap::advance_starmap_viewport(self);
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
                self.refresh_terminal_fit_state();
            }
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
        match self.popup {
            ActivePopup::QuitConfirm => match key.code {
                KeyCode::Char('y' | 'Y') => self.request_exit(DashboardExitRequest::ReturnToLobby),
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('n' | 'N') => {
                    self.apply_action(Action::ClosePopup);
                }
                _ => {}
            },
            ActivePopup::OwnedPlanet { .. } => match self.owned_planet_popup.mode {
                OwnedPlanetPopupMode::Browse => match key.code {
                    KeyCode::Esc => self.close_owned_planet_popup(),
                    KeyCode::Char('?') => self.open_overlay_help(HelpContext::OwnedPlanetPopup),
                    KeyCode::Char('b') | KeyCode::Char('B') => {
                        self.open_owned_planet_build_specify()
                    }
                    KeyCode::Char('c') | KeyCode::Char('C') => {
                        self.open_owned_planet_commission_select()
                    }
                    KeyCode::Char('m') | KeyCode::Char('M') => {
                        self.open_owned_planet_mass_commission_confirm()
                    }
                    KeyCode::Char('l') | KeyCode::Char('L') => self
                        .open_owned_planet_transport_fleet_select(
                            nc_engine::ArmyTransportMode::Load,
                        ),
                    KeyCode::Char('u') | KeyCode::Char('U') => self
                        .open_owned_planet_transport_fleet_select(
                            nc_engine::ArmyTransportMode::Unload,
                        ),
                    KeyCode::Char('x') | KeyCode::Char('X') => {
                        self.open_owned_planet_scorch_confirm()
                    }
                    _ => {}
                },
                OwnedPlanetPopupMode::CommissionSelect => match key.code {
                    KeyCode::Esc => self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse),
                    KeyCode::Enter => {
                        if let Err(err) = self.submit_owned_planet_commission() {
                            self.owned_planet_popup.status = Some(err.to_string());
                        }
                    }
                    KeyCode::Backspace => self.backspace_owned_planet_input(),
                    KeyCode::Char(ch) if ch.is_ascii_digit() => {
                        self.append_owned_planet_input_char(ch)
                    }
                    _ => {}
                },
                OwnedPlanetPopupMode::CommissionResult
                | OwnedPlanetPopupMode::MassCommissionReport => match key.code {
                    KeyCode::Esc | KeyCode::Enter => {
                        self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse)
                    }
                    _ => {}
                },
                OwnedPlanetPopupMode::MassCommissionConfirm => match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        if let Err(err) = self.confirm_owned_planet_mass_commission() {
                            self.show_owned_planet_status(err.to_string());
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Enter | KeyCode::Esc => {
                        self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse)
                    }
                    _ => {}
                },
                OwnedPlanetPopupMode::TransportFleetSelect { mode } => match key.code {
                    KeyCode::Esc => self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse),
                    KeyCode::Enter => {
                        if let Err(err) = self.submit_owned_planet_transport_fleet(mode) {
                            self.owned_planet_popup.status = Some(err.to_string());
                        }
                    }
                    KeyCode::Backspace => self.backspace_owned_planet_input(),
                    KeyCode::Char(ch) if ch.is_ascii_digit() => {
                        self.append_owned_planet_input_char(ch)
                    }
                    _ => {}
                },
                OwnedPlanetPopupMode::TransportQuantity { mode } => match key.code {
                    KeyCode::Esc => self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse),
                    KeyCode::Enter => {
                        if let Err(err) = self.submit_owned_planet_transport_quantity(mode) {
                            self.owned_planet_popup.status = Some(err.to_string());
                        }
                    }
                    KeyCode::Backspace => self.backspace_owned_planet_input(),
                    KeyCode::Char(ch) if ch.is_ascii_digit() => {
                        self.append_owned_planet_input_char(ch)
                    }
                    _ => {}
                },
                OwnedPlanetPopupMode::ScorchConfirm1
                | OwnedPlanetPopupMode::ScorchConfirm2
                | OwnedPlanetPopupMode::ScorchConfirm3 => match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        if let Err(err) = self.submit_owned_planet_scorch() {
                            self.owned_planet_popup.status = Some(err.to_string());
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Enter | KeyCode::Esc => {
                        self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse)
                    }
                    _ => {}
                },
            },
            _ => {
                let _ = key;
                self.apply_action(Action::ClosePopup);
            }
        }
        true
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        let widgets = dashboard::dashboard_layout(self).widgets;
        let map_frame = widgets.center_map;
        let modal_parent =
            crate::dashboard::overlays::frame::dashboard_overlay_parent_rect(widgets);
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if self.popup != ActivePopup::None {
                    self.handle_popup_mouse_down(mouse, map_frame);
                    return;
                }
                if self.overlay != ActiveOverlay::None {
                    self.handle_overlay_mouse_down(mouse, map_frame);
                    return;
                }
                self.mouse_gesture = ActiveMouseGesture::None;
                self.handle_map_left_click(mouse, map_frame);
            }
            MouseEventKind::Down(MouseButton::Right) => {
                if self.popup != ActivePopup::None || self.overlay != ActiveOverlay::None {
                    self.mouse_gesture = ActiveMouseGesture::None;
                    return;
                }
                self.mouse_gesture = ActiveMouseGesture::None;
                self.handle_map_right_click(mouse, map_frame);
            }
            MouseEventKind::Down(MouseButton::Middle) => {
                if self.popup != ActivePopup::None || self.overlay != ActiveOverlay::None {
                    self.mouse_gesture = ActiveMouseGesture::None;
                    return;
                }
                if let Some((cell_width, cell_height)) = self.last_starmap_cell_dims {
                    if map_frame.outer.contains_point(mouse.column as usize, mouse.row as usize) {
                        self.mouse_gesture = ActiveMouseGesture::DraggingStarmap {
                            anchor_col: mouse.column,
                            anchor_row: mouse.row,
                            start_x_min: self.starmap_viewport_x_min,
                            start_y_min: self.starmap_viewport_y_min,
                            cell_width,
                            cell_height,
                            drag_occurred: false,
                        };
                    }
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.popup != ActivePopup::None || self.overlay != ActiveOverlay::None {
                    self.handle_mouse_move(mouse, modal_parent);
                } else if self.client_settings.follow_mouse_on_map {
                    self.handle_map_hover(mouse, map_frame);
                }
            }
            MouseEventKind::Moved => {
                if self.popup != ActivePopup::None || self.overlay != ActiveOverlay::None {
                    self.mouse_gesture = ActiveMouseGesture::None;
                    return;
                }
                if self.client_settings.follow_mouse_on_map {
                    self.handle_map_hover(mouse, map_frame);
                }
            }
            MouseEventKind::Drag(MouseButton::Middle) => {
                if let ActiveMouseGesture::DraggingStarmap {
                    anchor_col,
                    anchor_row,
                    start_x_min,
                    start_y_min,
                    cell_width,
                    cell_height,
                    ref mut drag_occurred,
                } = self.mouse_gesture
                {
                    *drag_occurred = true;
                    let dx_cells =
                        (anchor_col as i32 - mouse.column as i32) / cell_width.max(1) as i32;
                    let dy_cells =
                        (anchor_row as i32 - mouse.row as i32) / cell_height.max(1) as i32;
                    self.pan_starmap_viewport(dx_cells, dy_cells);
                }
            }
            MouseEventKind::Up(MouseButton::Middle) => {
                if let ActiveMouseGesture::DraggingStarmap { drag_occurred, .. } =
                    self.mouse_gesture
                {
                    if !drag_occurred {
                        // Click-without-drag: re-centre viewport on crosshair.
                        self.starmap_viewport_x_min = 0;
                        self.starmap_viewport_y_min = 0;
                        starmap::advance_starmap_viewport(self);
                    }
                }
                self.mouse_gesture = ActiveMouseGesture::None;
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.mouse_gesture = ActiveMouseGesture::None;
            }
            MouseEventKind::Scroll { lines } => {
                if self.popup != ActivePopup::None {
                    return;
                }
                if self.overlay != ActiveOverlay::None {
                    self.handle_overlay_scroll(mouse, lines, map_frame);
                    return;
                }
                if map_frame.outer.contains_point(mouse.column as usize, mouse.row as usize) {
                    self.pan_starmap_viewport(0, -lines);
                }
            }
            _ => {}
        }
    }

    fn handle_overlay_scroll(
        &mut self,
        mouse: MouseEvent,
        lines: i32,
        map_frame: crate::dashboard::layout::MapWidgetFrame,
    ) {
        match self.overlay {
            ActiveOverlay::Diplomacy => {
                let total = self.game_data.player.records.len();
                self.diplomacy_scroll = scroll_clamp(
                    self.diplomacy_scroll as i32 - lines,
                    total.saturating_sub(1) as i32,
                );
            }
            ActiveOverlay::Inbox => {
                match inbox::hit_test_inbox_pane(self, map_frame, mouse.column as usize, mouse.row as usize) {
                    Some(inbox::InboxPane::List) => {
                        let total = inbox::inbox_items(self).len();
                        self.inbox_overlay.selected = scroll_selected(
                            self.inbox_overlay.selected,
                            lines,
                            total,
                        );
                        if lines > 0 {
                            self.inbox_overlay.scroll =
                                self.inbox_overlay.scroll.min(self.inbox_overlay.selected);
                        }
                    }
                    Some(inbox::InboxPane::Preview) => {
                        let selected = self.inbox_overlay.selected;
                        let items = inbox::inbox_items(self);
                        let max_preview = items
                            .get(selected)
                            .map(|item| item.body_lines.len().saturating_sub(1))
                            .unwrap_or(0);
                        self.inbox_overlay.preview_scroll = scroll_clamp(
                            self.inbox_overlay.preview_scroll as i32 - lines,
                            max_preview as i32,
                        );
                    }
                    None => {}
                }
            }
            ActiveOverlay::PlanetList => {
                if self.planet_overlay.prompt_mode != PlanetOverlayPromptMode::None {
                    return;
                }
                let total = planet_list::selection_rows(self).len();
                self.planet_overlay.selected = scroll_selected(self.planet_overlay.selected, lines, total);
                if lines > 0 {
                    self.planet_overlay.scroll = self.planet_overlay.scroll.min(self.planet_overlay.selected);
                }
            }
            ActiveOverlay::FleetList => {
                if self.fleet_overlay.prompt_mode != FleetOverlayPromptMode::None {
                    return;
                }
                let total = fleet_list::selection_rows(self).len();
                self.fleet_overlay.selected = scroll_selected(self.fleet_overlay.selected, lines, total);
                if lines > 0 {
                    self.fleet_overlay.scroll = self.fleet_overlay.scroll.min(self.fleet_overlay.selected);
                }
            }
            ActiveOverlay::IntelDatabase => {
                if self.intel_overlay.prompt_mode != IntelOverlayPromptMode::None {
                    return;
                }
                let total = intel_database::selection_rows(self).len();
                self.intel_overlay.selected = scroll_selected(self.intel_overlay.selected, lines, total);
                if lines > 0 {
                    self.intel_overlay.scroll = self.intel_overlay.scroll.min(self.intel_overlay.selected);
                }
            }
            ActiveOverlay::Settings | ActiveOverlay::Help => {}
            ActiveOverlay::None => {}
        }
    }

    fn handle_overlay_mouse_down(
        &mut self,
        mouse: MouseEvent,
        map_frame: crate::dashboard::layout::MapWidgetFrame,
    ) {
        let Some(popup) = self.current_overlay_popup_rect(map_frame) else {
            self.mouse_gesture = ActiveMouseGesture::None;
            return;
        };
        let mouse_col = mouse.column as usize;
        let mouse_row = mouse.row as usize;
        if modal_close_button_contains(popup, mouse_col, mouse_row) {
            self.close_active_overlay();
            return;
        }
        if self.overlay.is_draggable() && modal_chrome_contains(popup, mouse_col, mouse_row) {
            self.mouse_gesture = ActiveMouseGesture::DraggingOverlay {
                grab_col_offset: mouse_col.saturating_sub(popup.x as usize),
                grab_row_offset: mouse_row.saturating_sub(popup.y as usize),
            };
        } else {
            self.mouse_gesture = ActiveMouseGesture::None;
        }
    }

    fn handle_popup_mouse_down(
        &mut self,
        mouse: MouseEvent,
        map_frame: crate::dashboard::layout::MapWidgetFrame,
    ) {
        let Some(popup) = self.current_popup_rect(map_frame) else {
            self.mouse_gesture = ActiveMouseGesture::None;
            return;
        };
        let mouse_col = mouse.column as usize;
        let mouse_row = mouse.row as usize;
        if modal_close_button_contains(popup, mouse_col, mouse_row) {
            self.apply_action(Action::ClosePopup);
            return;
        }
        if modal_chrome_contains(popup, mouse_col, mouse_row) {
            self.mouse_gesture = ActiveMouseGesture::DraggingPopup {
                grab_col_offset: mouse_col.saturating_sub(popup.x as usize),
                grab_row_offset: mouse_row.saturating_sub(popup.y as usize),
            };
        } else {
            self.mouse_gesture = ActiveMouseGesture::None;
        }
    }

    fn handle_map_left_click(
        &mut self,
        mouse: MouseEvent,
        map_frame: crate::dashboard::layout::MapWidgetFrame,
    ) {
        let Some(coords) = starmap::screen_sector_at_point(
            self,
            map_frame,
            mouse.column as usize,
            mouse.row as usize,
        ) else {
            return;
        };
        self.set_crosshair_coords(coords);
        if !self.player_has_fleets_at(coords) {
            return;
        }
        self.open_fleet_overlay_for_location(coords);
    }

    fn handle_map_right_click(
        &mut self,
        mouse: MouseEvent,
        map_frame: crate::dashboard::layout::MapWidgetFrame,
    ) {
        let Some(coords) = starmap::screen_sector_at_point(
            self,
            map_frame,
            mouse.column as usize,
            mouse.row as usize,
        ) else {
            return;
        };
        self.set_crosshair_coords(coords);
        let Some(detail) = planet_view::selected_planet_detail(self) else {
            return;
        };
        let owner = self
            .game_data
            .planets
            .records
            .get(detail.planet_record_index_1_based.saturating_sub(1))
            .map(|planet| planet.owner_empire_slot_raw())
            .unwrap_or(0);
        if owner == self.player_record_index_1_based as u8 {
            self.open_owned_planet_popup(detail.planet_record_index_1_based);
        } else {
            self.open_planet_detail_popup_at_cursor();
        }
    }

    fn handle_map_hover(
        &mut self,
        mouse: MouseEvent,
        map_frame: crate::dashboard::layout::MapWidgetFrame,
    ) {
        self.mouse_gesture = ActiveMouseGesture::None;
        let mouse_col = mouse.column as usize;
        let mouse_row = mouse.row as usize;
        if let Some(coords) = starmap::screen_sector_at_point(self, map_frame, mouse_col, mouse_row)
        {
            self.set_crosshair_coords(coords);
        } else if !map_frame.outer.contains_point(mouse_col, mouse_row) {
            self.reset_crosshair_to_homeworld();
        }
    }

    fn set_crosshair_coords(&mut self, [x, y]: [u8; 2]) {
        if self.crosshair_x == x
            && self.crosshair_y == y
            && self.focus == state::PanelFocus::Map
            && self.map_coord_input.is_empty()
        {
            return;
        }
        self.crosshair_x = x;
        self.crosshair_y = y;
        self.focus = state::PanelFocus::Map;
        self.map_coord_input.clear();
        starmap::advance_starmap_viewport(self);
    }

    fn reset_crosshair_to_homeworld(&mut self) {
        let coords =
            state::initial_crosshair_coords(&self.game_data, self.player_record_index_1_based);
        if self.crosshair_x == coords[0]
            && self.crosshair_y == coords[1]
            && self.focus == state::PanelFocus::Map
            && self.map_coord_input.is_empty()
        {
            return;
        }
        self.set_crosshair_coords(coords);
    }

    fn pan_starmap_viewport(&mut self, dx_cells: i32, dy_cells: i32) {
        let map_size =
            nc_data::map_size_for_player_count(self.game_data.conquest.player_count());
        let frame = crate::dashboard::layout::dashboard_layout(self).widgets.center_map;
        let lattice_width = frame
            .grid
            .width
            .saturating_sub(frame.row_label_cols)
            .saturating_sub(1);
        let visible_x = starmap::max_visible_sector_count(
            lattice_width,
            map_size,
            frame.cell_width.max(1),
        );
        let visible_y = starmap::max_visible_sector_rows(frame.grid.height, map_size);
        let max_start_x = map_size.saturating_sub(visible_x).saturating_add(1).max(1);
        let max_start_y = map_size.saturating_sub(visible_y).saturating_add(1).max(1);
        self.starmap_viewport_x_min = (self.starmap_viewport_x_min as i32 + dx_cells)
            .clamp(1, max_start_x as i32) as u8;
        self.starmap_viewport_y_min = (self.starmap_viewport_y_min as i32 + dy_cells)
            .clamp(1, max_start_y as i32) as u8;
    }

    fn player_has_fleets_at(&self, coords: [u8; 2]) -> bool {
        let owner_slot = self.player_record_index_1_based as u8;
        self.game_data.fleets.records.iter().any(|fleet| {
            fleet.owner_empire_raw() == owner_slot
                && fleet.has_any_force()
                && fleet.current_location_coords_raw() == coords
        })
    }

    fn open_fleet_overlay_for_location(&mut self, coords: [u8; 2]) {
        self.fleet_overlay.location_filter = Some(coords);
        self.fleet_overlay.filter = FleetOverlayFilter::All;
        self.fleet_overlay.selected = 0;
        self.fleet_overlay.scroll = 0;
        self.fleet_overlay.jump_input.clear();
        self.fleet_overlay.clear_group_selection();
        self.fleet_overlay.clear_prompt();
        self.fleet_overlay.order_scope = FleetOrderScope::None;
        self.fleet_overlay.active_row_key = None;
        self.overlay_position = None;
        self.popup_position = None;
        self.mouse_gesture = ActiveMouseGesture::None;
        self.overlay = ActiveOverlay::FleetList;
        let rows = fleet_list::table_rows(self);
        self.fleet_overlay.selected = rows
            .iter()
            .position(|row| matches!(row.key, state::FleetOverlayRowKey::Fleet(_)))
            .unwrap_or(0);
        sync_scroll_to_cursor(
            &mut self.fleet_overlay.scroll,
            self.fleet_overlay.selected,
            1_000,
        );
    }

    fn handle_mouse_move(&mut self, mouse: MouseEvent, parent: Rect) {
        match self.mouse_gesture {
            ActiveMouseGesture::DraggingOverlay {
                grab_col_offset,
                grab_row_offset,
            } => {
                if self.overlay == ActiveOverlay::None || !self.overlay.is_draggable() {
                    self.mouse_gesture = ActiveMouseGesture::None;
                    return;
                }
                let target_x = mouse.column.saturating_sub(grab_col_offset as u16);
                let target_y = mouse.row.saturating_sub(grab_row_offset as u16);
                self.overlay_position =
                    Some(crate::dashboard::overlays::frame::RelativePopupOrigin {
                        col_offset: target_x.saturating_sub(parent.x) as usize,
                        row_offset: target_y.saturating_sub(parent.y) as usize,
                    });
            }
            ActiveMouseGesture::DraggingPopup {
                grab_col_offset,
                grab_row_offset,
            } => {
                if self.popup == ActivePopup::None {
                    self.mouse_gesture = ActiveMouseGesture::None;
                    return;
                }
                let target_x = mouse.column.saturating_sub(grab_col_offset as u16);
                let target_y = mouse.row.saturating_sub(grab_row_offset as u16);
                self.popup_position =
                    Some(crate::dashboard::overlays::frame::RelativePopupOrigin {
                        col_offset: target_x.saturating_sub(parent.x) as usize,
                        row_offset: target_y.saturating_sub(parent.y) as usize,
                    });
            }
            ActiveMouseGesture::DraggingStarmap {
                anchor_col,
                anchor_row,
                start_x_min,
                start_y_min,
                cell_width,
                cell_height,
                ref mut drag_occurred,
            } => {
                *drag_occurred = true;
                let dx_cells =
                    (anchor_col as i32 - mouse.column as i32) / cell_width.max(1) as i32;
                let dy_cells =
                    (anchor_row as i32 - mouse.row as i32) / cell_height.max(1) as i32;
                self.pan_starmap_viewport(dx_cells, dy_cells);
            }
            ActiveMouseGesture::None => {}
        }
    }

    /// Returns true if any active overlay or popup covers the given cell rect
    /// (inclusive, screen cell coordinates). Keeps ratatui confined to this module.
    pub(crate) fn popup_covers_cell_rect(
        &self,
        map_frame: crate::dashboard::layout::MapWidgetFrame,
        left: usize,
        right: usize,
        top: usize,
        bottom: usize,
    ) -> bool {
        let covers = |r: Rect| {
            let r_left = r.x as usize;
            let r_top = r.y as usize;
            let r_right = r_left + r.width.saturating_sub(1) as usize;
            let r_bottom = r_top + r.height.saturating_sub(1) as usize;
            left <= r_right && right >= r_left && top <= r_bottom && bottom >= r_top
        };
        self.current_overlay_popup_rect(map_frame)
            .is_some_and(covers)
            || self.current_popup_rect(map_frame).is_some_and(covers)
    }

    fn current_overlay_popup_rect(
        &self,
        map_frame: crate::dashboard::layout::MapWidgetFrame,
    ) -> Option<Rect> {
        match self.overlay {
            ActiveOverlay::None => None,
            ActiveOverlay::PlanetList => planet_list::popup_rect(self, map_frame),
            ActiveOverlay::FleetList => fleet_list::popup_rect(self, map_frame),
            ActiveOverlay::IntelDatabase => Some(intel_database::popup_rect(self, map_frame)),
            ActiveOverlay::Inbox => Some(inbox::popup_rect(self, map_frame)),
            ActiveOverlay::Diplomacy => Some(crate::dashboard::overlays::diplomacy::popup_rect(
                self, map_frame,
            )),
            ActiveOverlay::Settings => Some(crate::dashboard::overlays::settings::popup_rect(
                self, map_frame,
            )),
            ActiveOverlay::Help => Some(crate::dashboard::overlays::help::popup_rect(
                self, map_frame,
            )),
        }
    }

    fn current_popup_rect(
        &self,
        map_frame: crate::dashboard::layout::MapWidgetFrame,
    ) -> Option<Rect> {
        match self.popup {
            ActivePopup::None => None,
            ActivePopup::QuitConfirm => Some(
                crate::dashboard::overlays::frame::overlay_popup_rect_in_map(
                    map_frame,
                    crate::dashboard::QUIT_CONFIRM_TITLE,
                    crate::dashboard::quit_confirm_popup_width(quit_confirm_message()),
                    crate::dashboard::QUIT_CONFIRM_HEIGHT,
                    self.popup_position,
                ),
            ),
            ActivePopup::PlanetDetail {
                planet_record_index_1_based,
            } => Some(crate::dashboard::popups::planet_detail::popup_rect(
                self,
                map_frame,
                planet_record_index_1_based,
            )),
            ActivePopup::OwnedPlanet {
                planet_record_index_1_based,
            } => Some(crate::dashboard::popups::owned_planet::popup_rect(
                self,
                map_frame,
                planet_record_index_1_based,
            )),
        }
    }

    fn jump_crosshair_to_planet(&mut self, direction: starmap::PlanetJumpDirection) {
        if let Some(target) = starmap::jump_planet_target_for_app(
            self,
            [self.crosshair_x, self.crosshair_y],
            direction,
        ) {
            self.set_crosshair_coords(target);
        }
    }

    fn open_planet_detail_popup_at_cursor(&mut self) {
        let Some(detail) = planet_view::selected_planet_detail(self) else {
            return;
        };
        self.popup_position = None;
        self.mouse_gesture = ActiveMouseGesture::None;
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
        starmap::advance_starmap_viewport(self);
        matched.is_terminal_exact_match
    }

    fn refresh_terminal_fit_state(&mut self) {
        let required = dashboard::required_dashboard_frame(self);
        let layout = dashboard::dashboard_layout(self);
        self.is_terminal_too_small = self.geometry.width() < required.width()
            || self.geometry.height() < required.height()
            || !dashboard::dashboard_fits_canvas(self.geometry, &layout);
        if !self.is_terminal_too_small {
            self.frame = required;
        }
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
            ActiveOverlay::Settings => {
                if self.handle_overlay_close_or_help(key, HelpContext::Settings) {
                    return true;
                }
                match key.code {
                    KeyCode::Char('m') | KeyCode::Char('M') => {
                        self.toggle_follow_mouse_on_map_setting();
                    }
                    _ => {}
                }
                true
            }
            ActiveOverlay::Help => {
                let _ = key;
                self.close_active_overlay();
                true
            }
        }
    }

    fn handle_overlay_close_or_help(&mut self, key: KeyEvent, help_context: HelpContext) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.close_active_overlay();
                true
            }
            KeyCode::Char('?') if self.overlay == ActiveOverlay::Help => {
                self.close_active_overlay();
                true
            }
            KeyCode::Char('?') => {
                self.help_return_overlay = self.overlay;
                self.help_return_overlay_position = self.overlay_position.take();
                self.help_context = help_context;
                self.overlay = ActiveOverlay::Help;
                self.mouse_gesture = ActiveMouseGesture::None;
                true
            }
            _ => false,
        }
    }

    fn close_active_overlay(&mut self) {
        if self.overlay == ActiveOverlay::Help {
            self.overlay = self.help_return_overlay;
            self.help_return_overlay = ActiveOverlay::None;
            self.overlay_position = self.help_return_overlay_position.take();
            self.help_context = HelpContext::Global;
        } else {
            if self.overlay == ActiveOverlay::FleetList {
                self.fleet_overlay.clear_group_selection();
                self.fleet_overlay.clear_transient_location_filter();
            }
            if self.overlay == ActiveOverlay::Settings {
                self.clear_settings_status();
            }
            self.overlay = ActiveOverlay::None;
            self.overlay_position = None;
        }
        self.mouse_gesture = ActiveMouseGesture::None;
    }

    fn handle_planet_overlay_key(&mut self, key: KeyEvent) {
        let prompt_mode = self.planet_overlay.prompt_mode;
        match prompt_mode {
            PlanetOverlayPromptMode::BuildSpecify => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PlanetBuildSpecify),
                KeyCode::Esc => self.close_planet_build_overlay(),
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    if let Err(err) = self.queue_selected_planet_build_unit() {
                        self.planet_overlay.build_unit_status = Some(err.to_string());
                    }
                }
                KeyCode::Char('-') => {
                    if let Err(err) = self.remove_selected_planet_build_unit() {
                        self.planet_overlay.build_unit_status = Some(err.to_string());
                    }
                }
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    self.select_previous_planet_build_kind()
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    self.select_next_planet_build_kind()
                }
                KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => {
                    self.select_left_planet_build_kind()
                }
                KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
                    self.select_right_planet_build_kind()
                }
                KeyCode::Enter => {
                    if let Err(err) = self.submit_planet_build_browse_input() {
                        self.planet_overlay.build_unit_status = Some(err.to_string());
                    }
                }
                KeyCode::Backspace => self.backspace_planet_build_unit_input(),
                KeyCode::Char(ch) if ch.is_ascii_digit() => self.append_planet_build_unit_char(ch),
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    if let Err(err) = self.clear_selected_planet_build_kind_queue() {
                        self.planet_overlay.build_unit_status = Some(err.to_string());
                    }
                }
                _ => {}
            },
            PlanetOverlayPromptMode::BuildQuantity => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PlanetBuildQuantity),
                KeyCode::Esc => {
                    self.cancel_planet_build_quantity();
                }
                KeyCode::Enter => {
                    if let Err(err) = self.submit_planet_build_quantity() {
                        self.planet_overlay.build_quantity_status = Some(err.to_string());
                    }
                }
                KeyCode::Backspace => self.backspace_planet_build_quantity_input(),
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    self.append_planet_build_quantity_char(ch)
                }
                _ => {}
            },
            PlanetOverlayPromptMode::SortMenu => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PlanetListSort),
                KeyCode::Esc => {
                    self.planet_overlay.close_prompt();
                }
                KeyCode::Enter => {
                    let raw = if self.planet_overlay.prompt_input.trim().is_empty() {
                        self.planet_overlay.prompt_default.trim()
                    } else {
                        self.planet_overlay.prompt_input.trim()
                    };
                    match parse_column_code(planet_list::filter_columns(), raw) {
                        Ok(column) => match planet_sort_from_code(column.code) {
                            Some(sort) => self.apply_planet_overlay_sort(sort),
                            None => {
                                self.planet_overlay.prompt_input.clear();
                                self.planet_overlay.prompt_status =
                                    Some(" Enter a valid sort column.".to_string());
                            }
                        },
                        Err(err) => {
                            self.planet_overlay.prompt_status =
                                Some(format!(" {}", format_column_code_error(&err)));
                        }
                    }
                }
                KeyCode::Backspace => {
                    self.planet_overlay.prompt_input.pop();
                    self.planet_overlay.prompt_status = None;
                }
                KeyCode::Char(ch) if is_filter_column_char(ch) => {
                    self.planet_overlay.prompt_input.push(ch);
                    self.planet_overlay.prompt_status = None;
                }
                _ => {}
            },
            PlanetOverlayPromptMode::FilterMenu => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PlanetListFilter),
                KeyCode::Esc => {
                    self.planet_overlay.close_prompt();
                }
                KeyCode::Enter => {
                    let raw = if self.planet_overlay.prompt_input.trim().is_empty() {
                        self.planet_overlay.prompt_default.trim()
                    } else {
                        self.planet_overlay.prompt_input.trim()
                    };
                    if raw.eq_ignore_ascii_case("a") || raw.eq_ignore_ascii_case("all") {
                        self.apply_planet_overlay_filter_clause(None);
                    } else {
                        match parse_column_code(planet_list::filter_columns(), raw) {
                            Ok(column) => {
                                self.planet_overlay.pending_filter_column = Some(column);
                                self.planet_overlay.prompt_mode =
                                    PlanetOverlayPromptMode::FilterValueInput;
                                self.planet_overlay.prompt_input.clear();
                                self.planet_overlay.prompt_default =
                                    planet_list::filter_default_value(self, column);
                                self.planet_overlay.prompt_status = None;
                            }
                            Err(err) => {
                                self.planet_overlay.prompt_status =
                                    Some(format!(" {}", format_column_code_error(&err)));
                            }
                        }
                    }
                }
                KeyCode::Backspace => {
                    self.planet_overlay.prompt_input.pop();
                    self.planet_overlay.prompt_status = None;
                }
                KeyCode::Char(ch) if ch.is_ascii_alphabetic() => {
                    self.planet_overlay.prompt_input.push(ch);
                    self.planet_overlay.prompt_status = None;
                }
                _ => {}
            },
            PlanetOverlayPromptMode::FilterValueInput => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PromptInput),
                KeyCode::Esc => {
                    self.planet_overlay.clear_prompt();
                }
                KeyCode::Enter => {
                    let Some(column) = self.planet_overlay.pending_filter_column else {
                        return;
                    };
                    let raw = if self.planet_overlay.prompt_input.trim().is_empty() {
                        self.planet_overlay.prompt_default.trim()
                    } else {
                        self.planet_overlay.prompt_input.trim()
                    };
                    match parse_filter_clause(column, raw) {
                        Ok(clause) => self.apply_planet_overlay_filter_clause(Some(clause)),
                        Err(err) => self.planet_overlay.prompt_status = Some(format!(" {err}")),
                    }
                }
                KeyCode::Backspace => {
                    self.planet_overlay.prompt_input.pop();
                    self.planet_overlay.prompt_status = None;
                }
                KeyCode::Char(ch)
                    if matches!(
                        ch,
                        ' ' | '-' | '#' | '*' | '/' | '?' | '=' | '!' | '>' | '<' | '+' | ','
                    ) || ch.is_ascii_alphanumeric() =>
                {
                    self.planet_overlay.prompt_input.push(ch);
                    self.planet_overlay.prompt_status = None;
                }
                _ => {}
            },
            PlanetOverlayPromptMode::None => {}
        }
        if prompt_mode != PlanetOverlayPromptMode::None {
            return;
        }
        match key.code {
            KeyCode::Esc => self.close_active_overlay(),
            KeyCode::Char('?') => self.open_overlay_help(HelpContext::PlanetList),
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.clear_planet_overlay_footer_notice();
                self.planet_overlay
                    .open_prompt(PlanetOverlayPromptMode::FilterMenu);
                self.planet_overlay.prompt_input.clear();
                self.planet_overlay.prompt_default = "all".to_string();
                self.planet_overlay.prompt_status = None;
                self.planet_overlay.pending_filter_column = None;
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.clear_planet_overlay_footer_notice();
                self.planet_overlay
                    .open_prompt(PlanetOverlayPromptMode::SortMenu);
                self.planet_overlay.prompt_input.clear();
                self.planet_overlay.prompt_default =
                    planet_sort_code(self.planet_overlay.sort).to_string();
                self.planet_overlay.prompt_status = None;
            }
            KeyCode::Char('b') | KeyCode::Char('B') => self.open_planet_build_specify(),
            KeyCode::Char(ch)
                if self.planet_overlay.jump_input.len() < 16
                    && table_selection::is_coordinate_input_char(ch) =>
            {
                self.clear_planet_overlay_footer_notice();
                self.planet_overlay.jump_input.push(ch);
                if planet_list::sync_cursor_to_jump_input(self) {
                    self.planet_overlay.jump_input.clear();
                }
            }
            KeyCode::Backspace => {
                self.clear_planet_overlay_footer_notice();
                self.planet_overlay.jump_input.pop();
            }
            _ => {
                self.clear_planet_overlay_footer_notice();
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
        let prompt_mode = self.fleet_overlay.prompt_mode;
        match prompt_mode {
            FleetOverlayPromptMode::ChangeField
            | FleetOverlayPromptMode::ChangeValue
            | FleetOverlayPromptMode::MergeHost
            | FleetOverlayPromptMode::MergeConfirm
            | FleetOverlayPromptMode::TransferHost
            | FleetOverlayPromptMode::TransferStage => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::FleetOrderInput),
                KeyCode::Enter => {
                    let result = match prompt_mode {
                        FleetOverlayPromptMode::ChangeField
                        | FleetOverlayPromptMode::ChangeValue => self.submit_fleet_change_prompt(),
                        FleetOverlayPromptMode::MergeHost
                        | FleetOverlayPromptMode::MergeConfirm => self.submit_fleet_merge_prompt(),
                        FleetOverlayPromptMode::TransferHost
                        | FleetOverlayPromptMode::TransferStage => {
                            self.submit_fleet_transfer_prompt()
                        }
                        _ => Ok(()),
                    };
                    if let Err(err) = result {
                        self.fleet_overlay.aux_status = Some(err.to_string());
                    }
                }
                KeyCode::Backspace => {
                    self.fleet_overlay.aux_input.pop();
                    self.fleet_overlay.aux_status = None;
                }
                KeyCode::Esc => {
                    self.cancel_fleet_aux_prompt();
                }
                KeyCode::Char(ch)
                    if match prompt_mode {
                        FleetOverlayPromptMode::ChangeField => ch.is_ascii_alphabetic(),
                        FleetOverlayPromptMode::ChangeValue
                        | FleetOverlayPromptMode::TransferHost
                        | FleetOverlayPromptMode::MergeHost => ch.is_ascii_alphanumeric(),
                        FleetOverlayPromptMode::MergeConfirm => matches!(ch, 'y' | 'Y' | 'n' | 'N'),
                        FleetOverlayPromptMode::TransferStage => {
                            match self.fleet_overlay.transfer_mode {
                                crate::dashboard::app::state::FleetOverlayTransferMode::ChoosingClass => {
                                    ch.is_ascii_alphanumeric() || ch == '*'
                                }
                                crate::dashboard::app::state::FleetOverlayTransferMode::EnteringQuantity(
                                    _,
                                ) => ch.is_ascii_digit(),
                            }
                        }
                        _ => false,
                    } =>
                {
                    self.fleet_overlay.aux_input.push(ch.to_ascii_uppercase());
                    self.fleet_overlay.aux_status = None;
                }
                _ => {}
            },
            FleetOverlayPromptMode::MissionPicker => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::FleetMissionPicker),
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    self.move_fleet_mission_picker(-1)
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    self.move_fleet_mission_picker(1)
                }
                KeyCode::PageUp => self.move_fleet_mission_picker(-8),
                KeyCode::PageDown => self.move_fleet_mission_picker(8),
                KeyCode::Enter => self.submit_fleet_mission_picker(),
                KeyCode::Backspace => self.backspace_fleet_mission_picker_input(),
                KeyCode::Esc => {
                    self.cancel_fleet_order_input();
                }
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    self.append_fleet_mission_picker_char(ch)
                }
                _ => {}
            },
            FleetOverlayPromptMode::OrderTarget => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::FleetOrderInput),
                KeyCode::Enter => {
                    if let Err(err) = self.submit_fleet_order() {
                        self.fleet_overlay.order_status = Some(err.to_string());
                    }
                }
                KeyCode::Backspace => self.backspace_fleet_order_input(),
                KeyCode::Esc => {
                    self.cancel_fleet_order_input();
                }
                KeyCode::Char(ch)
                    if match nc_engine::fleet_target_input_kind(
                        self.fleet_overlay.order_mission_code,
                    ) {
                        nc_engine::FleetTargetInputKind::Coordinates
                        | nc_engine::FleetTargetInputKind::None => {
                            table_selection::is_coordinate_input_char(ch)
                        }
                        nc_engine::FleetTargetInputKind::StarbaseId
                        | nc_engine::FleetTargetInputKind::FleetId => ch.is_ascii_digit(),
                    } =>
                {
                    self.append_fleet_order_char(ch)
                }
                _ => {}
            },
            FleetOverlayPromptMode::OrderTargetX | FleetOverlayPromptMode::OrderTargetY => {
                match key.code {
                    KeyCode::Char('?') => self.open_overlay_help(HelpContext::FleetOrderInput),
                    KeyCode::Enter => {
                        if let Err(err) = self.submit_fleet_order() {
                            self.fleet_overlay.order_status = Some(err.to_string());
                        }
                    }
                    KeyCode::Backspace => self.backspace_fleet_order_input(),
                    KeyCode::Esc => {
                        self.cancel_fleet_order_input();
                    }
                    KeyCode::Char(ch) if ch.is_ascii_digit() => self.append_fleet_order_char(ch),
                    _ => {}
                }
            }
            FleetOverlayPromptMode::OrderConfirm => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::FleetOrderInput),
                KeyCode::Enter => {
                    self.fleet_overlay.order_confirm_input.clear();
                    if let Err(err) = self.submit_fleet_order() {
                        self.fleet_overlay.order_status = Some(err.to_string());
                    }
                }
                KeyCode::Esc => {
                    self.fleet_overlay.order_confirm_input.clear();
                    self.cancel_fleet_order_input();
                }
                KeyCode::Char(ch) if matches!(ch, 'y' | 'Y' | 'n' | 'N') => {
                    self.fleet_overlay.order_confirm_input = ch.to_ascii_uppercase().to_string();
                    if let Err(err) = self.submit_fleet_order() {
                        self.fleet_overlay.order_status = Some(err.to_string());
                    }
                }
                _ => {}
            },
            FleetOverlayPromptMode::StarbaseMoveDecision => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::StarbaseMove),
                KeyCode::Enter => {
                    if let Err(err) = self.submit_starbase_move() {
                        self.fleet_overlay.starbase_move_status = Some(err.to_string());
                    }
                }
                KeyCode::Backspace => self.backspace_starbase_move_input(),
                KeyCode::Esc => {
                    self.cancel_fleet_order_input();
                }
                KeyCode::Char(ch) if ch.is_ascii_alphabetic() => self.append_starbase_move_char(ch),
                _ => {}
            },
            FleetOverlayPromptMode::StarbaseMoveDestination => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::StarbaseMove),
                KeyCode::Enter => {
                    if let Err(err) = self.submit_starbase_move() {
                        self.fleet_overlay.starbase_move_status = Some(err.to_string());
                    }
                }
                KeyCode::Backspace => self.backspace_starbase_move_input(),
                KeyCode::Esc => {
                    self.cancel_fleet_order_input();
                }
                KeyCode::Char(ch) if table_selection::is_coordinate_input_char(ch) => {
                    self.append_starbase_move_char(ch)
                }
                _ => {}
            },
            FleetOverlayPromptMode::StarbaseHaltConfirm => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::StarbaseMove),
                KeyCode::Enter => {
                    if let Err(err) = self.submit_starbase_move() {
                        self.fleet_overlay.starbase_move_status = Some(err.to_string());
                    }
                }
                KeyCode::Backspace => self.backspace_starbase_move_input(),
                KeyCode::Esc => {
                    self.cancel_fleet_order_input();
                }
                KeyCode::Char('n') | KeyCode::Char('N') => self.cancel_fleet_order_input(),
                KeyCode::Char(ch) if matches!(ch, 'y' | 'Y') => self.append_starbase_move_char(ch),
                _ => {}
            },
            FleetOverlayPromptMode::SortMenu => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::FleetListSort),
                KeyCode::Esc => {
                    self.fleet_overlay.close_prompt();
                }
                KeyCode::Enter => {
                    let raw = if self.fleet_overlay.filter_prompt_input.trim().is_empty() {
                        self.fleet_overlay.filter_prompt_default.trim()
                    } else {
                        self.fleet_overlay.filter_prompt_input.trim()
                    };
                    match parse_column_code(fleet_list::filter_columns(), raw) {
                        Ok(column) => match fleet_sort_from_code(column.code) {
                            Some(sort) => self.apply_fleet_overlay_sort(sort),
                            None => {
                                self.fleet_overlay.filter_prompt_input.clear();
                                self.fleet_overlay.filter_prompt_status =
                                    Some(" Enter a valid sort column.".to_string());
                            }
                        },
                        Err(err) => {
                            self.fleet_overlay.filter_prompt_status =
                                Some(format!(" {}", format_column_code_error(&err)));
                        }
                    }
                }
                KeyCode::Backspace => {
                    self.fleet_overlay.filter_prompt_input.pop();
                    self.fleet_overlay.filter_prompt_status = None;
                }
                KeyCode::Char(ch) if is_filter_column_char(ch) => {
                    self.fleet_overlay.filter_prompt_input.push(ch);
                    self.fleet_overlay.filter_prompt_status = None;
                }
                _ => {}
            },
            FleetOverlayPromptMode::FilterMenu => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::FleetListFilter),
                KeyCode::Esc => {
                    self.fleet_overlay.close_prompt();
                }
                KeyCode::Enter => {
                    let raw = if self.fleet_overlay.filter_prompt_input.trim().is_empty() {
                        self.fleet_overlay.filter_prompt_default.trim()
                    } else {
                        self.fleet_overlay.filter_prompt_input.trim()
                    };
                    if raw.eq_ignore_ascii_case("a") || raw.eq_ignore_ascii_case("all") {
                        self.apply_fleet_overlay_filter_clause(None);
                    } else {
                        match parse_column_code(fleet_list::filter_columns(), raw) {
                            Ok(column) => {
                                self.fleet_overlay.pending_filter_column = Some(column);
                                self.fleet_overlay.prompt_mode =
                                    FleetOverlayPromptMode::FilterValueInput;
                                self.fleet_overlay.filter_prompt_input.clear();
                                self.fleet_overlay.filter_prompt_default =
                                    fleet_list::filter_default_value(self, column);
                                self.fleet_overlay.filter_prompt_status = None;
                            }
                            Err(err) => {
                                self.fleet_overlay.filter_prompt_status =
                                    Some(format!(" {}", format_column_code_error(&err)));
                            }
                        }
                    }
                }
                KeyCode::Backspace => {
                    self.fleet_overlay.filter_prompt_input.pop();
                    self.fleet_overlay.filter_prompt_status = None;
                }
                KeyCode::Char(ch) if ch.is_ascii_alphabetic() => {
                    self.fleet_overlay.filter_prompt_input.push(ch);
                    self.fleet_overlay.filter_prompt_status = None;
                }
                _ => {}
            },
            FleetOverlayPromptMode::FilterValueInput => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PromptInput),
                KeyCode::Esc => {
                    self.fleet_overlay.clear_prompt();
                }
                KeyCode::Enter => {
                    let Some(column) = self.fleet_overlay.pending_filter_column else {
                        return;
                    };
                    let raw = if self.fleet_overlay.filter_prompt_input.trim().is_empty() {
                        self.fleet_overlay.filter_prompt_default.trim()
                    } else {
                        self.fleet_overlay.filter_prompt_input.trim()
                    };
                    match parse_filter_clause(column, raw) {
                        Ok(clause) => self.apply_fleet_overlay_filter_clause(Some(clause)),
                        Err(err) => {
                            self.fleet_overlay.filter_prompt_status = Some(format!(" {err}"))
                        }
                    }
                }
                KeyCode::Backspace => {
                    self.fleet_overlay.filter_prompt_input.pop();
                    self.fleet_overlay.filter_prompt_status = None;
                }
                KeyCode::Char(ch)
                    if matches!(
                        ch,
                        ' ' | '-' | '#' | '*' | '/' | '?' | '=' | '!' | '>' | '<' | '+' | ','
                    ) || ch.is_ascii_alphanumeric() =>
                {
                    self.fleet_overlay.filter_prompt_input.push(ch);
                    self.fleet_overlay.filter_prompt_status = None;
                }
                _ => {}
            },
            FleetOverlayPromptMode::None => {}
        }
        if prompt_mode != FleetOverlayPromptMode::None {
            return;
        }
        match key.code {
            KeyCode::Esc => self.close_active_overlay(),
            KeyCode::Char('?') => self.open_overlay_help(HelpContext::FleetList),
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.fleet_overlay
                    .open_prompt(FleetOverlayPromptMode::FilterMenu);
                self.fleet_overlay.filter_prompt_input.clear();
                self.fleet_overlay.filter_prompt_default = "all".to_string();
                self.fleet_overlay.filter_prompt_status = None;
                self.fleet_overlay.pending_filter_column = None;
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.fleet_overlay
                    .open_prompt(FleetOverlayPromptMode::SortMenu);
                self.fleet_overlay.filter_prompt_input.clear();
                self.fleet_overlay.filter_prompt_default =
                    fleet_sort_code(self.fleet_overlay.sort).to_string();
                self.fleet_overlay.filter_prompt_status = None;
            }
            KeyCode::Char('o') | KeyCode::Char('O') => self.open_selected_fleet_order_flow(),
            KeyCode::Char('c') | KeyCode::Char('C') => self.open_selected_fleet_change_flow(),
            KeyCode::Char('m') | KeyCode::Char('M') => self.open_selected_fleet_merge_flow(),
            KeyCode::Char('t') | KeyCode::Char('T') => self.open_selected_fleet_transfer_flow(),
            KeyCode::Char(' ') => self.toggle_selected_fleet_row_for_group_order(),
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
        let prompt_mode = self.intel_overlay.prompt_mode;
        match prompt_mode {
            IntelOverlayPromptMode::SortMenu => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::IntelDatabaseSort),
                KeyCode::Esc => {
                    self.intel_overlay.close_prompt();
                }
                KeyCode::Enter => {
                    let raw = if self.intel_overlay.prompt_input.trim().is_empty() {
                        self.intel_overlay.prompt_default.trim()
                    } else {
                        self.intel_overlay.prompt_input.trim()
                    };
                    if raw.eq_ignore_ascii_case("rng") || raw.eq_ignore_ascii_case("range") {
                        self.intel_overlay
                            .open_prompt(IntelOverlayPromptMode::SortRangeInput);
                        self.intel_overlay.prompt_input.clear();
                        self.intel_overlay.prompt_default = match self.intel_overlay.sort {
                            IntelOverlaySort::Range(anchor) => {
                                crate::dashboard::coords::format_sector_coords_default(anchor)
                            }
                            _ => intel_database::table_rows(self)
                                .get(self.intel_overlay.selected)
                                .map(|row| {
                                    crate::dashboard::coords::format_sector_coords_default(
                                        row.coords,
                                    )
                                })
                                .unwrap_or_else(|| "00,00".to_string()),
                        };
                        self.intel_overlay.prompt_status = None;
                    } else {
                        match parse_column_code(intel_database::filter_columns(), raw) {
                            Ok(column) => match intel_sort_from_code(column.code) {
                                Some(sort) => self.apply_intel_overlay_sort(sort),
                                None => {
                                    self.intel_overlay.prompt_input.clear();
                                    self.intel_overlay.prompt_status =
                                        Some(" Enter a valid sort column or RNG.".to_string());
                                }
                            },
                            Err(err) => {
                                self.intel_overlay.prompt_status =
                                    Some(format!(" {}", format_column_code_error(&err)));
                            }
                        }
                    }
                }
                KeyCode::Backspace => {
                    self.intel_overlay.prompt_input.pop();
                    self.intel_overlay.prompt_status = None;
                }
                KeyCode::Char(ch) if is_filter_column_char(ch) => {
                    self.intel_overlay.prompt_input.push(ch);
                    self.intel_overlay.prompt_status = None;
                }
                _ => {}
            },
            IntelOverlayPromptMode::SortRangeInput => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PromptInput),
                KeyCode::Esc => {
                    self.intel_overlay.close_prompt();
                }
                KeyCode::Enter => {
                    let default = intel_database::parse_coords_input(
                        &self.intel_overlay.prompt_default,
                        [0, 0],
                    )
                    .unwrap_or([0, 0]);
                    if let Some(anchor) = intel_database::parse_coords_input(
                        &self.intel_overlay.prompt_input,
                        default,
                    ) {
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
                KeyCode::Esc => {
                    self.intel_overlay.close_prompt();
                }
                KeyCode::Enter => {
                    let raw = if self.intel_overlay.prompt_input.trim().is_empty() {
                        self.intel_overlay.prompt_default.trim()
                    } else {
                        self.intel_overlay.prompt_input.trim()
                    };
                    if raw.eq_ignore_ascii_case("a") || raw.eq_ignore_ascii_case("all") {
                        self.apply_intel_overlay_filter_clause(None);
                    } else {
                        match parse_column_code(intel_database::filter_columns(), raw) {
                            Ok(column) => {
                                self.intel_overlay.pending_filter_column = Some(column);
                                self.intel_overlay.prompt_mode =
                                    IntelOverlayPromptMode::FilterValueInput;
                                self.intel_overlay.prompt_input.clear();
                                self.intel_overlay.prompt_default =
                                    intel_database::filter_default_value(self, column);
                                self.intel_overlay.prompt_status = None;
                            }
                            Err(err) => {
                                self.intel_overlay.prompt_status =
                                    Some(format!(" {}", format_column_code_error(&err)));
                            }
                        }
                    }
                }
                KeyCode::Backspace => {
                    self.intel_overlay.prompt_input.pop();
                    self.intel_overlay.prompt_status = None;
                }
                KeyCode::Char(ch) if ch.is_ascii_alphabetic() => {
                    self.intel_overlay.prompt_input.push(ch);
                    self.intel_overlay.prompt_status = None;
                }
                _ => {}
            },
            IntelOverlayPromptMode::FilterValueInput => match key.code {
                KeyCode::Char('?') => self.open_overlay_help(HelpContext::PromptInput),
                KeyCode::Esc => {
                    self.intel_overlay.clear_prompt();
                }
                KeyCode::Enter => {
                    let Some(column) = self.intel_overlay.pending_filter_column else {
                        return;
                    };
                    let raw = if self.intel_overlay.prompt_input.trim().is_empty() {
                        self.intel_overlay.prompt_default.trim()
                    } else {
                        self.intel_overlay.prompt_input.trim()
                    };
                    match parse_filter_clause(column, raw) {
                        Ok(clause) => self.apply_intel_overlay_filter_clause(Some(clause)),
                        Err(err) => self.intel_overlay.prompt_status = Some(format!(" {err}")),
                    }
                }
                KeyCode::Backspace => {
                    self.intel_overlay.prompt_input.pop();
                    self.intel_overlay.prompt_status = None;
                }
                KeyCode::Char(ch)
                    if matches!(
                        ch,
                        ' ' | '-' | '#' | '*' | '/' | '?' | '=' | '!' | '>' | '<' | '+' | ','
                    ) || ch.is_ascii_alphanumeric() =>
                {
                    self.intel_overlay.prompt_input.push(ch);
                    self.intel_overlay.prompt_status = None;
                }
                _ => {}
            },
            IntelOverlayPromptMode::None => {}
        }
        if prompt_mode != IntelOverlayPromptMode::None {
            return;
        }
        match key.code {
            KeyCode::Esc => self.close_active_overlay(),
            KeyCode::Char('?') => self.open_overlay_help(HelpContext::IntelDatabase),
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.intel_overlay
                    .open_prompt(IntelOverlayPromptMode::FilterMenu);
                self.intel_overlay.prompt_input.clear();
                self.intel_overlay.prompt_default = "all".to_string();
                self.intel_overlay.prompt_status = None;
                self.intel_overlay.pending_filter_column = None;
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.intel_overlay
                    .open_prompt(IntelOverlayPromptMode::SortMenu);
                self.intel_overlay.prompt_input.clear();
                self.intel_overlay.prompt_default =
                    intel_sort_code(self.intel_overlay.sort).to_string();
                self.intel_overlay.prompt_status = None;
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

impl NativeApp for DashApp {
    fn window_title(&self) -> &'static str {
        "Nostrian Conquest Dashboard"
    }

    fn geometry(&self) -> ScreenGeometry {
        self.geometry
    }

    fn wants_window_focus(&self) -> bool {
        true
    }

    fn saved_window_state(
        &self,
    ) -> Option<crate::dashboard::client_settings::PersistedWindowState> {
        self.client_settings.persisted_window_state()
    }

    fn persist_window_state(
        &mut self,
        state: crate::dashboard::client_settings::PersistedWindowState,
    ) -> Result<(), String> {
        let Some(path) = self.client_settings_path.as_deref() else {
            return Ok(());
        };
        self.client_settings.set_persisted_window_state(state);
        crate::dashboard::client_settings::save_client_settings_to(&self.client_settings, path)
            .map_err(|err| err.to_string())
    }

    fn dispatch_key_event(&mut self, key: crate::dashboard::input::KeyEvent) {
        Self::dispatch_key_event(self, key);
    }

    fn dispatch_mouse_event(&mut self, mouse: MouseEvent) -> bool {
        Self::dispatch_mouse_event(self, mouse)
    }

    fn resize_canvas(&mut self, cols: u16, rows: u16) {
        Self::resize_canvas(self, cols, rows);
    }

    fn render_scene(&self) -> Result<UiScene, Box<dyn std::error::Error>> {
        Ok(UiScene::from(Self::render_playfield(self)?))
    }

    fn debug_render_signature(&self) -> Option<String> {
        Some(format!(
            "focus={:?} overlay={:?} popup={:?} crosshair={},{} map_view={:?} popup_pos={} overlay_pos={} gesture={:?} toast={} report_blocks={} mail={} too_small={}",
            self.focus,
            self.overlay,
            self.popup,
            self.crosshair_x,
            self.crosshair_y,
            self.map_view_mode,
            self.popup_position.is_some(),
            self.overlay_position.is_some(),
            self.mouse_gesture,
            self.active_command_line_toast().unwrap_or("-"),
            self.report_block_rows.len(),
            self.queued_mail.len(),
            self.is_terminal_too_small,
        ))
    }

    fn on_idle(&mut self) -> bool {
        self.update_command_line_toast_state(Instant::now())
    }

    fn next_wakeup(&self) -> Option<Instant> {
        self.command_line_toast_deadline
    }

    fn is_dragging_surface(&self) -> bool {
        matches!(
            self.mouse_gesture,
            ActiveMouseGesture::DraggingOverlay { .. }
                | ActiveMouseGesture::DraggingPopup { .. }
                | ActiveMouseGesture::DraggingStarmap { .. }
        )
    }

    fn should_quit(&self) -> bool {
        self.should_quit
    }

    fn set_should_quit(&mut self, should_quit: bool) {
        self.should_quit = should_quit;
    }
}

impl DashApp {
    fn open_overlay_help(&mut self, help_context: HelpContext) {
        self.help_return_overlay = self.overlay;
        self.help_return_overlay_position = self.overlay_position.take();
        self.help_context = help_context;
        self.overlay = ActiveOverlay::Help;
        self.mouse_gesture = ActiveMouseGesture::None;
    }

    fn reset_planet_overlay_prompt(&mut self) {
        self.planet_overlay.clear_prompt();
    }

    fn apply_planet_overlay_sort(&mut self, sort: PlanetOverlaySort) {
        let selected_record = planet_list::table_rows(self)
            .get(self.planet_overlay.selected)
            .map(|row| row.planet_record_index_1_based);
        if self.planet_overlay.sort == sort {
            self.planet_overlay.sort_direction = self.planet_overlay.sort_direction.toggle();
        } else {
            self.planet_overlay.sort = sort;
            self.planet_overlay.sort_direction = default_planet_overlay_sort_direction(sort);
        }
        self.reset_planet_overlay_prompt();
        let rows = planet_list::table_rows(self);
        self.planet_overlay.selected = selected_record
            .and_then(|record| {
                rows.iter()
                    .position(|row| row.planet_record_index_1_based == record)
            })
            .unwrap_or(0);
        sync_scroll_to_cursor(
            &mut self.planet_overlay.scroll,
            self.planet_overlay.selected,
            1_000,
        );
    }

    fn enforce_valid_planet_filter(&mut self) {
        if self.planet_overlay.filter == crate::dashboard::app::state::PlanetOverlayFilter::All
            && self.planet_overlay.filter_clause.is_none()
        {
            return;
        }
        if !crate::dashboard::overlays::planet_list::table_rows(self).is_empty() {
            return;
        }

        let previous_filter = self.planet_overlay.filter;
        let previous_clause = self.planet_overlay.filter_clause.clone();
        self.planet_overlay.filter = crate::dashboard::app::state::PlanetOverlayFilter::All;
        self.planet_overlay.filter_clause = None;
        if crate::dashboard::overlays::planet_list::table_rows(self).is_empty() {
            self.planet_overlay.filter = previous_filter;
            self.planet_overlay.filter_clause = previous_clause;
            return;
        }

        self.planet_overlay.selected = 0;
        self.planet_overlay.scroll = 0;
    }

    #[cfg(test)]
    fn apply_planet_overlay_filter(&mut self, filter: PlanetOverlayFilter) {
        let selected_record = planet_list::table_rows(self)
            .get(self.planet_overlay.selected)
            .map(|row| row.planet_record_index_1_based);
        self.planet_overlay.filter = filter;
        self.planet_overlay.filter_clause = None;
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
        sync_scroll_to_cursor(
            &mut self.planet_overlay.scroll,
            self.planet_overlay.selected,
            1_000,
        );
    }

    fn apply_fleet_overlay_sort(&mut self, sort: FleetOverlaySort) {
        let selected_key = fleet_list::table_rows(self)
            .get(self.fleet_overlay.selected)
            .map(|row| row.key);
        if self.fleet_overlay.sort == sort {
            self.fleet_overlay.sort_direction = self.fleet_overlay.sort_direction.toggle();
        } else {
            self.fleet_overlay.sort = sort;
            self.fleet_overlay.sort_direction = default_fleet_overlay_sort_direction(sort);
        }
        self.fleet_overlay.clear_prompt();
        let rows = fleet_list::table_rows(self);
        self.fleet_overlay.selected = selected_key
            .and_then(|key| rows.iter().position(|row| row.key == key))
            .unwrap_or(0);
        sync_scroll_to_cursor(
            &mut self.fleet_overlay.scroll,
            self.fleet_overlay.selected,
            1_000,
        );
    }

    fn enforce_valid_fleet_filter(&mut self) {
        if self.fleet_overlay.filter == FleetOverlayFilter::All
            && self.fleet_overlay.filter_clause.is_none()
        {
            return;
        }
        if !fleet_list::table_rows(self).is_empty() {
            return;
        }

        let previous_filter = self.fleet_overlay.filter;
        let previous_clause = self.fleet_overlay.filter_clause.clone();
        self.fleet_overlay.filter = FleetOverlayFilter::All;
        self.fleet_overlay.filter_clause = None;
        if fleet_list::table_rows(self).is_empty() {
            self.fleet_overlay.filter = previous_filter;
            self.fleet_overlay.filter_clause = previous_clause;
            return;
        }

        self.fleet_overlay.clear_group_selection();
        self.fleet_overlay.selected = 0;
        self.fleet_overlay.scroll = 0;
    }

    #[cfg(test)]
    fn apply_fleet_overlay_filter(&mut self, filter: FleetOverlayFilter) {
        let selected_key = fleet_list::table_rows(self)
            .get(self.fleet_overlay.selected)
            .map(|row| row.key);
        self.fleet_overlay.filter = filter;
        self.fleet_overlay.filter_clause = None;
        self.fleet_overlay.clear_group_selection();
        self.fleet_overlay.clear_prompt();
        let rows = fleet_list::table_rows(self);
        if rows.is_empty() {
            self.fleet_overlay.filter = FleetOverlayFilter::All;
        }
        let rows = fleet_list::table_rows(self);
        self.fleet_overlay.selected = selected_key
            .and_then(|key| rows.iter().position(|row| row.key == key))
            .unwrap_or(0);
        sync_scroll_to_cursor(
            &mut self.fleet_overlay.scroll,
            self.fleet_overlay.selected,
            1_000,
        );
    }

    fn reset_intel_overlay_prompt(&mut self) {
        self.intel_overlay.clear_prompt();
    }

    fn enforce_valid_intel_filter(&mut self) {
        if self.intel_overlay.filter == IntelOverlayFilter::All
            && self.intel_overlay.filter_clause.is_none()
        {
            return;
        }
        if !intel_database::table_rows(self).is_empty() {
            return;
        }

        let previous_filter = self.intel_overlay.filter;
        let previous_clause = self.intel_overlay.filter_clause.clone();
        self.intel_overlay.filter = IntelOverlayFilter::All;
        self.intel_overlay.filter_clause = None;
        if intel_database::table_rows(self).is_empty() {
            self.intel_overlay.filter = previous_filter;
            self.intel_overlay.filter_clause = previous_clause;
            return;
        }

        self.intel_overlay.selected = 0;
        self.intel_overlay.scroll = 0;
    }

    fn normalize_table_overlay_filters(&mut self) {
        self.enforce_valid_fleet_filter();
        self.enforce_valid_planet_filter();
        self.enforce_valid_intel_filter();
    }

    fn apply_intel_overlay_sort(&mut self, sort: IntelOverlaySort) {
        let selected_record = intel_database::table_rows(self)
            .get(self.intel_overlay.selected)
            .map(|row| row.planet_record_index_1_based);
        if self.intel_overlay.sort == sort {
            self.intel_overlay.sort_direction = self.intel_overlay.sort_direction.toggle();
        } else {
            self.intel_overlay.sort = sort;
            self.intel_overlay.sort_direction = default_intel_overlay_sort_direction(sort);
        }
        self.reset_intel_overlay_prompt();
        let rows = intel_database::table_rows(self);
        self.intel_overlay.selected = selected_record
            .and_then(|record| {
                rows.iter()
                    .position(|row| row.planet_record_index_1_based == record)
            })
            .unwrap_or(0);
        sync_scroll_to_cursor(
            &mut self.intel_overlay.scroll,
            self.intel_overlay.selected,
            10_000,
        );
    }

    #[cfg(test)]
    fn apply_intel_overlay_filter(&mut self, filter: IntelOverlayFilter) {
        let selected_record = intel_database::table_rows(self)
            .get(self.intel_overlay.selected)
            .map(|row| row.planet_record_index_1_based);
        self.intel_overlay.filter = filter;
        self.intel_overlay.filter_clause = None;
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
        sync_scroll_to_cursor(
            &mut self.intel_overlay.scroll,
            self.intel_overlay.selected,
            10_000,
        );
    }

    fn apply_planet_overlay_filter_clause(
        &mut self,
        clause: Option<crate::dashboard::table_filter::TableFilterClause>,
    ) {
        let selected_record = planet_list::table_rows(self)
            .get(self.planet_overlay.selected)
            .map(|row| row.planet_record_index_1_based);
        self.planet_overlay.filter = PlanetOverlayFilter::All;
        self.planet_overlay.filter_clause = clause;
        self.reset_planet_overlay_prompt();
        let rows = planet_list::table_rows(self);
        if rows.is_empty() {
            self.planet_overlay.filter_clause = None;
        }
        let rows = planet_list::table_rows(self);
        self.planet_overlay.selected = selected_record
            .and_then(|record| {
                rows.iter()
                    .position(|row| row.planet_record_index_1_based == record)
            })
            .unwrap_or(0);
        sync_scroll_to_cursor(
            &mut self.planet_overlay.scroll,
            self.planet_overlay.selected,
            1_000,
        );
    }

    fn apply_fleet_overlay_filter_clause(
        &mut self,
        clause: Option<crate::dashboard::table_filter::TableFilterClause>,
    ) {
        let selected_key = fleet_list::table_rows(self)
            .get(self.fleet_overlay.selected)
            .map(|row| row.key);
        self.fleet_overlay.filter = FleetOverlayFilter::All;
        self.fleet_overlay.filter_clause = clause;
        self.fleet_overlay.clear_group_selection();
        self.fleet_overlay.clear_prompt();
        let rows = fleet_list::table_rows(self);
        if rows.is_empty() {
            self.fleet_overlay.filter_clause = None;
        }
        let rows = fleet_list::table_rows(self);
        self.fleet_overlay.selected = selected_key
            .and_then(|key| rows.iter().position(|row| row.key == key))
            .unwrap_or(0);
        sync_scroll_to_cursor(
            &mut self.fleet_overlay.scroll,
            self.fleet_overlay.selected,
            1_000,
        );
    }

    fn apply_intel_overlay_filter_clause(
        &mut self,
        clause: Option<crate::dashboard::table_filter::TableFilterClause>,
    ) {
        let selected_record = intel_database::table_rows(self)
            .get(self.intel_overlay.selected)
            .map(|row| row.planet_record_index_1_based);
        self.intel_overlay.filter = IntelOverlayFilter::All;
        self.intel_overlay.filter_clause = clause;
        self.reset_intel_overlay_prompt();
        let rows = intel_database::table_rows(self);
        if rows.is_empty() {
            self.intel_overlay.filter_clause = None;
        }
        let rows = intel_database::table_rows(self);
        self.intel_overlay.selected = selected_record
            .and_then(|record| {
                rows.iter()
                    .position(|row| row.planet_record_index_1_based == record)
            })
            .unwrap_or(0);
        sync_scroll_to_cursor(
            &mut self.intel_overlay.scroll,
            self.intel_overlay.selected,
            10_000,
        );
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

fn scroll_clamp(value: i32, max: i32) -> usize {
    value.clamp(0, max.max(0)) as usize
}

fn scroll_selected(selected: usize, lines: i32, total: usize) -> usize {
    let last = total.saturating_sub(1);
    if lines < 0 {
        selected.saturating_add((-lines) as usize).min(last)
    } else {
        selected.saturating_sub(lines as usize)
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
    .filter(|item| matches_filter(item, state.filter, state.current_year_only, current_year))
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

fn modal_chrome_contains(popup: Rect, col: usize, row: usize) -> bool {
    let left = popup.x as usize;
    let top = popup.y as usize;
    let right = left + popup.width as usize - 1;
    let bottom = top + popup.height as usize - 1;
    (row == top && col >= left && col <= right)
        || (row == bottom && col >= left && col <= right)
        || (col == left && row >= top && row <= bottom)
        || (col == right && row >= top && row <= bottom)
}

#[cfg(test)]
mod tests {
    use super::{map_coord_rows, parse_table_coord};
    use crate::dashboard::app::state::{
        ActiveOverlay, ActivePopup, DashApp, DashboardExitRequest, FleetOrderScope,
        FleetOverlayFilter, FleetOverlayPromptMode, FleetOverlayRowKey, FleetOverlaySort,
        HelpContext, IntelOverlayFilter, IntelOverlayPromptMode, IntelOverlaySort, MapViewMode,
        OwnedPlanetPopupMode, PlanetOverlayFilter, PlanetOverlayPromptMode, PlanetOverlaySort,
        SortDirection,
    };
    use crate::dashboard::buffer::PlayfieldBuffer;
    use crate::dashboard::geometry::ScreenGeometry;
    use crate::dashboard::input::{
        KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };
    use crate::dashboard::layout::dashboard::dashboard_layout;
    use crate::dashboard::native::NativeApp;
    use crate::dashboard::panels::starmap;
    use crate::dashboard::overlays::{fleet_list, intel_database, planet_list};
    use crate::dashboard::planet_view;
    use crate::dashboard::table_selection::{wrap_next_index, wrap_prev_index};
    use nc_data::{CampaignStore, GameStateBuilder, IntelTier, Order, PlanetIntelSnapshot, QueuedPlayerMail, ReportBlockRow};
    use nc_engine::{
        build_seeded_initialized_game, fleet_target_input_kind, recommended_coordinate_target,
        recommended_coordinate_target_y_for_entered_x,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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
            crate::dashboard::input::KeyModifiers::NONE,
        ));
        app.handle_key(KeyEvent::new(
            KeyCode::Char('2'),
            crate::dashboard::input::KeyModifiers::NONE,
        ));
        app.handle_key(KeyEvent::new(
            KeyCode::Char(','),
            crate::dashboard::input::KeyModifiers::NONE,
        ));
        app.handle_key(KeyEvent::new(
            KeyCode::Char('0'),
            crate::dashboard::input::KeyModifiers::NONE,
        ));
        app.handle_key(KeyEvent::new(
            KeyCode::Char('3'),
            crate::dashboard::input::KeyModifiers::NONE,
        ));

        assert_eq!([app.crosshair_x, app.crosshair_y], [2, 3]);
        assert!(app.map_coord_input.is_empty());
    }

    #[test]
    fn typed_map_coords_keep_partial_input_visible() {
        let mut app = dash_app();
        app.handle_key(KeyEvent::new(
            KeyCode::Char('0'),
            crate::dashboard::input::KeyModifiers::NONE,
        ));
        app.handle_key(KeyEvent::new(
            KeyCode::Char('2'),
            crate::dashboard::input::KeyModifiers::NONE,
        ));

        assert_eq!([app.crosshair_x, app.crosshair_y], [2, 1]);
        assert_eq!(app.map_coord_input, "02");
    }

    #[test]
    fn typed_map_coords_do_not_enter_readable_void_rows() {
        let mut app = dash_app();
        app.handle_key(KeyEvent::new(
            KeyCode::Char('0'),
            crate::dashboard::input::KeyModifiers::NONE,
        ));
        app.handle_key(KeyEvent::new(
            KeyCode::Char('1'),
            crate::dashboard::input::KeyModifiers::NONE,
        ));
        app.handle_key(KeyEvent::new(
            KeyCode::Char(','),
            crate::dashboard::input::KeyModifiers::NONE,
        ));
        app.handle_key(KeyEvent::new(
            KeyCode::Char('2'),
            crate::dashboard::input::KeyModifiers::NONE,
        ));
        app.handle_key(KeyEvent::new(
            KeyCode::Char('3'),
            crate::dashboard::input::KeyModifiers::NONE,
        ));

        assert!(app.crosshair_x <= 18);
        assert!(app.crosshair_y <= 18);
    }

    #[test]
    fn dashboard_actions_clear_partial_map_coord_input() {
        let mut app = dash_app();
        app.handle_key(KeyEvent::new(
            KeyCode::Char('0'),
            crate::dashboard::input::KeyModifiers::NONE,
        ));
        app.handle_key(KeyEvent::new(
            KeyCode::Char('2'),
            crate::dashboard::input::KeyModifiers::NONE,
        ));
        app.handle_key(KeyEvent::new(
            KeyCode::Char(']'),
            crate::dashboard::input::KeyModifiers::NONE,
        ));

        assert!(app.map_coord_input.is_empty());
    }

    #[test]
    fn map_view_mode_key_toggles_readable_and_fill() {
        let mut app = dash_app();

        assert_eq!(app.map_view_mode, MapViewMode::Readable);
        app.handle_key(KeyEvent::new(
            KeyCode::Char('v'),
            crate::dashboard::input::KeyModifiers::NONE,
        ));
        assert_eq!(app.map_view_mode, MapViewMode::Fill);
        app.handle_key(KeyEvent::new(
            KeyCode::Char('v'),
            crate::dashboard::input::KeyModifiers::NONE,
        ));
        assert_eq!(app.map_view_mode, MapViewMode::Readable);
    }

    #[test]
    fn toggling_map_view_rechecks_too_small_state() {
        let mut app = dash_app();
        app.geometry = ScreenGeometry::new(40, 20);
        app.is_terminal_too_small = false;

        app.handle_key(KeyEvent::new(
            KeyCode::Char('v'),
            crate::dashboard::input::KeyModifiers::NONE,
        ));

        assert!(app.is_terminal_too_small);
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

    #[test]
    fn closing_planet_build_modal_returns_to_planet_list_overlay() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::PlanetList;
        app.game_data.planets.records[0].set_stored_production_points(50);
        app.open_planet_build_specify();
        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::BuildSpecify
        );

        app.handle_key(key(KeyCode::Esc));

        assert_eq!(app.overlay, ActiveOverlay::PlanetList);
        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::None
        );
    }

    #[test]
    fn opening_build_specify_with_no_budget_keeps_build_table_open() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::PlanetList;
        app.game_data.planets.records[0].set_stored_production_points(1);

        app.open_planet_build_specify();

        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::BuildSpecify
        );
        assert_eq!(app.planet_overlay.footer_notice, None);
        assert_eq!(
            app.planet_overlay.build_planet_record_index_1_based,
            Some(1)
        );
    }

    #[test]
    fn successful_build_that_exhausts_budget_stays_in_build_table() {
        let mut app = dash_app_with_store();
        app.overlay = ActiveOverlay::PlanetList;
        app.game_data.planets.records[0].set_stored_production_points(2);

        app.open_planet_build_specify();
        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::BuildSpecify
        );

        app.handle_key(key(KeyCode::Char('+')));

        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::BuildSpecify
        );
        assert_eq!(app.overlay, ActiveOverlay::PlanetList);
        assert_eq!(app.planet_overlay.footer_notice, None);
        assert_eq!(
            app.planet_overlay.build_planet_record_index_1_based,
            Some(1)
        );
    }

    #[test]
    fn equals_key_queues_selected_build_unit() {
        let mut app = dash_app_with_store();
        app.overlay = ActiveOverlay::PlanetList;
        app.game_data.planets.records[0].set_stored_production_points(80);
        app.open_planet_build_specify();

        app.handle_key(key(KeyCode::Char('=')));

        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::BuildSpecify
        );
        let orders = nc_engine::planet_build_orders(&app.game_data.planets.records[0]);
        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].kind, nc_data::ProductionItemKind::Destroyer);
        assert_eq!(orders[0].points_remaining, 5);
    }

    #[test]
    fn empty_build_browse_enter_opens_quantity_for_highlighted_unit() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::PlanetList;
        app.game_data.planets.records[0].set_stored_production_points(80);
        app.open_planet_build_specify();

        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.overlay, ActiveOverlay::PlanetList);
        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::BuildQuantity
        );
    }

    #[test]
    fn zero_build_browse_enter_exits_overlay() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::PlanetList;
        app.open_planet_build_specify();

        app.handle_key(key(KeyCode::Char('0')));
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.overlay, ActiveOverlay::PlanetList);
        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::None
        );
    }

    #[test]
    fn numeric_build_browse_enter_selects_unit_and_opens_quantity() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::PlanetList;
        app.game_data.planets.records[0].set_stored_production_points(80);
        app.open_planet_build_specify();

        app.handle_key(key(KeyCode::Char('2')));
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::BuildQuantity
        );
        assert_eq!(
            app.planet_overlay.build_selected_kind,
            Some(nc_data::ProductionItemKind::Cruiser)
        );
    }

    #[test]
    fn numeric_build_browse_input_highlights_matching_unit_before_enter() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::PlanetList;
        app.game_data.planets.records[0].set_stored_production_points(80);
        app.open_planet_build_specify();

        app.handle_key(key(KeyCode::Char('2')));

        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::BuildSpecify
        );
        assert_eq!(
            app.planet_overlay.build_selected_kind,
            Some(nc_data::ProductionItemKind::Cruiser)
        );
    }

    #[test]
    fn planet_overlay_delete_clears_selected_kind_queue_in_place() {
        let mut app = dash_app_with_store();
        app.overlay = ActiveOverlay::PlanetList;
        app.game_data
            .append_planet_build_order(1, 10, 1)
            .expect("queue build order");
        app.open_planet_build_specify();

        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::BuildSpecify
        );
        assert_eq!(
            app.planet_overlay.build_selected_kind,
            Some(nc_data::ProductionItemKind::Destroyer)
        );

        app.handle_key(key(KeyCode::Char('d')));

        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::BuildSpecify
        );
        assert!(
            nc_engine::planet_build_orders(&app.game_data.planets.records[0]).is_empty(),
            "build queue should be cleared for the selected unit"
        );
    }

    #[test]
    fn planet_overlay_minus_removes_one_selected_unit_in_place() {
        let mut app = dash_app_with_store();
        app.overlay = ActiveOverlay::PlanetList;
        app.game_data
            .append_planet_build_order(1, 15, 1)
            .expect("queue build order");
        app.open_planet_build_specify();

        app.handle_key(key(KeyCode::Char('-')));

        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::BuildSpecify
        );
        let orders = nc_engine::planet_build_orders(&app.game_data.planets.records[0]);
        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].points_remaining, 10);
    }

    #[test]
    fn planet_overlay_delete_reports_empty_selected_kind_queue_inline() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::PlanetList;
        app.open_planet_build_specify();

        app.handle_key(key(KeyCode::Char('d')));

        assert_eq!(
            app.planet_overlay.build_unit_status.as_deref(),
            Some("No queued units of this type.")
        );
    }

    #[test]
    fn planet_overlay_footer_notice_clears_after_navigation() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::PlanetList;
        app.planet_overlay.footer_notice = Some("No build budget remains.".to_string());

        app.handle_key(key(KeyCode::Down));

        assert_eq!(app.planet_overlay.footer_notice, None);
    }

    #[test]
    fn nested_planet_filter_modals_unwind_one_level_at_a_time() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::PlanetList;

        app.handle_key(key(KeyCode::Char('f')));
        app.handle_key(key(KeyCode::Char('c')));
        app.handle_key(key(KeyCode::Char('o')));
        app.handle_key(key(KeyCode::Char('o')));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::FilterValueInput
        );

        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.overlay, ActiveOverlay::PlanetList);
        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::None
        );
    }

    #[test]
    fn planet_filter_prompt_accepts_unique_prefix_and_reports_ambiguity_inline() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::PlanetList;

        app.handle_key(key(KeyCode::Char('f')));
        app.handle_key(key(KeyCode::Char('s')));
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.overlay, ActiveOverlay::PlanetList);
        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::FilterMenu
        );
        assert_eq!(
            app.planet_overlay.prompt_status.as_deref(),
            Some(" Ambiguous: sbs/sta")
        );
        assert!(
            render_planet_footer_line(&app, "Ambiguous: sbs/sta")
                .contains("COMMAND <-  Ambiguous: sbs/sta")
        );

        app.handle_key(key(KeyCode::Backspace));
        assert_eq!(app.planet_overlay.prompt_status, None);
        app.handle_key(key(KeyCode::Char('d')));
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::FilterValueInput
        );
        assert_eq!(
            app.planet_overlay
                .pending_filter_column
                .expect("pending column")
                .code,
            "sta"
        );
    }

    #[test]
    fn closing_fleet_order_modal_returns_to_fleet_list_overlay() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::FleetList;
        app.open_selected_fleet_order_flow();

        app.handle_key(key(KeyCode::Esc));

        assert_eq!(app.overlay, ActiveOverlay::FleetList);
        assert_eq!(app.fleet_overlay.prompt_mode, FleetOverlayPromptMode::None);
    }

    #[test]
    fn nested_fleet_order_modals_unwind_one_level_at_a_time() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::FleetList;
        app.open_selected_fleet_order_flow();
        app.fleet_overlay.mission_picker_input = Order::MoveOnly.to_raw().to_string();
        app.submit_fleet_mission_picker();
        assert_eq!(
            app.fleet_overlay.prompt_mode,
            FleetOverlayPromptMode::OrderTargetX
        );

        app.handle_key(key(KeyCode::Enter));
        assert_eq!(
            app.fleet_overlay.prompt_mode,
            FleetOverlayPromptMode::OrderTargetY
        );

        app.handle_key(key(KeyCode::Esc));
        assert_eq!(
            app.fleet_overlay.prompt_mode,
            FleetOverlayPromptMode::OrderTargetX
        );

        app.handle_key(key(KeyCode::Esc));
        assert_eq!(
            app.fleet_overlay.prompt_mode,
            FleetOverlayPromptMode::MissionPicker
        );

        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.overlay, ActiveOverlay::FleetList);
        assert_eq!(app.fleet_overlay.prompt_mode, FleetOverlayPromptMode::None);
    }

    #[test]
    fn empty_coordinate_submissions_accept_defaults() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::FleetList;
        select_first_fleet_row(&mut app);
        app.open_selected_fleet_order_flow();
        app.fleet_overlay.order_mission_code = Some(Order::MoveOnly.to_raw());
        app.fleet_overlay.prompt_mode = FleetOverlayPromptMode::OrderTargetX;

        let expected_x = app.fleet_order_target_x_default_value();
        assert!(!expected_x.is_empty());

        app.handle_key(key(KeyCode::Enter));

        assert_eq!(
            app.fleet_overlay.prompt_mode,
            FleetOverlayPromptMode::OrderTargetY
        );
        assert_eq!(app.fleet_overlay.order_target_x_input, expected_x);

        let expected_y = app.fleet_order_target_y_default_value();
        assert!(!expected_y.is_empty());

        app.handle_key(key(KeyCode::Enter));

        assert_eq!(
            app.fleet_overlay.prompt_mode,
            FleetOverlayPromptMode::OrderConfirm
        );
        assert_eq!(app.fleet_overlay.order_target_y_input, expected_y);
    }

    #[test]
    fn fleet_missions_route_to_expected_target_prompt_modes() {
        let mut app = audit_ready_dash_app();
        app.overlay = ActiveOverlay::FleetList;

        for mission_code in 0..=15 {
            select_first_fleet_row(&mut app);
            app.open_selected_fleet_order_flow();
            app.fleet_overlay.mission_picker_input = mission_code.to_string();

            app.submit_fleet_mission_picker();

            let expected_prompt_mode = match fleet_target_input_kind(Some(mission_code)) {
                nc_engine::FleetTargetInputKind::StarbaseId
                | nc_engine::FleetTargetInputKind::FleetId
                | nc_engine::FleetTargetInputKind::None => FleetOverlayPromptMode::OrderTarget,
                nc_engine::FleetTargetInputKind::Coordinates => {
                    FleetOverlayPromptMode::OrderTargetX
                }
            };
            assert_eq!(
                app.fleet_overlay.prompt_mode, expected_prompt_mode,
                "mission {mission_code} routed to wrong prompt"
            );
        }
    }

    #[test]
    fn coordinate_fleet_order_defaults_match_engine_recommendations() {
        let mut app = audit_ready_dash_app();
        app.overlay = ActiveOverlay::FleetList;

        for mission_code in [0, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 14, 15] {
            select_first_fleet_row(&mut app);
            app.open_selected_fleet_order_flow();
            app.fleet_overlay.mission_picker_input = mission_code.to_string();
            app.submit_fleet_mission_picker();

            let selected_row = app.selected_fleet_order_row().expect("selected fleet row");
            let snapshots = app
                .planet_intel_snapshots
                .iter()
                .cloned()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>();
            let expected_target = recommended_coordinate_target(
                &app.game_data,
                &snapshots,
                app.player_record_index_1_based as u8,
                mission_code,
                selected_row.coords,
                &BTreeSet::new(),
            );

            assert_eq!(
                app.fleet_order_target_x_default_value(),
                expected_target
                    .map(|coords| format!("{:02}", coords[0]))
                    .unwrap_or_default(),
                "mission {mission_code} XX default drifted",
            );

            let expected_y = recommended_coordinate_target_y_for_entered_x(
                &app.game_data,
                &snapshots,
                app.player_record_index_1_based as u8,
                mission_code,
                selected_row.coords,
                &BTreeSet::new(),
                "",
            );
            assert_eq!(
                app.fleet_order_target_y_default_value(),
                expected_y
                    .map(|value| format!("{value:02}"))
                    .unwrap_or_default(),
                "mission {mission_code} YY default drifted",
            );

            app.fleet_overlay.order_target_x_input = app.fleet_order_target_x_default_value();
            let expected_y_for_entered_x = recommended_coordinate_target_y_for_entered_x(
                &app.game_data,
                &snapshots,
                app.player_record_index_1_based as u8,
                mission_code,
                selected_row.coords,
                &BTreeSet::new(),
                &app.fleet_overlay.order_target_x_input,
            );
            assert_eq!(
                app.fleet_order_target_y_default_value(),
                expected_y_for_entered_x
                    .map(|value| format!("{value:02}"))
                    .unwrap_or_default(),
                "mission {mission_code} YY entered-X adaptation drifted",
            );
        }
    }

    #[test]
    fn join_fleet_empty_submission_uses_default_target() {
        let mut app = dash_app_with_store();
        app.overlay = ActiveOverlay::FleetList;
        select_first_fleet_row(&mut app);
        let selected_record = match fleet_list::table_rows(&app)[app.fleet_overlay.selected].key {
            FleetOverlayRowKey::Fleet(record_index) => record_index,
            FleetOverlayRowKey::Starbase(_) => panic!("expected fleet row"),
        };
        app.open_selected_fleet_order_flow();
        app.fleet_overlay.mission_picker_input = Order::JoinAnotherFleet.to_raw().to_string();

        app.submit_fleet_mission_picker();

        let expected_host_fleet_number = app
            .fleet_order_target_default_value()
            .parse::<u16>()
            .expect("default host fleet number");
        app.submit_fleet_order().expect("submit join target");

        let selected_fleet = &app.game_data.fleets.records[selected_record - 1];
        let expected_host = app
            .game_data
            .fleets
            .records
            .iter()
            .find(|fleet| {
                fleet.owner_empire_raw() == 1
                    && fleet.local_slot_word_raw() == expected_host_fleet_number
            })
            .expect("default host fleet");
        assert_eq!(
            selected_fleet.standing_order_kind(),
            Order::JoinAnotherFleet
        );
        assert_eq!(
            selected_fleet.standing_order_target_coords_raw(),
            expected_host.current_location_coords_raw()
        );
    }

    #[test]
    fn guard_starbase_empty_submission_uses_default_target() {
        let mut app = dash_app_with_starbase_store();
        app.overlay = ActiveOverlay::FleetList;
        select_first_fleet_row(&mut app);
        let selected_record = match fleet_list::table_rows(&app)[app.fleet_overlay.selected].key {
            FleetOverlayRowKey::Fleet(record_index) => record_index,
            FleetOverlayRowKey::Starbase(_) => panic!("expected fleet row"),
        };
        app.open_selected_fleet_order_flow();
        app.fleet_overlay.mission_picker_input = Order::GuardStarbase.to_raw().to_string();

        app.submit_fleet_mission_picker();

        let expected_base_id = app
            .fleet_order_target_default_value()
            .parse::<u8>()
            .expect("default starbase id");
        app.submit_fleet_order().expect("submit guard target");

        let selected_fleet = &app.game_data.fleets.records[selected_record - 1];
        let expected_base = app
            .game_data
            .bases
            .records
            .iter()
            .find(|base| base.base_id_raw() == expected_base_id)
            .expect("default starbase");
        assert_eq!(selected_fleet.standing_order_kind(), Order::GuardStarbase);
        assert_eq!(
            selected_fleet.standing_order_target_coords_raw(),
            expected_base.coords_raw()
        );
    }

    #[test]
    fn view_world_empty_coordinate_submission_uses_unknown_intel_default() {
        let mut app = audit_ready_dash_app();
        app.overlay = ActiveOverlay::FleetList;
        select_first_fleet_row(&mut app);
        app.open_selected_fleet_order_flow();
        app.fleet_overlay.mission_picker_input = Order::ViewWorld.to_raw().to_string();

        app.submit_fleet_mission_picker();

        let expected_x = app.fleet_order_target_x_default_value();
        let expected_y = app.fleet_order_target_y_default_value();
        assert!(!expected_x.is_empty());
        assert!(!expected_y.is_empty());

        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.fleet_overlay.order_target_x_input, expected_x);
        assert_eq!(
            app.fleet_overlay.prompt_mode,
            FleetOverlayPromptMode::OrderTargetY
        );

        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.fleet_overlay.order_target_y_input, expected_y);
        assert_eq!(
            app.fleet_overlay.prompt_mode,
            FleetOverlayPromptMode::OrderConfirm
        );
    }

    #[test]
    fn view_world_falls_back_to_closest_non_owned_world_when_unknowns_are_exhausted() {
        let mut app = audit_ready_dash_app();
        for snapshot in &mut app.planet_intel_snapshots {
            if snapshot.known_owner_empire_id == Some(1) {
                continue;
            }
            snapshot.intel_tier = IntelTier::Partial;
            if snapshot.known_owner_empire_id.is_none() {
                snapshot.known_owner_empire_id = Some(0);
            }
        }
        app.overlay = ActiveOverlay::FleetList;
        select_first_fleet_row(&mut app);
        app.open_selected_fleet_order_flow();
        app.fleet_overlay.mission_picker_input = Order::ViewWorld.to_raw().to_string();

        app.submit_fleet_mission_picker();

        let selected_row = app.selected_fleet_order_row().expect("selected fleet row");
        let snapshots = app
            .planet_intel_snapshots
            .iter()
            .cloned()
            .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
            .collect::<BTreeMap<_, _>>();
        let expected_target = recommended_coordinate_target(
            &app.game_data,
            &snapshots,
            app.player_record_index_1_based as u8,
            Order::ViewWorld.to_raw(),
            selected_row.coords,
            &BTreeSet::new(),
        )
        .expect("fallback target");

        assert_eq!(
            app.fleet_order_target_x_default_value(),
            format!("{:02}", expected_target[0])
        );
        assert_eq!(
            app.fleet_order_target_y_default_value(),
            format!("{:02}", expected_target[1])
        );
    }

    #[test]
    fn fleet_selection_toggles_on_space() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::FleetList;
        let rows = fleet_list::table_rows(&app);
        let record_index = match rows[0].key {
            FleetOverlayRowKey::Fleet(record_index) => record_index,
            FleetOverlayRowKey::Starbase(_) => panic!("expected fleet row"),
        };

        app.handle_key(key(KeyCode::Char(' ')));
        assert!(
            app.fleet_overlay
                .selected_fleet_record_indexes
                .contains(&record_index)
        );

        app.handle_key(key(KeyCode::Char(' ')));
        assert!(app.fleet_overlay.selected_fleet_record_indexes.is_empty());
    }

    #[test]
    fn fleet_sort_preserves_checked_selection() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::FleetList;
        let rows = fleet_list::table_rows(&app);
        let record_index = match rows[0].key {
            FleetOverlayRowKey::Fleet(record_index) => record_index,
            FleetOverlayRowKey::Starbase(_) => panic!("expected fleet row"),
        };
        app.handle_key(key(KeyCode::Char(' ')));

        app.apply_fleet_overlay_sort(crate::dashboard::app::state::FleetOverlaySort::Eta);

        assert!(
            app.fleet_overlay
                .selected_fleet_record_indexes
                .contains(&record_index)
        );
    }

    #[test]
    fn fleet_sort_repeated_selection_toggles_direction() {
        let mut app = dash_app();

        assert_eq!(app.fleet_overlay.sort_direction, SortDirection::Desc);

        app.apply_fleet_overlay_sort(crate::dashboard::app::state::FleetOverlaySort::Id);
        assert_eq!(app.fleet_overlay.sort_direction, SortDirection::Asc);

        app.apply_fleet_overlay_sort(crate::dashboard::app::state::FleetOverlaySort::Id);
        assert_eq!(app.fleet_overlay.sort_direction, SortDirection::Desc);
    }

    #[test]
    fn fleet_sort_new_key_resets_default_direction() {
        let mut app = dash_app();
        app.fleet_overlay.sort_direction = SortDirection::Asc;

        app.apply_fleet_overlay_sort(crate::dashboard::app::state::FleetOverlaySort::Strength);

        assert_eq!(
            app.fleet_overlay.sort,
            crate::dashboard::app::state::FleetOverlaySort::Strength
        );
        assert_eq!(app.fleet_overlay.sort_direction, SortDirection::Desc);
    }

    #[test]
    fn intel_range_sort_same_anchor_toggles_direction() {
        let mut app = dash_app();
        let anchor = [8, 8];

        app.apply_intel_overlay_sort(crate::dashboard::app::state::IntelOverlaySort::Range(
            anchor,
        ));
        assert_eq!(app.intel_overlay.sort_direction, SortDirection::Asc);

        app.apply_intel_overlay_sort(crate::dashboard::app::state::IntelOverlaySort::Range(
            anchor,
        ));
        assert_eq!(app.intel_overlay.sort_direction, SortDirection::Desc);

        app.apply_intel_overlay_sort(crate::dashboard::app::state::IntelOverlaySort::Range([
            9, 9,
        ]));
        assert_eq!(app.intel_overlay.sort_direction, SortDirection::Asc);
    }

    #[test]
    fn fleet_filter_clears_checked_selection() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::FleetList;
        app.handle_key(key(KeyCode::Char(' ')));

        app.apply_fleet_overlay_filter(FleetOverlayFilter::Combat);

        assert!(app.fleet_overlay.selected_fleet_record_indexes.is_empty());
    }

    #[test]
    fn fleet_list_excludes_starbases() {
        let mut app = dash_app_with_starbase();
        app.overlay = ActiveOverlay::FleetList;
        assert!(
            fleet_list::table_rows(&app)
                .iter()
                .all(|row| !matches!(row.key, FleetOverlayRowKey::Starbase(_)))
        );
    }

    #[test]
    fn checked_fleets_open_group_order_flow() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::FleetList;
        app.handle_key(key(KeyCode::Char(' ')));
        app.fleet_overlay.selected = 1;
        app.handle_key(key(KeyCode::Char(' ')));
        app.fleet_overlay.selected = 0;

        app.handle_key(key(KeyCode::Char('o')));

        assert_eq!(app.fleet_overlay.order_scope, FleetOrderScope::Group);
        assert_eq!(
            app.fleet_overlay.prompt_mode,
            FleetOverlayPromptMode::MissionPicker
        );
    }

    #[test]
    fn checked_fleets_still_open_group_order_flow_when_starbases_exist_in_game() {
        let mut app = dash_app_with_starbase();
        app.overlay = ActiveOverlay::FleetList;
        app.fleet_overlay.selected = fleet_list::table_rows(&app)
            .iter()
            .position(|row| matches!(row.key, FleetOverlayRowKey::Fleet(_)))
            .expect("fleet row");
        app.handle_key(key(KeyCode::Char(' ')));

        app.handle_key(key(KeyCode::Char('o')));

        assert_eq!(app.fleet_overlay.order_scope, FleetOrderScope::Group);
        assert_eq!(
            app.fleet_overlay.prompt_mode,
            FleetOverlayPromptMode::MissionPicker
        );
        assert_eq!(app.fleet_overlay.selected_fleet_record_indexes.len(), 1);
    }

    #[test]
    fn group_order_success_clears_checked_selection_and_unwinds_prompts() {
        let mut app = dash_app_with_store();
        app.overlay = ActiveOverlay::FleetList;
        let selected_records = select_first_two_fleet_rows(&mut app);

        app.open_selected_fleet_order_flow();
        app.fleet_overlay.mission_picker_input = Order::MoveOnly.to_raw().to_string();
        app.submit_fleet_mission_picker();
        app.fleet_overlay.order_target_x_input = "10".to_string();
        app.submit_fleet_order().expect("submit x");
        app.fleet_overlay.order_target_y_input = "10".to_string();
        app.submit_fleet_order().expect("submit y");
        app.fleet_overlay.order_confirm_input = "Y".to_string();
        app.submit_fleet_order().expect("submit confirm");

        assert!(app.fleet_overlay.selected_fleet_record_indexes.is_empty());
        assert_eq!(app.fleet_overlay.prompt_mode, FleetOverlayPromptMode::None);
        assert_eq!(app.fleet_overlay.order_scope, FleetOrderScope::None);
        for record_index in selected_records {
            let fleet = &app.game_data.fleets.records[record_index - 1];
            assert_eq!(fleet.standing_order_kind(), Order::MoveOnly);
            assert_eq!(fleet.standing_order_target_coords_raw(), [10, 10]);
        }
    }

    #[test]
    fn fleet_order_confirm_y_submits_without_enter() {
        let mut app = dash_app_with_store();
        app.overlay = ActiveOverlay::FleetList;

        app.open_selected_fleet_order_flow();
        let selected_record_index = app
            .selected_fleet_order_row()
            .expect("selected fleet row")
            .fleet_record_index_1_based;
        app.fleet_overlay.mission_picker_input = Order::MoveOnly.to_raw().to_string();
        app.submit_fleet_mission_picker();
        app.fleet_overlay.order_target_x_input = "10".to_string();
        app.submit_fleet_order().expect("submit x");
        app.fleet_overlay.order_target_y_input = "10".to_string();
        app.submit_fleet_order().expect("submit y");

        app.handle_key(key(KeyCode::Char('y')));

        assert_eq!(app.fleet_overlay.prompt_mode, FleetOverlayPromptMode::None);
        let fleet = &app.game_data.fleets.records[selected_record_index - 1];
        assert_eq!(fleet.standing_order_kind(), Order::MoveOnly);
        assert_eq!(fleet.standing_order_target_coords_raw(), [10, 10]);
    }

    #[test]
    fn backing_out_of_group_order_keeps_checked_selection() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::FleetList;
        let selected_records = select_first_two_fleet_rows(&mut app);

        app.open_selected_fleet_order_flow();
        app.fleet_overlay.mission_picker_input = Order::MoveOnly.to_raw().to_string();
        app.submit_fleet_mission_picker();
        app.handle_key(key(KeyCode::Esc));

        assert_eq!(
            app.fleet_overlay.prompt_mode,
            FleetOverlayPromptMode::MissionPicker
        );
        assert_eq!(
            app.fleet_overlay.selected_fleet_record_indexes,
            selected_records.into_iter().collect()
        );
    }

    #[test]
    fn checked_change_applies_roe_to_all_checked_fleets() {
        let mut app = dash_app_with_store();
        app.overlay = ActiveOverlay::FleetList;
        let selected_records = select_first_two_fleet_rows(&mut app);

        app.handle_key(key(KeyCode::Char('c')));
        assert_eq!(
            app.fleet_overlay.prompt_mode,
            FleetOverlayPromptMode::ChangeField
        );

        app.handle_key(key(KeyCode::Char('r')));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(
            app.fleet_overlay.prompt_mode,
            FleetOverlayPromptMode::ChangeValue
        );

        app.handle_key(key(KeyCode::Char('4')));
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.fleet_overlay.prompt_mode, FleetOverlayPromptMode::None);
        assert!(app.fleet_overlay.selected_fleet_record_indexes.is_empty());
        for record_index in selected_records {
            let fleet = &app.game_data.fleets.records[record_index - 1];
            assert_eq!(fleet.rules_of_engagement(), 4);
        }
    }

    #[test]
    fn checked_change_clears_only_successful_fleets_on_partial_roe_update() {
        let mut app = dash_app_with_store();
        app.overlay = ActiveOverlay::FleetList;
        let selected_records = select_first_two_fleet_rows(&mut app);
        let combat_record = selected_records[0];
        let support_record = selected_records[1];

        {
            let combat = &mut app.game_data.fleets.records[combat_record - 1];
            combat.set_destroyer_count(1);
            combat.set_cruiser_count(0);
            combat.set_battleship_count(0);
            combat.set_scout_count(0);
            combat.set_troop_transport_count(0);
            combat.set_army_count(0);
            combat.set_etac_count(0);
            combat.recompute_max_speed_from_composition();
            combat.set_rules_of_engagement(0);

            let support = &mut app.game_data.fleets.records[support_record - 1];
            support.set_destroyer_count(0);
            support.set_cruiser_count(0);
            support.set_battleship_count(0);
            support.set_scout_count(0);
            support.set_troop_transport_count(1);
            support.set_army_count(1);
            support.set_etac_count(0);
            support.recompute_max_speed_from_composition();
            support.set_rules_of_engagement(0);
        }

        app.handle_key(key(KeyCode::Char('c')));
        app.handle_key(key(KeyCode::Char('r')));
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Char('6')));
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(
            app.fleet_overlay.prompt_mode,
            FleetOverlayPromptMode::ChangeValue
        );
        assert_eq!(
            app.fleet_overlay.selected_fleet_record_indexes,
            [support_record].into_iter().collect()
        );
        assert_eq!(
            app.game_data.fleets.records[combat_record - 1].rules_of_engagement(),
            6
        );
        assert_eq!(
            app.game_data.fleets.records[support_record - 1].rules_of_engagement(),
            0
        );
        assert!(app.fleet_overlay.aux_status.is_some());
    }

    #[test]
    fn checked_merge_uses_lowest_numbered_host() {
        let mut app = dash_app_with_store();
        app.overlay = ActiveOverlay::FleetList;
        let selected_records = select_first_two_fleet_rows(&mut app);
        let mut selected_fleets = selected_records
            .iter()
            .map(|record_index| {
                let fleet = &app.game_data.fleets.records[*record_index - 1];
                (
                    *record_index,
                    fleet.local_slot_word_raw(),
                    fleet.current_location_coords_raw(),
                )
            })
            .collect::<Vec<_>>();
        selected_fleets.sort_by_key(|(_, fleet_number, _)| *fleet_number);
        let host_record_index = selected_fleets[0].0;
        let host_coords = selected_fleets[0].2;

        app.handle_key(key(KeyCode::Char('m')));
        assert_eq!(
            app.fleet_overlay.prompt_mode,
            FleetOverlayPromptMode::MergeConfirm
        );

        app.handle_key(key(KeyCode::Char('y')));
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.fleet_overlay.prompt_mode, FleetOverlayPromptMode::None);
        assert!(app.fleet_overlay.selected_fleet_record_indexes.is_empty());
        for (record_index, _, _) in selected_fleets.into_iter().skip(1) {
            let fleet = &app.game_data.fleets.records[record_index - 1];
            assert_eq!(fleet.standing_order_kind(), Order::JoinAnotherFleet);
            assert_eq!(fleet.standing_order_target_coords_raw(), host_coords);
        }
        let host = &app.game_data.fleets.records[host_record_index - 1];
        assert_ne!(host.standing_order_kind(), Order::JoinAnotherFleet);
    }

    #[test]
    fn checked_transfer_uses_highlighted_checked_fleet_as_donor() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::FleetList;
        let selected_records = select_first_two_fleet_rows(&mut app);
        let donor_record_index = selected_records[1];
        app.fleet_overlay.selected = fleet_list::table_rows(&app)
            .iter()
            .position(|row| row.key == FleetOverlayRowKey::Fleet(donor_record_index))
            .expect("selected donor row");

        app.handle_key(key(KeyCode::Char('t')));

        assert_eq!(
            app.fleet_overlay.prompt_mode,
            FleetOverlayPromptMode::TransferStage
        );
        assert_eq!(
            app.fleet_overlay.transfer_donor_record_index_1_based,
            Some(donor_record_index)
        );
        assert_eq!(
            app.fleet_overlay.transfer_host_record_index_1_based,
            Some(selected_records[0])
        );
    }

    #[test]
    fn guard_starbase_mission_reports_specific_unavailable_target_message() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::FleetList;
        app.open_selected_fleet_order_flow();
        app.fleet_overlay.mission_picker_input = Order::GuardStarbase.to_raw().to_string();

        app.submit_fleet_mission_picker();

        assert_eq!(
            app.fleet_overlay.mission_picker_status.as_deref(),
            Some("You have no starbases available to guard.")
        );
    }

    #[test]
    fn join_fleet_mission_reports_specific_unavailable_target_message() {
        let mut app = dash_app();
        app.game_data.fleets.records.truncate(1);
        app.overlay = ActiveOverlay::FleetList;
        app.open_selected_fleet_order_flow();
        app.fleet_overlay.mission_picker_input = Order::JoinAnotherFleet.to_raw().to_string();

        app.submit_fleet_mission_picker();

        assert_eq!(
            app.fleet_overlay.mission_picker_status.as_deref(),
            Some("You need another fleet available to join.")
        );
    }

    #[test]
    fn fleet_order_default_target_ignores_existing_target_coords_shortcut() {
        let mut baseline = dash_app();
        baseline.overlay = ActiveOverlay::FleetList;
        baseline.open_selected_fleet_order_flow();
        baseline.fleet_overlay.order_mission_code = Some(Order::MoveOnly.to_raw());
        baseline.fleet_overlay.prompt_mode = FleetOverlayPromptMode::OrderTargetX;
        let baseline_footer = render_fleet_footer_line(&baseline, "COMMAND <- Target XX ");

        let mut stale_target = dash_app();
        let selected_record =
            match fleet_list::table_rows(&stale_target)[stale_target.fleet_overlay.selected].key {
                FleetOverlayRowKey::Fleet(record_index) => record_index,
                FleetOverlayRowKey::Starbase(_) => panic!("expected fleet row"),
            };
        stale_target.game_data.fleets.records[selected_record - 1]
            .set_standing_order_target_coords_raw([18, 18]);
        stale_target.overlay = ActiveOverlay::FleetList;
        stale_target.open_selected_fleet_order_flow();
        stale_target.fleet_overlay.order_mission_code = Some(Order::MoveOnly.to_raw());
        stale_target.fleet_overlay.prompt_mode = FleetOverlayPromptMode::OrderTargetX;
        let stale_footer = render_fleet_footer_line(&stale_target, "COMMAND <- Target XX ");

        assert_eq!(stale_footer, baseline_footer);
    }

    #[test]
    fn nested_intel_filter_modals_unwind_one_level_at_a_time() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::IntelDatabase;

        app.handle_key(key(KeyCode::Char('f')));
        app.handle_key(key(KeyCode::Char('c')));
        app.handle_key(key(KeyCode::Char('o')));
        app.handle_key(key(KeyCode::Char('o')));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(
            app.intel_overlay.prompt_mode,
            IntelOverlayPromptMode::FilterValueInput
        );

        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.overlay, ActiveOverlay::IntelDatabase);
        assert_eq!(app.intel_overlay.prompt_mode, IntelOverlayPromptMode::None);
    }

    #[test]
    fn fleet_filter_prompt_accepts_unique_prefix_and_reports_ambiguity_inline() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::FleetList;

        app.handle_key(key(KeyCode::Char('f')));
        app.handle_key(key(KeyCode::Char('s')));
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.overlay, ActiveOverlay::FleetList);
        assert_eq!(
            app.fleet_overlay.prompt_mode,
            FleetOverlayPromptMode::FilterMenu
        );
        assert_eq!(
            app.fleet_overlay.filter_prompt_status.as_deref(),
            Some(" Ambiguous: sel/shi/spd")
        );
        assert!(
            render_fleet_footer_line(&app, "Ambiguous: sel/shi/spd")
                .contains("COMMAND <-  Ambiguous: sel/shi/spd")
        );

        app.handle_key(key(KeyCode::Char('p')));
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(
            app.fleet_overlay.prompt_mode,
            FleetOverlayPromptMode::FilterValueInput
        );
        assert_eq!(
            app.fleet_overlay
                .pending_filter_column
                .expect("pending column")
                .code,
            "spd"
        );
    }

    #[test]
    fn intel_filter_prompt_accepts_unique_prefix_and_reports_ambiguity_inline() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::IntelDatabase;

        app.handle_key(key(KeyCode::Char('f')));
        app.handle_key(key(KeyCode::Char('s')));
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.overlay, ActiveOverlay::IntelDatabase);
        assert_eq!(
            app.intel_overlay.prompt_mode,
            IntelOverlayPromptMode::FilterMenu
        );
        assert_eq!(
            app.intel_overlay.prompt_status.as_deref(),
            Some(" Ambiguous: sbs/sco/see")
        );
        assert!(
            render_intel_footer_line(&app, "Ambiguous: sbs/sco/see")
                .contains("COMMAND <-  Ambiguous: sbs/sco/see")
        );

        app.handle_key(key(KeyCode::Char('c')));
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(
            app.intel_overlay.prompt_mode,
            IntelOverlayPromptMode::FilterValueInput
        );
        assert_eq!(
            app.intel_overlay
                .pending_filter_column
                .expect("pending column")
                .code,
            "sco"
        );
    }

    #[test]
    fn empty_fleet_filter_clause_resets_to_all() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::FleetList;

        app.handle_key(key(KeyCode::Char('f')));
        for ch in ['o', 'r', 'd'] {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));
        for ch in ['z', 'z', 'z', 'z'] {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.overlay, ActiveOverlay::FleetList);
        assert!(app.fleet_overlay.filter_clause.is_none());
        assert!(!fleet_list::table_rows(&app).is_empty());
    }

    #[test]
    fn empty_planet_filter_clause_resets_to_all() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::PlanetList;

        app.handle_key(key(KeyCode::Char('f')));
        for ch in ['p', 'l', 'a'] {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));
        for ch in ['z', 'z', 'z', 'z'] {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.overlay, ActiveOverlay::PlanetList);
        assert!(app.planet_overlay.filter_clause.is_none());
        assert!(!planet_list::table_rows(&app).is_empty());
    }

    #[test]
    fn empty_intel_filter_clause_resets_to_all() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::IntelDatabase;

        app.handle_key(key(KeyCode::Char('f')));
        for ch in ['p', 'l', 'a'] {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));
        for ch in ['z', 'z', 'z', 'z'] {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.overlay, ActiveOverlay::IntelDatabase);
        assert!(app.intel_overlay.filter_clause.is_none());
        assert!(!intel_database::table_rows(&app).is_empty());
    }

    #[test]
    fn stale_fleet_filter_clause_resets_to_all_after_rows_change() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::FleetList;

        let owned_fleet_indexes = app
            .game_data
            .fleets
            .records
            .iter()
            .enumerate()
            .filter_map(|(idx, fleet)| (fleet.owner_empire_raw() == 1).then_some(idx))
            .collect::<Vec<_>>();
        assert!(
            !owned_fleet_indexes.is_empty(),
            "fixture should have owned fleets"
        );
        let target_fleet_number =
            app.game_data.fleets.records[owned_fleet_indexes[0]].local_slot_word_raw();

        app.handle_key(key(KeyCode::Char('f')));
        for ch in ['i', 'd'] {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));
        for ch in target_fleet_number.to_string().chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));
        assert!(app.fleet_overlay.filter_clause.is_some());

        app.game_data.fleets.records[owned_fleet_indexes[0]].set_owner_empire_raw(0);
        app.handle_key(key(KeyCode::Down));

        assert!(app.fleet_overlay.filter_clause.is_none());
        assert!(!fleet_list::table_rows(&app).is_empty());
        assert!(
            render_fleet_title_line(&app, "FLEET LIST:").contains("FLEET LIST: ID DESCENDING ALL")
        );
    }

    #[test]
    fn stale_planet_filter_clause_resets_to_all_after_rows_change() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::PlanetList;

        let owned_planet_indexes = app
            .game_data
            .planets
            .records
            .iter()
            .enumerate()
            .filter_map(|(idx, planet)| (planet.owner_empire_slot_raw() == 1).then_some(idx))
            .collect::<Vec<_>>();
        let target_planet_idx = *owned_planet_indexes.first().expect("owned planet exists");
        for &idx in &owned_planet_indexes {
            app.game_data.planets.records[idx].set_army_count_raw(0);
        }
        app.game_data.planets.records[target_planet_idx].set_army_count_raw(5);

        app.handle_key(key(KeyCode::Char('f')));
        for ch in ['a', 'r', 's'] {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));
        for ch in "5".chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));
        assert!(app.planet_overlay.filter_clause.is_some());

        app.game_data.planets.records[target_planet_idx].set_army_count_raw(0);
        app.handle_key(key(KeyCode::Down));

        assert!(app.planet_overlay.filter_clause.is_none());
        assert!(!planet_list::table_rows(&app).is_empty());
        assert!(
            render_planet_title_line(&app, "PLANET LIST:")
                .contains("PLANET LIST: CUR DESCENDING ALL")
        );
    }

    #[test]
    fn stale_intel_filter_clause_resets_to_all_after_rows_change() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::IntelDatabase;

        for planet in &mut app.game_data.planets.records {
            planet.set_army_count_raw(0);
        }
        let target_planet_idx = app
            .game_data
            .planets
            .records
            .iter()
            .position(|planet| planet.owner_empire_slot_raw() == 1)
            .expect("owned planet exists");
        app.game_data.planets.records[target_planet_idx].set_army_count_raw(7);

        app.handle_key(key(KeyCode::Char('f')));
        for ch in ['a', 'r', 's'] {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));
        for ch in "7".chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));
        assert!(app.intel_overlay.filter_clause.is_some());

        app.game_data.planets.records[target_planet_idx].set_army_count_raw(0);
        app.handle_key(key(KeyCode::Down));

        assert!(app.intel_overlay.filter_clause.is_none());
        assert!(!intel_database::table_rows(&app).is_empty());
        assert!(
            render_intel_title_line(&app, "TOTAL PLANET DATABASE:")
                .contains("TOTAL PLANET DATABASE: COO ASCENDING ALL")
        );
    }

    #[test]
    fn fleet_sort_prompt_accepts_every_column_code() {
        let cases = [
            ("id", FleetOverlaySort::Id, SortDirection::Asc, "ID"),
            (
                "sel",
                FleetOverlaySort::Selected,
                SortDirection::Desc,
                "SEL",
            ),
            ("loc", FleetOverlaySort::Location, SortDirection::Asc, "LOC"),
            ("ord", FleetOverlaySort::Order, SortDirection::Asc, "ORD"),
            ("tar", FleetOverlaySort::Target, SortDirection::Asc, "TAR"),
            ("spd", FleetOverlaySort::Speed, SortDirection::Desc, "SPD"),
            ("eta", FleetOverlaySort::Eta, SortDirection::Asc, "ETA"),
            ("roe", FleetOverlaySort::Roe, SortDirection::Desc, "ROE"),
            ("ars", FleetOverlaySort::Armies, SortDirection::Desc, "ARS"),
            (
                "shi",
                FleetOverlaySort::Strength,
                SortDirection::Desc,
                "SHI",
            ),
        ];

        for (code, expected_sort, expected_direction, expected_label) in cases {
            let mut app = dash_app();
            app.overlay = ActiveOverlay::FleetList;
            app.handle_key(key(KeyCode::Char('s')));
            assert_eq!(
                app.fleet_overlay.prompt_mode,
                FleetOverlayPromptMode::SortMenu
            );
            for ch in code.chars() {
                app.handle_key(key(KeyCode::Char(ch)));
            }
            app.handle_key(key(KeyCode::Enter));

            assert_eq!(app.fleet_overlay.prompt_mode, FleetOverlayPromptMode::None);
            assert_eq!(app.fleet_overlay.sort, expected_sort);
            assert_eq!(app.fleet_overlay.sort_direction, expected_direction);

            let title = render_fleet_title_line(&app, "FLEET LIST:");
            assert!(title.contains(expected_label), "{title}");
            assert!(
                title.contains(match expected_direction {
                    SortDirection::Asc => "ASCENDING",
                    SortDirection::Desc => "DESCENDING",
                }),
                "{title}"
            );
        }
    }

    #[test]
    fn planet_sort_prompt_accepts_every_column_code() {
        let cases = [
            (
                "coo",
                PlanetOverlaySort::Location,
                SortDirection::Asc,
                "COO",
            ),
            (
                "pla",
                PlanetOverlaySort::PlanetName,
                SortDirection::Asc,
                "PLA",
            ),
            (
                "max",
                PlanetOverlaySort::MaxProduction,
                SortDirection::Desc,
                "MAX",
            ),
            (
                "cur",
                PlanetOverlaySort::CurrentProduction,
                SortDirection::Asc,
                "CUR",
            ),
            (
                "trs",
                PlanetOverlaySort::Treasury,
                SortDirection::Desc,
                "TRS",
            ),
            ("bdg", PlanetOverlaySort::Budget, SortDirection::Desc, "BDG"),
            (
                "rev",
                PlanetOverlaySort::Revenue,
                SortDirection::Desc,
                "REV",
            ),
            ("gro", PlanetOverlaySort::Growth, SortDirection::Desc, "GRO"),
            (
                "bui",
                PlanetOverlaySort::BuildQueue,
                SortDirection::Desc,
                "BUI",
            ),
            (
                "sta",
                PlanetOverlaySort::Stardock,
                SortDirection::Desc,
                "STA",
            ),
            (
                "sbs",
                PlanetOverlaySort::Starbase,
                SortDirection::Desc,
                "SBS",
            ),
            ("ars", PlanetOverlaySort::Armies, SortDirection::Desc, "ARS"),
            (
                "gbs",
                PlanetOverlaySort::Batteries,
                SortDirection::Desc,
                "GBS",
            ),
        ];

        for (code, expected_sort, expected_direction, expected_label) in cases {
            let mut app = dash_app();
            app.overlay = ActiveOverlay::PlanetList;
            app.handle_key(key(KeyCode::Char('s')));
            assert_eq!(
                app.planet_overlay.prompt_mode,
                PlanetOverlayPromptMode::SortMenu
            );
            for ch in code.chars() {
                app.handle_key(key(KeyCode::Char(ch)));
            }
            app.handle_key(key(KeyCode::Enter));

            assert_eq!(
                app.planet_overlay.prompt_mode,
                PlanetOverlayPromptMode::None
            );
            assert_eq!(app.planet_overlay.sort, expected_sort);
            assert_eq!(app.planet_overlay.sort_direction, expected_direction);

            let title = render_planet_title_line(&app, "PLANET LIST:");
            assert!(title.contains(expected_label), "{title}");
            assert!(
                title.contains(match expected_direction {
                    SortDirection::Asc => "ASCENDING",
                    SortDirection::Desc => "DESCENDING",
                }),
                "{title}"
            );
        }
    }

    #[test]
    fn intel_sort_prompt_accepts_every_column_code() {
        let cases = [
            (
                "coo",
                IntelOverlaySort::Location,
                SortDirection::Desc,
                "COO",
            ),
            (
                "pla",
                IntelOverlaySort::PlanetName,
                SortDirection::Asc,
                "PLA",
            ),
            ("own", IntelOverlaySort::Owner, SortDirection::Asc, "OWN"),
            (
                "max",
                IntelOverlaySort::MaxProduction,
                SortDirection::Desc,
                "MAX",
            ),
            (
                "see",
                IntelOverlaySort::YearSeen,
                SortDirection::Desc,
                "SEE",
            ),
            ("ars", IntelOverlaySort::Armies, SortDirection::Desc, "ARS"),
            (
                "gbs",
                IntelOverlaySort::Batteries,
                SortDirection::Desc,
                "GBS",
            ),
            (
                "sbs",
                IntelOverlaySort::Starbases,
                SortDirection::Desc,
                "SBS",
            ),
            (
                "cur",
                IntelOverlaySort::CurrentProduction,
                SortDirection::Desc,
                "CUR",
            ),
            (
                "trs",
                IntelOverlaySort::Treasury,
                SortDirection::Desc,
                "TRS",
            ),
            (
                "sco",
                IntelOverlaySort::ScoutYear,
                SortDirection::Desc,
                "SCO",
            ),
        ];

        for (code, expected_sort, expected_direction, expected_label) in cases {
            let mut app = dash_app();
            app.overlay = ActiveOverlay::IntelDatabase;
            app.handle_key(key(KeyCode::Char('s')));
            assert_eq!(
                app.intel_overlay.prompt_mode,
                IntelOverlayPromptMode::SortMenu
            );
            for ch in code.chars() {
                app.handle_key(key(KeyCode::Char(ch)));
            }
            app.handle_key(key(KeyCode::Enter));

            assert_eq!(app.intel_overlay.prompt_mode, IntelOverlayPromptMode::None);
            assert_eq!(app.intel_overlay.sort, expected_sort);
            assert_eq!(app.intel_overlay.sort_direction, expected_direction);

            let title = render_intel_title_line(&app, "TOTAL PLANET DATABASE:");
            assert!(title.contains(expected_label), "{title}");
            assert!(
                title.contains(match expected_direction {
                    SortDirection::Asc => "ASCENDING",
                    SortDirection::Desc => "DESCENDING",
                }),
                "{title}"
            );
        }
    }

    #[test]
    fn fleet_filter_prompt_accepts_every_appendix_e_column_code() {
        let codes = [
            "id", "loc", "ord", "tar", "spd", "eta", "roe", "ars", "shi", "sel",
        ];

        for code in codes {
            let mut app = dash_app();
            app.overlay = ActiveOverlay::FleetList;
            app.handle_key(key(KeyCode::Char('f')));
            assert_eq!(
                app.fleet_overlay.prompt_mode,
                FleetOverlayPromptMode::FilterMenu
            );
            for ch in code.chars() {
                app.handle_key(key(KeyCode::Char(ch)));
            }
            app.handle_key(key(KeyCode::Enter));

            assert_eq!(
                app.fleet_overlay.prompt_mode,
                FleetOverlayPromptMode::FilterValueInput
            );
            assert_eq!(
                app.fleet_overlay
                    .pending_filter_column
                    .map(|column| column.code),
                Some(code)
            );
        }
    }

    #[test]
    fn planet_filter_prompt_accepts_every_appendix_e_column_code() {
        let codes = [
            "coo", "pla", "max", "cur", "trs", "bdg", "rev", "gro", "bui", "sta", "sbs", "ars",
            "gbs",
        ];

        for code in codes {
            let mut app = dash_app();
            app.overlay = ActiveOverlay::PlanetList;
            app.handle_key(key(KeyCode::Char('f')));
            assert_eq!(
                app.planet_overlay.prompt_mode,
                PlanetOverlayPromptMode::FilterMenu
            );
            for ch in code.chars() {
                app.handle_key(key(KeyCode::Char(ch)));
            }
            app.handle_key(key(KeyCode::Enter));

            assert_eq!(
                app.planet_overlay.prompt_mode,
                PlanetOverlayPromptMode::FilterValueInput
            );
            assert_eq!(
                app.planet_overlay
                    .pending_filter_column
                    .map(|column| column.code),
                Some(code)
            );
        }
    }

    #[test]
    fn intel_filter_prompt_accepts_every_appendix_e_column_code() {
        let codes = [
            "coo", "pla", "own", "max", "see", "ars", "gbs", "sbs", "cur", "trs", "sco",
        ];

        for code in codes {
            let mut app = dash_app();
            app.overlay = ActiveOverlay::IntelDatabase;
            app.handle_key(key(KeyCode::Char('f')));
            assert_eq!(
                app.intel_overlay.prompt_mode,
                IntelOverlayPromptMode::FilterMenu
            );
            for ch in code.chars() {
                app.handle_key(key(KeyCode::Char(ch)));
            }
            app.handle_key(key(KeyCode::Enter));

            assert_eq!(
                app.intel_overlay.prompt_mode,
                IntelOverlayPromptMode::FilterValueInput
            );
            assert_eq!(
                app.intel_overlay
                    .pending_filter_column
                    .map(|column| column.code),
                Some(code)
            );
        }
    }

    #[test]
    fn sort_prompts_accept_natural_column_names() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::FleetList;
        app.handle_key(key(KeyCode::Char('s')));
        for ch in "speed".chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.fleet_overlay.sort, FleetOverlaySort::Speed);

        let mut app = dash_app();
        app.overlay = ActiveOverlay::PlanetList;
        app.handle_key(key(KeyCode::Char('s')));
        for ch in "dock".chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.planet_overlay.sort, PlanetOverlaySort::Stardock);

        let mut app = dash_app();
        app.overlay = ActiveOverlay::PlanetList;
        app.handle_key(key(KeyCode::Char('s')));
        for ch in "treasury points".chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.planet_overlay.sort, PlanetOverlaySort::Treasury);

        let mut app = dash_app();
        app.overlay = ActiveOverlay::PlanetList;
        app.handle_key(key(KeyCode::Char('s')));
        for ch in "bgdt".chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.planet_overlay.sort, PlanetOverlaySort::Budget);

        let mut app = dash_app();
        app.overlay = ActiveOverlay::IntelDatabase;
        app.handle_key(key(KeyCode::Char('s')));
        for ch in "year".chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.intel_overlay.sort, IntelOverlaySort::YearSeen);
    }

    #[test]
    fn filter_prompts_accept_natural_column_names() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::FleetList;
        app.handle_key(key(KeyCode::Char('f')));
        for ch in "speed".chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(
            app.fleet_overlay
                .pending_filter_column
                .map(|column| column.code),
            Some("spd")
        );

        let mut app = dash_app();
        app.overlay = ActiveOverlay::PlanetList;
        app.handle_key(key(KeyCode::Char('f')));
        for ch in "dock".chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(
            app.planet_overlay
                .pending_filter_column
                .map(|column| column.code),
            Some("sta")
        );

        let mut app = dash_app();
        app.overlay = ActiveOverlay::PlanetList;
        app.handle_key(key(KeyCode::Char('f')));
        for ch in "treasury points".chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(
            app.planet_overlay
                .pending_filter_column
                .map(|column| column.code),
            Some("trs")
        );

        let mut app = dash_app();
        app.overlay = ActiveOverlay::PlanetList;
        app.handle_key(key(KeyCode::Char('f')));
        for ch in "bgdt".chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(
            app.planet_overlay
                .pending_filter_column
                .map(|column| column.code),
            Some("bdg")
        );

        let mut app = dash_app();
        app.overlay = ActiveOverlay::IntelDatabase;
        app.handle_key(key(KeyCode::Char('f')));
        for ch in "year".chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(
            app.intel_overlay
                .pending_filter_column
                .map(|column| column.code),
            Some("see")
        );
    }

    #[test]
    fn dragging_top_level_overlay_updates_position() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::Inbox;
        let map_frame = dashboard_layout(&app).widgets.center_map;
        let popup = app
            .current_overlay_popup_rect(map_frame)
            .expect("inbox popup rect");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            popup.x + 2,
            popup.y,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            popup.x + 9,
            popup.y + 4,
        ));

        assert!(app.overlay_position.is_some());

        let moved_popup = app
            .current_overlay_popup_rect(map_frame)
            .expect("moved popup rect");
        assert!(moved_popup.x > popup.x);
        assert!(moved_popup.y > popup.y);
    }

    #[test]
    fn dragging_overlay_from_bottom_border_updates_position() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::Inbox;
        let map_frame = dashboard_layout(&app).widgets.center_map;
        let popup = app
            .current_overlay_popup_rect(map_frame)
            .expect("inbox popup rect");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            popup.x + 4,
            popup.y + popup.height.saturating_sub(1),
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            popup.x + 9,
            popup.y + popup.height + 3,
        ));

        let moved_popup = app
            .current_overlay_popup_rect(map_frame)
            .expect("moved popup rect");
        assert!(moved_popup.y > popup.y);
    }

    #[test]
    fn dragging_overlay_can_move_into_left_column() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::Inbox;
        let widgets = dashboard_layout(&app).widgets;
        let map_frame = widgets.center_map;
        let popup = app
            .current_overlay_popup_rect(map_frame)
            .expect("inbox popup rect");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            popup.x + 2,
            popup.y,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            widgets.left_economy.outer.col as u16 + 2,
            popup.y + 1,
        ));

        let moved_popup = app
            .current_overlay_popup_rect(map_frame)
            .expect("moved popup rect");
        assert!(moved_popup.x < widgets.center_map.outer.col as u16);
    }

    #[test]
    fn clicking_overlay_close_button_closes_overlay_without_dragging() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::Inbox;
        let map_frame = dashboard_layout(&app).widgets.center_map;
        let popup = app
            .current_overlay_popup_rect(map_frame)
            .expect("inbox popup rect");
        let close_col =
            crate::dashboard::modal::modal_close_button_col(popup).expect("overlay close col");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            close_col,
            popup.y,
        ));

        assert_eq!(app.overlay, ActiveOverlay::None);
        assert!(!<DashApp as NativeApp>::is_dragging_surface(&app));
    }

    #[test]
    fn dragging_planet_detail_popup_updates_position() {
        let mut app = dash_app();
        app.popup = ActivePopup::PlanetDetail {
            planet_record_index_1_based: 1,
        };
        let map_frame = dashboard_layout(&app).widgets.center_map;
        let popup = app
            .current_popup_rect(map_frame)
            .expect("planet detail popup");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            popup.x + 2,
            popup.y,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            popup.x + 12,
            popup.y + 3,
        ));

        let moved_popup = app.current_popup_rect(map_frame).expect("moved popup");
        assert!(moved_popup.x > popup.x);
        assert!(moved_popup.y > popup.y);
    }

    #[test]
    fn clicking_popup_close_button_closes_popup_without_dragging() {
        let mut app = dash_app();
        app.popup = ActivePopup::PlanetDetail {
            planet_record_index_1_based: 1,
        };
        let map_frame = dashboard_layout(&app).widgets.center_map;
        let popup = app
            .current_popup_rect(map_frame)
            .expect("planet detail popup");
        let close_col =
            crate::dashboard::modal::modal_close_button_col(popup).expect("popup close col");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            close_col,
            popup.y,
        ));

        assert_eq!(app.popup, ActivePopup::None);
        assert!(!<DashApp as NativeApp>::is_dragging_surface(&app));
    }

    #[test]
    fn clicking_quit_confirm_close_button_closes_popup() {
        let mut app = dash_app();
        app.popup = ActivePopup::QuitConfirm;
        let map_frame = dashboard_layout(&app).widgets.center_map;
        let popup = app
            .current_popup_rect(map_frame)
            .expect("quit confirm popup");
        let close_col =
            crate::dashboard::modal::modal_close_button_col(popup).expect("popup close col");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            close_col,
            popup.y,
        ));

        assert_eq!(app.popup, ActivePopup::None);
    }

    #[test]
    fn clicking_help_overlay_close_button_restores_underlay_overlay() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::Inbox;
        app.open_overlay_help(HelpContext::Inbox);
        let map_frame = dashboard_layout(&app).widgets.center_map;
        let popup = app
            .current_overlay_popup_rect(map_frame)
            .expect("help popup rect");
        let close_col =
            crate::dashboard::modal::modal_close_button_col(popup).expect("help close col");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            close_col,
            popup.y,
        ));

        assert_eq!(app.overlay, ActiveOverlay::Inbox);
        assert_eq!(app.help_return_overlay, ActiveOverlay::None);
    }

    #[test]
    fn clicking_quit_confirm_close_button_restores_underlying_popup() {
        let mut app = dash_app();
        app.popup = ActivePopup::PlanetDetail {
            planet_record_index_1_based: 1,
        };
        app.dispatch_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT));
        let map_frame = dashboard_layout(&app).widgets.center_map;
        let popup = app
            .current_popup_rect(map_frame)
            .expect("quit confirm popup");
        let close_col =
            crate::dashboard::modal::modal_close_button_col(popup).expect("popup close col");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            close_col,
            popup.y,
        ));

        assert_eq!(
            app.popup,
            ActivePopup::PlanetDetail {
                planet_record_index_1_based: 1
            }
        );
    }

    #[test]
    fn fleet_helper_modal_rect_is_draggable_surface() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::FleetList;
        app.fleet_overlay.prompt_mode = FleetOverlayPromptMode::MissionPicker;
        let map_frame = dashboard_layout(&app).widgets.center_map;

        let popup = app
            .current_overlay_popup_rect(map_frame)
            .expect("mission picker popup");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            popup.x + 2,
            popup.y,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            popup.x + 8,
            popup.y + 2,
        ));

        assert!(app.overlay_position.is_some());
    }

    #[test]
    fn closing_and_reopening_overlay_recenters_dragged_position() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::Inbox;
        let map_frame = dashboard_layout(&app).widgets.center_map;
        let popup = app
            .current_overlay_popup_rect(map_frame)
            .expect("inbox popup rect");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            popup.x + 2,
            popup.y,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            popup.x + 12,
            popup.y + 3,
        ));
        let moved_popup = app
            .current_overlay_popup_rect(map_frame)
            .expect("moved popup rect");
        assert_ne!(moved_popup.x, popup.x);

        app.close_active_overlay();
        app.apply_action(super::input::Action::OpenOverlay(ActiveOverlay::Inbox));

        let recentered_popup = app
            .current_overlay_popup_rect(map_frame)
            .expect("reopened popup rect");
        assert_eq!(recentered_popup.x, popup.x);
        assert_eq!(recentered_popup.y, popup.y);
    }

    #[test]
    fn help_overlay_restores_dragged_underlay_position() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::Inbox;
        let map_frame = dashboard_layout(&app).widgets.center_map;
        let popup = app
            .current_overlay_popup_rect(map_frame)
            .expect("inbox popup rect");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            popup.x + 2,
            popup.y,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            popup.x + 10,
            popup.y + 2,
        ));
        let moved_popup = app
            .current_overlay_popup_rect(map_frame)
            .expect("moved popup rect");

        app.open_overlay_help(crate::dashboard::app::state::HelpContext::Inbox);
        app.close_active_overlay();

        let restored_popup = app
            .current_overlay_popup_rect(map_frame)
            .expect("restored popup rect");
        assert_eq!(restored_popup.x, moved_popup.x);
        assert_eq!(restored_popup.y, moved_popup.y);
    }

    #[test]
    fn dragging_overlay_reports_dragging_surface_state() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::Inbox;
        let map_frame = dashboard_layout(&app).widgets.center_map;
        let popup = app
            .current_overlay_popup_rect(map_frame)
            .expect("inbox popup rect");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            popup.x + 2,
            popup.y,
        ));
        assert!(<DashApp as NativeApp>::is_dragging_surface(&app));

        app.handle_mouse(mouse(
            MouseEventKind::Up(MouseButton::Left),
            popup.x + 2,
            popup.y,
        ));
        assert!(!<DashApp as NativeApp>::is_dragging_surface(&app));
    }

    #[test]
    fn clicking_map_sector_moves_crosshair() {
        let mut app = dash_app();
        let target = first_empty_sector_coords(&app);
        let (column, row) = screen_point_for_sector(&app, target);

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), column, row));

        assert_eq!([app.crosshair_x, app.crosshair_y], target);
    }

    #[test]
    fn hovering_visible_sector_moves_crosshair() {
        let mut app = dash_app();
        let target = first_empty_sector_coords(&app);
        let (column, row) = screen_point_for_sector(&app, target);

        app.handle_mouse(mouse(MouseEventKind::Moved, column, row));

        assert_eq!([app.crosshair_x, app.crosshair_y], target);
    }

    #[test]
    fn dispatch_mouse_move_without_visible_change_reports_no_redraw() {
        let mut app = dash_app();
        app.client_settings.follow_mouse_on_map = false;
        let target = first_empty_sector_coords(&app);
        let (column, row) = screen_point_for_sector(&app, target);

        assert!(!app.dispatch_mouse_event(mouse(MouseEventKind::Moved, column, row)));
    }

    #[test]
    fn dispatch_mouse_move_with_crosshair_change_reports_redraw() {
        let mut app = dash_app();
        let target = first_empty_sector_coords(&app);
        let (column, row) = screen_point_for_sector(&app, target);

        assert!(app.dispatch_mouse_event(mouse(MouseEventKind::Moved, column, row)));
    }

    #[test]
    fn hovering_visible_sector_does_not_move_crosshair_when_hover_follow_is_disabled() {
        let mut app = dash_app();
        let starting = [app.crosshair_x, app.crosshair_y];
        let target = first_empty_sector_coords(&app);
        let (column, row) = screen_point_for_sector(&app, target);
        app.client_settings.follow_mouse_on_map = false;

        app.handle_mouse(mouse(MouseEventKind::Moved, column, row));

        assert_eq!([app.crosshair_x, app.crosshair_y], starting);
    }

    #[test]
    fn moving_mouse_outside_map_widget_resets_crosshair_to_homeworld() {
        let mut app = dash_app();
        let homeworld = [app.crosshair_x, app.crosshair_y];
        let target = first_empty_sector_coords(&app);
        let (column, row) = screen_point_for_sector(&app, target);
        let outside = outside_map_point(&app);

        app.handle_mouse(mouse(MouseEventKind::Moved, column, row));
        assert_eq!([app.crosshair_x, app.crosshair_y], target);

        app.handle_mouse(mouse(MouseEventKind::Moved, outside.0, outside.1));

        assert_eq!([app.crosshair_x, app.crosshair_y], homeworld);
    }

    #[test]
    fn settings_overlay_toggle_keys_update_client_settings() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::Settings;

        app.handle_key(key(KeyCode::Char('m')));

        assert!(!app.client_settings.follow_mouse_on_map);
        assert_eq!(app.overlay, ActiveOverlay::Settings);
    }

    #[test]
    fn left_click_on_sector_with_player_fleets_opens_filtered_fleet_list() {
        let mut app = dash_app_with_starbase();
        let fleet_coords = first_owned_fleet_coords(&app);
        let (column, row) = screen_point_for_sector(&app, fleet_coords);

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), column, row));

        assert_eq!(app.overlay, ActiveOverlay::FleetList);
        assert_eq!(app.fleet_overlay.location_filter, Some(fleet_coords));
        assert!(fleet_list::table_rows(&app).iter().all(|row| {
            matches!(row.key, FleetOverlayRowKey::Fleet(_)) && row.coords == fleet_coords
        }));
    }

    #[test]
    fn left_click_on_empty_sector_fleet_glyph_opens_filtered_fleet_list() {
        let mut app = dash_app();
        let empty_coords = first_empty_sector_coords(&app);
        let fleet = app
            .game_data
            .fleets
            .records
            .iter_mut()
            .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.has_any_force())
            .expect("owned fleet");
        fleet.set_current_location_coords_raw(empty_coords);
        let (column, row) = screen_point_for_sector(&app, empty_coords);

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), column, row));

        assert_eq!(app.overlay, ActiveOverlay::FleetList);
        assert_eq!(app.fleet_overlay.location_filter, Some(empty_coords));
        assert!(
            fleet_list::table_rows(&app)
                .iter()
                .all(|row| row.coords == empty_coords)
        );
    }

    #[test]
    fn left_click_without_player_fleets_does_not_open_anything() {
        let mut app = audit_ready_dash_app();
        let target = first_visible_foreign_planet_coords(&mut app);
        let (column, row) = screen_point_for_sector(&app, target);

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), column, row));

        assert_eq!(app.overlay, ActiveOverlay::None);
        assert_eq!(app.popup, ActivePopup::None);
        assert_eq!([app.crosshair_x, app.crosshair_y], target);
    }

    #[test]
    fn right_click_on_owned_planet_opens_owned_planet_popup() {
        let mut app = dash_app();
        let owned_coords = first_owned_planet_coords(&app);
        let expected_record = app
            .game_data
            .planets
            .records
            .iter()
            .enumerate()
            .find(|(_, planet)| {
                planet.owner_empire_slot_raw() == 1 && planet.coords_raw() == owned_coords
            })
            .map(|(idx, _)| idx + 1)
            .expect("owned planet");
        let (column, row) = screen_point_for_sector(&app, owned_coords);

        app.planet_overlay.filter = PlanetOverlayFilter::Starbase;
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), column, row));

        assert_eq!(app.overlay, ActiveOverlay::None);
        assert_eq!(app.planet_overlay.filter, PlanetOverlayFilter::Starbase);
        assert_eq!(
            app.popup,
            ActivePopup::OwnedPlanet {
                planet_record_index_1_based: expected_record
            }
        );
    }

    #[test]
    fn owned_planet_popup_footer_shows_command_rail() {
        let mut app = dash_app();
        let owned_coords = first_owned_planet_coords(&app);
        let (column, row) = screen_point_for_sector(&app, owned_coords);

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), column, row));

        assert!(
            render_owned_planet_popup_line(&app, "COMMAND <- ? B C M L U X <ESC> ->")
                .contains("COMMAND <- ? B C M L U X <ESC> ->")
        );
    }

    #[test]
    fn owned_planet_popup_browse_uses_planet_status_title() {
        let mut app = dash_app();
        let owned_coords = first_owned_planet_coords(&app);
        let (column, row) = screen_point_for_sector(&app, owned_coords);

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), column, row));

        assert!(render_owned_planet_popup_line(&app, "PLANET STATUS").contains("PLANET STATUS"));
    }

    #[test]
    fn owned_planet_popup_question_mark_opens_help_overlay() {
        let mut app = dash_app();
        let owned_coords = first_owned_planet_coords(&app);
        let expected_record = app
            .game_data
            .planets
            .records
            .iter()
            .enumerate()
            .find(|(_, planet)| {
                planet.owner_empire_slot_raw() == 1 && planet.coords_raw() == owned_coords
            })
            .map(|(idx, _)| idx + 1)
            .expect("owned planet");
        let (column, row) = screen_point_for_sector(&app, owned_coords);

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), column, row));
        app.handle_key(key(KeyCode::Char('?')));

        assert_eq!(app.overlay, ActiveOverlay::Help);
        assert_eq!(app.help_context, HelpContext::OwnedPlanetPopup);
        assert_eq!(
            app.popup,
            ActivePopup::OwnedPlanet {
                planet_record_index_1_based: expected_record
            }
        );
        assert!(
            render_dashboard_line(&app, "Commission a completed stardock slot")
                .contains("Commission a completed stardock slot")
        );

        app.handle_key(key(KeyCode::Esc));

        assert_eq!(app.overlay, ActiveOverlay::None);
        assert_eq!(
            app.popup,
            ActivePopup::OwnedPlanet {
                planet_record_index_1_based: expected_record
            }
        );
    }

    #[test]
    fn owned_planet_popup_build_opens_shared_overlay_with_command_line_footer() {
        let mut app = dash_app();
        let owned_coords = first_owned_planet_coords(&app);
        let expected_record = owned_planet_record_index(&app, owned_coords);
        let planet = app
            .game_data
            .planets
            .records
            .get_mut(expected_record.saturating_sub(1))
            .expect("owned planet record");
        let _ = planet.set_present_production_points(80);
        planet.set_stored_production_points(80);
        let (column, row) = screen_point_for_sector(&app, owned_coords);

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), column, row));
        app.handle_key(key(KeyCode::Char('b')));

        assert_eq!(app.overlay, ActiveOverlay::PlanetList);
        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::BuildSpecify
        );
        assert_eq!(
            app.planet_overlay.build_planet_record_index_1_based,
            Some(expected_record)
        );
        assert_eq!(app.owned_planet_popup.mode, OwnedPlanetPopupMode::Browse);
        assert!(
            render_dashboard_line(&app, "COMMAND <- ? + - D <ESC> [0] ->")
                .contains("COMMAND <- ? + - D <ESC> [0] ->")
        );
    }

    #[test]
    fn owned_planet_build_overlay_uses_planet_title_instead_of_build_on_planet() {
        let mut app = dash_app();
        let owned_coords = first_owned_planet_coords(&app);
        let expected_record = owned_planet_record_index(&app, owned_coords);
        let planet = app
            .game_data
            .planets
            .records
            .get_mut(expected_record.saturating_sub(1))
            .expect("owned planet record");
        let _ = planet.set_present_production_points(80);
        planet.set_stored_production_points(80);
        let (column, row) = screen_point_for_sector(&app, owned_coords);

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), column, row));
        app.handle_key(key(KeyCode::Char('b')));

        let title = app.planet_build_title();
        let title_line = render_dashboard_line(&app, &title);

        assert_eq!(app.overlay, ActiveOverlay::PlanetList);
        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::BuildSpecify
        );
        assert!(title_line.contains(&title));
        assert!(!title_line.contains("SPECIFY BUILD ORDERS"));
        assert!(!title_line.contains("BUILD ON PLANET"));
    }

    #[test]
    fn owned_planet_build_quantity_stays_on_shared_overlay() {
        let mut app = dash_app();
        let owned_coords = first_owned_planet_coords(&app);
        let expected_record = owned_planet_record_index(&app, owned_coords);
        let planet = app
            .game_data
            .planets
            .records
            .get_mut(expected_record.saturating_sub(1))
            .expect("owned planet record");
        let _ = planet.set_present_production_points(80);
        planet.set_stored_production_points(80);
        let (column, row) = screen_point_for_sector(&app, owned_coords);

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), column, row));
        app.handle_key(key(KeyCode::Char('b')));
        app.handle_key(key(KeyCode::Char('1')));
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.overlay, ActiveOverlay::PlanetList);
        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::BuildQuantity
        );
        assert_eq!(
            app.planet_overlay.build_planet_record_index_1_based,
            Some(expected_record)
        );
        assert!(
            render_dashboard_line(&app, "COMMAND <- Qty [16] ->")
                .contains("COMMAND <- Qty [16] ->")
        );
    }

    #[test]
    fn owned_planet_no_budget_still_opens_build_overlay() {
        let mut app = dash_app();
        let owned_coords = first_owned_planet_coords(&app);
        let expected_record = owned_planet_record_index(&app, owned_coords);
        let planet = app
            .game_data
            .planets
            .records
            .get_mut(expected_record.saturating_sub(1))
            .expect("owned planet record");
        let _ = planet.set_present_production_points(1);
        planet.set_stored_production_points(1);
        let (column, row) = screen_point_for_sector(&app, owned_coords);

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), column, row));
        app.handle_key(key(KeyCode::Char('b')));

        assert_eq!(app.overlay, ActiveOverlay::PlanetList);
        assert_eq!(app.owned_planet_popup.mode, OwnedPlanetPopupMode::Browse);
        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::BuildSpecify
        );
        assert_eq!(
            app.planet_overlay.build_planet_record_index_1_based,
            Some(expected_record)
        );
    }

    #[test]
    fn popup_launched_build_escape_returns_to_owned_planet_popup() {
        let mut app = dash_app();
        let owned_coords = first_owned_planet_coords(&app);
        let expected_record = owned_planet_record_index(&app, owned_coords);
        let planet = app
            .game_data
            .planets
            .records
            .get_mut(expected_record.saturating_sub(1))
            .expect("owned planet record");
        let _ = planet.set_present_production_points(80);
        planet.set_stored_production_points(80);
        let (column, row) = screen_point_for_sector(&app, owned_coords);

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), column, row));
        app.handle_key(key(KeyCode::Char('b')));
        app.handle_key(key(KeyCode::Esc));

        assert_eq!(app.overlay, ActiveOverlay::None);
        assert_eq!(app.owned_planet_popup.mode, OwnedPlanetPopupMode::Browse);
        assert!(matches!(
            app.popup,
            ActivePopup::OwnedPlanet {
                planet_record_index_1_based
            } if planet_record_index_1_based == expected_record
        ));
    }

    #[test]
    fn popup_launched_build_success_stays_in_build_overlay() {
        let mut app = dash_app_with_store();
        let owned_coords = first_owned_planet_coords(&app);
        let expected_record = owned_planet_record_index(&app, owned_coords);
        let planet = app
            .game_data
            .planets
            .records
            .get_mut(expected_record.saturating_sub(1))
            .expect("owned planet record");
        let _ = planet.set_present_production_points(80);
        planet.set_stored_production_points(80);
        let (column, row) = screen_point_for_sector(&app, owned_coords);

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), column, row));
        app.handle_key(key(KeyCode::Char('b')));
        app.handle_key(key(KeyCode::Char('1')));
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Char('1')));
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.overlay, ActiveOverlay::PlanetList);
        assert_eq!(app.owned_planet_popup.mode, OwnedPlanetPopupMode::Browse);
        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::BuildSpecify
        );
        assert_eq!(
            app.planet_overlay.build_planet_record_index_1_based,
            Some(expected_record)
        );
        assert_eq!(app.owned_planet_popup.status, None);
    }

    #[test]
    fn popup_launched_build_delete_clears_selected_kind_in_place() {
        let mut app = dash_app_with_store();
        let owned_coords = first_owned_planet_coords(&app);
        let expected_record = owned_planet_record_index(&app, owned_coords);
        let planet = app
            .game_data
            .planets
            .records
            .get_mut(expected_record.saturating_sub(1))
            .expect("owned planet record");
        planet.set_build_kind_raw(0, 1);
        planet.set_build_count_raw(0, 10);
        let (column, row) = screen_point_for_sector(&app, owned_coords);

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), column, row));
        app.handle_key(key(KeyCode::Char('b')));
        app.handle_key(key(KeyCode::Char('d')));

        assert_eq!(app.overlay, ActiveOverlay::PlanetList);
        assert_eq!(app.owned_planet_popup.mode, OwnedPlanetPopupMode::Browse);
        assert_eq!(
            app.planet_overlay.prompt_mode,
            PlanetOverlayPromptMode::BuildSpecify
        );
        assert!(
            nc_engine::planet_build_orders(&app.game_data.planets.records[expected_record - 1])
                .is_empty(),
            "selected queued build should be deleted in place"
        );
    }

    #[test]
    fn right_click_on_visible_foreign_planet_opens_planet_detail_popup() {
        let mut app = audit_ready_dash_app();
        let target = first_visible_foreign_planet_coords(&mut app);
        let (column, row) = screen_point_for_sector(&app, target);

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), column, row));

        assert!(matches!(app.popup, ActivePopup::PlanetDetail { .. }));
    }

    #[test]
    fn clicks_do_not_leak_through_open_overlay_to_map() {
        let mut app = dash_app();
        let starting = [app.crosshair_x, app.crosshair_y];
        app.overlay = ActiveOverlay::Help;
        let map_frame = dashboard_layout(&app).widgets.center_map;
        let popup = app
            .current_overlay_popup_rect(map_frame)
            .expect("help popup rect");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            popup.x + popup.width / 2,
            popup.y + popup.height / 2,
        ));

        assert_eq!([app.crosshair_x, app.crosshair_y], starting);
    }

    #[test]
    fn hover_and_clicks_do_not_leak_through_open_popup_to_map() {
        let mut app = dash_app();
        let starting = [app.crosshair_x, app.crosshair_y];
        app.popup = ActivePopup::PlanetDetail {
            planet_record_index_1_based: 1,
        };
        let target = first_empty_sector_coords(&app);
        let (column, row) = screen_point_for_sector(&app, target);

        app.handle_mouse(mouse(MouseEventKind::Moved, column, row));
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), column, row));

        assert_eq!([app.crosshair_x, app.crosshair_y], starting);
    }

    #[test]
    fn closing_location_filtered_fleet_overlay_clears_transient_filter() {
        let mut app = dash_app();
        let fleet_coords = first_owned_fleet_coords(&app);

        app.open_fleet_overlay_for_location(fleet_coords);
        assert_eq!(app.fleet_overlay.location_filter, Some(fleet_coords));

        app.close_active_overlay();

        assert_eq!(app.overlay, ActiveOverlay::None);
        assert_eq!(app.fleet_overlay.location_filter, None);
    }

    #[test]
    fn keyboard_opening_fleet_list_clears_transient_location_filter() {
        let mut app = dash_app();
        let fleet_coords = first_owned_fleet_coords(&app);
        app.fleet_overlay.location_filter = Some(fleet_coords);

        app.apply_action(super::input::Action::OpenOverlay(ActiveOverlay::FleetList));

        assert_eq!(app.overlay, ActiveOverlay::FleetList);
        assert_eq!(app.fleet_overlay.location_filter, None);
    }

    fn dash_app() -> DashApp {
        DashApp::new_for_tests(
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
            crate::dashboard::geometry::ScreenGeometry::new(160, 40),
            crate::dashboard::geometry::ScreenGeometry::new(108, 26),
            1,
        )
    }

    fn audit_ready_dash_app() -> DashApp {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .with_homeworld_coords(vec![[16, 13], [12, 6], [4, 15], [15, 15]])
                .with_guard_starbase(1, 1, [16, 13], 1)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            crate::dashboard::geometry::ScreenGeometry::new(160, 40),
            crate::dashboard::geometry::ScreenGeometry::new(108, 26),
            1,
        );
        seed_unowned_target_world(&mut app.game_data, [8, 8]);
        strengthen_first_owned_fleet(&mut app.game_data);
        app.planet_intel_snapshots = view_world_audit_snapshots(&app.game_data, 1);
        app
    }

    fn dash_app_with_starbase() -> DashApp {
        DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .with_guard_starbase(1, 1, [16, 13], 1)
                .build_initialized_baseline()
                .expect("baseline with starbase"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            crate::dashboard::geometry::ScreenGeometry::new(160, 40),
            crate::dashboard::geometry::ScreenGeometry::new(108, 26),
            1,
        )
    }

    fn dash_app_with_starbase_store() -> DashApp {
        let root = std::env::temp_dir().join(format!(
            "nc-dash-fleet-order-starbase-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).expect("create temp test dir");
        let mut game_data = GameStateBuilder::new()
            .with_player_count(4)
            .with_guard_starbase(1, 1, [16, 13], 1)
            .build_initialized_baseline()
            .expect("baseline with starbase");
        strengthen_first_owned_fleet(&mut game_data);
        let store = CampaignStore::open_default_in_dir(&root).expect("open campaign store");
        let planet_intel_by_viewer = (0..game_data.conquest.player_count())
            .map(|_| BTreeMap::new())
            .collect::<Vec<_>>();
        let player_activity_states = store
            .latest_player_activity_states(game_data.conquest.player_count())
            .expect("default player activity");
        let player_lifecycle_states = store
            .latest_player_lifecycle_states(game_data.conquest.player_count())
            .expect("default player lifecycle");
        store
            .save_runtime_state_structured_with_intel_and_activity(
                &game_data,
                &BTreeSet::new(),
                &[],
                &[],
                &planet_intel_by_viewer,
                &player_activity_states,
            )
            .expect("seed campaign store");

        DashApp::new(
            root,
            Some(store),
            game_data,
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            player_activity_states,
            player_lifecycle_states,
            nc_data::WinnerState::default(),
            crate::dashboard::geometry::ScreenGeometry::new(160, 40),
            crate::dashboard::geometry::ScreenGeometry::new(108, 26),
            1,
        )
    }

    fn dash_app_with_store() -> DashApp {
        let root = std::env::temp_dir().join(format!(
            "nc-dash-fleet-order-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).expect("create temp test dir");
        let game_data = GameStateBuilder::new()
            .with_player_count(4)
            .build_initialized_baseline()
            .expect("baseline");
        let store = CampaignStore::open_default_in_dir(&root).expect("open campaign store");
        let planet_intel_by_viewer = (0..game_data.conquest.player_count())
            .map(|_| BTreeMap::new())
            .collect::<Vec<_>>();
        let player_activity_states = store
            .latest_player_activity_states(game_data.conquest.player_count())
            .expect("default player activity");
        let player_lifecycle_states = store
            .latest_player_lifecycle_states(game_data.conquest.player_count())
            .expect("default player lifecycle");
        store
            .save_runtime_state_structured_with_intel_and_activity(
                &game_data,
                &BTreeSet::new(),
                &[],
                &[],
                &planet_intel_by_viewer,
                &player_activity_states,
            )
            .expect("seed campaign store");

        DashApp::new(
            root,
            Some(store),
            game_data,
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            player_activity_states,
            player_lifecycle_states,
            nc_data::WinnerState::default(),
            crate::dashboard::geometry::ScreenGeometry::new(160, 40),
            crate::dashboard::geometry::ScreenGeometry::new(108, 26),
            1,
        )
    }

    fn strengthen_first_owned_fleet(game_data: &mut nc_data::CoreGameData) {
        for fleet in game_data
            .fleets
            .records
            .iter_mut()
            .filter(|fleet| fleet.owner_empire_raw() == 1 && fleet.has_any_force())
        {
            fleet.set_battleship_count(1);
            fleet.set_destroyer_count(1);
            fleet.set_troop_transport_count(1);
            fleet.set_army_count(2);
            fleet.set_scout_count(1);
            fleet.set_etac_count(1);
            fleet.recompute_max_speed_from_composition();
            fleet.set_current_speed(fleet.max_speed());
        }
    }

    fn seed_unowned_target_world(game_data: &mut nc_data::CoreGameData, coords: [u8; 2]) {
        let planet = game_data
            .planets
            .records
            .iter_mut()
            .find(|planet| planet.owner_empire_slot_raw() == 0 && planet.coords_raw() == [0, 0])
            .expect("unused unowned planet slot");
        planet.set_coords_raw(coords);
        planet.set_owner_empire_slot_raw(0);
    }

    fn view_world_audit_snapshots(
        game_data: &nc_data::CoreGameData,
        viewer_empire_id: u8,
    ) -> Vec<PlanetIntelSnapshot> {
        let target_record_index = game_data
            .planets
            .records
            .iter()
            .enumerate()
            .find(|(_, planet)| {
                planet.owner_empire_slot_raw() != viewer_empire_id && planet.coords_raw() != [0, 0]
            })
            .map(|(idx, _)| idx + 1)
            .expect("non-owned target planet");

        game_data
            .planets
            .records
            .iter()
            .enumerate()
            .filter_map(|(planet_index, planet)| {
                let planet_record_index_1_based = planet_index + 1;
                if planet.owner_empire_slot_raw() == viewer_empire_id {
                    return None;
                }
                if planet_record_index_1_based == target_record_index {
                    return Some(PlanetIntelSnapshot {
                        planet_record_index_1_based,
                        intel_tier: IntelTier::Unknown,
                        compat_is_orbit_seed: false,
                        last_intel_year: None,
                        seen_year: None,
                        scout_year: None,
                        known_name: None,
                        known_owner_empire_id: None,
                        known_potential_production: None,
                        known_armies: None,
                        known_ground_batteries: None,
                        known_starbase_count: None,
                        known_current_production: None,
                        known_stored_points: None,
                        known_docked_summary: None,
                        known_orbit_summary: None,
                        compat_word_1e: None,
                    });
                }
                Some(PlanetIntelSnapshot {
                    planet_record_index_1_based,
                    intel_tier: IntelTier::Partial,
                    compat_is_orbit_seed: false,
                    last_intel_year: Some(game_data.conquest.game_year()),
                    seen_year: Some(game_data.conquest.game_year()),
                    scout_year: None,
                    known_name: None,
                    known_owner_empire_id: Some(planet.owner_empire_slot_raw()),
                    known_potential_production: None,
                    known_armies: None,
                    known_ground_batteries: None,
                    known_starbase_count: None,
                    known_current_production: None,
                    known_stored_points: None,
                    known_docked_summary: None,
                    known_orbit_summary: None,
                    compat_word_1e: None,
                })
            })
            .collect()
    }

    fn screen_point_for_sector(app: &DashApp, target: [u8; 2]) -> (u16, u16) {
        let map_frame = dashboard_layout(app).widgets.center_map;
        for row in map_frame.grid.row..map_frame.grid.row + map_frame.grid.height {
            for col in map_frame.grid.col..map_frame.grid.col + map_frame.grid.width {
                if crate::dashboard::panels::starmap::screen_sector_at_point(
                    app, map_frame, col, row,
                ) == Some(target)
                {
                    return (col as u16, row as u16);
                }
            }
        }
        panic!("no screen point for sector {target:?}");
    }

    fn outside_map_point(app: &DashApp) -> (u16, u16) {
        let outer = dashboard_layout(app).widgets.center_map.outer;
        if outer.col > 0 {
            return ((outer.col - 1) as u16, outer.row as u16);
        }
        ((outer.last_col() + 1) as u16, outer.row as u16)
    }

    fn first_owned_fleet_coords(app: &DashApp) -> [u8; 2] {
        app.game_data
            .fleets
            .records
            .iter()
            .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.has_any_force())
            .map(|fleet| fleet.current_location_coords_raw())
            .expect("owned fleet")
    }

    fn first_owned_planet_coords(app: &DashApp) -> [u8; 2] {
        app.game_data
            .planets
            .records
            .iter()
            .find(|planet| planet.owner_empire_slot_raw() == 1 && planet.coords_raw() != [0, 0])
            .map(|planet| planet.coords_raw())
            .expect("owned planet")
    }

    fn owned_planet_record_index(app: &DashApp, coords: [u8; 2]) -> usize {
        app.game_data
            .planets
            .records
            .iter()
            .enumerate()
            .find(|(_, planet)| {
                planet.owner_empire_slot_raw() == 1 && planet.coords_raw() == coords
            })
            .map(|(idx, _)| idx + 1)
            .expect("owned planet")
    }

    fn first_empty_sector_coords(app: &DashApp) -> [u8; 2] {
        first_visible_sector_matching(app, |coords| {
            !sector_has_planet(app, coords) && !sector_has_player_fleet(app, coords)
        })
    }

    fn first_visible_foreign_planet_coords(app: &mut DashApp) -> [u8; 2] {
        for planet in
            app.game_data.planets.records.iter().filter(|planet| {
                planet.owner_empire_slot_raw() != 1 && planet.coords_raw() != [0, 0]
            })
        {
            let coords = planet.coords_raw();
            app.crosshair_x = coords[0];
            app.crosshair_y = coords[1];
            if planet_view::selected_planet_detail(app).is_some() {
                return coords;
            }
        }
        panic!("visible foreign planet");
    }

    fn first_visible_sector_matching<F>(app: &DashApp, predicate: F) -> [u8; 2]
    where
        F: Fn([u8; 2]) -> bool,
    {
        let map_frame = dashboard_layout(app).widgets.center_map;
        for row in map_frame.grid.row..map_frame.grid.row + map_frame.grid.height {
            for col in map_frame.grid.col..map_frame.grid.col + map_frame.grid.width {
                let Some(coords) = crate::dashboard::panels::starmap::screen_sector_at_point(
                    app, map_frame, col, row,
                ) else {
                    continue;
                };
                if predicate(coords) {
                    return coords;
                }
            }
        }
        panic!("expected visible sector");
    }

    fn sector_has_planet(app: &DashApp, coords: [u8; 2]) -> bool {
        app.game_data
            .planets
            .records
            .iter()
            .any(|planet| planet.coords_raw() == coords)
    }

    fn sector_has_player_fleet(app: &DashApp, coords: [u8; 2]) -> bool {
        app.game_data.fleets.records.iter().any(|fleet| {
            fleet.owner_empire_raw() == app.player_record_index_1_based as u8
                && fleet.has_any_force()
                && fleet.current_location_coords_raw() == coords
        })
    }

    fn select_first_fleet_row(app: &mut DashApp) {
        app.fleet_overlay.selected = fleet_list::table_rows(app)
            .iter()
            .position(|row| matches!(row.key, FleetOverlayRowKey::Fleet(_)))
            .expect("fleet row");
    }

    fn select_first_two_fleet_rows(app: &mut DashApp) -> Vec<usize> {
        let rows = fleet_list::table_rows(app);
        let fleet_indexes = rows
            .iter()
            .enumerate()
            .filter_map(|(row_index, row)| match row.key {
                FleetOverlayRowKey::Fleet(record_index) => Some((row_index, record_index)),
                FleetOverlayRowKey::Starbase(_) => None,
            })
            .take(2)
            .collect::<Vec<_>>();
        assert_eq!(fleet_indexes.len(), 2, "expected at least two fleet rows");
        app.fleet_overlay.selected = fleet_indexes[0].0;
        app.handle_key(key(KeyCode::Char(' ')));
        app.fleet_overlay.selected = fleet_indexes[1].0;
        app.handle_key(key(KeyCode::Char(' ')));
        app.fleet_overlay.selected = fleet_indexes[0].0;
        fleet_indexes
            .into_iter()
            .map(|(_, record_index)| record_index)
            .collect()
    }

    fn render_fleet_footer_line(app: &DashApp, needle: &str) -> String {
        let layout = dashboard_layout(app);
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            crate::dashboard::theme::body_style(),
        );
        fleet_list::draw(&mut buffer, app, layout.widgets.center_map);
        (0..buffer.height())
            .map(|row| buffer.plain_line(row))
            .find(|line| line.contains(needle))
            .expect("fleet footer")
    }

    fn render_fleet_title_line(app: &DashApp, needle: &str) -> String {
        let layout = dashboard_layout(app);
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            crate::dashboard::theme::body_style(),
        );
        fleet_list::draw(&mut buffer, app, layout.widgets.center_map);
        (0..buffer.height())
            .map(|row| buffer.plain_line(row))
            .find(|line| line.contains(needle))
            .expect("fleet title")
    }

    fn render_planet_footer_line(app: &DashApp, needle: &str) -> String {
        let layout = dashboard_layout(app);
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            crate::dashboard::theme::body_style(),
        );
        planet_list::draw(&mut buffer, app, layout.widgets.center_map);
        (0..buffer.height())
            .map(|row| buffer.plain_line(row))
            .find(|line| line.contains(needle))
            .expect("planet footer")
    }

    fn render_planet_title_line(app: &DashApp, needle: &str) -> String {
        let layout = dashboard_layout(app);
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            crate::dashboard::theme::body_style(),
        );
        planet_list::draw(&mut buffer, app, layout.widgets.center_map);
        (0..buffer.height())
            .map(|row| buffer.plain_line(row))
            .find(|line| line.contains(needle))
            .expect("planet title")
    }

    fn render_owned_planet_popup_line(app: &DashApp, needle: &str) -> String {
        let layout = dashboard_layout(app);
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            crate::dashboard::theme::body_style(),
        );
        let planet_record_index_1_based = match app.popup {
            ActivePopup::OwnedPlanet {
                planet_record_index_1_based,
            } => planet_record_index_1_based,
            other => panic!("expected owned planet popup, got {other:?}"),
        };
        crate::dashboard::popups::owned_planet::draw(
            &mut buffer,
            app,
            layout.widgets.center_map,
            planet_record_index_1_based,
        );
        (0..buffer.height())
            .map(|row| buffer.plain_line(row))
            .find(|line| line.contains(needle))
            .expect("owned planet popup line")
    }

    fn render_dashboard_line(app: &DashApp, needle: &str) -> String {
        let buffer = crate::dashboard::app::render::render(app).expect("dashboard render");
        (0..buffer.height())
            .map(|row| buffer.plain_line(row))
            .find(|line| line.contains(needle))
            .expect("dashboard line")
    }

    fn render_intel_footer_line(app: &DashApp, needle: &str) -> String {
        let layout = dashboard_layout(app);
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            crate::dashboard::theme::body_style(),
        );
        intel_database::draw(&mut buffer, app, layout.widgets.center_map);
        (0..buffer.height())
            .map(|row| buffer.plain_line(row))
            .find(|line| line.contains(needle))
            .expect("intel footer")
    }

    fn render_intel_title_line(app: &DashApp, needle: &str) -> String {
        let layout = dashboard_layout(app);
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            crate::dashboard::theme::body_style(),
        );
        intel_database::draw(&mut buffer, app, layout.widgets.center_map);
        (0..buffer.height())
            .map(|row| buffer.plain_line(row))
            .find(|line| line.contains(needle))
            .expect("intel title")
    }

    fn render_settings_line(app: &DashApp, needle: &str) -> String {
        let layout = dashboard_layout(app);
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            crate::dashboard::theme::body_style(),
        );
        crate::dashboard::overlays::settings::draw(&mut buffer, app, layout.widgets.center_map);
        (0..buffer.height())
            .map(|row| buffer.plain_line(row))
            .find(|line| line.contains(needle))
            .expect("settings line")
    }

    fn render_settings_contains(app: &DashApp, needle: &str) -> bool {
        let layout = dashboard_layout(app);
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            crate::dashboard::theme::body_style(),
        );
        crate::dashboard::overlays::settings::draw(&mut buffer, app, layout.widgets.center_map);
        (0..buffer.height())
            .map(|row| buffer.plain_line(row))
            .any(|line| line.contains(needle))
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn settings_status_uses_command_line_toast_instead_of_body_row() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::Settings;
        app.settings_overlay.status_message = Some("Saved local settings".to_string());

        assert!(
            render_settings_line(&app, "Saved local settings")
                .contains("COMMAND <- Saved local settings")
        );
    }

    #[test]
    fn command_line_toast_clears_after_one_second() {
        let mut app = dash_app();
        let now = Instant::now();
        app.overlay = ActiveOverlay::Settings;
        app.settings_overlay.status_message = Some("Saved local settings".to_string());

        assert!(app.update_command_line_toast_state(now));
        assert_eq!(
            app.command_line_toast_deadline,
            Some(now + Duration::from_secs(1))
        );
        assert_eq!(
            app.settings_overlay.status_message.as_deref(),
            Some("Saved local settings")
        );

        assert!(app.update_command_line_toast_state(now + Duration::from_secs(1)));
        assert_eq!(app.settings_overlay.status_message, None);
        assert!(!render_settings_contains(&app, "Saved local settings"));
    }

    #[test]
    fn command_line_toast_key_dismissal_still_runs_normal_action() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::Settings;
        app.settings_overlay.status_message = Some("Saved local settings".to_string());

        app.dispatch_key_event(key(KeyCode::Esc));

        assert_eq!(app.overlay, ActiveOverlay::None);
        assert_eq!(app.settings_overlay.status_message, None);
    }

    #[test]
    fn prompt_validation_toast_clears_when_opening_help() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::PlanetList;
        app.planet_overlay
            .open_prompt(PlanetOverlayPromptMode::FilterMenu);
        app.planet_overlay.prompt_status = Some(" Ambiguous: sbs/sta".to_string());

        app.dispatch_key_event(key(KeyCode::Char('?')));

        assert_eq!(app.overlay, ActiveOverlay::Help);
        assert_eq!(app.planet_overlay.prompt_status, None);
    }

    #[test]
    fn root_escape_opens_quit_confirm_popup() {
        let mut app = dash_app();

        app.dispatch_key_event(key(KeyCode::Esc));

        assert_eq!(app.popup, ActivePopup::QuitConfirm);
        assert_eq!(app.overlay, ActiveOverlay::None);
        assert_eq!(app.take_exit_request(), None);
    }

    #[test]
    fn alt_q_opens_quit_confirm_popup() {
        let mut app = dash_app();

        app.dispatch_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT));

        assert_eq!(app.popup, ActivePopup::QuitConfirm);
        assert_eq!(app.take_exit_request(), None);
    }

    #[test]
    fn quit_confirm_cancel_restores_underlying_popup() {
        let mut app = dash_app();
        app.popup = ActivePopup::PlanetDetail {
            planet_record_index_1_based: 1,
        };

        app.dispatch_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT));
        app.dispatch_key_event(key(KeyCode::Enter));

        assert_eq!(
            app.popup,
            ActivePopup::PlanetDetail {
                planet_record_index_1_based: 1
            }
        );
        assert_eq!(app.take_exit_request(), None);
    }

    #[test]
    fn quit_confirm_yes_requests_return_to_lobby() {
        let mut app = dash_app();
        app.dispatch_key_event(key(KeyCode::Esc));

        app.dispatch_key_event(key(KeyCode::Char('y')));

        assert_eq!(
            app.take_exit_request(),
            Some(DashboardExitRequest::ReturnToLobby)
        );
        assert!(!app.should_quit);
    }

    #[test]
    fn control_c_requests_client_quit() {
        let mut app = dash_app();

        app.dispatch_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

        assert_eq!(
            app.take_exit_request(),
            Some(DashboardExitRequest::QuitClient)
        );
    }

    #[test]
    fn middle_drag_on_starmap_pans_viewport_without_moving_crosshair() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            build_seeded_initialized_game(25, 3000, 1515).expect("seeded game"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(80, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.crosshair_x = 23;
        app.crosshair_y = 23;
        starmap::advance_starmap_viewport(&mut app);
        let initial_x_min = app.starmap_viewport_x_min;
        let initial_y_min = app.starmap_viewport_y_min;
        let initial_crosshair = [app.crosshair_x, app.crosshair_y];

        let map_frame = dashboard_layout(&app).widgets.center_map;
        let col = map_frame.outer.col as u16 + 1;
        let row = map_frame.outer.row as u16 + 1;

        // Middle-down inside the map widget.
        app.dispatch_mouse_event_for_repro(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Middle),
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        });

        // Drag 8 columns right → viewport should pan 2 sectors left.
        app.dispatch_mouse_event_for_repro(MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Middle),
            column: col + 8,
            row,
            modifiers: KeyModifiers::NONE,
        });

        app.dispatch_mouse_event_for_repro(MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Middle),
            column: col + 8,
            row,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!([app.crosshair_x, app.crosshair_y], initial_crosshair);
        assert_eq!(app.starmap_viewport_x_min, initial_x_min.saturating_sub(2));
        assert_eq!(app.starmap_viewport_y_min, initial_y_min);
    }

    #[test]
    fn middle_click_without_drag_recenters_viewport() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            build_seeded_initialized_game(25, 3000, 1515).expect("seeded game"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(80, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.crosshair_x = 23;
        app.crosshair_y = 23;
        starmap::advance_starmap_viewport(&mut app);
        let centered_x_min = app.starmap_viewport_x_min;
        let centered_y_min = app.starmap_viewport_y_min;

        let map_frame = dashboard_layout(&app).widgets.center_map;
        let col = map_frame.outer.col as u16 + 1;
        let row = map_frame.outer.row as u16 + 1;

        // Pan the viewport away from centre with a drag.
        app.dispatch_mouse_event_for_repro(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Middle),
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        });
        app.dispatch_mouse_event_for_repro(MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Middle),
            column: col + 8,
            row,
            modifiers: KeyModifiers::NONE,
        });
        app.dispatch_mouse_event_for_repro(MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Middle),
            column: col + 8,
            row,
            modifiers: KeyModifiers::NONE,
        });
        let dragged_x_min = app.starmap_viewport_x_min;
        let dragged_y_min = app.starmap_viewport_y_min;
        // Sanity: drag actually moved the viewport.
        assert_ne!(dragged_x_min, centered_x_min);

        // Middle-click without drag resets the viewport to centre on crosshair.
        app.dispatch_mouse_event_for_repro(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Middle),
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        });
        app.dispatch_mouse_event_for_repro(MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Middle),
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(app.starmap_viewport_x_min, centered_x_min);
        assert_eq!(app.starmap_viewport_y_min, centered_y_min);
    }

    #[test]
    fn mouse_wheel_on_starmap_pans_viewport_without_moving_crosshair() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            build_seeded_initialized_game(25, 3000, 1515).expect("seeded game"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(80, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.crosshair_x = 23;
        app.crosshair_y = 23;
        starmap::advance_starmap_viewport(&mut app);
        let initial_x_min = app.starmap_viewport_x_min;
        let initial_y_min = app.starmap_viewport_y_min;
        let initial_crosshair = [app.crosshair_x, app.crosshair_y];

        let map_frame = dashboard_layout(&app).widgets.center_map;
        let col = map_frame.outer.col as u16 + 1;
        let row = map_frame.outer.row as u16 + 1;

        // Scroll up (positive lines) → viewport origin moves up (decrease y).
        app.dispatch_mouse_event_for_repro(MouseEvent {
            kind: MouseEventKind::Scroll { lines: 2 },
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!([app.crosshair_x, app.crosshair_y], initial_crosshair);
        assert_eq!(app.starmap_viewport_y_min, initial_y_min.saturating_sub(2));
        assert_eq!(app.starmap_viewport_x_min, initial_x_min);

        // Scroll down (negative lines) → viewport origin moves down (increase y).
        app.dispatch_mouse_event_for_repro(MouseEvent {
            kind: MouseEventKind::Scroll { lines: -3 },
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(app.starmap_viewport_y_min, initial_y_min.saturating_add(1));
    }

    #[test]
    fn mouse_wheel_on_fitting_starmap_does_nothing() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            build_seeded_initialized_game(10, 3000, 1515).expect("seeded game"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(80, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.crosshair_x = 5;
        app.crosshair_y = 5;
        starmap::advance_starmap_viewport(&mut app);
        let initial_x_min = app.starmap_viewport_x_min;
        let initial_y_min = app.starmap_viewport_y_min;

        let map_frame = dashboard_layout(&app).widgets.center_map;
        let col = map_frame.outer.col as u16 + 1;
        let row = map_frame.outer.row as u16 + 1;

        app.dispatch_mouse_event_for_repro(MouseEvent {
            kind: MouseEventKind::Scroll { lines: 5 },
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(app.starmap_viewport_x_min, initial_x_min);
        assert_eq!(app.starmap_viewport_y_min, initial_y_min);
    }

    #[test]
    fn mouse_wheel_in_diplomacy_overlay_scrolls_list() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::Diplomacy;
        app.diplomacy_scroll = 0;

        let map_frame = dashboard_layout(&app).widgets.center_map;
        let popup = app.current_overlay_popup_rect(map_frame).unwrap();
        let col = popup.x + 2;
        let row = popup.y + 2;

        // Scroll down (negative lines) → diplomacy_scroll increases.
        app.dispatch_mouse_event_for_repro(MouseEvent {
            kind: MouseEventKind::Scroll { lines: -3 },
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(app.diplomacy_scroll, 3);

        // Scroll up (positive lines) → diplomacy_scroll decreases.
        app.dispatch_mouse_event_for_repro(MouseEvent {
            kind: MouseEventKind::Scroll { lines: 2 },
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(app.diplomacy_scroll, 1);
    }

    #[test]
    fn mouse_wheel_in_inbox_list_moves_selection() {
        let mut app = dash_app();
        app.report_block_rows = vec![
            ReportBlockRow {
                viewer_empire_id: 0,
                block_index: 0,
                decoded_text: "Stardate: 03/3012\nLine one.".to_string(),
                raw_bytes: None,
                recipient_deleted: false,
            },
            ReportBlockRow {
                viewer_empire_id: 0,
                block_index: 1,
                decoded_text: "Stardate: 04/3012\nLine two.".to_string(),
                raw_bytes: None,
                recipient_deleted: false,
            },
        ];
        app.queued_mail = vec![QueuedPlayerMail {
            sender_empire_id: 2,
            recipient_empire_id: 1,
            year: 3012,
            subject: "Test subject".to_string(),
            body: "Test body".to_string(),
            recipient_deleted: false,
        }];
        app.overlay = ActiveOverlay::Inbox;
        app.inbox_overlay.selected = 2;
        app.inbox_overlay.scroll = 0;
        app.inbox_overlay.preview_scroll = 5;

        let map_frame = dashboard_layout(&app).widgets.center_map;
        let popup = app.current_overlay_popup_rect(map_frame).unwrap();
        // List pane is on the left side of the body.
        let col = popup.x + 3;
        let row = popup.y + 5;

        // Scroll up (positive lines) → selected decreases.
        app.dispatch_mouse_event_for_repro(MouseEvent {
            kind: MouseEventKind::Scroll { lines: 2 },
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(app.inbox_overlay.selected, 0);
        assert_eq!(app.inbox_overlay.scroll, 0);
        assert_eq!(app.inbox_overlay.preview_scroll, 5); // unchanged

        // Scroll down (negative lines) → selected increases.
        app.dispatch_mouse_event_for_repro(MouseEvent {
            kind: MouseEventKind::Scroll { lines: -1 },
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(app.inbox_overlay.selected, 1);
    }

    #[test]
    fn mouse_wheel_in_inbox_preview_scrolls_preview() {
        let mut app = dash_app();
        app.report_block_rows = vec![
            ReportBlockRow {
                viewer_empire_id: 0,
                block_index: 0,
                decoded_text: "Stardate: 03/3012\nLine one.".to_string(),
                raw_bytes: None,
                recipient_deleted: false,
            },
        ];
        app.queued_mail = vec![QueuedPlayerMail {
            sender_empire_id: 2,
            recipient_empire_id: 1,
            year: 3012,
            subject: "Test subject".to_string(),
            body: "Test body with enough length to create a preview.".to_string(),
            recipient_deleted: false,
        }];
        app.overlay = ActiveOverlay::Inbox;
        app.inbox_overlay.selected = 0;
        app.inbox_overlay.scroll = 0;
        app.inbox_overlay.preview_scroll = 0;

        let map_frame = dashboard_layout(&app).widgets.center_map;
        let popup = app.current_overlay_popup_rect(map_frame).unwrap();
        // Preview pane is on the right side of the body.
        let col = popup.x + popup.width - 3;
        let row = popup.y + 5;

        app.dispatch_mouse_event_for_repro(MouseEvent {
            kind: MouseEventKind::Scroll { lines: -2 },
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        });

        // First item has 2-line body; max preview scroll is 1.
        assert_eq!(app.inbox_overlay.preview_scroll, 1);
        assert_eq!(app.inbox_overlay.selected, 0); // unchanged
    }

    #[test]
    fn mouse_wheel_in_planet_list_overlay_moves_selection() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::PlanetList;
        app.planet_overlay.selected = 3;
        app.planet_overlay.scroll = 0;

        let map_frame = dashboard_layout(&app).widgets.center_map;
        let popup = app.current_overlay_popup_rect(map_frame).unwrap();
        let col = popup.x + 3;
        let row = popup.y + 3;

        app.dispatch_mouse_event_for_repro(MouseEvent {
            kind: MouseEventKind::Scroll { lines: 2 },
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(app.planet_overlay.selected, 1);
        assert_eq!(app.planet_overlay.scroll, 0);
    }

    #[test]
    fn mouse_wheel_ignored_when_planet_prompt_open() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::PlanetList;
        app.planet_overlay.selected = 3;
        app.planet_overlay.scroll = 0;
        app.planet_overlay.open_prompt(PlanetOverlayPromptMode::SortMenu);

        let map_frame = dashboard_layout(&app).widgets.center_map;
        let popup = app.current_overlay_popup_rect(map_frame).unwrap();
        let col = popup.x + 3;
        let row = popup.y + 3;

        app.dispatch_mouse_event_for_repro(MouseEvent {
            kind: MouseEventKind::Scroll { lines: -2 },
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(app.planet_overlay.selected, 3);
        assert_eq!(app.planet_overlay.scroll, 0);
    }

    #[test]
    fn mouse_wheel_in_fleet_list_overlay_moves_selection() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::FleetList;
        app.fleet_overlay.selected = 2;
        app.fleet_overlay.scroll = 0;

        let map_frame = dashboard_layout(&app).widgets.center_map;
        let popup = app.current_overlay_popup_rect(map_frame).unwrap();
        let col = popup.x + 3;
        let row = popup.y + 3;

        app.dispatch_mouse_event_for_repro(MouseEvent {
            kind: MouseEventKind::Scroll { lines: -1 },
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(app.fleet_overlay.selected, 3);
    }

    #[test]
    fn mouse_wheel_in_intel_overlay_moves_selection() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::IntelDatabase;
        app.intel_overlay.selected = 1;
        app.intel_overlay.scroll = 0;

        let map_frame = dashboard_layout(&app).widgets.center_map;
        let popup = app.current_overlay_popup_rect(map_frame).unwrap();
        let col = popup.x + 3;
        let row = popup.y + 3;

        app.dispatch_mouse_event_for_repro(MouseEvent {
            kind: MouseEventKind::Scroll { lines: -1 },
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(app.intel_overlay.selected, 2);
    }
}
