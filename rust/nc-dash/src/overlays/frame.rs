//! Shared modal shell for dashboard overlays.

use crate::buffer::{CellStyle, PlayfieldBuffer};
use crate::table::{
    TableFooter, draw_table_footer_in_span, table_footer_row_count, table_footer_scaffold_width,
};

use crate::layout::{MapWidgetFrame, widgets::DashboardWidgetFrames};
use crate::modal::{
    ModalPlacement, ModalTheme, Rect, draw_modal_frame_in_parent_with_placement, placed_rect,
};
#[cfg(test)]
use crate::modal::{draw_modal_frame, draw_modal_frame_in_parent};
use crate::theme;

#[derive(Debug, Clone, Copy)]
pub struct OverlayFrame {
    pub body_col: usize,
    pub body_row: usize,
    pub body_width: usize,
    pub body_height: usize,
    pub footer_row: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RelativePopupOrigin {
    pub col_offset: usize,
    pub row_offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverlayAxisSize {
    #[default]
    FitContent,
    #[allow(dead_code)]
    Fixed(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OverlaySizePolicy {
    pub width: OverlayAxisSize,
    pub height: OverlayAxisSize,
}

impl OverlaySizePolicy {
    #[allow(dead_code)]
    pub const fn fixed(width: usize, height: usize) -> Self {
        Self {
            width: OverlayAxisSize::Fixed(width),
            height: OverlayAxisSize::Fixed(height),
        }
    }
}

pub fn overlay_parent_rect(map_frame: MapWidgetFrame) -> Rect {
    Rect::new(
        (map_frame.outer.col + 1) as u16,
        (map_frame.outer.row + 1) as u16,
        map_frame.outer.width.saturating_sub(2) as u16,
        map_frame.outer.height.saturating_sub(2) as u16,
    )
}

pub fn dashboard_overlay_parent_rect(widgets: DashboardWidgetFrames) -> Rect {
    let left = widgets.left_economy.outer.col + 1;
    let top = widgets.header_divider_row + 1;
    let right = widgets
        .right_sector_detail
        .outer
        .last_col()
        .saturating_sub(1);
    let bottom = widgets.footer_divider_row.saturating_sub(1);
    Rect::new(
        left as u16,
        top as u16,
        right.saturating_sub(left).saturating_add(1) as u16,
        bottom.saturating_sub(top).saturating_add(1) as u16,
    )
}

pub fn max_overlay_body_width(map_frame: MapWidgetFrame) -> usize {
    map_frame.outer.width.saturating_sub(8).max(1)
}

pub fn max_overlay_body_height_in_parent(parent: Rect, footer: TableFooter<'_>) -> usize {
    parent
        .height
        .saturating_sub(5 + table_footer_row_count(footer) as u16) as usize
}

#[cfg(test)]
pub fn max_overlay_body_height(map_frame: MapWidgetFrame) -> usize {
    map_frame.outer.height.saturating_sub(8).max(1)
}

pub const fn standard_table_body_height(visible_rows: usize) -> usize {
    visible_rows + 4
}

pub const fn stacked_table_body_height(visible_rows: usize) -> usize {
    visible_rows + 5
}

#[cfg(test)]
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

    overlay_frame_from_popup(buf, popup, footer)
}

#[cfg(test)]
pub fn draw_overlay_frame_in_map(
    buf: &mut PlayfieldBuffer,
    map_frame: MapWidgetFrame,
    title: &str,
    preferred_width: usize,
    preferred_height: usize,
    footer: TableFooter<'_>,
) -> OverlayFrame {
    let popup = draw_modal_frame_in_parent(
        buf,
        title,
        preferred_width,
        preferred_height as u16,
        overlay_parent_rect(map_frame),
        ModalTheme {
            body_style: theme::body_style(),
            pad_style: theme::dim_style(),
            chrome_style: theme::border_style(),
            title_style: theme::title_style(),
        },
    );

    overlay_frame_from_popup(buf, popup, footer)
}

#[allow(dead_code)]
pub fn overlay_popup_rect_in_map(
    map_frame: MapWidgetFrame,
    _title: &str,
    preferred_width: usize,
    preferred_height: usize,
    origin: Option<RelativePopupOrigin>,
) -> Rect {
    let parent = overlay_parent_rect(map_frame);
    let max_width = parent.width.saturating_sub(2).max(1);
    let max_height = parent.height.saturating_sub(2).max(1);
    let placement = origin
        .map(|origin| ModalPlacement::Origin {
            x: parent.x.saturating_add(origin.col_offset as u16),
            y: parent.y.saturating_add(origin.row_offset as u16),
        })
        .unwrap_or(ModalPlacement::Centered);
    placed_rect(
        preferred_width.min(max_width as usize) as u16,
        preferred_height.min(max_height as usize) as u16,
        parent,
        placement,
    )
}

pub fn overlay_popup_rect_in_parent(
    parent: Rect,
    preferred_width: usize,
    preferred_height: usize,
    origin: Option<RelativePopupOrigin>,
) -> Rect {
    let max_width = parent.width.saturating_sub(2).max(1);
    let max_height = parent.height.saturating_sub(2).max(1);
    let placement = origin
        .map(|origin| ModalPlacement::Origin {
            x: parent.x.saturating_add(origin.col_offset as u16),
            y: parent.y.saturating_add(origin.row_offset as u16),
        })
        .unwrap_or(ModalPlacement::Centered);
    placed_rect(
        preferred_width.min(max_width as usize) as u16,
        preferred_height.min(max_height as usize) as u16,
        parent,
        placement,
    )
}

#[allow(dead_code)]
pub fn draw_overlay_frame_in_map_with_origin(
    buf: &mut PlayfieldBuffer,
    map_frame: MapWidgetFrame,
    title: &str,
    preferred_width: usize,
    preferred_height: usize,
    footer: TableFooter<'_>,
    origin: Option<RelativePopupOrigin>,
) -> OverlayFrame {
    let parent = overlay_parent_rect(map_frame);
    let placement = origin
        .map(|origin| ModalPlacement::Origin {
            x: parent.x.saturating_add(origin.col_offset as u16),
            y: parent.y.saturating_add(origin.row_offset as u16),
        })
        .unwrap_or(ModalPlacement::Centered);
    let popup = draw_modal_frame_in_parent_with_placement(
        buf,
        title,
        preferred_width,
        preferred_height as u16,
        parent,
        placement,
        ModalTheme {
            body_style: theme::body_style(),
            pad_style: theme::dim_style(),
            chrome_style: theme::border_style(),
            title_style: theme::title_style(),
        },
    );
    overlay_frame_from_popup(buf, popup, footer)
}

fn overlay_frame_from_popup(
    buf: &mut PlayfieldBuffer,
    popup: Rect,
    footer: TableFooter<'_>,
) -> OverlayFrame {
    let inner_left = popup.x as usize + 1;
    let inner_right = popup.x as usize + popup.width as usize - 2;
    let footer_height = table_footer_row_count(footer);
    let footer_row = popup.y as usize + popup.height as usize - 2;
    let first_footer_row = footer_row.saturating_sub(footer_height.saturating_sub(1));
    let divider_row = first_footer_row.saturating_sub(1);
    let chrome = theme::border_style();

    for col in inner_left..=inner_right {
        buf.set_cell(divider_row, col, '─', chrome);
    }
    buf.set_cell(divider_row, inner_left.saturating_sub(1), '├', chrome);
    buf.set_cell(divider_row, inner_right + 1, '┤', chrome);
    draw_table_footer_in_span(
        buf,
        first_footer_row,
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

#[cfg(test)]
pub fn draw_overlay_frame_for_body_in_map(
    buf: &mut PlayfieldBuffer,
    map_frame: MapWidgetFrame,
    title: &str,
    body_width: usize,
    body_height: usize,
    footer: TableFooter<'_>,
) -> OverlayFrame {
    draw_overlay_frame_for_body_in_map_with_policy(
        buf,
        map_frame,
        title,
        body_width,
        body_height,
        OverlaySizePolicy::default(),
        footer,
    )
}

#[allow(dead_code)]
pub fn draw_overlay_frame_for_body_in_map_with_origin(
    buf: &mut PlayfieldBuffer,
    map_frame: MapWidgetFrame,
    title: &str,
    body_width: usize,
    body_height: usize,
    footer: TableFooter<'_>,
    origin: Option<RelativePopupOrigin>,
) -> OverlayFrame {
    draw_overlay_frame_for_body_in_map_with_policy_and_origin(
        buf,
        map_frame,
        title,
        body_width,
        body_height,
        OverlaySizePolicy::default(),
        footer,
        origin,
    )
}

#[cfg(test)]
pub fn draw_overlay_frame_for_body_in_map_with_policy(
    buf: &mut PlayfieldBuffer,
    map_frame: MapWidgetFrame,
    title: &str,
    natural_body_width: usize,
    natural_body_height: usize,
    size_policy: OverlaySizePolicy,
    footer: TableFooter<'_>,
) -> OverlayFrame {
    draw_overlay_frame_for_body_in_map_with_policy_and_origin(
        buf,
        map_frame,
        title,
        natural_body_width,
        natural_body_height,
        size_policy,
        footer,
        None,
    )
}

pub fn draw_overlay_frame_for_body_in_map_with_policy_and_origin(
    buf: &mut PlayfieldBuffer,
    map_frame: MapWidgetFrame,
    title: &str,
    natural_body_width: usize,
    natural_body_height: usize,
    size_policy: OverlaySizePolicy,
    footer: TableFooter<'_>,
    origin: Option<RelativePopupOrigin>,
) -> OverlayFrame {
    draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        overlay_parent_rect(map_frame),
        title,
        natural_body_width,
        natural_body_height,
        size_policy,
        footer,
        origin,
    )
}

#[cfg(test)]
pub fn overlay_popup_rect_for_body_in_map(
    map_frame: MapWidgetFrame,
    title: &str,
    natural_body_width: usize,
    natural_body_height: usize,
    size_policy: OverlaySizePolicy,
    footer: TableFooter<'_>,
    origin: Option<RelativePopupOrigin>,
) -> Rect {
    overlay_popup_rect_for_body_in_parent(
        overlay_parent_rect(map_frame),
        title,
        natural_body_width,
        natural_body_height,
        size_policy,
        footer,
        origin,
    )
}

pub fn draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
    buf: &mut PlayfieldBuffer,
    parent: Rect,
    title: &str,
    natural_body_width: usize,
    natural_body_height: usize,
    size_policy: OverlaySizePolicy,
    footer: TableFooter<'_>,
    origin: Option<RelativePopupOrigin>,
) -> OverlayFrame {
    let requested_body_width = resolve_requested_axis(natural_body_width, size_policy.width);
    let requested_body_height = resolve_requested_axis(natural_body_height, size_policy.height);
    let footer_width = table_footer_scaffold_width(footer);
    let preferred_width =
        (requested_body_width.max(footer_width) + 4).max(title.chars().count() + 6);
    let preferred_height = requested_body_height + 3 + table_footer_row_count(footer);
    let placement = origin
        .map(|origin| ModalPlacement::Origin {
            x: parent.x.saturating_add(origin.col_offset as u16),
            y: parent.y.saturating_add(origin.row_offset as u16),
        })
        .unwrap_or(ModalPlacement::Centered);
    let popup = draw_modal_frame_in_parent_with_placement(
        buf,
        title,
        preferred_width,
        preferred_height as u16,
        parent,
        placement,
        ModalTheme {
            body_style: theme::body_style(),
            pad_style: theme::dim_style(),
            chrome_style: theme::border_style(),
            title_style: theme::title_style(),
        },
    );
    overlay_frame_from_popup(buf, popup, footer)
}

pub fn overlay_popup_rect_for_body_in_parent(
    parent: Rect,
    title: &str,
    natural_body_width: usize,
    natural_body_height: usize,
    size_policy: OverlaySizePolicy,
    footer: TableFooter<'_>,
    origin: Option<RelativePopupOrigin>,
) -> Rect {
    let requested_body_width = resolve_requested_axis(natural_body_width, size_policy.width);
    let requested_body_height = resolve_requested_axis(natural_body_height, size_policy.height);
    let preferred_width = (requested_body_width.max(table_footer_scaffold_width(footer)) + 4)
        .max(title.chars().count() + 6);
    let preferred_height = requested_body_height + 3 + table_footer_row_count(footer);
    overlay_popup_rect_in_parent(parent, preferred_width, preferred_height, origin)
}

#[cfg(test)]
pub fn draw_overlay_frame_for_body(
    buf: &mut PlayfieldBuffer,
    title: &str,
    body_width: usize,
    body_height: usize,
    footer: TableFooter<'_>,
) -> OverlayFrame {
    draw_overlay_frame_for_body_with_policy(
        buf,
        title,
        body_width,
        body_height,
        OverlaySizePolicy::default(),
        footer,
    )
}

#[cfg(test)]
pub fn draw_overlay_frame_for_body_with_policy(
    buf: &mut PlayfieldBuffer,
    title: &str,
    natural_body_width: usize,
    natural_body_height: usize,
    size_policy: OverlaySizePolicy,
    footer: TableFooter<'_>,
) -> OverlayFrame {
    let requested_body_width = resolve_requested_axis(natural_body_width, size_policy.width);
    let requested_body_height = resolve_requested_axis(natural_body_height, size_policy.height);
    let preferred_width = (requested_body_width.max(table_footer_scaffold_width(footer)) + 4)
        .max(title.chars().count() + 6);
    let preferred_height = requested_body_height + 3 + table_footer_row_count(footer);
    draw_overlay_frame(buf, title, preferred_width, preferred_height, footer)
}

pub fn assert_overlay_body_write_fits(
    frame: OverlayFrame,
    title: &str,
    used_width: usize,
    used_height: usize,
) {
    assert!(
        used_width <= frame.body_width,
        "{title} overlay write width overruns body: need {used_width}, have {}",
        frame.body_width
    );
    assert!(
        used_height <= frame.body_height,
        "{title} overlay write height overruns body: need {used_height}, have {}",
        frame.body_height
    );
}

fn resolve_requested_axis(natural: usize, policy: OverlayAxisSize) -> usize {
    match policy {
        OverlayAxisSize::FitContent => natural,
        OverlayAxisSize::Fixed(size) => size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::MapWidgetFrame;
    use crate::layout::widgets::WidgetRect;

    #[test]
    fn overlay_frame_keeps_footer_inside_modal_box() {
        let mut buffer = PlayfieldBuffer::new(120, 40, theme::body_style());
        let frame = draw_overlay_frame(
            &mut buffer,
            "PLANET LIST",
            80,
            20,
            TableFooter::CommandBar {
                hotkeys_markup: "? <ESC>",
                default: None,
                input: "",
            },
        );

        assert!(frame.footer_row < buffer.height());
        assert!(frame.body_row < frame.footer_row);
        assert_eq!(
            buffer
                .plain_line(frame.footer_row)
                .contains("COMMAND <- ? <ESC> ->"),
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
                hotkeys_markup: "? O C M T I <ESC>",
                default: None,
                input: "",
            },
        );

        assert_eq!(frame.body_width, 72);
        assert_eq!(frame.body_height, 14);
    }

    #[test]
    fn overlay_frame_can_lock_width_and_height() {
        let mut buffer = PlayfieldBuffer::new(120, 40, theme::body_style());
        let frame = draw_overlay_frame_for_body_with_policy(
            &mut buffer,
            "TEST",
            10,
            4,
            OverlaySizePolicy::fixed(22, 9),
            TableFooter::Dismiss,
        );

        assert_eq!(frame.body_width, 22);
        assert_eq!(frame.body_height, 9);
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
                hotkeys_markup: "? <ESC>",
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
    fn stacked_footer_adds_a_second_footer_row_without_shrinking_body() {
        let mut buffer = PlayfieldBuffer::new(60, 24, theme::body_style());
        let footer_rows = [
            TableFooter::CommandInput {
                label: "COMMAND",
                prompt: "Target XX ",
                default: "03",
                input: "03",
            },
            TableFooter::CommandInput {
                label: "COMMAND",
                prompt: "Target YY ",
                default: "11",
                input: "",
            },
        ];
        let frame = draw_overlay_frame_for_body(
            &mut buffer,
            "ORDER FLEET",
            32,
            6,
            TableFooter::Stacked {
                rows: &footer_rows,
                active_row: 1,
            },
        );

        assert_eq!(frame.body_height, 6);
        assert!(
            buffer
                .plain_line(frame.footer_row.saturating_sub(1))
                .contains("COMMAND <- Target XX [03] <ESC> ->")
        );
        assert!(
            buffer
                .plain_line(frame.footer_row)
                .contains("COMMAND <- Target YY [11] <ESC> ->")
        );
        assert!(
            !buffer
                .plain_line(frame.footer_row.saturating_sub(2))
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
                hotkeys_markup: "? <ESC>",
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
    fn map_scoped_overlay_frame_stays_inside_center_widget() {
        let mut buffer = PlayfieldBuffer::new(120, 40, theme::body_style());
        let map_frame = MapWidgetFrame {
            outer: WidgetRect {
                col: 20,
                row: 5,
                width: 60,
                height: 24,
            },
            map_block: WidgetRect {
                col: 21,
                row: 6,
                width: 58,
                height: 22,
            },
            axis_row: 6,
            grid: WidgetRect {
                col: 24,
                row: 7,
                width: 54,
                height: 20,
            },
            bottom_pad_row: 27,
            row_label_cols: 3,
            cell_width: 3,
        };
        let frame = draw_overlay_frame_in_map(
            &mut buffer,
            map_frame,
            "PLANET LIST",
            80,
            20,
            TableFooter::CommandBar {
                hotkeys_markup: "? <ESC>",
                default: None,
                input: "",
            },
        );

        assert!(frame.body_col >= map_frame.outer.col + 2);
        assert!(frame.body_row >= map_frame.outer.row + 2);
        assert!(frame.body_col + frame.body_width <= map_frame.outer.last_col());
        assert!(frame.footer_row < map_frame.outer.last_row());
    }

    #[test]
    fn map_scoped_overlay_origin_clamps_inside_center_widget() {
        let map_frame = MapWidgetFrame {
            outer: WidgetRect {
                col: 20,
                row: 5,
                width: 60,
                height: 24,
            },
            map_block: WidgetRect {
                col: 21,
                row: 6,
                width: 58,
                height: 22,
            },
            axis_row: 6,
            grid: WidgetRect {
                col: 24,
                row: 7,
                width: 54,
                height: 20,
            },
            bottom_pad_row: 27,
            row_label_cols: 3,
            cell_width: 3,
        };
        let popup = overlay_popup_rect_for_body_in_map(
            map_frame,
            "HELP",
            48,
            10,
            OverlaySizePolicy::default(),
            TableFooter::Dismiss,
            Some(RelativePopupOrigin {
                col_offset: 999,
                row_offset: 999,
            }),
        );
        let parent = overlay_parent_rect(map_frame);

        assert!(popup.x >= parent.x);
        assert!(popup.y >= parent.y);
        assert!(popup.x + popup.width <= parent.x + parent.width);
        assert!(popup.y + popup.height <= parent.y + parent.height);
    }

    #[test]
    fn overlay_body_limits_reflect_center_widget_capacity() {
        let map_frame = MapWidgetFrame {
            outer: WidgetRect {
                col: 10,
                row: 4,
                width: 50,
                height: 22,
            },
            map_block: WidgetRect {
                col: 11,
                row: 5,
                width: 48,
                height: 20,
            },
            axis_row: 5,
            grid: WidgetRect {
                col: 14,
                row: 6,
                width: 44,
                height: 18,
            },
            bottom_pad_row: 24,
            row_label_cols: 3,
            cell_width: 3,
        };

        assert_eq!(max_overlay_body_width(map_frame), 42);
        assert_eq!(max_overlay_body_height(map_frame), 14);
    }

    #[test]
    #[should_panic(expected = "overlay write height overruns body")]
    fn overlay_body_write_assert_catches_height_overrun() {
        assert_overlay_body_write_fits(
            OverlayFrame {
                body_col: 0,
                body_row: 0,
                body_width: 10,
                body_height: 4,
                footer_row: 0,
            },
            "TEST",
            8,
            5,
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
