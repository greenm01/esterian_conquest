#![allow(dead_code)]

use crate::dashboard::buffer::{CellStyle, PlayfieldBuffer};

pub const MODAL_CLOSE_BUTTON: &str = crate::chrome_tags::CLOSE_TAG_LABEL;

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
    Origin {
        x: u16,
        y: u16,
    },
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

pub fn modal_min_width_for_title(title: &str) -> usize {
    crate::chrome_tags::min_width_for_top_tags(title, Some(MODAL_CLOSE_BUTTON))
}

pub fn modal_close_button_col(rect: Rect) -> Option<u16> {
    if rect.width < 2 {
        return None;
    }
    crate::chrome_tags::close_label_col(rect.x as usize, rect.width as usize).map(|col| col as u16)
}

pub fn modal_close_button_contains(rect: Rect, col: usize, row: usize) -> bool {
    if row != rect.y as usize {
        return false;
    }
    let Some(start_col) = modal_close_button_col(rect) else {
        return false;
    };
    let end_col = start_col as usize + MODAL_CLOSE_BUTTON.len();
    col >= start_col as usize && col < end_col
}

pub fn modal_box_rect_for_lines(
    parent: Rect,
    title: &str,
    wrapped: &WrappedTextLines,
    max_popup_width: usize,
) -> Rect {
    let width = (wrapped.content_width + 4)
        .max(modal_min_width_for_title(title))
        .min(max_popup_width);
    let height = (wrapped.lines.len() + 2)
        .min(parent.height.saturating_sub(2) as usize)
        .max(2) as u16;
    placed_rect(width as u16, height, parent, ModalPlacement::Centered)
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
            Rect::new(
                x.clamp(parent.x, max_x),
                y.clamp(parent.y, max_y),
                width,
                height,
            )
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
    draw_box_with_close_button(buffer, rect, title, chrome_style, title_style, true);
}

pub fn draw_box_without_close_button(
    buffer: &mut PlayfieldBuffer,
    rect: Rect,
    title: &str,
    chrome_style: CellStyle,
    title_style: CellStyle,
) {
    draw_box_with_close_button(buffer, rect, title, chrome_style, title_style, false);
}

fn draw_box_with_close_button(
    buffer: &mut PlayfieldBuffer,
    rect: Rect,
    title: &str,
    chrome_style: CellStyle,
    title_style: CellStyle,
    show_close_button: bool,
) {
    if rect.width < 2 || rect.height < 2 {
        return;
    }
    let left = rect.x as usize;
    let top = rect.y as usize;
    crate::chrome_box::draw_box_outline(
        buffer.width(),
        buffer.height(),
        left,
        top,
        rect.width as usize,
        rect.height as usize,
        chrome_style,
        |row, col, ch, style| buffer.set_cell(row, col, ch, style),
    );
    if !title.is_empty() && rect.width > 4 && top < buffer.height() && left + 2 < buffer.width() {
        let available_width = if show_close_button {
            let close_tag_width = crate::chrome_tags::tag_width(MODAL_CLOSE_BUTTON);
            rect.width
                .saturating_sub(close_tag_width as u16)
                .saturating_sub(4) as usize
        } else {
            rect.width.saturating_sub(4) as usize
        };
        crate::chrome_tags::draw_tag(
            top,
            left + 2,
            available_width,
            title,
            chrome_style,
            title_style,
            crate::chrome_tags::TOP_TAG_LEFT,
            crate::chrome_tags::TOP_TAG_RIGHT,
            |op| match op {
                crate::chrome_tags::TagDrawOp::SetCell {
                    row,
                    col,
                    ch,
                    style,
                } => buffer.set_cell(row, col, ch, style),
                crate::chrome_tags::TagDrawOp::WriteText {
                    row,
                    col,
                    text,
                    style,
                } => {
                    buffer.write_text(row, col, text, style);
                }
            },
        );
    }
    if show_close_button {
        if let Some(close_col) = crate::chrome_tags::close_tag_col(left, rect.width as usize)
            .filter(|col| {
                top < buffer.height()
                    && *col < buffer.width()
                    && *col + crate::chrome_tags::tag_width(MODAL_CLOSE_BUTTON) <= buffer.width()
            })
        {
            crate::chrome_tags::draw_tag(
                top,
                close_col,
                left.saturating_add(rect.width as usize)
                    .saturating_sub(close_col),
                MODAL_CLOSE_BUTTON,
                chrome_style,
                title_style,
                crate::chrome_tags::TOP_TAG_LEFT,
                crate::chrome_tags::TOP_TAG_RIGHT,
                |op| match op {
                    crate::chrome_tags::TagDrawOp::SetCell {
                        row,
                        col,
                        ch,
                        style,
                    } => buffer.set_cell(row, col, ch, style),
                    crate::chrome_tags::TagDrawOp::WriteText {
                        row,
                        col,
                        text,
                        style,
                    } => {
                        buffer.write_text(row, col, text, style);
                    }
                },
            );
        }
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
    draw_modal_frame_in_parent_with_placement_and_close_button(
        buffer,
        title,
        preferred_width,
        height,
        parent,
        placement,
        theme,
        true,
    )
}

fn draw_modal_frame_in_parent_with_placement_and_close_button(
    buffer: &mut PlayfieldBuffer,
    title: &str,
    preferred_width: usize,
    height: u16,
    parent: Rect,
    placement: ModalPlacement,
    theme: ModalTheme,
    show_close_button: bool,
) -> Rect {
    let max_width = parent.width.saturating_sub(2).max(1);
    let max_height = parent.height.saturating_sub(2).max(1);
    let popup = placed_rect(
        preferred_width.min(max_width as usize) as u16,
        height.min(max_height),
        parent,
        placement,
    );
    // Horizontal pad is 2 cells to compensate for terminal cells being
    // roughly twice as tall as they are wide, so the dim border band
    // reads as visually uniform on all sides.
    let pad_x = popup.x.saturating_sub(2).max(parent.x);
    let pad_y = popup.y.saturating_sub(1).max(parent.y);
    let popup_right = popup.x + popup.width.saturating_sub(1);
    let popup_bottom = popup.y + popup.height.saturating_sub(1);
    let parent_right = parent.x + parent.width.saturating_sub(1);
    let parent_bottom = parent.y + parent.height.saturating_sub(1);
    let pad_right = popup_right.saturating_add(2).min(parent_right);
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
    if show_close_button {
        draw_box(buffer, popup, title, theme.chrome_style, theme.title_style);
    } else {
        draw_box_without_close_button(buffer, popup, title, theme.chrome_style, theme.title_style);
    }
    buffer.fill_rect(
        popup.y as usize + 1,
        popup.x as usize + 1,
        popup.width.saturating_sub(2) as usize,
        popup.height.saturating_sub(2) as usize,
        theme.body_style,
    );
    popup
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrappedTextLines {
    pub lines: Vec<String>,
    pub content_width: usize,
}

pub fn max_content_width(parent: Rect) -> usize {
    parent.width.saturating_sub(6).max(1) as usize
}

pub fn compact_content_width(parent: Rect) -> usize {
    max_content_width(parent).min(30).max(1)
}

pub fn measure_modal_text_lines(lines: &[String], max_content_width: usize) -> WrappedTextLines {
    let lines = wrap_modal_text_lines(lines, max_content_width);
    let content_width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    WrappedTextLines {
        lines,
        content_width,
    }
}

pub fn render_modal_box(
    buffer: &mut PlayfieldBuffer,
    title: &str,
    lines: &[String],
    theme: ModalTheme,
) -> Rect {
    let wrapped = measure_modal_text_lines(lines, buffer.width().saturating_sub(12));
    let max_popup_width = buffer.width().saturating_sub(8);
    let popup = modal_box_rect_for_lines(
        Rect::new(0, 0, buffer.width() as u16, buffer.height() as u16),
        title,
        &wrapped,
        max_popup_width,
    );
    let popup = draw_modal_frame(buffer, title, popup.width as usize, popup.height, theme);
    write_modal_lines(buffer, popup, &wrapped.lines, theme.body_style);
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

pub fn wrap_modal_text_lines(lines: &[String], max_content_width: usize) -> Vec<String> {
    if lines.is_empty() || max_content_width == 0 {
        return Vec::new();
    }

    let mut wrapped = Vec::new();
    for line in lines {
        wrapped.extend(wrap_modal_line(line, max_content_width));
    }
    wrapped
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

fn wrap_modal_line(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return Vec::new();
    }
    if text.is_empty() {
        return vec![String::new()];
    }
    if let Some((label, value)) = text.split_once(" : ") {
        return wrap_labeled_line(label, value, max_width);
    }

    let indent_width = text.chars().take_while(|ch| ch.is_whitespace()).count();
    if indent_width > 0 && indent_width < max_width {
        let indent: String = text.chars().take(indent_width).collect();
        let content = text.chars().skip(indent_width).collect::<String>();
        return wrap_text_to_width(&content, max_width - indent_width)
            .into_iter()
            .map(|segment| format!("{indent}{segment}"))
            .collect();
    }

    wrap_text_to_width(text, max_width)
}

fn wrap_labeled_line(label: &str, value: &str, max_width: usize) -> Vec<String> {
    let prefix = format!("{label} : ");
    let prefix_width = prefix.chars().count();
    if prefix_width >= max_width {
        return split_long_word(&format!("{label} : {value}"), max_width);
    }

    let wrapped_value = wrap_text_to_width(value, max_width - prefix_width);
    if wrapped_value.is_empty() {
        return vec![prefix];
    }

    let continuation = " ".repeat(prefix_width);
    wrapped_value
        .into_iter()
        .enumerate()
        .map(|(idx, segment)| {
            if idx == 0 {
                format!("{prefix}{segment}")
            } else {
                format!("{continuation}{segment}")
            }
        })
        .collect()
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

pub fn clip_to_width(text: &str, max_width: usize) -> String {
    text.chars().take(max_width).collect()
}

pub fn truncate_with_continuation(text: &str, max_width: usize) -> String {
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
    use crate::dashboard::theme::classic;

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

    #[test]
    fn wrap_modal_text_lines_aligns_labeled_continuations() {
        let lines = vec![String::from(
            "Message : This is a deliberately long status message for a narrow dialog",
        )];

        let wrapped = wrap_modal_text_lines(&lines, 22);

        assert_eq!(wrapped[0], "Message : This is a");
        assert!(wrapped[1].starts_with("          "));
        assert!(wrapped.iter().all(|line| line.chars().count() <= 22));
    }

    #[test]
    fn wrap_modal_text_lines_preserves_indented_lines() {
        let lines = vec![String::from("  alpha beta gamma delta")];

        let wrapped = wrap_modal_text_lines(&lines, 10);

        assert_eq!(wrapped[0], "  alpha");
        assert_eq!(wrapped[1], "  beta");
        assert_eq!(wrapped[2], "  gamma");
        assert_eq!(wrapped[3], "  delta");
    }

    #[test]
    fn measure_modal_text_lines_reports_content_width() {
        let wrapped = measure_modal_text_lines(
            &[
                String::from("Label : this is a wrapped row"),
                String::from("Second line"),
            ],
            16,
        );

        assert!(wrapped.lines.len() > 2);
        assert_eq!(
            wrapped.content_width,
            wrapped
                .lines
                .iter()
                .map(|line| line.chars().count())
                .max()
                .unwrap_or(0)
        );
    }

    #[test]
    fn draw_box_renders_close_button_in_top_right_border() {
        let mut buffer = PlayfieldBuffer::new(40, 12, classic::body_style());
        let rect = Rect::new(10, 3, 20, 6);

        draw_box(
            &mut buffer,
            rect,
            "TEST",
            classic::table_chrome_style(),
            classic::table_header_style(),
        );

        let close_col = modal_close_button_col(rect).expect("close button col") as usize;
        let top_row: String = buffer
            .row(rect.y as usize)
            .iter()
            .map(|cell| cell.ch)
            .collect();
        assert!(top_row.contains("┐TEST┌"));
        assert!(top_row.contains("┐[X]┌"));
        assert_eq!(buffer.row(rect.y as usize)[close_col].ch, '[');
        assert_eq!(buffer.row(rect.y as usize)[close_col + 1].ch, 'X');
        assert_eq!(buffer.row(rect.y as usize)[close_col + 2].ch, ']');
        assert_eq!(
            buffer.row(rect.y as usize)[rect.x as usize + rect.width as usize - 1].ch,
            '┐'
        );
    }

    #[test]
    fn draw_box_without_close_button_leaves_top_right_border_clean() {
        let mut buffer = PlayfieldBuffer::new(40, 12, classic::body_style());
        let rect = Rect::new(10, 3, 20, 6);

        draw_box_without_close_button(
            &mut buffer,
            rect,
            "TEST",
            classic::table_chrome_style(),
            classic::table_header_style(),
        );

        let close_col = modal_close_button_col(rect).expect("close button col") as usize;
        assert_eq!(buffer.row(rect.y as usize)[close_col].ch, '─');
        assert_eq!(buffer.row(rect.y as usize)[close_col + 1].ch, '─');
        assert_eq!(buffer.row(rect.y as usize)[close_col + 2].ch, '─');
        assert_eq!(buffer.row(rect.y as usize)[rect.x as usize].ch, '┌');
        assert_eq!(
            buffer.row(rect.y as usize)[rect.x as usize + rect.width as usize - 1].ch,
            '┐'
        );
    }

    #[test]
    fn modal_min_width_for_title_reserves_close_button_space() {
        let width = modal_min_width_for_title("HELLO");
        assert_eq!(
            width,
            crate::chrome_tags::min_width_for_top_tags("HELLO", Some(MODAL_CLOSE_BUTTON))
        );
    }

    #[test]
    fn modal_close_button_hit_test_matches_rendered_cells() {
        let rect = Rect::new(12, 4, 24, 7);
        let close_col = modal_close_button_col(rect).expect("close button col") as usize;

        assert!(modal_close_button_contains(
            rect,
            close_col,
            rect.y as usize
        ));
        assert!(modal_close_button_contains(
            rect,
            close_col + 2,
            rect.y as usize
        ));
        assert!(!modal_close_button_contains(
            rect,
            close_col.saturating_sub(1),
            rect.y as usize
        ));
        assert!(!modal_close_button_contains(
            rect,
            close_col,
            rect.y as usize + 1
        ));
    }
}
