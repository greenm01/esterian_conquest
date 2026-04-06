//! Shared centered modal shell for dashboard overlays.

use crate::app::state::ActiveOverlay;
use nc_ui::modal::{ModalTheme, draw_modal_frame};
use nc_ui::table::{TableFooter, draw_table_footer_in_span, table_footer_scaffold_width};
use nc_ui::{CellStyle, PlayfieldBuffer};

use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayBackdrop {
    None,
    FullBackdrop,
}

#[derive(Debug, Clone, Copy)]
pub struct OverlayFrame {
    pub body_col: usize,
    pub body_row: usize,
    pub body_width: usize,
    pub body_height: usize,
    pub footer_row: usize,
}

pub fn overlay_backdrop(overlay: ActiveOverlay) -> OverlayBackdrop {
    match overlay {
        ActiveOverlay::PlanetList
        | ActiveOverlay::FleetList
        | ActiveOverlay::IntelDatabase
        | ActiveOverlay::Inbox => OverlayBackdrop::FullBackdrop,
        ActiveOverlay::None
        | ActiveOverlay::Diplomacy
        | ActiveOverlay::Settings
        | ActiveOverlay::Help => OverlayBackdrop::None,
    }
}

pub fn draw_full_backdrop(buf: &mut PlayfieldBuffer) {
    buf.fill_rect(0, 0, buf.width(), buf.height(), theme::dim_style());
}

pub fn draw_overlay_frame(
    buf: &mut PlayfieldBuffer,
    title: &str,
    preferred_width: usize,
    preferred_height: usize,
    footer: TableFooter<'_>,
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
    draw_table_footer_in_span(
        buf,
        footer_row,
        popup.x as usize + 2,
        popup.width.saturating_sub(4) as usize,
        footer,
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
    footer: TableFooter<'_>,
) -> OverlayFrame {
    let preferred_width =
        (body_width.max(table_footer_scaffold_width(footer)) + 4).max(title.chars().count() + 6);
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
            TableFooter::CommandBar {
                hotkeys_markup: "? <Q>",
                default: None,
                input: "",
            },
        );

        assert!(frame.footer_row < buffer.height());
        assert!(frame.body_row < frame.footer_row);
        assert_eq!(
            buffer
                .plain_line(frame.footer_row)
                .contains("COMMAND <- ? <Q> ->"),
            true
        );
    }

    #[test]
    fn content_sized_overlay_frame_wraps_requested_body_dimensions() {
        let mut buffer = PlayfieldBuffer::new(120, 40, theme::body_style());
        let frame = draw_overlay_frame_for_body(
            &mut buffer,
            "FLEET LIST",
            72,
            14,
            TableFooter::CommandBar {
                hotkeys_markup: "? O C M T I <Q>",
                default: None,
                input: "",
            },
        );

        assert_eq!(frame.body_width, 72);
        assert_eq!(frame.body_height, 14);
    }

    #[test]
    fn overlay_footer_renders_inside_modal_footer_row() {
        let mut buffer = PlayfieldBuffer::new(40, 20, theme::body_style());
        let frame = draw_overlay_frame(
            &mut buffer,
            "TEST",
            18,
            8,
            TableFooter::CommandBar {
                hotkeys_markup: "? <Q>",
                default: Some("12,03"),
                input: "1",
            },
        );

        assert!(buffer.plain_line(frame.footer_row).contains("COMMAND <- "));
        assert!(
            !buffer
                .plain_line(frame.footer_row.saturating_sub(1))
                .contains("COMMAND <- ")
        );
    }

    #[test]
    fn dismiss_footer_keeps_modal_side_borders_intact() {
        let mut buffer = PlayfieldBuffer::new(40, 20, theme::body_style());
        let frame = draw_overlay_frame(&mut buffer, "HELP", 18, 8, TableFooter::Dismiss);

        let footer_row = frame.footer_row;
        let footer_line = buffer
            .row(footer_row)
            .iter()
            .map(|cell| cell.ch)
            .collect::<String>();
        let left_border = footer_line.find('│').expect("left footer border");
        let right_border = footer_line.rfind('│').expect("right footer border");

        assert!(footer_line.contains("(slap a key)"));
        assert!(left_border < right_border);
    }

    #[test]
    fn overlay_border_and_title_use_themed_background() {
        let mut buffer = PlayfieldBuffer::new(40, 20, theme::body_style());
        draw_overlay_frame(
            &mut buffer,
            "TEST",
            18,
            8,
            TableFooter::CommandBar {
                hotkeys_markup: "? <Q>",
                default: None,
                input: "",
            },
        );

        let expected_bg = theme::body_style().bg;
        let top_row = buffer.row(6).iter().map(|cell| cell.ch).collect::<String>();
        let left = top_row.find('┌').expect("modal left border");

        assert_eq!(buffer.row(6)[left].style.bg, expected_bg);
        assert_eq!(buffer.row(6)[left + 3].style.bg, expected_bg);
    }

    #[test]
    fn dense_table_overlays_use_full_backdrop() {
        assert_eq!(
            overlay_backdrop(ActiveOverlay::PlanetList),
            OverlayBackdrop::FullBackdrop
        );
        assert_eq!(
            overlay_backdrop(ActiveOverlay::FleetList),
            OverlayBackdrop::FullBackdrop
        );
        assert_eq!(
            overlay_backdrop(ActiveOverlay::IntelDatabase),
            OverlayBackdrop::FullBackdrop
        );
        assert_eq!(
            overlay_backdrop(ActiveOverlay::Inbox),
            OverlayBackdrop::FullBackdrop
        );
        assert_eq!(
            overlay_backdrop(ActiveOverlay::Diplomacy),
            OverlayBackdrop::None
        );
        assert_eq!(overlay_backdrop(ActiveOverlay::Help), OverlayBackdrop::None);
    }

    #[test]
    fn full_backdrop_clears_existing_dashboard_text() {
        let mut buffer = PlayfieldBuffer::new(40, 12, theme::body_style());
        buffer.write_text(2, 4, "dashboard clutter", theme::label_style());
        buffer.write_text(9, 8, "more clutter", theme::alert_style());

        draw_full_backdrop(&mut buffer);

        assert_eq!(buffer.plain_line(2), "");
        assert_eq!(buffer.plain_line(9), "");
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
