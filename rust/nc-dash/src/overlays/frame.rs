//! Shared centered modal shell for dashboard overlays.

use nc_ui::modal::{ModalTheme, draw_modal_frame};
use nc_ui::{CellStyle, PlayfieldBuffer};

use crate::theme;

#[derive(Debug, Clone, Copy)]
pub struct OverlayFrame {
    pub body_col: usize,
    pub body_row: usize,
    pub body_width: usize,
    pub body_height: usize,
    pub footer_row: usize,
}

pub fn draw_overlay_frame(
    buf: &mut PlayfieldBuffer,
    title: &str,
    preferred_width: usize,
    preferred_height: usize,
    footer: &str,
) -> OverlayFrame {
    let popup = draw_modal_frame(
        buf,
        title,
        preferred_width,
        preferred_height as u16,
        ModalTheme {
            body_style: theme::body_style(),
            pad_style: theme::dim_style(),
            chrome_style: theme::border_style(),
            title_style: theme::title_style(),
        },
    );

    let inner_left = popup.x as usize + 1;
    let inner_right = popup.x as usize + popup.width as usize - 2;
    let footer_row = popup.y as usize + popup.height as usize - 2;
    let divider_row = footer_row.saturating_sub(1);
    let chrome = theme::border_style();

    for col in inner_left..=inner_right {
        buf.set_cell(divider_row, col, '─', chrome);
    }
    buf.set_cell(divider_row, inner_left.saturating_sub(1), '├', chrome);
    buf.set_cell(divider_row, inner_right + 1, '┤', chrome);
    write_strict_span(
        buf,
        footer_row,
        popup.x as usize + 2,
        popup.width.saturating_sub(4) as usize,
        footer,
        theme::footer_style(),
        "overlay footer",
    );

    OverlayFrame {
        body_col: popup.x as usize + 2,
        body_row: popup.y as usize + 1,
        body_width: popup.width.saturating_sub(4) as usize,
        body_height: divider_row.saturating_sub(popup.y as usize + 1),
        footer_row,
    }
}

pub fn draw_overlay_frame_for_body(
    buf: &mut PlayfieldBuffer,
    title: &str,
    body_width: usize,
    body_height: usize,
    footer: &str,
) -> OverlayFrame {
    let preferred_width = (body_width.max(footer.chars().count()) + 4).max(title.chars().count() + 6);
    let preferred_height = body_height + 4;
    draw_overlay_frame(buf, title, preferred_width, preferred_height, footer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_frame_keeps_footer_inside_modal_box() {
        let mut buffer = PlayfieldBuffer::new(120, 40, theme::body_style());
        let frame = draw_overlay_frame(
            &mut buffer,
            "PLANET LIST",
            80,
            20,
            "COMMAND <- ? J K <Q> ->",
        );

        assert!(frame.footer_row < buffer.height());
        assert!(frame.body_row < frame.footer_row);
        assert_eq!(buffer.plain_line(frame.footer_row).contains("COMMAND <- ? J K <Q> ->"), true);
    }

    #[test]
    fn content_sized_overlay_frame_wraps_requested_body_dimensions() {
        let mut buffer = PlayfieldBuffer::new(120, 40, theme::body_style());
        let frame = draw_overlay_frame_for_body(
            &mut buffer,
            "FLEET LIST",
            72,
            14,
            "COMMAND <- ? J K ^U ^D O C M T I <Q> ->",
        );

        assert_eq!(frame.body_width, 72);
        assert_eq!(frame.body_height, 14);
    }

    #[test]
    #[should_panic(expected = "overlay footer overruns its widget span")]
    fn overlay_footer_panics_when_it_overruns_modal_footer_row() {
        let mut buffer = PlayfieldBuffer::new(40, 20, theme::body_style());
        let _ = draw_overlay_frame(
            &mut buffer,
            "TEST",
            18,
            8,
            "COMMAND <- this footer is far too wide for the popup ->",
        );
    }
}

pub fn write_clipped(
    buf: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    width: usize,
    text: &str,
    style: CellStyle,
) {
    if width == 0 {
        return;
    }
    let clipped: String = text.chars().take(width).collect();
    buf.write_text_clipped(row, col, &clipped, style);
}

pub fn write_strict_span(
    buf: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    width: usize,
    text: &str,
    style: CellStyle,
    context: &str,
) {
    let text_width = text.chars().count();
    assert!(
        row < buf.height(),
        "{context} row {row} is outside buffer height {}",
        buf.height()
    );
    assert!(
        col < buf.width(),
        "{context} col {col} is outside buffer width {}",
        buf.width()
    );
    assert!(
        col + width <= buf.width(),
        "{context} span overruns buffer width: end {} exceeds {}",
        col + width,
        buf.width()
    );
    assert!(
        text_width <= width,
        "{context} overruns its widget span: text width {text_width} exceeds allowed width {width}"
    );
    buf.write_text(row, col, text, style);
}

pub fn draw_hline(
    buf: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    width: usize,
    style: CellStyle,
) {
    for offset in 0..width {
        buf.set_cell(row, col + offset, '─', style);
    }
}

pub fn draw_vline(
    buf: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    height: usize,
    style: CellStyle,
) {
    for offset in 0..height {
        buf.set_cell(row + offset, col, '│', style);
    }
}
