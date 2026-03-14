use std::io::{self, IsTerminal, Write};

use crate::screen::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};
use crate::screen::{CellStyle, PlayfieldBuffer};
use crate::terminal::Terminal;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyEvent},
    execute,
    style::{Color, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};

pub struct StdoutTerminal;

impl StdoutTerminal {
    pub fn new() -> Self {
        Self
    }
}

impl Terminal for StdoutTerminal {
    fn render(&mut self, playfield: &PlayfieldBuffer) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = io::stdout();
        if stdout.is_terminal() {
            let (term_width, term_height) = terminal::size()?;
            let offset_x = term_width.saturating_sub(PLAYFIELD_WIDTH as u16) / 2;
            let offset_y = term_height.saturating_sub(PLAYFIELD_HEIGHT as u16) / 2;
            execute!(
                stdout,
                SetBackgroundColor(Color::Black),
                SetForegroundColor(Color::Grey),
                Clear(ClearType::All),
                MoveTo(0, 0)
            )?;
            let blank_playfield = " ".repeat(PLAYFIELD_WIDTH);
            let blank_terminal_row = " ".repeat(term_width as usize);
            for row in 0..term_height {
                execute!(stdout, MoveTo(0, row))?;
                stdout.write_all(blank_terminal_row.as_bytes())?;
            }
            for row in 0..playfield.height() {
                execute!(stdout, MoveTo(offset_x, offset_y + row as u16))?;
                stdout.write_all(blank_playfield.as_bytes())?;
                execute!(stdout, MoveTo(offset_x, offset_y + row as u16))?;
                stdout.write_all(render_ansi_row(playfield.row(row)).as_bytes())?;
            }
            match playfield.cursor() {
                Some((column, row)) => {
                    execute!(stdout, Show, MoveTo(offset_x + column, offset_y + row))?;
                }
                None => {
                    execute!(stdout, Hide)?;
                }
            }
        } else {
            for row in 0..playfield.height() {
                let line = playfield.plain_line(row);
                stdout.write_all(line.as_bytes())?;
                stdout.write_all(b"\n")?;
            }
        }
        stdout.flush()?;
        Ok(())
    }

    fn read_key(&mut self) -> Result<KeyEvent, Box<dyn std::error::Error>> {
        loop {
            match event::read()? {
                Event::Key(key) => return Ok(key),
                _ => continue,
            }
        }
    }

    fn dump_text_capture(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = io::stdout();
        if stdout.is_terminal() {
            execute!(
                stdout,
                SetBackgroundColor(Color::Black),
                SetForegroundColor(Color::Grey),
                Clear(ClearType::All),
                MoveTo(0, 0),
                Show
            )?;
            stdout.write_all(b"\x1b[0m")?;
        }
        stdout.write_all(text.as_bytes())?;
        if !text.ends_with('\n') {
            stdout.write_all(b"\n")?;
        }
        stdout.flush()?;
        Ok(())
    }

    fn clear_and_restore(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = io::stdout();
        if stdout.is_terminal() {
            execute!(
                stdout,
                SetBackgroundColor(Color::Black),
                SetForegroundColor(Color::Grey),
                Clear(ClearType::All),
                MoveTo(0, 0),
                Show
            )?;
            stdout.write_all(b"\x1b[0m")?;
            stdout.flush()?;
        }
        Ok(())
    }
}

fn render_ansi_row(row: &[crate::screen::Cell]) -> String {
    let mut output = String::new();
    let mut current_style = None;

    for cell in row {
        if current_style != Some(cell.style) {
            output.push_str(&ansi_style(cell.style));
            current_style = Some(cell.style);
        }
        output.push(cell.ch);
    }
    output.push_str("\x1b[0m");
    output
}

fn ansi_style(style: CellStyle) -> String {
    let weight = if style.bold { "1" } else { "0" };
    format!(
        "\x1b[{weight};38;2;{};{};{};48;2;{};{};{}m",
        style.fg.red,
        style.fg.green,
        style.fg.blue,
        style.bg.red,
        style.bg.green,
        style.bg.blue
    )
}
