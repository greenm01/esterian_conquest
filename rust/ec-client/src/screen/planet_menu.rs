use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::screen::layout::{
    MenuEntry, draw_command_prompt, draw_menu_entry, draw_status_line, draw_title_bar,
    new_playfield,
};
use crate::screen::{
    CommandMenu, PlanetListMode, PlanetListSort, PlanetTransportMode, PlayfieldBuffer, Screen,
    ScreenFrame,
};
use crate::theme::classic;

pub struct PlanetMenuScreen;

const PLANET_COL_1: usize = 2;
const PLANET_COL_2: usize = 20;
const PLANET_COL_3: usize = 39;
const PLANET_COL_4: usize = 60;

const TOP_ROW: [MenuEntry<'static>; 2] = [
    MenuEntry::new(PLANET_COL_2, "V", "iew Partial Map"),
    MenuEntry::new(PLANET_COL_4, "T", "ax rate: Empire"),
];

const ROW_1: [MenuEntry<'static>; 4] = [
    MenuEntry::new(PLANET_COL_1, "H", "elp on Options"),
    MenuEntry::new(PLANET_COL_2, "C", "OMMISSION MENU"),
    MenuEntry::new(PLANET_COL_3, "D", "etail Planet List"),
    MenuEntry::new(PLANET_COL_4, "S", "corch planets"),
];

const ROW_2: [MenuEntry<'static>; 4] = [
    MenuEntry::new(PLANET_COL_1, "Q", "uit: Main Menu"),
    MenuEntry::new(PLANET_COL_2, "A", "UTO-COMMISSION"),
    MenuEntry::new(PLANET_COL_3, "P", "lanet: Brief List"),
    MenuEntry::new(PLANET_COL_4, "L", "oad TTs w/Armies"),
];

const ROW_3: [MenuEntry<'static>; 4] = [
    MenuEntry::new(PLANET_COL_1, "X", "pert mode"),
    MenuEntry::new(PLANET_COL_2, "B", "UILD MENU..."),
    MenuEntry::new(PLANET_COL_3, "I", "nfo about Planet"),
    MenuEntry::new(PLANET_COL_4, "U", "nload TT Armies"),
];

impl PlanetMenuScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_with_notice(
        &mut self,
        notice: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "PLANET COMMANDS:");
        for entry in TOP_ROW {
            draw_menu_entry(&mut buffer, 0, entry.col, entry.hotkey, entry.label);
        }
        for (row_idx, row) in [ROW_1.as_slice(), ROW_2.as_slice(), ROW_3.as_slice()]
            .into_iter()
            .enumerate()
        {
            buffer.fill_row(row_idx + 1, classic::menu_style());
            for entry in row {
                draw_menu_entry(&mut buffer, row_idx + 1, entry.col, entry.hotkey, entry.label);
            }
        }
        if let Some(notice) = notice {
            draw_status_line(&mut buffer, 16, "Notice: ", notice);
        }
        draw_command_prompt(
            &mut buffer,
            19,
            "PLANET COMMAND",
            "H,Q,X,V,C,A,B,I,D,P,T,S,L,U",
        );
        Ok(buffer)
    }
}

impl Screen for PlanetMenuScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_with_notice(None)
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
            KeyCode::Char('b') | KeyCode::Char('B') => Action::OpenPlanetBuildMenu,
            KeyCode::Char('c') | KeyCode::Char('C') => Action::OpenPlanetCommissionMenu,
            KeyCode::Char('a') | KeyCode::Char('A') => Action::OpenPlanetAutoCommissionConfirm,
            KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Char('x') | KeyCode::Char('X') => {
                Action::OpenPlanetListSortPrompt(PlanetListMode::Stub(
                    planet_stub_label(key.code).unwrap_or(""),
                ))
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                Action::OpenPlanetTransportPlanetSelect(PlanetTransportMode::Load)
            }
            KeyCode::Char('u') | KeyCode::Char('U') => {
                Action::OpenPlanetTransportPlanetSelect(PlanetTransportMode::Unload)
            }
            KeyCode::Char('t') | KeyCode::Char('T') => Action::OpenPlanetTaxPrompt,
            _ => Action::Noop,
        }
    }
}

fn planet_stub_label(code: KeyCode) -> Option<&'static str> {
    match code {
        KeyCode::Char('s') | KeyCode::Char('S') => {
            Some("Scorch-planet orders are not implemented yet.")
        }
        KeyCode::Char('x') | KeyCode::Char('X') => {
            Some("Expert mode will follow after all command menus are finished.")
        }
        _ => None,
    }
}
