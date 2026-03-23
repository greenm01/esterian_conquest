use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::domains::planet::PlanetAction;
use crate::domains::starmap::StarmapAction;
use crate::screen::layout::{
    MenuEntry, draw_command_prompt_at, draw_menu_entry, draw_title_bar, draw_wrapped_status,
    menu_prompt_row, new_playfield,
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
                draw_menu_entry(
                    &mut buffer,
                    row_idx + 1,
                    entry.col,
                    entry.hotkey,
                    entry.label,
                );
            }
        }
        let mut last_content_row = 3;
        if let Some(notice) = notice {
            let rows_used = draw_wrapped_status(&mut buffer, 16, 3, "Notice: ", notice);
            last_content_row = 16 + rows_used.saturating_sub(1);
        }
        draw_command_prompt_at(
            &mut buffer,
            menu_prompt_row(last_content_row),
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
            KeyCode::Char('h') | KeyCode::Char('H') => Action::Planet(PlanetAction::OpenHelp),
            KeyCode::Char('v') | KeyCode::Char('V') => {
                Action::Starmap(StarmapAction::OpenPartialPrompt(CommandMenu::Planet))
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                Action::Planet(PlanetAction::OpenInfoPrompt(CommandMenu::Planet))
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                Action::Planet(PlanetAction::SubmitListSort(
                    PlanetListMode::Brief,
                    PlanetListSort::CurrentProduction,
                ))
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                Action::Planet(PlanetAction::SubmitListSort(
                    PlanetListMode::Detail,
                    PlanetListSort::CurrentProduction,
                ))
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenMainMenu,
            KeyCode::Char('b') | KeyCode::Char('B') => Action::Planet(PlanetAction::OpenBuildMenu),
            KeyCode::Char('c') | KeyCode::Char('C') => {
                Action::Planet(PlanetAction::OpenCommissionMenu)
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                Action::Planet(PlanetAction::OpenAutoCommissionConfirm)
            }
            KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Char('x') | KeyCode::Char('X') => {
                Action::Planet(PlanetAction::OpenListSortPrompt(PlanetListMode::Stub(
                    planet_stub_label(key.code).unwrap_or(""),
                )))
            }
            KeyCode::Char('l') | KeyCode::Char('L') => Action::Planet(
                PlanetAction::OpenTransportPlanetSelect(PlanetTransportMode::Load),
            ),
            KeyCode::Char('u') | KeyCode::Char('U') => Action::Planet(
                PlanetAction::OpenTransportPlanetSelect(PlanetTransportMode::Unload),
            ),
            KeyCode::Char('t') | KeyCode::Char('T') => Action::Planet(PlanetAction::OpenTaxPrompt),
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
