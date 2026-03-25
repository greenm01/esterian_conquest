use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::domains::planet::PlanetAction;
use crate::domains::starmap::StarmapAction;
use crate::screen::layout::{
    EXPERT_MENU_PROMPT_ROW, MenuEntry, draw_command_prompt_at, draw_expert_menu,
    draw_inline_confirm_block, draw_inline_confirm_prompt, draw_inline_planet_info_prompt,
    draw_inline_tax_prompt, draw_menu_entry, draw_menu_notice, draw_title_bar, menu_prompt_row,
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

const TOP_ROW: [MenuEntry<'static>; 1] = [MenuEntry::new(PLANET_COL_4, "T", "ax rate: Empire")];

const ROW_1: [MenuEntry<'static>; 4] = [
    MenuEntry::new(PLANET_COL_1, "H", "elp on Options"),
    MenuEntry::new(PLANET_COL_2, "C", "OMMISSION MENU"),
    MenuEntry::new(PLANET_COL_3, "V", "iew Partial Map"),
    MenuEntry::new(PLANET_COL_4, "S", "corch planets"),
];

const ROW_2: [MenuEntry<'static>; 4] = [
    MenuEntry::new(PLANET_COL_1, "Q", "uit: Main Menu"),
    MenuEntry::new(PLANET_COL_2, "A", "UTO-COMMISSION"),
    MenuEntry::new(PLANET_COL_3, "P", "lanet List"),
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
        expert_mode: bool,
        inline_planet_info: bool,
        info_default_coords: [u8; 2],
        info_input: &str,
        info_notice: Option<&str>,
        inline_tax: bool,
        current_tax: &str,
        tax_input: &str,
        tax_error: Option<&str>,
        tax_notice: Option<&str>,
        inline_auto_commission: bool,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        if expert_mode {
            if inline_planet_info {
                draw_inline_planet_info_prompt(
                    &mut buffer,
                    EXPERT_MENU_PROMPT_ROW,
                    info_default_coords,
                    info_input,
                    info_notice,
                    notice,
                );
            } else if inline_tax {
                draw_inline_tax_prompt(
                    &mut buffer,
                    EXPERT_MENU_PROMPT_ROW,
                    current_tax,
                    tax_input,
                    tax_error,
                    tax_notice,
                );
            } else if inline_auto_commission {
                draw_inline_confirm_prompt(&mut buffer, EXPERT_MENU_PROMPT_ROW, "COMMAND");
                draw_inline_confirm_block(
                    &mut buffer,
                    EXPERT_MENU_PROMPT_ROW,
                    "AUTO-COMMISSION SHIPS:",
                    &["Automatically commission all ships and starbases in stardock?"],
                    notice,
                );
            } else {
                draw_expert_menu(
                    &mut buffer,
                    "PLANET COMMAND",
                    "H,Q,X,V,C,A,B,I,P,T,S,L,U",
                    notice,
                );
            }
            return Ok(buffer);
        }
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
                if entry.hotkey.trim().is_empty() {
                    continue;
                }
                draw_menu_entry(
                    &mut buffer,
                    row_idx + 1,
                    entry.col,
                    entry.hotkey,
                    entry.label,
                );
            }
        }
        let command_row = menu_prompt_row(3);
        if inline_planet_info {
            draw_inline_planet_info_prompt(
                &mut buffer,
                command_row,
                info_default_coords,
                info_input,
                info_notice,
                notice,
            );
        } else if inline_tax {
            draw_inline_tax_prompt(
                &mut buffer,
                command_row,
                current_tax,
                tax_input,
                tax_error,
                tax_notice,
            );
        } else if inline_auto_commission {
            draw_inline_confirm_prompt(&mut buffer, command_row, "COMMAND");
            draw_inline_confirm_block(
                &mut buffer,
                command_row,
                "AUTO-COMMISSION SHIPS:",
                &["Automatically commission all ships and starbases in stardock?"],
                notice,
            );
        } else if let Some(notice) = notice {
            draw_menu_notice(&mut buffer, command_row, notice);
        }
        if !inline_planet_info && !inline_tax && !inline_auto_commission {
            draw_command_prompt_at(
                &mut buffer,
                command_row,
                "PLANET COMMAND",
                "H,Q,X,V,C,A,B,I,P,T,S,L,U",
            );
        }
        Ok(buffer)
    }
}

impl Screen for PlanetMenuScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_with_notice(
            None,
            false,
            false,
            [0, 0],
            "",
            None,
            false,
            "0",
            "",
            None,
            None,
            false,
        )
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('h') | KeyCode::Char('H') => Action::Planet(PlanetAction::OpenHelp),
            KeyCode::Char('v') | KeyCode::Char('V') => {
                Action::Starmap(StarmapAction::OpenPartialView(CommandMenu::Planet))
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
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenMainMenu,
            KeyCode::Char('b') | KeyCode::Char('B') => Action::Planet(PlanetAction::OpenBuildMenu),
            KeyCode::Char('c') | KeyCode::Char('C') => {
                Action::Planet(PlanetAction::OpenCommissionMenu)
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                Action::Planet(PlanetAction::OpenAutoCommissionPrompt)
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                Action::Planet(PlanetAction::OpenListSortPrompt(PlanetListMode::Stub(
                    planet_stub_label(key.code).unwrap_or(""),
                )))
            }
            KeyCode::Char('x') | KeyCode::Char('X') => Action::ToggleExpertMode,
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
        _ => None,
    }
}
