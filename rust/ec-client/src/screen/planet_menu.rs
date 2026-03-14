use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::screen::layout::{
    CMD_COL_1, CMD_COL_2, CMD_COL_3, MenuEntry, draw_command_center, new_playfield,
};
use crate::screen::{CommandMenu, PlanetListMode, PlanetListSort, PlayfieldBuffer, Screen, ScreenFrame};

pub struct PlanetMenuScreen;

const TOP_ROW: [MenuEntry<'static>; 2] = [
    MenuEntry::new(CMD_COL_2, "V", "iew Partial Map"),
    MenuEntry::new(CMD_COL_3, "T", "ax rate: Empire"),
];

const ROW_1: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "H", "elp on Options"),
    MenuEntry::new(CMD_COL_2, "C", "OMMISSION MENU"),
    MenuEntry::new(CMD_COL_3, "D", "etail Planet List"),
];

const ROW_2: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "Q", "uit: Main Menu"),
    MenuEntry::new(CMD_COL_2, "A", "UTO-COMMISSION"),
    MenuEntry::new(CMD_COL_3, "P", "lanet: Brief List"),
];

const ROW_3: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "X", "pert mode"),
    MenuEntry::new(CMD_COL_2, "B", "UILD MENU..."),
    MenuEntry::new(CMD_COL_3, "I", "nfo about Planet"),
];

const ROW_4: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "S", "corch planets"),
    MenuEntry::new(CMD_COL_2, "L", "oad TTs w/Armies"),
    MenuEntry::new(CMD_COL_3, "U", "nload TT Armies"),
];

impl PlanetMenuScreen {
    pub fn new() -> Self {
        Self
    }
}

impl Screen for PlanetMenuScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_command_center(
            &mut buffer,
            "PLANET COMMAND CENTER:",
            &TOP_ROW,
            &[&ROW_1, &ROW_2, &ROW_3, &ROW_4],
            "PLANET COMMAND",
            "H Q X V C A B I D P T S L U",
        );
        Ok(buffer)
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('h') | KeyCode::Char('H') => Action::OpenPlanetHelp,
            KeyCode::Char('v') | KeyCode::Char('V') => {
                Action::OpenPartialStarmapPrompt(CommandMenu::Planet)
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                Action::OpenPlanetInfoPrompt(CommandMenu::Planet)
            }
            KeyCode::Char('p') | KeyCode::Char('P') => Action::SubmitPlanetListSort(
                PlanetListMode::Brief,
                PlanetListSort::CurrentProduction,
            ),
            KeyCode::Char('d') | KeyCode::Char('D') => Action::SubmitPlanetListSort(
                PlanetListMode::Detail,
                PlanetListSort::CurrentProduction,
            ),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenMainMenu,
            KeyCode::Char('c') | KeyCode::Char('C')
            | KeyCode::Char('a') | KeyCode::Char('A')
            | KeyCode::Char('b') | KeyCode::Char('B')
            | KeyCode::Char('s') | KeyCode::Char('S')
            | KeyCode::Char('l') | KeyCode::Char('L')
            | KeyCode::Char('u') | KeyCode::Char('U')
            | KeyCode::Char('x') | KeyCode::Char('X') => Action::OpenPlanetListSortPrompt(
                PlanetListMode::Stub(planet_stub_label(key.code).unwrap_or("")),
            ),
            KeyCode::Char('t') | KeyCode::Char('T') => Action::OpenPlanetTaxPrompt,
            _ => Action::Noop,
        }
    }
}

fn planet_stub_label(code: KeyCode) -> Option<&'static str> {
    match code {
        KeyCode::Char('c') | KeyCode::Char('C') => Some("Commission menu not implemented yet."),
        KeyCode::Char('a') | KeyCode::Char('A') => Some("Auto-commission is not implemented yet."),
        KeyCode::Char('b') | KeyCode::Char('B') => Some("Build menu is not implemented yet."),
        KeyCode::Char('s') | KeyCode::Char('S') => Some("Scorch-planet orders are not implemented yet."),
        KeyCode::Char('l') | KeyCode::Char('L') => Some("Transport loading is not implemented yet."),
        KeyCode::Char('u') | KeyCode::Char('U') => Some("Transport unloading is not implemented yet."),
        KeyCode::Char('x') | KeyCode::Char('X') => Some("Expert mode will follow after all command menus are finished."),
        _ => None,
    }
}
