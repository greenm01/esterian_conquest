use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::domains::empire::EmpireAction;
use crate::domains::fleet::FleetAction;
use crate::domains::planet::PlanetAction;
use crate::domains::starmap::StarmapAction;
use crate::quotes::{self, Quote};
use crate::screen::layout::{
    EXPERT_MENU_PROMPT_ROW, MenuEntry, PLAYFIELD_WIDTH, draw_command_prompt_at, draw_expert_menu,
    draw_inline_planet_info_prompt, draw_menu_notice, draw_menu_row, draw_title_bar, last_body_row,
    new_playfield, wrap_text,
};
use crate::screen::{CommandMenu, PlayfieldBuffer, Screen, ScreenFrame};
use crate::theme::classic;
use crate::util::Lcg;

const MENU_PROMPT_ROW: usize = 6;
/// Rows available for the quote display below the command line.
const QUOTE_FIRST_ROW: usize = 8;
const QUOTE_LAST_ROW: usize = last_body_row();

/// Compute how many rows a quote block occupies: wrapped text + blank + author.
fn quote_block_height(text_lines: usize) -> usize {
    text_lines + 1 + 1 // text + blank separator + author attribution
}

/// Left margin for quote text (one space from the edge).
const QUOTE_LEFT_COL: usize = 1;

pub struct MainMenuScreen {
    quotes: Vec<Quote>,
}

impl MainMenuScreen {
    pub fn new() -> Self {
        Self {
            quotes: quotes::load_quotes(),
        }
    }

    pub fn render_with_notice(
        &mut self,
        notice: Option<&str>,
        expert_mode: bool,
        inline_planet_info: bool,
        info_default_coords: [u8; 2],
        info_input: &str,
        info_notice: Option<&str>,
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
            } else {
                draw_expert_menu(
                    &mut buffer,
                    "MAIN COMMAND",
                    "H,Q,X,V,A,G,P,F,T,I,B,D",
                    notice,
                );
            }
            return Ok(buffer);
        }
        draw_title_bar(&mut buffer, 0, "MAIN MENU: ");
        draw_menu_row(
            &mut buffer,
            1,
            &[
                MenuEntry::new(2, "H", "elp with commands"),
                MenuEntry::new(24, "A", "nsi Theme"),
                MenuEntry::new(51, "T", "otal Planet Database"),
            ],
        );
        draw_menu_row(
            &mut buffer,
            2,
            &[
                MenuEntry::new(2, "Q", "uit back to BBS"),
                MenuEntry::new(24, "G", "ENERAL COMMAND MENU..."),
                MenuEntry::new(51, "I", "nfo about a Planet"),
            ],
        );
        draw_menu_row(
            &mut buffer,
            3,
            &[
                MenuEntry::new(2, "X", "pert mode ON/OFF"),
                MenuEntry::new(24, "P", "LANET COMMAND MENU..."),
                MenuEntry::new(51, "B", "rief Empire Report"),
            ],
        );
        draw_menu_row(
            &mut buffer,
            4,
            &[
                MenuEntry::new(2, "V", "iew Partial Map"),
                MenuEntry::new(24, "F", "LEET COMMAND MENU..."),
                MenuEntry::new(51, "D", "etailed Empire Report"),
            ],
        );
        if inline_planet_info {
            draw_inline_planet_info_prompt(
                &mut buffer,
                MENU_PROMPT_ROW,
                info_default_coords,
                info_input,
                info_notice,
                notice,
            );
        } else if let Some(notice) = notice {
            draw_command_prompt_at(
                &mut buffer,
                MENU_PROMPT_ROW,
                "MAIN COMMAND",
                "H,Q,X,V,A,G,P,F,T,I,B,D",
            );
            draw_menu_notice(&mut buffer, MENU_PROMPT_ROW, notice);
        } else {
            draw_command_prompt_at(
                &mut buffer,
                MENU_PROMPT_ROW,
                "MAIN COMMAND",
                "H,Q,X,V,A,G,P,F,T,I,B,D",
            );
            self.draw_quote(&mut buffer);
        }
        Ok(buffer)
    }

    fn draw_quote(&self, buffer: &mut PlayfieldBuffer) {
        if self.quotes.is_empty() {
            return;
        }

        let index = Lcg::from_time().next_usize() % self.quotes.len();
        let quote = &self.quotes[index];

        let max_text_width = PLAYFIELD_WIDTH - QUOTE_LEFT_COL - 1;
        let wrapped = wrap_text(&quote.text, max_text_width, max_text_width);
        let author_line = format!("-- {}", quote.author);

        let available_rows = QUOTE_LAST_ROW - QUOTE_FIRST_ROW + 1;
        let text_lines = if quote_block_height(wrapped.len()) > available_rows {
            available_rows.saturating_sub(2) // leave room for blank + author
        } else {
            wrapped.len()
        };
        let author_row = QUOTE_LAST_ROW;
        let start_row = author_row
            .saturating_sub(text_lines + 1)
            .max(QUOTE_FIRST_ROW);

        for (i, line) in wrapped.iter().take(text_lines).enumerate() {
            buffer.write_text(start_row + i, QUOTE_LEFT_COL, line, classic::quote_style());
        }
        buffer.write_text(
            author_row,
            QUOTE_LEFT_COL,
            &author_line,
            classic::quote_author_style(),
        );
    }
}

impl Screen for MainMenuScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_with_notice(None, false, false, [0, 0], "", None)
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('h') | KeyCode::Char('H') => Action::OpenMainHelp,
            KeyCode::Char('a') | KeyCode::Char('A') => {
                Action::Startup(crate::domains::startup::StartupAction::OpenThemePicker)
            }
            KeyCode::Char('x') | KeyCode::Char('X') => Action::ToggleExpertMode,
            KeyCode::Char('b') | KeyCode::Char('B') => Action::Empire(EmpireAction::OpenStatus),
            KeyCode::Char('d') | KeyCode::Char('D') => Action::Empire(EmpireAction::OpenProfile),
            KeyCode::Char('f') | KeyCode::Char('F') => Action::Fleet(FleetAction::OpenMenu),
            KeyCode::Char('g') | KeyCode::Char('G') => Action::OpenGeneralMenu,
            KeyCode::Char('i') | KeyCode::Char('I') => {
                Action::Planet(PlanetAction::OpenInfoPrompt(CommandMenu::Main))
            }
            KeyCode::Char('p') | KeyCode::Char('P') => Action::Planet(PlanetAction::OpenMenu),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::Quit,
            KeyCode::Char('t') | KeyCode::Char('T') => Action::Planet(PlanetAction::OpenDatabase),
            KeyCode::Char('v') | KeyCode::Char('V') => {
                Action::Starmap(StarmapAction::OpenPartialView(CommandMenu::Main))
            }
            _ => Action::Noop,
        }
    }
}
