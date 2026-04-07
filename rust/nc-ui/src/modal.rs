use crate::buffer::{CellStyle, PlayfieldBuffer};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ModalPlacement {
    #[default]
    Centered,
    Origin { x: u16, y: u16 },
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

pub fn placed_rect(width: u16, height: u16, parent: Rect, placement: ModalPlacement) -> Rect {
    let width = width.min(parent.width);
    let height = height.min(parent.height);
    match placement {
        ModalPlacement::Centered => centered_rect(width, height, parent),
        ModalPlacement::Origin { x, y } => {
            let max_x = parent.x + parent.width.saturating_sub(width);
            let max_y = parent.y + parent.height.saturating_sub(height);
            Rect::new(x.clamp(parent.x, max_x), y.clamp(parent.y, max_y), width, height)
        }
    }
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
        let available_width = rect.width.saturating_sub(4) as usize;
        assert!(
            bordered.chars().count() <= available_width,
            "modal title overruns its border slot: text width {} exceeds allowed width {}",
            bordered.chars().count(),
            available_width
        );
        buffer.write_text(top, left + 2, &bordered, title_style);
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
    draw_modal_frame_in_parent(
        buffer,
        title,
        preferred_width,
        height,
        Rect::new(0, 0, buffer.width() as u16, buffer.height() as u16),
        theme,
    )
}

pub fn draw_modal_frame_in_parent(
    buffer: &mut PlayfieldBuffer,
    title: &str,
    preferred_width: usize,
    height: u16,
    parent: Rect,
    theme: ModalTheme,
) -> Rect {
    draw_modal_frame_in_parent_with_placement(
        buffer,
        title,
        preferred_width,
        height,
        parent,
        ModalPlacement::Centered,
        theme,
    )
}

pub fn draw_modal_frame_in_parent_with_placement(
    buffer: &mut PlayfieldBuffer,
    title: &str,
    preferred_width: usize,
    height: u16,
    parent: Rect,
    placement: ModalPlacement,
    theme: ModalTheme,
) -> Rect {
    let max_width = parent.width.saturating_sub(2).max(1);
    let max_height = parent.height.saturating_sub(2).max(1);
    let popup = placed_rect(
        preferred_width.min(max_width as usize) as u16,
        height.min(max_height),
        parent,
        placement,
    );
    let pad_x = popup.x.saturating_sub(1).max(parent.x);
    let pad_y = popup.y.saturating_sub(1).max(parent.y);
    let popup_right = popup.x + popup.width.saturating_sub(1);
    let popup_bottom = popup.y + popup.height.saturating_sub(1);
    let parent_right = parent.x + parent.width.saturating_sub(1);
    let parent_bottom = parent.y + parent.height.saturating_sub(1);
    let pad_right = popup_right.saturating_add(1).min(parent_right);
    let pad_bottom = popup_bottom.saturating_add(1).min(parent_bottom);
    let pad = Rect::new(
        pad_x,
        pad_y,
        pad_right.saturating_sub(pad_x).saturating_add(1),
        pad_bottom.saturating_sub(pad_y).saturating_add(1),
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
        .max(title.chars().count() + 6)
        .min(buffer.width().saturating_sub(8));
    let height = (lines.len() + 2) as u16;
    let popup = draw_modal_frame(buffer, title, width, height, theme);
    write_modal_lines(buffer, popup, lines, theme.body_style);
    popup
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrappedHelpLines {
    pub lines: Vec<String>,
    pub content_width: usize,
}

pub fn wrap_formatted_help_lines(lines: &[String], max_content_width: usize) -> WrappedHelpLines {
    if lines.is_empty() || max_content_width == 0 {
        return WrappedHelpLines {
            lines: Vec::new(),
            content_width: 0,
        };
    }

    let parsed = lines
        .iter()
        .map(|line| {
            line.split_once(" : ").map(|(command, description)| {
                (command.trim_end().to_string(), description.to_string())
            })
        })
        .collect::<Vec<_>>();
    let natural_width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let target_width = natural_width.min(max_content_width);
    let command_width = parsed
        .iter()
        .filter_map(|row| row.as_ref().map(|(command, _)| command.chars().count()))
        .max()
        .unwrap_or(0);
    let description_width = target_width.saturating_sub(command_width + 3).max(1);
    let mut wrapped = Vec::new();

    for (line, parsed_row) in lines.iter().zip(parsed.iter()) {
        if let Some((command, description)) = parsed_row {
            let segments = wrap_text_to_width(description, description_width);
            if segments.is_empty() {
                wrapped.push(format!("{command:<command_width$} : "));
                continue;
            }
            for (idx, segment) in segments.iter().enumerate() {
                let command_text = if idx == 0 { command.as_str() } else { "" };
                wrapped.push(format!("{command_text:<command_width$} : {segment}"));
            }
        } else {
            wrapped.extend(wrap_text_to_width(line, target_width));
        }
    }

    let content_width = wrapped
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);

    WrappedHelpLines {
        lines: wrapped,
        content_width,
    }
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
        .map(|(command, description)| format!("{command:<command_width$} : {description}"))
        .collect()
}

fn wrap_text_to_width(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return Vec::new();
    }

    let words = text.split_whitespace().collect::<Vec<_>>();
    if words.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();

    for word in words {
        for segment in split_long_word(word, max_width) {
            if current.is_empty() {
                current.push_str(&segment);
                continue;
            }

            let candidate_width = current.chars().count() + 1 + segment.chars().count();
            if candidate_width <= max_width {
                current.push(' ');
                current.push_str(&segment);
            } else {
                lines.push(std::mem::take(&mut current));
                current.push_str(&segment);
            }
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

fn split_long_word(word: &str, max_width: usize) -> Vec<String> {
    if word.chars().count() <= max_width {
        return vec![word.to_string()];
    }

    let mut chunks = Vec::new();
    let mut current = String::new();

    for ch in word.chars() {
        if current.chars().count() == max_width {
            chunks.push(std::mem::take(&mut current));
        }
        current.push(ch);
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::classic;

    #[test]
    fn modal_frame_in_parent_keeps_visible_padding_when_space_allows() {
        let mut buffer = PlayfieldBuffer::new(80, 30, classic::body_style());
        let parent = Rect::new(20, 8, 30, 12);
        let popup = draw_modal_frame_in_parent(
            &mut buffer,
            "TEST",
            18,
            8,
            parent,
            ModalTheme {
                body_style: classic::body_style(),
                pad_style: classic::help_panel_style(),
                chrome_style: classic::table_chrome_style(),
                title_style: classic::table_header_style(),
            },
        );

        assert!(popup.x > parent.x);
        assert!(popup.y > parent.y);
        assert!(popup.x + popup.width < parent.x + parent.width);
        assert!(popup.y + popup.height < parent.y + parent.height);
    }

    #[test]
    fn wrap_formatted_help_lines_wraps_descriptions_and_aligns_continuations() {
        let lines = vec![String::from(
            "Enter : Open the selected planet detail popup from the current map sector",
        )];

        let wrapped = wrap_formatted_help_lines(&lines, 30);

        assert_eq!(wrapped.lines.len(), 4);
        assert_eq!(wrapped.lines[0], "Enter : Open the selected");
        assert!(wrapped.lines[1].starts_with("      : "));
        assert!(wrapped.lines[2].starts_with("      : "));
        assert!(wrapped.lines[3].starts_with("      : "));
        assert_eq!(wrapped.content_width, 28);
    }

    #[test]
    fn wrap_formatted_help_lines_preserves_plain_lines() {
        let lines = vec![String::from("Plain helper prose still wraps when needed")];

        let wrapped = wrap_formatted_help_lines(&lines, 18);

        assert_eq!(wrapped.lines.len(), 3);
        assert!(wrapped.lines.iter().all(|line| line.chars().count() <= 18));
        assert_eq!(
            wrapped.lines.join(" "),
            "Plain helper prose still wraps when needed"
        );
    }
}
