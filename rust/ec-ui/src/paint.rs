use std::io::{self, Write};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute, queue,
    style::{Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};

use crate::buffer::{CellStyle, GameColor, PlayfieldBuffer};
use crate::theme::classic;

pub fn render_to_stdout(buffer: &PlayfieldBuffer) -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = io::stdout();
    let bg = resolve_color(classic::app_background());
    let fg = resolve_color(classic::terminal_foreground());
    let (offset_x, offset_y) = terminal::size()
        .map(|(width, height)| {
            (
                width.saturating_sub(buffer.width() as u16) / 2,
                height.saturating_sub(buffer.height() as u16) / 2,
            )
        })
        .unwrap_or((0, 0));
    execute!(
        stdout,
        Hide,
        SetBackgroundColor(bg),
        SetForegroundColor(fg),
        Clear(ClearType::All),
        MoveTo(0, 0)
    )?;
    for row in 0..buffer.height() {
        execute!(stdout, MoveTo(offset_x, offset_y + row as u16))?;
        write_styled_row(&mut stdout, buffer.row(row))?;
    }
    match buffer.cursor() {
        Some((column, row)) => {
            execute!(stdout, Show, MoveTo(offset_x + column, offset_y + row))?;
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
