use std::io::{self, Write};

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
            execute!(stdout, LeaveAlternateScreen)?;
            self.alternate_screen = false;
        }
        write_terminal_cleanup_sequence(&mut stdout)?;
        stdout.flush()?;
        Ok(())
    }
}

pub fn write_terminal_cleanup_sequence(out: &mut impl Write) -> io::Result<()> {
    // Keep the bridge/picker cleanup deterministic and serializable even when
    // tests run without an initialized Windows console. This matches the ANSI
    // sequences crossterm would emit for mouse disable, cursor show, and reset.
    out.write_all(
        b"\x1b[?1006l\x1b[?1015l\x1b[?1003l\x1b[?1002l\x1b[?1000l\x1b[0 q\x1b[?25h\x1b[0m\x1b[39m\x1b[49m\x1b(B\x1b]110\x07\x1b]111\x07\x1b]112\x07",
    )?;
    Ok(())
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::write_terminal_cleanup_sequence;

    #[test]
    fn session_module_is_linked() {
        let name = std::any::type_name::<super::TerminalSession>();
        assert!(name.ends_with("TerminalSession"));
    }

    #[test]
    fn cleanup_sequence_restores_default_cursor_shape_before_showing_cursor() {
        let mut out = Vec::new();
        write_terminal_cleanup_sequence(&mut out).expect("cleanup sequence should serialize");
        let output = String::from_utf8_lossy(&out);
        assert!(output.contains("\x1b[0 q\x1b[?25h"));
    }
}
