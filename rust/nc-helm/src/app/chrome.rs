use crate::{CellStyle, PlayfieldBuffer};

pub fn fill_rect(
    buffer: &mut PlayfieldBuffer,
    left: usize,
    top: usize,
    width: usize,
    height: usize,
    style: CellStyle,
) {
    crate::chrome_box::fill_rect(
        buffer.width(),
        buffer.height(),
        left,
        top,
        width,
        height,
        style,
        |row, col, ch, style| buffer.set_cell(row, col, ch, style),
    );
}

pub fn draw_panel(
    buffer: &mut PlayfieldBuffer,
    left: usize,
    top: usize,
    width: usize,
    height: usize,
    border_style: CellStyle,
    title_style: CellStyle,
    fill_style: Option<CellStyle>,
    top_title: Option<&str>,
    bottom_title: Option<&str>,
) {
    if width < 2 || height < 2 {
        return;
    }

    if let Some(style) = fill_style {
        fill_rect(buffer, left, top, width, height, style);
    }

    crate::chrome_box::draw_box_outline(
        buffer.width(),
        buffer.height(),
        left,
        top,
        width,
        height,
        border_style,
        |row, col, ch, style| buffer.set_cell(row, col, ch, style),
    );

    if let Some(title) = top_title {
        draw_top_tag(
            buffer,
            top,
            left + 2,
            width,
            title,
            border_style,
            title_style,
        );
    }
    if let Some(title) = bottom_title {
        draw_bottom_tag(
            buffer,
            top + height - 1,
            left + 2,
            width,
            title,
            border_style,
            title_style,
        );
    }
}

pub fn draw_modal_panel(
    buffer: &mut PlayfieldBuffer,
    left: usize,
    top: usize,
    width: usize,
    height: usize,
    border_style: CellStyle,
    title_style: CellStyle,
    fill_style: Option<CellStyle>,
    top_title: Option<&str>,
    bottom_title: Option<&str>,
) {
    if width < 2 || height < 2 {
        return;
    }

    if let Some(style) = fill_style {
        let pad_left = left.saturating_sub(1);
        let pad_top = top.saturating_sub(1);
        let pad_right = left
            .saturating_add(width)
            .min(buffer.width().saturating_sub(1));
        let pad_bottom = top
            .saturating_add(height)
            .min(buffer.height().saturating_sub(1));
        fill_rect(
            buffer,
            pad_left,
            pad_top,
            pad_right.saturating_sub(pad_left).saturating_add(1),
            pad_bottom.saturating_sub(pad_top).saturating_add(1),
            style,
        );
    }

    draw_panel(
        buffer,
        left,
        top,
        width,
        height,
        border_style,
        title_style,
        fill_style,
        top_title,
        bottom_title,
    );
}

pub fn draw_top_tag(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    available_width: usize,
    label: &str,
    border_style: CellStyle,
    title_style: CellStyle,
) -> usize {
    if row >= buffer.height() || col >= buffer.width() {
        return 0;
    }
    let max_width = buffer.width().saturating_sub(col);
    crate::chrome_tags::draw_tag(
        row,
        col,
        available_width.min(max_width),
        label,
        border_style,
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
    )
}

pub fn draw_top_tag_right(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    left: usize,
    panel_width: usize,
    label: &str,
    border_style: CellStyle,
    title_style: CellStyle,
) -> usize {
    let Some(col) = top_tag_right_col(left, panel_width, label) else {
        return 0;
    };
    draw_top_tag(
        buffer,
        row,
        col,
        left.saturating_add(panel_width).saturating_sub(col),
        label,
        border_style,
        title_style,
    )
}

pub fn top_tag_width(label: &str) -> usize {
    crate::chrome_tags::tag_width(label)
}

pub fn top_tag_right_col(left: usize, panel_width: usize, label: &str) -> Option<usize> {
    crate::chrome_tags::top_tag_right_col(left, panel_width, label)
}

fn draw_bottom_tag(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    available_width: usize,
    label: &str,
    border_style: CellStyle,
    title_style: CellStyle,
) -> usize {
    if row >= buffer.height() || col >= buffer.width() {
        return 0;
    }
    let max_width = buffer.width().saturating_sub(col);
    crate::chrome_tags::draw_tag(
        row,
        col,
        available_width.min(max_width),
        label,
        border_style,
        title_style,
        crate::chrome_tags::BOTTOM_TAG_LEFT,
        crate::chrome_tags::BOTTOM_TAG_RIGHT,
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
    )
}

#[cfg(test)]
mod tests {
    use super::draw_panel;
    use crate::{CellStyle, GameColor, PlayfieldBuffer};

    #[test]
    fn draw_panel_uses_square_corners() {
        let style = CellStyle::new(GameColor::White, GameColor::Black, false);
        let mut buffer = PlayfieldBuffer::new(16, 8, style);

        draw_panel(&mut buffer, 2, 1, 8, 4, style, style, None, None, None);

        assert_eq!(buffer.row(1)[2].ch, '┌');
        assert_eq!(buffer.row(1)[9].ch, '┐');
        assert_eq!(buffer.row(4)[2].ch, '└');
        assert_eq!(buffer.row(4)[9].ch, '┘');
    }
}
