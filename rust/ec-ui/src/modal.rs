use crate::buffer::{CellStyle, PlayfieldBuffer};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

pub fn centered_rect(width: u16, height: u16, parent: Rect) -> Rect {
    let width = width.min(parent.width);
    let height = height.min(parent.height);
    let x = parent.x + parent.width.saturating_sub(width) / 2;
    let y = parent.y + parent.height.saturating_sub(height) / 2;
    Rect::new(x, y, width, height)
}

pub fn modal_content_rect(popup: Rect) -> Rect {
    if popup.width <= 4 || popup.height <= 2 {
        return Rect::new(popup.x.saturating_add(2), popup.y.saturating_add(1), 0, 0);
    }
    Rect::new(popup.x + 2, popup.y + 1, popup.width - 4, popup.height - 2)
}

pub fn draw_box(
    buffer: &mut PlayfieldBuffer,
    rect: Rect,
    title: &str,
    chrome_style: CellStyle,
    title_style: CellStyle,
) {
    if rect.width < 2 || rect.height < 2 {
        return;
    }
    let left = rect.x as usize;
    let top = rect.y as usize;
    let right = left + rect.width as usize - 1;
    let bottom = top + rect.height as usize - 1;
    for x in left + 1..right {
        buffer.set_cell(top, x, '─', chrome_style);
        buffer.set_cell(bottom, x, '─', chrome_style);
    }
    for y in top + 1..bottom {
        buffer.set_cell(y, left, '│', chrome_style);
        buffer.set_cell(y, right, '│', chrome_style);
    }
    buffer.set_cell(top, left, '┌', chrome_style);
    buffer.set_cell(top, right, '┐', chrome_style);
    buffer.set_cell(bottom, left, '└', chrome_style);
    buffer.set_cell(bottom, right, '┘', chrome_style);
    if !title.is_empty() && rect.width > 4 {
        let bordered = format!(" {title} ");
        buffer.write_text_clipped(top, left + 2, &bordered, title_style);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ModalTheme {
    pub body_style: CellStyle,
    pub pad_style: CellStyle,
    pub chrome_style: CellStyle,
    pub title_style: CellStyle,
}

pub fn draw_modal_frame(
    buffer: &mut PlayfieldBuffer,
    title: &str,
    preferred_width: usize,
    height: u16,
    theme: ModalTheme,
) -> Rect {
    let max_width = buffer.width().saturating_sub(8).max(1);
    let popup = centered_rect(
        preferred_width.min(max_width) as u16,
        height,
        Rect::new(0, 0, buffer.width() as u16, buffer.height() as u16),
    );
    let popup = Rect::new(
        popup.x,
        popup.y,
        popup.width.min(buffer.width() as u16 - popup.x),
        popup.height.min(buffer.height() as u16 - popup.y),
    );
    let pad = Rect::new(
        popup.x.saturating_sub(1),
        popup.y.saturating_sub(1),
        (popup.width + 2).min(buffer.width() as u16 - popup.x.saturating_sub(1)),
        (popup.height + 2).min(buffer.height() as u16 - popup.y.saturating_sub(1)),
    );
    buffer.fill_rect(
        pad.y as usize,
        pad.x as usize,
        pad.width as usize,
        pad.height as usize,
        theme.pad_style,
    );
    draw_box(buffer, popup, title, theme.chrome_style, theme.title_style);
    buffer.fill_rect(
        popup.y as usize + 1,
        popup.x as usize + 1,
        popup.width.saturating_sub(2) as usize,
        popup.height.saturating_sub(2) as usize,
        theme.body_style,
    );
    popup
}

pub fn render_modal_box(
    buffer: &mut PlayfieldBuffer,
    title: &str,
    lines: &[String],
    theme: ModalTheme,
) -> Rect {
    let content_width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let width = (content_width + 4)
        .max(title.chars().count() + 4)
        .min(buffer.width().saturating_sub(8));
    let height = (lines.len() + 2) as u16;
    let popup = draw_modal_frame(buffer, title, width, height, theme);
    write_modal_lines(buffer, popup, lines, theme.body_style);
    popup
}

pub fn write_modal_lines(
    buffer: &mut PlayfieldBuffer,
    popup: Rect,
    lines: &[String],
    style: CellStyle,
) -> usize {
    let content = modal_content_rect(popup);
    let max_rows = content.height as usize;
    let max_width = content.width as usize;
    if max_rows == 0 || max_width == 0 {
        return 0;
    }

    let visible_rows = lines.len().min(max_rows);
    for idx in 0..visible_rows {
        let is_last_visible = idx + 1 == max_rows;
        let overflow_hidden = lines.len() > max_rows;
        let line = if is_last_visible && overflow_hidden {
            truncate_with_continuation(&lines[idx], max_width)
        } else {
            clip_to_width(&lines[idx], max_width)
        };
        buffer.write_text_clipped(content.y as usize + idx, content.x as usize, &line, style);
    }
    visible_rows
}

pub fn format_help_rows<'a, I>(rows: I) -> Vec<String>
where
    I: IntoIterator<Item = (&'a str, &'a str)>,
{
    let rows = rows.into_iter().collect::<Vec<_>>();
    let command_width = rows
        .iter()
        .map(|(command, _)| command.chars().count())
        .max()
        .unwrap_or(0);
    rows.into_iter()
        .map(|(command, description)| format!("{command:<command_width$} {description}"))
        .collect()
}

fn clip_to_width(text: &str, max_width: usize) -> String {
    text.chars().take(max_width).collect()
}

fn truncate_with_continuation(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if max_width <= 3 {
        return ".".repeat(max_width);
    }
    let clipped = clip_to_width(text, max_width.saturating_sub(3));
    format!("{clipped}...")
}
