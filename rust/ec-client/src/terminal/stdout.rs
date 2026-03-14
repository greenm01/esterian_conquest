use std::io::{self, IsTerminal, Write};

use crate::terminal::Terminal;
use crossterm::{
    cursor::MoveTo,
    event::{self, Event, KeyEvent},
    execute,
    terminal::{Clear, ClearType},
};

pub struct StdoutTerminal {
    buffer: String,
}

impl StdoutTerminal {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }
}

impl Terminal for StdoutTerminal {
    fn clear(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.buffer.clear();
        Ok(())
    }

    fn write_line(&mut self, line: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.buffer.push_str(line);
        self.buffer.push('\n');
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = io::stdout();
        if stdout.is_terminal() {
            execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
        }
        stdout.write_all(self.buffer.as_bytes())?;
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
