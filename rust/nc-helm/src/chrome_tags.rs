pub const TOP_TAG_LEFT: char = '┐';
pub const TOP_TAG_RIGHT: char = '┌';
pub const BOTTOM_TAG_LEFT: char = '┘';
pub const BOTTOM_TAG_RIGHT: char = '└';
pub const CLOSE_TAG_LABEL: &str = "[X]";

pub enum TagDrawOp<'a, S> {
    SetCell {
        row: usize,
        col: usize,
        ch: char,
        style: S,
    },
    WriteText {
        row: usize,
        col: usize,
        text: &'a str,
        style: S,
    },
}

pub fn tag_width(label: &str) -> usize {
    label.chars().count() + 2
}

pub fn min_width_for_top_tags(left_label: &str, right_label: Option<&str>) -> usize {
    let left = tag_width(left_label);
    match right_label {
        Some(right_label) => left + tag_width(right_label) + 4,
        None => left + 4,
    }
}

pub fn top_tag_right_col(left: usize, panel_width: usize, label: &str) -> Option<usize> {
    let width = tag_width(label);
    panel_width
        .checked_sub(width + 2)
        .map(|offset| left + offset)
}

pub fn close_tag_col(left: usize, panel_width: usize) -> Option<usize> {
    top_tag_right_col(left, panel_width, CLOSE_TAG_LABEL)
}

pub fn close_label_col(left: usize, panel_width: usize) -> Option<usize> {
    close_tag_col(left, panel_width).map(|col| col + 1)
}

pub fn draw_tag<S: Copy>(
    row: usize,
    col: usize,
    available_width: usize,
    label: &str,
    border_style: S,
    title_style: S,
    left_notch: char,
    right_notch: char,
    mut draw: impl FnMut(TagDrawOp<'_, S>),
) -> usize {
    if available_width < 5 {
        return 0;
    }

    let max_label_width = available_width.saturating_sub(2);
    if max_label_width == 0 {
        return 0;
    }

    let label = truncate_chars(label, max_label_width);
    let width = tag_width(&label);
    let label_width = label.chars().count();

    draw(TagDrawOp::SetCell {
        row,
        col,
        ch: left_notch,
        style: border_style,
    });
    draw(TagDrawOp::WriteText {
        row,
        col: col + 1,
        text: &label,
        style: title_style,
    });
    draw(TagDrawOp::SetCell {
        row,
        col: col + 1 + label_width,
        ch: right_notch,
        style: border_style,
    });
    width
}

fn truncate_chars(value: &str, width: usize) -> String {
    value.chars().take(width).collect()
}
