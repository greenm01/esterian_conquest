use crossterm::event::KeyEvent;

use crate::app::Action;
use crate::screen::layout::{draw_help_panel, new_playfield};
use crate::screen::{PlayfieldBuffer, Screen, ScreenFrame};

pub struct BuildHelpScreen;

const HELP_LINES: [&str; 13] = [
    "<S> - specify build orders using this planet's current-turn PP budget",
    "<L> - list units in the build queue and units already waiting in stardock",
    "<R> - review this planet's production, spending, and queued construction",
    "<C> - change to another owned planet for local build orders",
    "<N> - move to the next owned planet in the build cycle",
    "<A> - abort queued build orders on the current planet",
    "<Q> - return to the Build Command menu",
    "",
    "Build queue = work still in progress.  Those PP are already committed.",
    "Stardock   = completed ships and starbases waiting for commission.",
    "Armies and ground batteries do not enter stardock when they complete.",
    "If stardock is full, ship and starbase builds wait in queue until space opens.",
    "Use Commission to lift completed ships and starbases out of stardock.",
];

impl BuildHelpScreen {
    pub fn new() -> Self {
        Self
    }
}

impl Screen for BuildHelpScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_help_panel(
            &mut buffer,
            "BUILD COMMAND HELP:",
            "Help - Build Command option descriptions:",
            &HELP_LINES,
            "BUILD COMMAND",
        );
        Ok(buffer)
    }

    fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::OpenPlanetBuildMenu
    }
}
