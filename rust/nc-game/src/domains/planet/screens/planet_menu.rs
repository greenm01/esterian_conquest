use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::domains::planet::PlanetAction;
use crate::domains::planet::state::PlanetScorchPromptMode;
use crate::domains::starmap::StarmapAction;
use crate::screen::layout::{
    EXPERT_MENU_PROMPT_ROW, LEFT_WINDOW_PAD_COL, MenuEntry, PRIMARY_MENU_ROW,
    PRIMARY_MENU_TITLE_COL, draw_command_line_default_input_padded,
    draw_command_line_prompt_text_padded, draw_command_prompt_padded, draw_expert_menu_padded,
    draw_inline_confirm_block_padded, draw_inline_planet_info_prompt_padded,
    draw_inline_tax_prompt_padded, draw_menu_entry_item, draw_menu_notice_padded,
    draw_prompt_error_after_padded, draw_title_bar_at_col, draw_title_bar_padded, menu_prompt_row,
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
    MenuEntry::new(PLANET_COL_2, "M", "ASS-COMMISSION"),
    MenuEntry::featured(PLANET_COL_3, "P", "LANET LIST"),
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
        inline_scorch_mode: Option<PlanetScorchPromptMode>,
        scorch_warning_lines: &[String],
        menu_prompt_label: Option<&str>,
        menu_prompt_default: &str,
        menu_prompt_input: &str,
        menu_prompt_status: Option<&str>,
        inline_transport_mode: Option<PlanetTransportMode>,
        inline_transport_summary: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        if expert_mode {
            if inline_planet_info {
                draw_inline_planet_info_prompt_padded(
                    &mut buffer,
                    EXPERT_MENU_PROMPT_ROW,
                    info_default_coords,
                    info_input,
                    info_notice,
                    notice,
                );
            } else if inline_tax {
                draw_inline_tax_prompt_padded(
                    &mut buffer,
                    EXPERT_MENU_PROMPT_ROW,
                    current_tax,
                    tax_input,
                    tax_error,
                    tax_notice,
                );
            } else if inline_auto_commission {
                draw_command_line_prompt_text_padded(
                    &mut buffer,
                    EXPERT_MENU_PROMPT_ROW,
                    "COMMAND",
                    "[Y]/N -> ",
                );
                draw_inline_confirm_block_padded(
                    &mut buffer,
                    EXPERT_MENU_PROMPT_ROW,
                    "MASS COMMISSION:",
                    &["Automatically commission all ships and starbases in stardock?"],
                    notice,
                );
            } else if matches!(
                inline_scorch_mode,
                Some(PlanetScorchPromptMode::Confirm1)
                    | Some(PlanetScorchPromptMode::Confirm2)
                    | Some(PlanetScorchPromptMode::Confirm3)
            ) {
                let scorch_refs = scorch_warning_lines
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>();
                draw_command_line_prompt_text_padded(
                    &mut buffer,
                    EXPERT_MENU_PROMPT_ROW,
                    "PLANET COMMAND",
                    menu_prompt_label.unwrap_or("Y/[N] -> "),
                );
                draw_inline_confirm_block_padded(
                    &mut buffer,
                    EXPERT_MENU_PROMPT_ROW,
                    "SETTING SCORCH-EARTH POLICY:",
                    &scorch_refs,
                    notice,
                );
            } else if let Some(mode) = inline_transport_mode {
                draw_title_bar_padded(&mut buffer, 6, mode.title());
                if let Some(summary) = inline_transport_summary {
                    buffer.write_text(
                        8,
                        LEFT_WINDOW_PAD_COL,
                        summary,
                        classic::status_value_style(),
                    );
                }
                const EXPERT_TRANSPORT_COMMAND_ROW: usize = 10;
                draw_command_line_default_input_padded(
                    &mut buffer,
                    EXPERT_TRANSPORT_COMMAND_ROW,
                    "PLANET COMMAND",
                    menu_prompt_label.unwrap_or("How many armies? "),
                    menu_prompt_default,
                    menu_prompt_input,
                );
                if let Some(status) = menu_prompt_status {
                    draw_prompt_error_after_padded(
                        &mut buffer,
                        EXPERT_TRANSPORT_COMMAND_ROW,
                        status,
                    );
                }
            } else if menu_prompt_label.is_some() {
                draw_command_line_default_input_padded(
                    &mut buffer,
                    EXPERT_MENU_PROMPT_ROW,
                    "PLANET COMMAND",
                    menu_prompt_label.unwrap_or("Command "),
                    menu_prompt_default,
                    menu_prompt_input,
                );
                if let Some(status) = menu_prompt_status {
                    draw_prompt_error_after_padded(&mut buffer, EXPERT_MENU_PROMPT_ROW, status);
                }
            } else {
                draw_expert_menu_padded(
                    &mut buffer,
                    "PLANET COMMAND",
                    "? X V C M B I P T S L U <Q>",
                    notice,
                );
            }
            return Ok(buffer);
        }
        draw_title_bar_at_col(
            &mut buffer,
            PRIMARY_MENU_ROW,
            PRIMARY_MENU_TITLE_COL,
            "PLANET COMMANDS:",
        );
        for entry in TOP_ROW {
            draw_menu_entry_item(&mut buffer, PRIMARY_MENU_ROW, entry);
        }
        for (row_idx, row) in [ROW_1.as_slice(), ROW_2.as_slice(), ROW_3.as_slice()]
            .into_iter()
            .enumerate()
        {
            buffer.fill_row(PRIMARY_MENU_ROW + row_idx + 1, classic::menu_style());
            for entry in row {
                if entry.hotkey.trim().is_empty() {
                    continue;
                }
                draw_menu_entry_item(&mut buffer, PRIMARY_MENU_ROW + row_idx + 1, *entry);
            }
        }
        let command_row = menu_prompt_row(PRIMARY_MENU_ROW + 3);
        if inline_planet_info {
            draw_inline_planet_info_prompt_padded(
                &mut buffer,
                command_row,
                info_default_coords,
                info_input,
                info_notice,
                notice,
            );
        } else if inline_tax {
            draw_inline_tax_prompt_padded(
                &mut buffer,
                command_row,
                current_tax,
                tax_input,
                tax_error,
                tax_notice,
            );
        } else if inline_auto_commission {
            draw_command_line_prompt_text_padded(&mut buffer, command_row, "COMMAND", "[Y]/N -> ");
            draw_inline_confirm_block_padded(
                &mut buffer,
                command_row,
                "MASS COMMISSION:",
                &["Automatically commission all ships and starbases in stardock?"],
                notice,
            );
        } else if matches!(
            inline_scorch_mode,
            Some(PlanetScorchPromptMode::Confirm1)
                | Some(PlanetScorchPromptMode::Confirm2)
                | Some(PlanetScorchPromptMode::Confirm3)
        ) {
            let scorch_refs = scorch_warning_lines
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            draw_command_line_prompt_text_padded(
                &mut buffer,
                command_row,
                "PLANET COMMAND",
                menu_prompt_label.unwrap_or("Y/[N] -> "),
            );
            draw_inline_confirm_block_padded(
                &mut buffer,
                command_row,
                "SETTING SCORCH-EARTH POLICY:",
                &scorch_refs,
                notice,
            );
        } else if let Some(mode) = inline_transport_mode {
            draw_title_bar_padded(&mut buffer, 5, mode.title());
            if let Some(summary) = inline_transport_summary {
                buffer.write_text(
                    7,
                    LEFT_WINDOW_PAD_COL,
                    summary,
                    classic::status_value_style(),
                );
            }
            const MENU_TRANSPORT_COMMAND_ROW: usize = 9;
            draw_command_line_default_input_padded(
                &mut buffer,
                MENU_TRANSPORT_COMMAND_ROW,
                "PLANET COMMAND",
                menu_prompt_label.unwrap_or("How many armies? "),
                menu_prompt_default,
                menu_prompt_input,
            );
            if let Some(status) = menu_prompt_status {
                draw_prompt_error_after_padded(&mut buffer, MENU_TRANSPORT_COMMAND_ROW, status);
            }
        } else if menu_prompt_label.is_some() {
            draw_command_line_default_input_padded(
                &mut buffer,
                command_row,
                "PLANET COMMAND",
                menu_prompt_label.unwrap_or("Command "),
                menu_prompt_default,
                menu_prompt_input,
            );
            if let Some(status) = menu_prompt_status {
                draw_prompt_error_after_padded(&mut buffer, command_row, status);
            }
        } else if let Some(notice) = notice {
            draw_menu_notice_padded(&mut buffer, command_row, notice);
        }
        if !inline_planet_info
            && !inline_tax
            && !inline_auto_commission
            && inline_scorch_mode.is_none()
            && menu_prompt_label.is_none()
            && inline_transport_mode.is_none()
        {
            draw_command_prompt_padded(
                &mut buffer,
                command_row,
                "PLANET COMMAND",
                "? X V C M B I P T S L U <Q>",
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
            None,
            &[],
            None,
            "",
            "",
            None,
            None,
            None,
        )
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('h') | KeyCode::Char('H') => Action::OpenPopupHelp,
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
            KeyCode::Char('m') | KeyCode::Char('M') => {
                Action::Planet(PlanetAction::OpenAutoCommissionPrompt)
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                Action::Planet(PlanetAction::OpenScorchPrompt)
            }
            KeyCode::Char('x') | KeyCode::Char('X') => Action::ToggleExpertMode,
            KeyCode::Char('l') | KeyCode::Char('L') => {
                Action::Planet(PlanetAction::OpenTransportPrompt(PlanetTransportMode::Load))
            }
            KeyCode::Char('u') | KeyCode::Char('U') => Action::Planet(
                PlanetAction::OpenTransportPrompt(PlanetTransportMode::Unload),
            ),
            KeyCode::Char('t') | KeyCode::Char('T') => Action::Planet(PlanetAction::OpenTaxPrompt),
            _ => Action::Noop,
        }
    }
}
