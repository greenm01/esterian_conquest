//! App struct, main loop, and action dispatch.

pub mod input;
pub mod render;
pub mod state;

use crossterm::event::{KeyCode, KeyEvent};
use state::{ActiveOverlay, DashApp};
use input::{Action, key_to_action};
use nc_ui::Terminal;

use crate::inbox::{DashInboxItemSource, project_inbox_items};

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
            Action::OpenOverlay(overlay) => self.overlay = overlay,
            Action::CloseOverlay => self.overlay = ActiveOverlay::None,
            Action::MoveCrosshairUp => {
                // Up arrow → higher Y (row 18 at top of screen).
                let map_size = nc_data::map_size_for_player_count(
                    self.game_data.conquest.player_count(),
                );
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
                let map_size = nc_data::map_size_for_player_count(
                    self.game_data.conquest.player_count(),
                );
                if self.crosshair_x < map_size {
                    self.crosshair_x += 1;
                }
            }
            Action::ScrollUp => self.scroll_up(),
            Action::ScrollDown => self.scroll_down(),
            Action::PageUp => {
                for _ in 0..10 { self.scroll_up(); }
            }
            Action::PageDown => {
                for _ in 0..10 { self.scroll_down(); }
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
                if key.code == KeyCode::Esc {
                    self.overlay = ActiveOverlay::None;
                    return true;
                }
                handle_list_overlay_key(key, &mut self.planet_overlay, 1_000);
                true
            }
            ActiveOverlay::FleetList => {
                if key.code == KeyCode::Esc {
                    self.overlay = ActiveOverlay::None;
                    return true;
                }
                handle_list_overlay_key(key, &mut self.fleet_overlay, 1_000);
                true
            }
            ActiveOverlay::IntelDatabase => {
                if key.code == KeyCode::Esc {
                    self.overlay = ActiveOverlay::None;
                    return true;
                }
                handle_list_overlay_key(key, &mut self.intel_overlay, 10_000);
                true
            }
            ActiveOverlay::Diplomacy => {
                if key.code == KeyCode::Esc {
                    self.overlay = ActiveOverlay::None;
                    return true;
                }
                handle_list_overlay_key(key, &mut self.diplomacy_overlay, 10_000);
                true
            }
            ActiveOverlay::Inbox => {
                if key.code == KeyCode::Esc {
                    self.overlay = ActiveOverlay::None;
                    return true;
                }
                self.handle_inbox_overlay_key(key);
                true
            }
            ActiveOverlay::Settings | ActiveOverlay::Help => {
                if matches!(key.code, KeyCode::Esc | KeyCode::Char('?')) {
                    self.overlay = ActiveOverlay::None;
                    return true;
                }
                true
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

        let state = &mut self.inbox_overlay;
        match key.code {
            KeyCode::Tab => {
                state.focus = match state.focus {
                    state::InboxFocus::List => state::InboxFocus::Preview,
                    state::InboxFocus::Preview => state::InboxFocus::List,
                };
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                state.filter = state::InboxFilter::All;
                state.selected = 0;
                state.scroll = 0;
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                state.filter = state::InboxFilter::Reports;
                state.selected = 0;
                state.scroll = 0;
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                state.filter = state::InboxFilter::Messages;
                state.selected = 0;
                state.scroll = 0;
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                state.current_year_only = !state.current_year_only;
                state.selected = 0;
                state.scroll = 0;
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                state.delete_confirm = true;
            }
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                state.jump_input.push(ch);
                if let Ok(index) = state.jump_input.parse::<usize>() {
                    if index > 0 {
                        state.selected = index - 1;
                        if state.selected < state.scroll {
                            state.scroll = state.selected;
                        }
                    }
                }
            }
            KeyCode::Backspace => {
                state.jump_input.pop();
            }
            KeyCode::Up | KeyCode::Char('k') => match state.focus {
                state::InboxFocus::List => {
                    state.selected = state.selected.saturating_sub(1);
                    if state.selected < state.scroll {
                        state.scroll = state.selected;
                    }
                }
                state::InboxFocus::Preview => {
                    state.preview_scroll = state.preview_scroll.saturating_sub(1);
                }
            },
            KeyCode::Down | KeyCode::Char('j') => match state.focus {
                state::InboxFocus::List => {
                    state.selected += 1;
                }
                state::InboxFocus::Preview => {
                    state.preview_scroll += 1;
                }
            },
            KeyCode::PageUp => match state.focus {
                state::InboxFocus::List => {
                    state.selected = state.selected.saturating_sub(10);
                    state.scroll = state.scroll.saturating_sub(10);
                }
                state::InboxFocus::Preview => {
                    state.preview_scroll = state.preview_scroll.saturating_sub(10);
                }
            },
            KeyCode::PageDown => match state.focus {
                state::InboxFocus::List => {
                    state.selected += 10;
                }
                state::InboxFocus::Preview => {
                    state.preview_scroll += 10;
                }
            },
            KeyCode::Home => match state.focus {
                state::InboxFocus::List => {
                    state.selected = 0;
                    state.scroll = 0;
                }
                state::InboxFocus::Preview => {
                    state.preview_scroll = 0;
                }
            },
            KeyCode::End => {
                if matches!(state.focus, state::InboxFocus::List) {
                    state.selected = usize::MAX / 4;
                    state.scroll = state.selected.saturating_sub(5);
                } else {
                    state.preview_scroll = usize::MAX / 4;
                }
            }
            _ => {}
        }
    }
}

fn handle_list_overlay_key(
    key: KeyEvent,
    state: &mut state::ListOverlayState,
    end_marker: usize,
) {
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
