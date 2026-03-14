use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::screen::layout::write_prompt;
use crate::screen::{Screen, ScreenFrame};
use crate::terminal::Terminal;
use crate::theme::classic;

pub struct GeneralMenuScreen;

impl GeneralMenuScreen {
    pub fn new() -> Self {
        Self
    }
}

impl Screen for GeneralMenuScreen {
    fn render(
        &mut self,
        terminal: &mut dyn Terminal,
        _frame: &ScreenFrame<'_>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        terminal.clear()?;
        terminal.write_line(&general_title_row())?;
        terminal.write_line(&general_row_exact(
            ("H", "elp with commands"),
            ("A", "utopilot ON/OFF"),
            ("R", "eview messages/results"),
        ))?;
        terminal.write_line(&general_row_exact(
            ("Q", "uit to main menu"),
            ("S", "tatus, your"),
            ("D", "elete ALL messages/results"),
        ))?;
        terminal.write_line(&general_row_exact(
            ("X", "pert mode ON/OFF"),
            ("P", "rofile of your empire"),
            ("O", "ther empires (rankings)"),
        ))?;
        terminal.write_line(&general_row_exact(
            ("V", "iew Partial Starmap"),
            ("M", "ap of the galaxy"),
            ("E", "nemies, declare or list"),
        ))?;
        terminal.write_line("")?;
        write_prompt(
            terminal,
            6,
            &classic::command_prompt("GENERAL COMMAND", "H Q X V I A S P M C R D O E"),
        )?;
        terminal.flush()
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('r') | KeyCode::Char('R') => Action::OpenReports,
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenMainMenu,
            _ => Action::Noop,
        }
    }
}

fn general_title_row() -> String {
    let title = "\x1b[0;38;2;0;0;0;48;2;224;224;224mGENERAL COMMAND CENTER:";
    let left = pad_visible(&styled_menu_item("I", "nfo about a Planet"), 28);
    let right = pad_visible(&styled_menu_item("C", "ommunicate (send message)"), 30);
    format!(
        "{title}\x1b[0;38;2;224;224;224;48;2;0;0;170m  {left}     {right}\x1b[0;38;2;192;192;192;48;2;0;0;0m",
        title = title,
        left = left,
        right = right
    )
}

fn general_row_exact(
    a: (&str, &str),
    b: (&str, &str),
    c: (&str, &str),
) -> String {
    let a = pad_visible(&styled_menu_item(a.0, a.1), 22);
    let b = pad_visible(&styled_menu_item(b.0, b.1), 28);
    let c = pad_visible(&styled_menu_item(c.0, c.1), 26);
    format!(
        "\x1b[0;38;2;224;224;224;48;2;0;0;170m  {a}  {b}{c}\x1b[0;38;2;192;192;192;48;2;0;0;0m",
        a = a,
        b = b,
        c = c
    )
}

fn styled_menu_item(hotkey: &str, label: &str) -> String {
    format!(
        "\x1b[1;38;2;255;255;85;48;2;0;0;170m{hotkey}\x1b[0;38;2;224;224;224;48;2;0;0;170m>{label}"
    )
}

fn pad_visible(text: &str, width: usize) -> String {
    let visible = visible_width(text);
    let padding = width.saturating_sub(visible);
    format!("{text}{}", " ".repeat(padding))
}

fn visible_width(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut idx = 0;
    let mut width = 0;
    while idx < bytes.len() {
        if bytes[idx] == 0x1b {
            idx += 1;
            if idx < bytes.len() && bytes[idx] == b'[' {
                idx += 1;
                while idx < bytes.len() {
                    let byte = bytes[idx];
                    idx += 1;
                    if byte.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        width += 1;
        idx += 1;
    }
    width
}
