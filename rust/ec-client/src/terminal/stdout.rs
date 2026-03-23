use std::io::{self, IsTerminal, Write};

use crate::screen::{AnsiColor, CellStyle, PlayfieldBuffer};
use crate::screen::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};
use crate::terminal::OutputEncoding;
use crate::terminal::Terminal;
use crate::terminal::cp437;
use crate::theme::classic;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyEvent},
    execute, queue,
    style::{Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};

pub struct StdoutTerminal {
    encoding: OutputEncoding,
}

impl StdoutTerminal {
    pub fn new() -> Self {
        Self {
            encoding: OutputEncoding::Utf8,
        }
    }

    pub fn with_encoding(encoding: OutputEncoding) -> Self {
        Self { encoding }
    }
}

impl Terminal for StdoutTerminal {
    fn render(&mut self, playfield: &PlayfieldBuffer) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = io::stdout();
        let bg = color_from_ansi(classic::app_background());
        let fg = color_from_ansi(classic::terminal_foreground());
        if stdout.is_terminal() {
            let (term_width, term_height) = terminal::size()?;
            let offset_x = term_width.saturating_sub(PLAYFIELD_WIDTH as u16) / 2;
            let offset_y = term_height.saturating_sub(PLAYFIELD_HEIGHT as u16) / 2;
            execute!(
                stdout,
                SetBackgroundColor(bg),
                SetForegroundColor(fg),
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
                write_styled_row(&mut stdout, playfield.row(row), self.encoding)?;
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
        let bg = color_from_ansi(classic::app_background());
        let fg = color_from_ansi(classic::terminal_foreground());
        if stdout.is_terminal() {
            execute!(
                stdout,
                SetBackgroundColor(bg),
                SetForegroundColor(fg),
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
        let bg = color_from_ansi(classic::app_background());
        let fg = color_from_ansi(classic::terminal_foreground());
        if stdout.is_terminal() {
            execute!(
                stdout,
                SetBackgroundColor(bg),
                SetForegroundColor(fg),
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

fn write_styled_row(
    stdout: &mut io::Stdout,
    row: &[crate::screen::Cell],
    encoding: OutputEncoding,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut current_style = None;
    let mut run = String::new();
    for cell in row {
        if current_style != Some(cell.style) {
            if !run.is_empty() {
                flush_run(stdout, &run, encoding)?;
                run.clear();
            }
            apply_style(stdout, cell.style)?;
            current_style = Some(cell.style);
        }
        run.push(cell.ch);
    }
    if !run.is_empty() {
        flush_run(stdout, &run, encoding)?;
    }
    queue!(
        stdout,
        SetAttribute(Attribute::Reset),
        SetForegroundColor(color_from_ansi(classic::terminal_foreground())),
        SetBackgroundColor(color_from_ansi(classic::app_background()))
    )?;
    Ok(())
}

/// Write a text run to stdout, encoding it according to the output mode.
///
/// In UTF-8 mode, the run is emitted via crossterm `Print` (standard UTF-8).
/// In CP437 mode, each character is translated to its single-byte CP437
/// equivalent and written as raw bytes, bypassing crossterm's UTF-8 `Print`.
/// The surrounding SGR/CSI escape sequences are pure ASCII and identical in
/// both encodings, so only the content bytes need translation.
fn flush_run(
    stdout: &mut io::Stdout,
    run: &str,
    encoding: OutputEncoding,
) -> Result<(), Box<dyn std::error::Error>> {
    match encoding {
        OutputEncoding::Utf8 => {
            queue!(stdout, Print(run))?;
        }
        OutputEncoding::Cp437 => {
            stdout.write_all(&cp437::str_to_cp437(run))?;
        }
    }
    Ok(())
}

fn apply_style(
    stdout: &mut io::Stdout,
    style: CellStyle,
) -> Result<(), Box<dyn std::error::Error>> {
    queue!(
        stdout,
        SetForegroundColor(color_from_ansi(style.fg)),
        SetBackgroundColor(color_from_ansi(style.bg)),
        SetAttribute(if style.bold {
            Attribute::Bold
        } else {
            Attribute::NormalIntensity
        })
    )?;
    Ok(())
}

fn color_from_ansi(color: AnsiColor) -> Color {
    match color {
        AnsiColor::Black => Color::Black,
        AnsiColor::Red => Color::DarkRed,
        AnsiColor::Green => Color::DarkGreen,
        AnsiColor::Yellow => Color::DarkYellow,
        AnsiColor::Blue => Color::DarkBlue,
        AnsiColor::Magenta => Color::DarkMagenta,
        AnsiColor::Cyan => Color::DarkCyan,
        AnsiColor::White => Color::Grey,
        AnsiColor::BrightBlack => Color::DarkGrey,
        AnsiColor::BrightRed => Color::Red,
        AnsiColor::BrightGreen => Color::Green,
        AnsiColor::BrightYellow => Color::Yellow,
        AnsiColor::BrightBlue => Color::Blue,
        AnsiColor::BrightMagenta => Color::Magenta,
        AnsiColor::BrightCyan => Color::Cyan,
        AnsiColor::BrightWhite => Color::White,
    }
}
