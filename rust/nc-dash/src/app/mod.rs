//! App struct, main loop, and action dispatch.

pub mod input;
pub mod render;
pub mod state;

use state::{ActiveOverlay, DashApp};
use input::{Action, key_to_action};
use nc_ui::Terminal;

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
                if self.crosshair_y > 1 {
                    self.crosshair_y -= 1;
                }
            }
            Action::MoveCrosshairDown => {
                // Map height is 18 for the smallest map, up to 36.
                // We clamp to 36 as a safe upper bound without knowing map size.
                if self.crosshair_y < 36 {
                    self.crosshair_y += 1;
                }
            }
            Action::MoveCrosshairLeft => {
                if self.crosshair_x > 1 {
                    self.crosshair_x -= 1;
                }
            }
            Action::MoveCrosshairRight => {
                if self.crosshair_x < 36 {
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
            Action::SetTaxRate | Action::GotoCoords | Action::None => {}
        }
    }

    fn scroll_up(&mut self) {
        use state::PanelFocus::*;
        match self.focus {
            Planets => self.planets_scroll = self.planets_scroll.saturating_sub(1),
            Fleets => self.fleets_scroll = self.fleets_scroll.saturating_sub(1),
            Reports => self.reports_scroll = self.reports_scroll.saturating_sub(1),
            Diplomacy => self.diplomacy_scroll = self.diplomacy_scroll.saturating_sub(1),
            _ => {}
        }
    }

    fn scroll_down(&mut self) {
        use state::PanelFocus::*;
        match self.focus {
            Planets => self.planets_scroll += 1,
            Fleets => self.fleets_scroll += 1,
            Reports => self.reports_scroll += 1,
            Diplomacy => self.diplomacy_scroll += 1,
            _ => {}
        }
    }

    fn scroll_home(&mut self) {
        use state::PanelFocus::*;
        match self.focus {
            Planets => self.planets_scroll = 0,
            Fleets => self.fleets_scroll = 0,
            Reports => self.reports_scroll = 0,
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
            Reports => self.reports_scroll = usize::MAX,
            Diplomacy => self.diplomacy_scroll = usize::MAX,
            _ => {}
        }
    }
}
