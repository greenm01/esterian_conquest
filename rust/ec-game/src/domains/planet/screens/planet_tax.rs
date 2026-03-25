use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::domains::planet::PlanetAction;
use crate::screen::PlayfieldBuffer;
use crate::screen::layout::draw_inline_tax_prompt;

pub struct PlanetTaxScreen;

impl PlanetTaxScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_inline(
        &mut self,
        buffer: &mut PlayfieldBuffer,
        command_row: usize,
        current_tax: &str,
        input: &str,
        error: Option<&str>,
        notice: Option<&str>,
    ) {
        draw_inline_tax_prompt(buffer, command_row, current_tax, input, error, notice);
    }

    pub fn handle_inline_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Planet(PlanetAction::CloseTaxPrompt)
            }
            KeyCode::Enter => Action::Planet(PlanetAction::SubmitTax),
            KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceTaxInput),
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                Action::Planet(PlanetAction::AppendTaxChar(ch))
            }
            _ => Action::Noop,
        }
    }
}
