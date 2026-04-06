//! App struct, main loop, and action dispatch.

pub mod input;
pub mod render;
pub mod state;

use crossterm::event::{KeyCode, KeyEvent};
use input::{Action, key_to_action};
use nc_ui::Terminal;
use nc_ui::table_selection;
use state::{ActiveOverlay, DashApp, HelpContext};

use crate::inbox::{DashInboxItemSource, project_inbox_items};
use crate::overlays::{fleet_list, inbox, intel_database, planet_list};

impl DashApp {
    /// Run the main event loop.
    pub fn run(&mut self, terminal: &mut dyn Terminal) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let playfield = render::render(self)?;
            terminal.render(&playfield)?;
            if self.should_quit {
                break;
            }
            let key = terminal.read_key()?;
            self.handle_key(key);
        }
        Ok(())
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        if self.overlay != ActiveOverlay::None && self.handle_overlay_key(key) {
            return;
        }
        let action = key_to_action(key, self.focus, self.overlay);
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
            // SetTaxRate and GotoCoords require multi-key input prompts.
            // These will be implemented as mini prompt states in a future phase.
            Action::SetTaxRate | Action::GotoCoords | Action::None => {}
        }
    }

    fn scroll_up(&mut self) {
        use state::PanelFocus::*;
        match self.focus {
            Planets => self.planets_scroll = self.planets_scroll.saturating_sub(1),
            Fleets => self.fleets_scroll = self.fleets_scroll.saturating_sub(1),
            Diplomacy => self.diplomacy_scroll = self.diplomacy_scroll.saturating_sub(1),
            _ => {}
        }
    }

    fn scroll_down(&mut self) {
        use state::PanelFocus::*;
        match self.focus {
            Planets => self.planets_scroll += 1,
            Fleets => self.fleets_scroll += 1,
            Diplomacy => self.diplomacy_scroll += 1,
            _ => {}
        }
    }

    fn scroll_home(&mut self) {
        use state::PanelFocus::*;
        match self.focus {
            Planets => self.planets_scroll = 0,
            Fleets => self.fleets_scroll = 0,
            Diplomacy => self.diplomacy_scroll = 0,
            _ => {}
        }
    }

    fn scroll_end(&mut self) {
        // End scrolling: set to a large number; render will clamp.
        use state::PanelFocus::*;
        match self.focus {
            Planets => self.planets_scroll = usize::MAX,
            Fleets => self.fleets_scroll = usize::MAX,
            Diplomacy => self.diplomacy_scroll = usize::MAX,
            _ => {}
        }
    }

    fn handle_overlay_key(&mut self, key: KeyEvent) -> bool {
        match self.overlay {
            ActiveOverlay::None => false,
            ActiveOverlay::PlanetList => {
                if self.handle_overlay_close_or_help(key, HelpContext::PlanetList) {
                    return true;
                }
                self.handle_planet_overlay_key(key);
                true
            }
            ActiveOverlay::FleetList => {
                if self.handle_overlay_close_or_help(key, HelpContext::FleetList) {
                    return true;
                }
                self.handle_fleet_overlay_key(key);
                true
            }
            ActiveOverlay::IntelDatabase => {
                if self.handle_overlay_close_or_help(key, HelpContext::IntelDatabase) {
                    return true;
                }
                self.handle_intel_overlay_key(key);
                true
            }
            ActiveOverlay::Diplomacy => {
                if self.handle_overlay_close_or_help(key, HelpContext::Diplomacy) {
                    return true;
                }
                handle_list_overlay_key(key, &mut self.diplomacy_overlay, 10_000);
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
                let context = match self.overlay {
                    ActiveOverlay::Settings => HelpContext::Settings,
                    ActiveOverlay::Help => self.help_context,
                    _ => HelpContext::Global,
                };
                if self.handle_overlay_close_or_help(key, context) {
                    return true;
                }
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
        match key.code {
            KeyCode::Char(ch)
                if self.planet_overlay.jump_input.len() < 16
                    && table_selection::is_coordinate_input_char(ch) =>
            {
                self.planet_overlay.jump_input.push(ch);
                if self.sync_planet_overlay_cursor_to_input() {
                    self.planet_overlay.jump_input.clear();
                }
            }
            KeyCode::Backspace => {
                self.planet_overlay.jump_input.pop();
            }
            _ => handle_list_overlay_key(key, &mut self.planet_overlay, 1_000),
        }
    }

    fn handle_fleet_overlay_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(ch)
                if self.fleet_overlay.jump_input.len() < 8 && ch.is_ascii_alphanumeric() =>
            {
                self.fleet_overlay.jump_input.push(ch);
                if self.sync_fleet_overlay_cursor_to_input() {
                    self.fleet_overlay.jump_input.clear();
                }
            }
            KeyCode::Backspace => {
                self.fleet_overlay.jump_input.pop();
            }
            _ => handle_list_overlay_key(key, &mut self.fleet_overlay, 1_000),
        }
    }

    fn handle_intel_overlay_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(ch)
                if self.intel_overlay.jump_input.len() < 16
                    && table_selection::is_coordinate_input_char(ch) =>
            {
                self.intel_overlay.jump_input.push(ch);
                if self.sync_intel_overlay_cursor_to_input() {
                    self.intel_overlay.jump_input.clear();
                }
            }
            KeyCode::Backspace => {
                self.intel_overlay.jump_input.pop();
            }
            _ => handle_list_overlay_key(key, &mut self.intel_overlay, 10_000),
        }
    }

    fn sync_planet_overlay_cursor_to_input(&mut self) -> bool {
        let rows = planet_list::selection_rows(self);
        let Some(matched) =
            table_selection::find_typed_jump(&rows, 0, &self.planet_overlay.jump_input)
        else {
            return false;
        };
        self.planet_overlay.selected = matched.index;
        sync_scroll_to_cursor(&mut self.planet_overlay.scroll, matched.index, 1_000);
        matched.is_terminal_exact_match
    }

    fn sync_fleet_overlay_cursor_to_input(&mut self) -> bool {
        let rows = fleet_list::selection_rows(self);
        let Some(matched) =
            table_selection::find_typed_jump(&rows, 0, &self.fleet_overlay.jump_input)
        else {
            return false;
        };
        self.fleet_overlay.selected = matched.index;
        sync_scroll_to_cursor(&mut self.fleet_overlay.scroll, matched.index, 1_000);
        matched.is_terminal_exact_match
    }

    fn sync_intel_overlay_cursor_to_input(&mut self) -> bool {
        let rows = intel_database::selection_rows(self);
        let Some(matched) =
            table_selection::find_typed_jump(&rows, 0, &self.intel_overlay.jump_input)
        else {
            return false;
        };
        self.intel_overlay.selected = matched.index;
        sync_scroll_to_cursor(&mut self.intel_overlay.scroll, matched.index, 10_000);
        matched.is_terminal_exact_match
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
                    self.inbox_overlay.selected = self.inbox_overlay.selected.saturating_sub(1);
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
                    self.inbox_overlay.selected += 1;
                }
                state::InboxFocus::Preview => {
                    self.inbox_overlay.preview_scroll += 1;
                }
            },
            KeyCode::PageUp => match self.inbox_overlay.focus {
                state::InboxFocus::List => {
                    self.inbox_overlay.selected = self.inbox_overlay.selected.saturating_sub(10);
                    self.inbox_overlay.scroll = self.inbox_overlay.scroll.saturating_sub(10);
                }
                state::InboxFocus::Preview => {
                    self.inbox_overlay.preview_scroll =
                        self.inbox_overlay.preview_scroll.saturating_sub(10);
                }
            },
            KeyCode::PageDown => match self.inbox_overlay.focus {
                state::InboxFocus::List => {
                    self.inbox_overlay.selected += 10;
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
                    self.inbox_overlay.selected = usize::MAX / 4;
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

fn handle_list_overlay_key(key: KeyEvent, state: &mut state::ListOverlayState, end_marker: usize) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            state.selected = state.selected.saturating_sub(1);
            if state.selected < state.scroll {
                state.scroll = state.selected;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.selected += 1;
        }
        KeyCode::PageUp => {
            state.selected = state.selected.saturating_sub(10);
            state.scroll = state.scroll.saturating_sub(10);
        }
        KeyCode::PageDown => {
            state.selected += 10;
        }
        KeyCode::Home => {
            state.selected = 0;
            state.scroll = 0;
        }
        KeyCode::End => {
            state.selected = end_marker;
            state.scroll = end_marker.saturating_sub(10);
        }
        _ => {}
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
