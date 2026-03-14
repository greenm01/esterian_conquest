use std::io::{self, IsTerminal, Write};

use crate::screen::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};
use crate::terminal::Terminal;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyEvent},
    execute,
    style::{Color, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};

pub struct StdoutTerminal {
    lines: Vec<String>,
    cursor: Option<(u16, u16)>,
}

impl StdoutTerminal {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            cursor: None,
        }
    }
}

impl Terminal for StdoutTerminal {
    fn clear(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.lines.clear();
        self.cursor = None;
        Ok(())
    }

    fn write_line(&mut self, line: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.lines.push(line.to_string());
        Ok(())
    }

    fn set_cursor(&mut self, column: u16, row: u16) -> Result<(), Box<dyn std::error::Error>> {
        self.cursor = Some((column, row));
        Ok(())
    }

    fn clear_cursor(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.cursor = None;
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
            for row in 0..PLAYFIELD_HEIGHT {
                execute!(stdout, MoveTo(offset_x, offset_y + row as u16))?;
                stdout.write_all(blank_playfield.as_bytes())?;
                if let Some(line) = self.lines.get(row) {
                    execute!(stdout, MoveTo(offset_x, offset_y + row as u16))?;
                    stdout.write_all(line.as_bytes())?;
                }
            }
            match self.cursor {
                Some((column, row)) => {
                    execute!(stdout, Show, MoveTo(offset_x + column, offset_y + row))?;
                }
                None => {
                    execute!(stdout, Hide)?;
                }
            }
        } else {
            for line in &self.lines {
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
}
