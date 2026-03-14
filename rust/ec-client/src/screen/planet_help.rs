use crossterm::event::KeyEvent;

use crate::app::Action;
use crate::screen::layout::{draw_help_panel, new_playfield};
use crate::screen::{PlayfieldBuffer, Screen, ScreenFrame};

pub struct PlanetHelpScreen;

const HELP_LINES: [&str; 13] = [
    "<A> - automatically commission armies and batteries for your planets",
    "<B> - open the build menu to spend production on local construction",
    "<C> - open the commission menu for fine-grained ground-defense control",
    "<D> - display a detailed list of your planets and their economies",
    "<H> - describe Planet Command options",
    "<I> - display information you know about any planet",
    "<L> - load transport fleets with armies from the selected planet",
    "<P> - display a brief list of your planets",
    "<Q> - quit Planet Command and return to the Main Menu",
    "<S> - scorch your own planets as a last-resort denial order",
    "<T> - set the empire-wide tax rate used for yearly revenue",
    "<U> - unload transport fleets with armies at a planet",
    "<V> - display a partial starmap centered where you choose",
];

impl PlanetHelpScreen {
    pub fn new() -> Self {
        Self
    }
}

impl Screen for PlanetHelpScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_help_panel(
            &mut buffer,
            "PLANET COMMAND HELP:",
            "Help - Planet Command option descriptions:",
            &HELP_LINES,
            "PLANET COMMAND",
        );
        Ok(buffer)
    }

    fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::OpenPlanetMenu
    }
}
