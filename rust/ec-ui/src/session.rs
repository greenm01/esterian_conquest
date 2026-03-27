use std::io::{self, Write};

use crossterm::cursor::Show;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

pub struct TerminalSession {
    raw_mode: bool,
    mouse_capture: bool,
    alternate_screen: bool,
}

impl TerminalSession {
    pub fn enter_picker() -> Result<Self, Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        Ok(Self {
            raw_mode: true,
            mouse_capture: true,
            alternate_screen: true,
        })
    }

    pub fn suspend_for_bridge(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = io::stdout();
        if self.mouse_capture {
            execute!(stdout, DisableMouseCapture)?;
            self.mouse_capture = false;
        }
        if self.raw_mode {
            disable_raw_mode()?;
            self.raw_mode = false;
        }
        stdout.flush()?;
        Ok(())
    }

    pub fn resume_after_bridge(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.raw_mode {
            enable_raw_mode()?;
            self.raw_mode = true;
        }
        if !self.mouse_capture {
            let mut stdout = io::stdout();
            execute!(stdout, EnableMouseCapture)?;
            self.mouse_capture = true;
        }
        Ok(())
    }

    pub fn restore(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = io::stdout();
        if self.raw_mode {
            disable_raw_mode()?;
            self.raw_mode = false;
        }
        if self.mouse_capture {
            execute!(stdout, DisableMouseCapture)?;
            self.mouse_capture = false;
        }
        if self.alternate_screen {
            execute!(stdout, LeaveAlternateScreen, Show)?;
            self.alternate_screen = false;
        }
        stdout.flush()?;
        Ok(())
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn session_module_is_linked() {
        let name = std::any::type_name::<super::TerminalSession>();
        assert!(name.ends_with("TerminalSession"));
    }
}
