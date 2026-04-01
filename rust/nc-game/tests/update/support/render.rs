use super::*;

pub(crate) struct CaptureTerminal {
    pub(crate) lines: Vec<String>,
}

impl CaptureTerminal {
    pub(crate) fn new() -> Self {
        Self { lines: Vec::new() }
    }

    pub(crate) fn line(&self, row: usize) -> &str {
        let mapped_row = if row == 19 && self.lines.len() > COMMAND_LINE_ROW {
            COMMAND_LINE_ROW
        } else {
            row
        };
        &self.lines[mapped_row]
    }
}

pub(crate) fn line_containing<'a>(terminal: &'a CaptureTerminal, needle: &str) -> &'a str {
    terminal
        .lines
        .iter()
        .find(|line| line.contains(needle))
        .map(String::as_str)
        .unwrap_or("")
}

impl Terminal for CaptureTerminal {
    fn render(
        &mut self,
        playfield: &nc_game::screen::PlayfieldBuffer,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.lines = (0..playfield.height())
            .map(|row| playfield.plain_line(row))
            .collect();
        Ok(())
    }

    fn dump_text_capture(&mut self, _text: &str) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn read_key(&mut self) -> Result<KeyEvent, Box<dyn std::error::Error>> {
        Err("not used in tests".into())
    }

    fn clear_and_restore(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}
