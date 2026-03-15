use crossterm::event::KeyEvent;

use crate::app::Action;
use crate::screen::layout::{draw_help_panel, new_playfield};
use crate::screen::{PlayfieldBuffer, Screen, ScreenFrame};

pub struct FleetHelpScreen;

const HELP_LINES: [&str; 16] = [
    "<B> - display a brief list of your fleets with locations, destinations, etc.",
    "<C> - change a fleet's ROE, ID Number, or Speed of Travel.",
    "<D> - detach one or more starships from a fleet to form a new fleet",
    "<E> - calculate any fleet's travel time to any potential destination",
    "<F> - display a longer but detailed list of your fleets",
    "<G> - order a group of fleets; ex: g 1 2 3 4 m 0 orders fleets 1-4 on m0",
    "<H> - describe Fleet Command Center commands",
    "<I> - show Intelligence on what you know about any planet",
    "<L> - load one or more armies from a planet onto the transports of a fleet",
    "<M> - merge two fleets into a single fleet - also see merge mission under <O>",
    "<O> - allow you to assign orders to any fleet (ie, send it onto a mission)",
    "<Q> - quit the Fleet Command Center menu & returns you to the Main Menu",
    "<S> - bring up the Starbase Control menu",
    "<T> - transfer one or more starships from one fleet to another",
    "<U> - unload one or more armies from a fleet to a planet",
    "<V>/<X> - display a partial map / hide or show menus",
];

impl FleetHelpScreen {
    pub fn new() -> Self {
        Self
    }
}

impl Screen for FleetHelpScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_help_panel(
            &mut buffer,
            "FLEET COMMAND HELP:",
            "Help - Fleet Command Center command descriptions:",
            &HELP_LINES,
            "FLEET COMMAND",
        );
        Ok(buffer)
    }

    fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::OpenFleetMenu
    }
}
