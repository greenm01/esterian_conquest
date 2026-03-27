use std::io::{self, Write};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute, queue,
    style::{Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};

use crate::buffer::{CellStyle, GameColor, PlayfieldBuffer};
use crate::theme::classic;

pub fn render_to_stdout(buffer: &PlayfieldBuffer) -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = io::stdout();
    let bg = resolve_color(classic::app_background());
    let fg = resolve_color(classic::terminal_foreground());
    execute!(
        stdout,
        SetBackgroundColor(bg),
        SetForegroundColor(fg),
        Clear(ClearType::All),
        MoveTo(0, 0)
    )?;
    for row in 0..buffer.height() {
        execute!(stdout, MoveTo(0, row as u16))?;
        write_styled_row(&mut stdout, buffer.row(row))?;
    }
    match buffer.cursor() {
        Some((column, row)) => {
            execute!(stdout, Show, MoveTo(column, row))?;
        }
        None => {
            execute!(stdout, Hide)?;
        }
    }
    stdout.flush()?;
    Ok(())
}

fn write_styled_row(
    stdout: &mut io::Stdout,
    row: &[crate::buffer::Cell],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut current_style = None;
    let mut run = String::new();
    for cell in row {
        if current_style != Some(cell.style) {
            if !run.is_empty() {
                queue!(stdout, Print(&run))?;
                run.clear();
            }
            apply_style(stdout, cell.style)?;
            current_style = Some(cell.style);
        }
        run.push(cell.ch);
    }
    if !run.is_empty() {
        queue!(stdout, Print(&run))?;
    }
    queue!(
        stdout,
        SetAttribute(Attribute::Reset),
        SetForegroundColor(resolve_color(classic::terminal_foreground())),
        SetBackgroundColor(resolve_color(classic::app_background()))
    )?;
    Ok(())
}

fn apply_style(
    stdout: &mut io::Stdout,
    style: CellStyle,
) -> Result<(), Box<dyn std::error::Error>> {
    queue!(
        stdout,
        SetForegroundColor(resolve_color(style.fg)),
        SetBackgroundColor(resolve_color(style.bg)),
        SetAttribute(if style.bold {
            Attribute::Bold
        } else {
            Attribute::NormalIntensity
        })
    )?;
    Ok(())
}

fn resolve_color(color: GameColor) -> Color {
    match color {
        GameColor::Black => Color::Black,
        GameColor::Red => Color::DarkRed,
        GameColor::Green => Color::DarkGreen,
        GameColor::Yellow => Color::DarkYellow,
        GameColor::Blue => Color::DarkBlue,
        GameColor::Magenta => Color::DarkMagenta,
        GameColor::Cyan => Color::DarkCyan,
        GameColor::White => Color::Grey,
        GameColor::BrightBlack => Color::DarkGrey,
        GameColor::BrightRed => Color::Red,
        GameColor::BrightGreen => Color::Green,
        GameColor::BrightYellow => Color::Yellow,
        GameColor::BrightBlue => Color::Blue,
        GameColor::BrightMagenta => Color::Magenta,
        GameColor::BrightCyan => Color::Cyan,
        GameColor::BrightWhite => Color::White,
        GameColor::Indexed(idx) => Color::AnsiValue(idx),
        GameColor::Rgb(r, g, b) => Color::Rgb { r, g, b },
    }
}
