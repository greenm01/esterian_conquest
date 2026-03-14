use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::screen::layout::{
    CMD_COL_1, CMD_COL_2, CMD_COL_3, MenuEntry, draw_command_center, new_playfield,
};
use crate::screen::{PlayfieldBuffer, Screen, ScreenFrame};
use ec_data::EmpireProductionRankingSort;

pub struct GeneralMenuScreen;

const TOP_ROW: [MenuEntry<'static>; 2] = [
    MenuEntry::new(CMD_COL_2, "I", "nfo about a Planet"),
    MenuEntry::new(CMD_COL_3, "C", "ommunicate (send message)"),
];

const ROW_1: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "H", "elp with commands"),
    MenuEntry::new(CMD_COL_2, "A", "utopilot ON/OFF"),
    MenuEntry::new(CMD_COL_3, "R", "eview messages/results"),
];

const ROW_2: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "Q", "uit to main menu"),
    MenuEntry::new(CMD_COL_2, "S", "tatus, your"),
    MenuEntry::new(CMD_COL_3, "D", "elete ALL messages/results"),
];

const ROW_3: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "X", "pert mode ON/OFF"),
    MenuEntry::new(CMD_COL_2, "P", "rofile of your empire"),
    MenuEntry::new(CMD_COL_3, "O", "ther empires (rankings)"),
];

const ROW_4: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "V", "iew Partial Starmap"),
    MenuEntry::new(CMD_COL_2, "M", "ap of the galaxy"),
    MenuEntry::new(CMD_COL_3, "E", "nemies, declare or list"),
];

impl GeneralMenuScreen {
    pub fn new() -> Self {
        Self
    }
}

impl Screen for GeneralMenuScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_command_center(
            &mut buffer,
            "GENERAL COMMAND CENTER:",
            &TOP_ROW,
            &[&ROW_1, &ROW_2, &ROW_3, &ROW_4],
            "GENERAL COMMAND",
            "H Q X V I A S P M C R D O E",
        );
        Ok(buffer)
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('i') | KeyCode::Char('I') => Action::OpenPlanetInfoPrompt,
            KeyCode::Char('s') | KeyCode::Char('S') => Action::OpenEmpireStatus,
            KeyCode::Char('p') | KeyCode::Char('P') => Action::OpenEmpireProfile,
            KeyCode::Char('o') | KeyCode::Char('O') => {
                Action::OpenRankingsTable(EmpireProductionRankingSort::Production)
            }
            KeyCode::Char('r') | KeyCode::Char('R') => Action::OpenReports,
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenMainMenu,
            _ => Action::Noop,
        }
    }
}
